//! The deterministic sand grid — FS2/FS3's substrate, sand only.
//!
//! `falling-sand.md` §1 demands the falling-sand room be a **bounded,
//! deterministic cellular automaton with an explicit conservation law**, driven
//! one solver step per simulation tick. `bevy_falling_sand` cannot honor that
//! contract structurally (§4 of the plan doc records the evidence: private
//! movement systems pinned to `PostUpdate`, a step signal visible for two
//! frames, parallel chunk iteration + per-particle RNG), so SAND — the material
//! whose settled state becomes world geometry — runs on this grid instead.
//! Water and oil stay on the external crate until their own slice.
//!
//! # The two owners and the law between them
//!
//! Matter lives in exactly one of two places (§1's single-owner rule, extended
//! by FS3's transfer):
//!
//! - **Loose** sand: a [`SandCell::Sand`] cell in the grid, moved by [`SandGrid::step`].
//! - **Settled** sand: mass recorded in the [`SettledSandLedger`], which owns
//!   the persistent collision contribution. Its grid cell becomes
//!   [`SandCell::Settled`] — geometry from the CA's point of view, exactly like
//!   a wall, so later sand piles on top of it. The cell is no longer matter;
//!   the ledger's count is.
//!
//! [`SandGrid::settle_into`] is the ONLY door between the owners, and it is
//! atomic per cell: flip the cell to `Settled`, decrement `loose`, increment
//! the ledger — one function, same tick, no state in which the grain is in
//! both owners or neither. The conservation law is therefore an equality over
//! counters, checked by [`SandGrid::conserved_with`] and asserted every tick
//! by the driving system:
//!
//! ```text
//! loose(grid) + total(ledger) == emitted
//! ```
//!
//! # Determinism (ADR 0023)
//!
//! No RNG, no entity iteration, no hash maps. The step scans rows bottom-up
//! (a falling column moves as a unit); within a row the scan direction
//! alternates by `(tick + row)` parity and the diagonal preference by
//! `(tick ^ x ^ y)` parity, so streams don't drift sideways yet every choice
//! is a pure function of (state, tick). The ledger is a `BTreeMap`. Two grids
//! fed the same emissions are equal cell-for-cell — pinned by a test.
//!
//! # Finite settling
//!
//! Every move strictly decreases a grain's height, which is bounded by the
//! floor walls, so a finite emission reaches a fixed point. At the fixed point
//! every loose grain has all three lower neighbors blocked; `settle_into`
//! then transfers pile interiors whose lower neighbors are already static
//! (walls or settled cells) — support grows upward from the floor, and one
//! pass per tick converts the whole resting pile bottom-up. End state: zero
//! loose cells, ledger total == emitted. Pinned by the fixed-point test.
//!
//! Gravity is +y (this codebase's world convention: y grows downward). The
//! symmetry-room C4 generalization (gravity from `GravityCtx`) is future work;
//! this room authors normal gravity.

use std::collections::BTreeMap;

use ambition_engine_core as ae;

/// Cells per tile edge — must match the room's LDtk tile size and the tile
/// keys the liquid projection uses ([`super::TILE_SIZE`]).
const TILE_CELLS: i32 = 16;

/// How many cells a loose grain may fall straight down in one tick. Matches
/// the old `Speed::new(3, 4)` feel: a visually continuous stream at 60 Hz.
const FALL_CELLS_PER_TICK: i32 = 3;

/// Minimum settled cells before a tile contributes a collision block: 4 full
/// rows (4 px of depth). Below this the sand is visible but too thin to stand
/// on. BLIND feel constant — chosen, not playtested.
const SETTLED_BLOCK_MIN_CELLS: u32 = 64;

/// One cell of the sand world.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SandCell {
    #[default]
    Empty,
    /// Static geometry seeded from the room (side walls, floor strips, lips).
    /// Never matter; never counted.
    Wall,
    /// A loose grain. Matter, owned by the grid.
    Sand,
    /// Ground that used to be a grain. Geometry; the matter now lives in the
    /// [`SettledSandLedger`]. Kept distinct from [`SandCell::Wall`] so the
    /// visual can draw it as sand and a later slice can re-fluidize it.
    Settled,
}

impl SandCell {
    /// Static from the CA's point of view: nothing here will ever move.
    fn is_static(self) -> bool {
        matches!(self, SandCell::Wall | SandCell::Settled)
    }
}

