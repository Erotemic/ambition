//! The air of a room for one flying body: where its centre can be, and the
//! routes through it.
//!
//! A [`FlightMap`] is a grid over the room of the places the body's centre can
//! be with the body clear of every block (a hazard by a wider margin) and of
//! each exit that would take it out of the room. Each free place knows how far
//! it is from the nearest place that is not, its clearance. A route
//! ([`FlightMap::route`]) is the cheapest way through the free places, where a
//! place near a wall costs more than one in open air, so a flight keeps to the
//! middle of the air: over the bodies that walk on the floor, and clear of the
//! turns it makes. The route is then cut to the few points a flight goes
//! straight between, and each straight part keeps the clearance its ends have.
//!
//! The map is a pure function of the room, the body's size and its exits, so
//! it is a derived cache, built with the room's [`super::NavGraph`] and kept
//! in it. A search reuses the buffers of a [`FlightSearch`], so a route plans
//! with no allocation once the buffers have grown to the room.
//!
//! NOT MODELLED: a block that moves, water, and the bodies in the air.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use ae::navigation::FlightLaw;
use ae::{BlockKind, Vec2, World};
use ambition_platformer2d_core as ae;

/// The side of a cell of the map (px).
pub const FLIGHT_CELL: f32 = 8.0;
/// The room a flying body keeps from each block, beyond touching (px).
const MARGIN: f32 = 4.0;
/// ... and from a hazard (px).
const HAZARD_MARGIN: f32 = 16.0;
/// The clearance a flight is content with (px): nearer a wall than this, a
/// place costs more, the nearer the more.
const PREFERRED_CLEARANCE: f32 = 96.0;
/// What a place at a wall costs above one in open air, as a part of its
/// length.
const CLEARANCE_WEIGHT: f32 = 3.0;
/// How far from a place the body is in a search looks for a free place to
/// start from or end at (cells): a body that stands on a floor is nearer
/// the floor than its margin.
const SNAP_CELLS: i32 = 12;
/// Chamfer distances of the clearance: one cell across, one cell diagonal.
const ACROSS: u16 = 3;
const DIAGONAL: u16 = 4;

/// The free air of one room for one flying body.
#[derive(Clone, Debug)]
pub struct FlightMap {
    pub law: FlightLaw,
    /// Half the body.
    pub half: Vec2,
    cols: i32,
    rows: i32,
    /// For each cell, its clearance in chamfer units (`ACROSS` to a cell);
    /// `0` is a cell the centre cannot be in.
    clearance: Vec<u16>,
}

/// The buffers of a search. A caller keeps one, and each search reuses it.
#[derive(Clone, Debug, Default)]
pub struct FlightSearch {
    cost: Vec<f32>,
    parent: Vec<u32>,
    /// The search a cell's `cost` and `parent` are from.
    stamp: Vec<u32>,
    search: u32,
    open: BinaryHeap<Reverse<(u32, u32)>>,
    cells: Vec<u32>,
    /// The length of the route's cells up to each one.
    along: Vec<f32>,
}

impl FlightMap {
    /// The air of `world` for a body of half size `half` that flies by `law`,
    /// avoiding `exits`.
    pub fn build(world: &World, exits: &[ae::Aabb], half: Vec2, law: FlightLaw) -> Self {
        let cols = (world.size.x / FLIGHT_CELL).ceil().max(1.0) as i32;
        let rows = (world.size.y / FLIGHT_CELL).ceil().max(1.0) as i32;
        let mut free = vec![true; (cols * rows) as usize];
        let mut close = |min: Vec2, max: Vec2, margin: f32| {
            let lo = min - half - Vec2::splat(margin);
            let hi = max + half + Vec2::splat(margin);
            let first = |lo: f32| ((lo / FLIGHT_CELL - 0.5).floor() as i32 + 1).max(0);
            let last = |hi: f32, n: i32| ((hi / FLIGHT_CELL - 0.5).ceil() as i32 - 1).min(n - 1);
            for row in first(lo.y)..=last(hi.y, rows) {
                for col in first(lo.x)..=last(hi.x, cols) {
                    free[(row * cols + col) as usize] = false;
                }
            }
        };
        for block in &world.blocks {
            let margin = if matches!(block.kind, BlockKind::Hazard) {
                HAZARD_MARGIN
            } else {
                MARGIN
            };
            close(block.aabb.min, block.aabb.max, margin);
        }
        for exit in exits {
            close(exit.min, exit.max, MARGIN);
        }
        // The body stays in the room.
        let inside = |col: i32, row: i32| {
            let center = cell_center(col, row);
            center.x - half.x >= MARGIN
                && center.y - half.y >= MARGIN
                && center.x + half.x <= world.size.x - MARGIN
                && center.y + half.y <= world.size.y - MARGIN
        };
        let mut clearance = vec![0u16; free.len()];
        for row in 0..rows {
            for col in 0..cols {
                let index = (row * cols + col) as usize;
                if free[index] && inside(col, row) {
                    clearance[index] = u16::MAX;
                }
            }
        }
        chamfer(&mut clearance, cols, rows);
        Self {
            law,
            half,
            cols,
            rows,
            clearance,
        }
    }

    /// The clearance of the place `at` (px): how far the body's centre can go
    /// from it in each direction before it is not free. Zero: not free.
    pub fn clearance_at(&self, at: Vec2) -> f32 {
        self.cell_of(at)
            .map_or(0.0, |index| self.clearance_px(index))
    }

    /// The route of a flight from `from` to `to`, both centre points, into
    /// `out`: `from` first, then the points it flies straight between, and the
    /// end last. False, and `out` empty, when no route gets there.
    pub fn route(
        &self,
        from: Vec2,
        to: Vec2,
        search: &mut FlightSearch,
        out: &mut Vec<Vec2>,
    ) -> bool {
        out.clear();
        let (Some(start), Some(goal)) = (self.free_near(from), self.free_near(to)) else {
            return false;
        };
        if !self.search(start, goal, search) {
            return false;
        }
        // The cells of the route, from the start.
        search.cells.clear();
        let mut cell = goal;
        loop {
            search.cells.push(cell);
            if cell == start {
                break;
            }
            cell = search.parent[cell as usize];
        }
        search.cells.reverse();
        search.along.clear();
        let mut length = 0.0;
        for (index, cell) in search.cells.iter().enumerate() {
            if index > 0 {
                length +=
                    (self.center_of(*cell) - self.center_of(search.cells[index - 1])).length();
            }
            search.along.push(length);
        }
        out.push(from);
        // Each straight part goes as far as it is as clear as the cells it
        // stands for: the cut does not take back what the search paid for.
        let mut anchor = 0;
        while anchor + 1 < search.cells.len() {
            let mut reach = anchor + 1;
            while reach + 1 < search.cells.len()
                && self.straight(&search.cells, &search.along, anchor, reach + 1)
            {
                reach += 1;
            }
            out.push(self.center_of(search.cells[reach]));
            anchor = reach;
        }
        // The end itself, when the body can be there.
        let end = if self.clearance_at(to) > 0.0 {
            to
        } else {
            self.center_of(goal)
        };
        if out.len() > 1 {
            *out.last_mut().expect("the route has its end") = end;
        } else {
            out.push(end);
        }
        true
    }

    /// A weighted A* over the free cells, from `start` to `goal`.
    fn search(&self, start: u32, goal: u32, search: &mut FlightSearch) -> bool {
        let cells = self.clearance.len();
        if search.stamp.len() != cells {
            search.cost = vec![0.0; cells];
            search.parent = vec![0; cells];
            search.stamp = vec![0; cells];
            search.search = 0;
        }
        search.search = search.search.wrapping_add(1);
        if search.search == 0 {
            search.stamp.iter_mut().for_each(|stamp| *stamp = 0);
            search.search = 1;
        }
        let this = search.search;
        search.open.clear();
        let goal_at = self.center_of(goal);
        let estimate = |cell: u32| octile(self.center_of(cell), goal_at);
        search.cost[start as usize] = 0.0;
        search.parent[start as usize] = start;
        search.stamp[start as usize] = this;
        search
            .open
            .push(Reverse((estimate(start).to_bits(), start)));
        while let Some(Reverse((priority, cell))) = search.open.pop() {
            if cell == goal {
                return true;
            }
            let here = search.cost[cell as usize];
            // A cheaper way to this cell was found after this entry was put
            // in: the cell was gone through from that one.
            if f32::from_bits(priority) > here + estimate(cell) {
                continue;
            }
            let (col, row) = (cell as i32 % self.cols, cell as i32 / self.cols);
            for (dc, dr) in [
                (-1, 0),
                (1, 0),
                (0, -1),
                (0, 1),
                (-1, -1),
                (1, -1),
                (-1, 1),
                (1, 1),
            ] {
                let (c, r) = (col + dc, row + dr);
                if c < 0 || r < 0 || c >= self.cols || r >= self.rows {
                    continue;
                }
                let next = (r * self.cols + c) as u32;
                if self.clearance[next as usize] == 0 {
                    continue;
                }
                // A diagonal step does not cut the corner of a closed cell.
                if dc != 0
                    && dr != 0
                    && (self.clearance[(row * self.cols + c) as usize] == 0
                        || self.clearance[(r * self.cols + col) as usize] == 0)
                {
                    continue;
                }
                let length = if dc != 0 && dr != 0 {
                    FLIGHT_CELL * std::f32::consts::SQRT_2
                } else {
                    FLIGHT_CELL
                };
                let near = (PREFERRED_CLEARANCE - self.clearance_px(next as usize)).max(0.0)
                    / PREFERRED_CLEARANCE;
                let cost = here + length * (1.0 + CLEARANCE_WEIGHT * near);
                let index = next as usize;
                if search.stamp[index] == this && search.cost[index] <= cost {
                    continue;
                }
                search.stamp[index] = this;
                search.cost[index] = cost;
                search.parent[index] = cell;
                // Costs are positive, so their bits order as they do.
                search
                    .open
                    .push(Reverse(((cost + estimate(next)).to_bits(), next)));
            }
        }
        false
    }