/// The settled owner (FS3): per-tile mass of transferred sand, plus the total.
///
/// Persistent for the life of the room visit — the per-frame overlay is
/// rebuilt from it, so collision survives every rebuild without the grid
/// re-proving density each frame (the transient-projection flicker defect).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SettledSandLedger {
    /// Tile `(x, y)` (world-pixel `div` 16, same keys as the liquid
    /// projection) → settled cell count. `BTreeMap` for deterministic
    /// iteration (ADR 0023).
    tiles: BTreeMap<(i32, i32), u32>,
    total: u64,
}

impl SettledSandLedger {
    /// Total settled mass, in cells — the right-hand side of the conservation
    /// law's settled column.
    pub fn total(&self) -> u64 {
        self.total
    }

    pub fn is_empty(&self) -> bool {
        self.total == 0
    }

    fn add_cell(&mut self, tile: (i32, i32)) {
        *self.tiles.entry(tile).or_default() += 1;
        self.total += 1;
    }

    /// Tiles dense enough to own collision, in deterministic (sorted) order.
    pub fn solid_tiles(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.tiles
            .iter()
            .filter(|(_, count)| **count >= SETTLED_BLOCK_MIN_CELLS)
            .map(|(tile, _)| *tile)
    }

    /// The persistent collision contribution: one one-way block per dense
    /// tile, **bottom-aligned and proportional to fill** — a quarter-full tile
    /// yields a 4-px platform at the tile's bottom, so the standable surface
    /// tracks the sand's actual height instead of floating at the tile top.
    pub fn blocks(&self) -> impl Iterator<Item = ae::Block> + '_ {
        self.tiles.iter().filter_map(|(&(tile_x, tile_y), &count)| {
            if count < SETTLED_BLOCK_MIN_CELLS {
                return None;
            }
            let height = (count as i32 / TILE_CELLS).min(TILE_CELLS);
            let tile_bottom = (tile_y + 1) * TILE_CELLS;
            Some(ae::Block::one_way(
                format!("falling_sand:settled:{tile_x}:{tile_y}"),
                ae::Vec2::new((tile_x * TILE_CELLS) as f32, (tile_bottom - height) as f32),
                ae::Vec2::new(TILE_CELLS as f32, height as f32),
            ))
        })
    }
}

/// The loose owner: a bounded, deterministic sand CA over world-pixel cells.
///
/// Coordinates are the room's world pixels directly — `(0, 0)` top-left,
/// x right, y **down** (gravity) — so tile keys and block AABBs need no
/// conversion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SandGrid {
    width: i32,
    height: i32,
    cells: Vec<SandCell>,
    /// Loose-sand count per row: rows holding nothing skip in O(1), which is
    /// what keeps the full-grid scan cheap while the room is mostly empty.
    row_loose: Vec<u32>,
    tick: u64,
    emitted: u64,
    loose: u64,
}

impl SandGrid {
    pub fn new(width: i32, height: i32) -> Self {
        assert!(width > 0 && height > 0, "a sand grid has area");
        Self {
            width,
            height,
            cells: vec![SandCell::Empty; (width as usize) * (height as usize)],
            row_loose: vec![0; height as usize],
            tick: 0,
            emitted: 0,
            loose: 0,
        }
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    /// Loose grains currently in the grid — the left column of the
    /// conservation law.
    pub fn loose(&self) -> u64 {
        self.loose
    }

    /// Every grain that ever entered through [`SandGrid::emit_sand`].
    pub fn emitted(&self) -> u64 {
        self.emitted
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// The conservation law, as a checkable fact: no grain has been created,
    /// lost, or double-owned since the grid was built.
    pub fn conserved_with(&self, ledger: &SettledSandLedger) -> bool {
        self.loose + ledger.total() == self.emitted
    }

    fn idx(&self, x: i32, y: i32) -> usize {
        (y as usize) * (self.width as usize) + (x as usize)
    }

    /// Out-of-bounds reads as [`SandCell::Wall`]: the world edge is static
    /// support, so nothing tunnels out and edge grains can settle.
    pub fn get(&self, x: i32, y: i32) -> SandCell {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return SandCell::Wall;
        }
        self.cells[self.idx(x, y)]
    }

    /// Seed static geometry. Overwrites anything, including sand — callers
    /// seed before emission, so in practice it only claims empty cells.
    pub fn set_wall(&mut self, x: i32, y: i32) {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return;
        }
        let i = self.idx(x, y);
        if self.cells[i] == SandCell::Sand {
            // A wall may never silently delete matter.
            self.loose -= 1;
            self.row_loose[y as usize] -= 1;
            self.emitted -= 1;
        }
        self.cells[i] = SandCell::Wall;
    }