    /// Whether a flight goes straight from `cells[a]` to `cells[b]`: each
    /// place of the straight part is free, and as clear (up to the preferred
    /// clearance, less a cell) as the place as far along the cells between.
    fn straight(&self, cells: &[u32], along: &[f32], a: usize, b: usize) -> bool {
        let (from, to) = (self.center_of(cells[a]), self.center_of(cells[b]));
        let steps = ((to - from).length() / (FLIGHT_CELL * 0.5)).ceil().max(1.0) as usize;
        (0..=steps).all(|step| {
            let part = step as f32 / steps as f32;
            let at = from.lerp(to, part);
            let length = along[a] + part * (along[b] - along[a]);
            let peer = a + along[a..=b].partition_point(|l| *l < length).min(b - a);
            let need = self
                .clearance_px(cells[peer] as usize)
                .min(PREFERRED_CLEARANCE)
                - FLIGHT_CELL;
            self.cell_of(at).is_some_and(|index| {
                let clearance = self.clearance_px(index);
                clearance > 0.0 && clearance >= need
            })
        })
    }

    /// The free cell nearest `at`, within `SNAP_CELLS`.
    fn free_near(&self, at: Vec2) -> Option<u32> {
        let col = (at.x / FLIGHT_CELL).floor() as i32;
        let row = (at.y / FLIGHT_CELL).floor() as i32;
        let mut best: Option<(f32, u32)> = None;
        for radius in 0..=SNAP_CELLS {
            for r in (row - radius)..=(row + radius) {
                for c in (col - radius)..=(col + radius) {
                    let ring = (r - row).abs() == radius || (c - col).abs() == radius;
                    if !ring || c < 0 || r < 0 || c >= self.cols || r >= self.rows {
                        continue;
                    }
                    let index = (r * self.cols + c) as u32;
                    if self.clearance[index as usize] == 0 {
                        continue;
                    }
                    let distance = (cell_center(c, r) - at).length_squared();
                    if best.is_none_or(|(d, _)| distance < d) {
                        best = Some((distance, index));
                    }
                }
            }
            // A ring holds no cell nearer than the next ring's nearest.
            if best.is_some_and(|(d, _)| d.sqrt() <= radius as f32 * FLIGHT_CELL) {
                break;
            }
        }
        best.map(|(_, index)| index)
    }

    fn cell_of(&self, at: Vec2) -> Option<usize> {
        let col = (at.x / FLIGHT_CELL).floor() as i32;
        let row = (at.y / FLIGHT_CELL).floor() as i32;
        (col >= 0 && row >= 0 && col < self.cols && row < self.rows)
            .then(|| (row * self.cols + col) as usize)
    }

    fn center_of(&self, cell: u32) -> Vec2 {
        cell_center(cell as i32 % self.cols, cell as i32 / self.cols)
    }

    fn clearance_px(&self, index: usize) -> f32 {
        self.clearance[index] as f32 / ACROSS as f32 * FLIGHT_CELL
    }
}

fn cell_center(col: i32, row: i32) -> Vec2 {
    Vec2::new(
        (col as f32 + 0.5) * FLIGHT_CELL,
        (row as f32 + 0.5) * FLIGHT_CELL,
    )
}

fn octile(a: Vec2, b: Vec2) -> f32 {
    let d = (a - b).abs();
    d.max_element() + (std::f32::consts::SQRT_2 - 1.0) * d.min_element()
}

/// The two-pass chamfer distance of each free cell (`u16::MAX`) to the nearest
/// closed one (`0`), in place: a free cell next to a closed one is `ACROSS`.
fn chamfer(field: &mut [u16], cols: i32, rows: i32) {
    let at = |c: i32, r: i32| (r * cols + c) as usize;
    let relax = |field: &mut [u16], here: usize, c: i32, r: i32, step: u16| {
        // Past the edge of the map is closed.
        let there = if c < 0 || r < 0 || c >= cols || r >= rows {
            0
        } else {
            field[at(c, r)]
        };
        field[here] = field[here].min(there.saturating_add(step));
    };
    for r in 0..rows {
        for c in 0..cols {
            let here = at(c, r);
            if field[here] == 0 {
                continue;
            }
            relax(field, here, c - 1, r, ACROSS);
            relax(field, here, c, r - 1, ACROSS);
            relax(field, here, c - 1, r - 1, DIAGONAL);
            relax(field, here, c + 1, r - 1, DIAGONAL);
        }
    }
    for r in (0..rows).rev() {
        for c in (0..cols).rev() {
            let here = at(c, r);
            if field[here] == 0 {
                continue;
            }
            relax(field, here, c + 1, r, ACROSS);
            relax(field, here, c, r + 1, ACROSS);
            relax(field, here, c + 1, r + 1, DIAGONAL);
            relax(field, here, c - 1, r + 1, DIAGONAL);
        }
    }
}

#[cfg(test)]
mod tests;