    /// Fill a rectangle with wall — the seeding shape the room systems use.
    pub fn fill_wall_rect(&mut self, x: i32, y: i32, w: i32, h: i32) {
        for dy in 0..h.max(0) {
            for dx in 0..w.max(0) {
                self.set_wall(x + dx, y + dy);
            }
        }
    }

    /// One grain enters the world, if the mouth cell is free. Returns whether
    /// it did — the emitter's budget accounting reads the answer. Never
    /// overwrites: destroying an existing grain would break conservation.
    pub fn emit_sand(&mut self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return false;
        }
        let i = self.idx(x, y);
        if self.cells[i] != SandCell::Empty {
            return false;
        }
        self.cells[i] = SandCell::Sand;
        self.row_loose[y as usize] += 1;
        self.loose += 1;
        self.emitted += 1;
        true
    }

    fn move_grain(&mut self, from: (i32, i32), to: (i32, i32)) {
        let fi = self.idx(from.0, from.1);
        let ti = self.idx(to.0, to.1);
        self.cells[fi] = SandCell::Empty;
        self.cells[ti] = SandCell::Sand;
        self.row_loose[from.1 as usize] -= 1;
        self.row_loose[to.1 as usize] += 1;
    }

    /// **One solver step.** Exactly one per simulation tick, called by the
    /// sim-schedule system — never by a render-frame schedule. Returns how
    /// many grains moved, so a caller can observe quiescence.
    ///
    /// Rows scan bottom-up (largest y first): a cell freed by the grain below
    /// is available to the grain above within the same tick, so a free-falling
    /// column translates as a unit. A grain falls up to [`FALL_CELLS_PER_TICK`]
    /// cells; only when it cannot fall at all does it try one diagonal slide.
    pub fn step(&mut self) -> u64 {
        let mut moved = 0u64;
        let tick = self.tick;
        for y in (0..self.height).rev() {
            if self.row_loose[y as usize] == 0 {
                continue;
            }
            // Alternate scan direction per (tick, row) so lateral resolution
            // order doesn't bias every pile toward one side.
            let leftward = (tick + y as u64) % 2 == 0;
            let mut x = if leftward { self.width - 1 } else { 0 };
            let dx_scan = if leftward { -1 } else { 1 };
            while x >= 0 && x < self.width {
                if self.cells[self.idx(x, y)] == SandCell::Sand {
                    if let Some(to) = self.fall_target(x, y, tick) {
                        self.move_grain((x, y), to);
                        moved += 1;
                    }
                }
                x += dx_scan;
            }
        }
        self.tick += 1;
        moved
    }

    /// Where a grain at `(x, y)` moves this tick, if anywhere.
    fn fall_target(&self, x: i32, y: i32, tick: u64) -> Option<(i32, i32)> {
        // Straight fall, up to FALL_CELLS_PER_TICK while the way is clear.
        let mut fell_to = y;
        while fell_to - y < FALL_CELLS_PER_TICK && self.get(x, fell_to + 1) == SandCell::Empty {
            fell_to += 1;
        }
        if fell_to > y {
            return Some((x, fell_to));
        }
        // Blocked below: try one diagonal slide, preference alternating by a
        // parity that mixes position and tick — deterministic, unbiased.
        let first = if (tick ^ x as u64 ^ y as u64) % 2 == 0 {
            -1
        } else {
            1
        };
        for side in [first, -first] {
            if self.get(x + side, y + 1) == SandCell::Empty {
                return Some((x + side, y + 1));
            }
        }
        None
    }

    /// **FS3's atomic ownership transfer.** A grain whose three lower
    /// neighbors (below, below-left, below-right) are all static can never
    /// move again by the CA's own rules — so it stops being a grain: the cell
    /// becomes [`SandCell::Settled`] geometry and its mass moves into the
    /// ledger, both in this call, conserving the total.
    ///
    /// Scanning bottom-up lets support propagate through the whole resting
    /// pile in ONE pass: a cell settled earlier in the pass is static support
    /// for the row above it. Returns how many cells transferred.
    pub fn settle_into(&mut self, ledger: &mut SettledSandLedger) -> u64 {
        let mut transferred = 0u64;
        for y in (0..self.height).rev() {
            if self.row_loose[y as usize] == 0 {
                continue;
            }
            for x in 0..self.width {
                if self.cells[self.idx(x, y)] != SandCell::Sand {
                    continue;
                }
                let supported = self.get(x, y + 1).is_static()
                    && self.get(x - 1, y + 1).is_static()
                    && self.get(x + 1, y + 1).is_static();
                if !supported {
                    continue;
                }
                let i = self.idx(x, y);
                self.cells[i] = SandCell::Settled;
                self.row_loose[y as usize] -= 1;
                self.loose -= 1;
                ledger.add_cell((x.div_euclid(TILE_CELLS), y.div_euclid(TILE_CELLS)));
                transferred += 1;
            }
        }
        transferred
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small box: floor across the bottom row, walls up both sides.
    fn walled_box(w: i32, h: i32) -> SandGrid {
        let mut grid = SandGrid::new(w, h);
        grid.fill_wall_rect(0, h - 1, w, 1);
        grid.fill_wall_rect(0, 0, 1, h);
        grid.fill_wall_rect(w - 1, 0, 1, h);
        grid
    }

    /// Step + settle until nothing changes; return ticks taken. Panics past
    /// the budget — that IS the finite-settling assertion.
    fn run_to_fixed_point(grid: &mut SandGrid, ledger: &mut SettledSandLedger, budget: u32) -> u32 {
        for tick in 0..budget {
            let moved = grid.step();
            let transferred = grid.settle_into(ledger);
            assert!(
                grid.conserved_with(ledger),
                "conservation broke at tick {tick}: loose={} settled={} emitted={}",
                grid.loose(),
                ledger.total(),
                grid.emitted()
            );
            if moved == 0 && transferred == 0 {
                return tick;
            }
        }
        panic!("no fixed point within {budget} ticks — a grain is falling forever");
    }

    #[test]
    fn a_single_grain_falls_lands_settles_and_transfers() {
        let mut grid = walled_box(9, 12);
        let mut ledger = SettledSandLedger::default();
        assert!(grid.emit_sand(4, 1));

        run_to_fixed_point(&mut grid, &mut ledger, 32);

        assert_eq!(grid.loose(), 0, "the grain is no longer loose");
        assert_eq!(ledger.total(), 1, "…because the ledger owns it now");
        // It rests ON the floor: bottom row is wall (y=11), so the grain
        // settled at y=10.
        assert_eq!(grid.get(4, 10), SandCell::Settled);
    }

    /// The conservation law, under sustained emission and burial. Checked
    /// EVERY tick by the helper, not just at the end.
    #[test]
    fn conservation_holds_every_tick_under_emission() {
        let mut grid = walled_box(33, 40);
        let mut ledger = SettledSandLedger::default();
        for _ in 0..30 {
            for x in 14..=18 {
                grid.emit_sand(x, 1);
            }
            grid.step();
            grid.settle_into(&mut ledger);
            assert!(grid.conserved_with(&ledger));
        }
        let poured = grid.emitted();
        assert!(poured > 100, "the spout actually poured (got {poured})");

        run_to_fixed_point(&mut grid, &mut ledger, 400);
        assert_eq!(
            ledger.total(),
            poured,
            "at the fixed point EVERY grain has settled: loose={}",
            grid.loose()
        );
    }

    /// §1's settle guarantee: any finite input reaches a fixed point, and the
    /// fixed point is genuinely fixed — more ticks change nothing.
    #[test]
    fn a_finite_pour_reaches_a_true_fixed_point() {
        let mut grid = walled_box(21, 30);
        let mut ledger = SettledSandLedger::default();
        for burst in 0..10 {
            for x in 8..=12 {
                grid.emit_sand(x, 1 + (burst % 3));
            }
            grid.step();
            grid.settle_into(&mut ledger);
        }
        run_to_fixed_point(&mut grid, &mut ledger, 300);

        // Genuinely fixed: a grain move is the only mutation `step` performs
        // and a transfer the only one `settle_into` performs, so ten more
        // ticks returning zero of each is "nothing drifts". (The grids
        // themselves are NOT compared — `tick` advances at quiescence, and it
        // is real state: scan parity reads it.)
        let frozen_ledger = ledger.clone();
        let frozen_loose = grid.loose();
        for _ in 0..10 {
            assert_eq!(grid.step(), 0, "a settled world moves nothing");
            assert_eq!(grid.settle_into(&mut ledger), 0);
        }
        assert_eq!(grid.loose(), frozen_loose);
        assert_eq!(ledger, frozen_ledger);
    }

    /// ADR 0023: the CA is a pure function of (state, tick). Two grids fed
    /// the same emission schedule agree cell-for-cell, every tick.
    #[test]
    fn two_identical_runs_produce_identical_worlds() {
        let mut runs: Vec<(SandGrid, SettledSandLedger)> = (0..2)
            .map(|_| (walled_box(25, 25), SettledSandLedger::default()))
            .collect();
        for tick in 0..80 {
            for (grid, ledger) in runs.iter_mut() {
                if tick < 20 {
                    for x in 10..=14 {
                        grid.emit_sand(x, 1);
                    }
                }
                grid.step();
                grid.settle_into(ledger);
            }
        }
        let (a, la) = &runs[0];
        let (b, lb) = &runs[1];
        assert_eq!(a, b, "the same inputs produced different sand worlds");
        assert_eq!(la, lb);
    }

    /// FS3's atomicity, at the single-cell level: the transfer flips the cell,
    /// debits `loose`, and credits the ledger in one call — never a state
    /// where the grain is in both owners or neither.
    #[test]
    fn a_transfer_moves_matter_between_owners_atomically() {
        let mut grid = walled_box(9, 6);
        let mut ledger = SettledSandLedger::default();
        // Resting directly on the floor row, fully supported.
        assert!(grid.emit_sand(4, 4));
        assert_eq!(grid.loose(), 1);

        let transferred = grid.settle_into(&mut ledger);
        assert_eq!(transferred, 1);
        assert_eq!(
            grid.get(4, 4),
            SandCell::Settled,
            "the cell is geometry now"
        );
        assert_eq!(grid.loose(), 0, "…and no longer loose matter");
        assert_eq!(ledger.total(), 1, "…because the ledger owns it");
        assert!(grid.conserved_with(&ledger));
    }

    /// A grain over a hole may not settle: its mass must stay loose until it
    /// actually rests. (The negative case — the guard that `settle_into`
    /// can't just fossilize everything.)
    #[test]
    fn an_unsupported_grain_stays_loose() {
        let mut grid = walled_box(9, 12);
        let mut ledger = SettledSandLedger::default();
        assert!(grid.emit_sand(4, 2));
        // No step: it hangs mid-air. Settling must refuse it.
        assert_eq!(grid.settle_into(&mut ledger), 0);
        assert_eq!(grid.loose(), 1);
        assert!(ledger.is_empty());
    }

    /// Emission never overwrites: a blocked mouth refuses the grain rather
    /// than silently destroying the one already there.
    #[test]
    fn a_blocked_mouth_refuses_rather_than_destroys() {
        let mut grid = walled_box(9, 6);
        assert!(grid.emit_sand(4, 2));
        assert!(!grid.emit_sand(4, 2), "cell already occupied");
        assert_eq!(grid.emitted(), 1, "the refusal was not counted as matter");
    }

    /// The ledger's collision contribution: a dense tile yields a one-way
    /// block bottom-aligned and proportional to fill; a thin tile yields none.
    #[test]
    fn settled_tiles_project_bottom_aligned_proportional_blocks() {
        let mut ledger = SettledSandLedger::default();
        // Fill tile (2, 3) with 8 full rows (128 cells) and tile (5, 5) with
        // a sub-threshold dusting.
        for _ in 0..128 {
            ledger.add_cell((2, 3));
        }
        for _ in 0..(SETTLED_BLOCK_MIN_CELLS - 1) {
            ledger.add_cell((5, 5));
        }
        let blocks: Vec<ae::Block> = ledger.blocks().collect();
        assert_eq!(blocks.len(), 1, "only the dense tile stands");
        let block = &blocks[0];
        // 128 cells / 16 per row = 8 px tall, sitting on the tile's bottom
        // edge (tile y=3 spans world y 48..64, so the block spans 56..64).
        assert_eq!(block.aabb.min, ae::Vec2::new(32.0, 56.0));
        assert_eq!(block.aabb.max, ae::Vec2::new(48.0, 64.0));
    }

    /// A pile against the box wall uses the out-of-bounds-is-wall rule: edge
    /// grains still settle instead of jittering forever against the boundary.
    #[test]
    fn sand_settles_against_the_world_edge() {
        let mut grid = SandGrid::new(6, 8);
        // Floor only — no side walls; the world edge itself is the wall.
        grid.fill_wall_rect(0, 7, 6, 1);
        let mut ledger = SettledSandLedger::default();
        for _ in 0..4 {
            grid.emit_sand(0, 1);
            grid.emit_sand(5, 1);
            grid.step();
            grid.settle_into(&mut ledger);
        }
        run_to_fixed_point(&mut grid, &mut ledger, 64);
        assert_eq!(grid.loose(), 0);
        assert_eq!(ledger.total(), grid.emitted());
    }
}
