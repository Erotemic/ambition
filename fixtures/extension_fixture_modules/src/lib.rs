//! Test-only extension modules. The game does not link this crate: a test
//! builds it for `wasm32-unknown-unknown` and loads the file
//! (`ExtensionModuleFiles`), so what it declares is NEW to the running game.
//!
//! `session_tally` proves fast-iteration I5 on the loaded road: a schema the
//! host was never compiled with, attached to the SESSION, counts every use of
//! the shockwave in the session, and every third use fires a bolt — a record
//! field that decides a later spawn.
//!
//! `trail_loop` is I7.3's graph fixture: an algorithm the technique vocabulary
//! cannot say. While a body holds the shockwave, the module keeps the cells
//! the body passes through as a bounded GRAPH (nodes, and an edge for each
//! move between two cells). A move that joins two nodes that are already
//! connected closes a CYCLE: the module finds the cycle's path, puts a damage
//! box over it, and takes the cycle's edges out of the graph.

use ambition_combat_port::{DamageBox, DamageBoxPort, WieldedUsePort, Wielder};
use ambition_extension_sdk::{
    phases::WIELDED_USE, record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};
use ambition_projectile_spec::{ProjectileSpawn, ProjectileSpawnPort};
use bevy_math::Vec2;

/// The provider namespace of every module and schema in this crate.
pub const PROVIDER: &str = "fixture";

/// Every third use fires.
pub const EVERY: u32 = 3;

record! {
    /// The session's count of shockwave uses.
    pub struct Tally = SchemaKey::new(PROVIDER, "session_tally.tally", 1), per session;
    1 uses: u32,
}

pub fn session_tally() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(PROVIDER, "session_tally"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![Tally::schema()],
        entries: vec![EntryDescriptor {
            key: "count".into(),
            phase: WIELDED_USE,
            trigger: TriggerBinding {
                port: WieldedUsePort::KEY,
                selector: "shockwave".into(),
            },
            reads: Vec::new(),
            writes: vec![Tally::KEY],
            requests: vec![ProjectileSpawnPort::KEY],
            after: Vec::new(),
            limits: Limits { max_requests: 1 },
            // Not used this tick: no call. The tally is the session's, so an
            // idle body does not reset it.
            on_idle: IdlePolicy::ResetState,
            run: EntryCode::Native(count),
        }],
    }
}

fn count(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let w: Wielder = inv.trigger::<WieldedUsePort>()?.clone();
    let mut tally = Tally::load(inv)?;
    tally.uses += 1;
    if tally.uses % EVERY == 0 {
        inv.submit::<ProjectileSpawnPort>(ProjectileSpawn {
            origin: Vec2::from(w.position),
            dir: Vec2::X,
            speed: 300.0,
            damage: 1,
            max_lifetime: 2.0,
            half_extent: Vec2::new(5.0, 5.0),
            gravity: 0.0,
            visual_id: String::new(),
            bounces: 0,
            bounce_on_world_contact: false,
            splash_half_extent: 0.0,
            boomerang_return_s: None,
        })?;
    }
    tally.store(inv)
}

/// The side of one trail cell, in world units.
pub const CELL: f32 = 48.0;
/// The graph's bounds: past either, the trail starts again at the body.
pub const MAX_NODES: usize = 48;
pub const MAX_EDGES: usize = 64;

record! {
    /// The body's trail graph.
    pub struct Trail = SchemaKey::new(PROVIDER, "trail_loop.trail", 1);
    /// The cell the body was in on its last tick.
    1 last: Option<u32>,
    /// Every cell in the graph.
    2 nodes: Vec<u32> [max MAX_NODES as u32],
    /// Each edge as two cells, `a < b`, flattened: `[a0, b0, a1, b1, ..]`.
    3 edges: Vec<u32> [max 2 * MAX_EDGES as u32],
    /// The cycles closed so far.
    4 loops: u32,
}

/// The cell that holds `p`, as one number: x and y in 16 bits each.
pub fn cell_of(p: [f32; 2]) -> u32 {
    let x = (p[0] / CELL).floor() as i32;
    let y = (p[1] / CELL).floor() as i32;
    (((x + 0x8000) as u32 & 0xFFFF) << 16) | ((y + 0x8000) as u32 & 0xFFFF)
}

/// The centre of a cell, in world units.
pub fn centre_of(cell: u32) -> [f32; 2] {
    let x = ((cell >> 16) as i32 - 0x8000) as f32;
    let y = ((cell & 0xFFFF) as i32 - 0x8000) as f32;
    [(x + 0.5) * CELL, (y + 0.5) * CELL]
}

fn edge(a: u32, b: u32) -> (u32, u32) {
    (a.min(b), a.max(b))
}

impl Trail {
    fn edge_list(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.edges.chunks_exact(2).map(|e| (e[0], e[1]))
    }

    fn has_edge(&self, a: u32, b: u32) -> bool {
        let e = edge(a, b);
        self.edge_list().any(|x| x == e)
    }

    /// The path from `from` to `to` over the edges, breadth first, the
    /// neighbours in edge order: one answer for one graph. `None` when the
    /// two are not connected.
    fn path(&self, from: u32, to: u32) -> Option<Vec<u32>> {
        let mut came_from: Vec<(u32, u32)> = vec![(from, from)];
        let mut frontier = vec![from];
        while !frontier.is_empty() {
            let mut next = Vec::new();
            for at in frontier {
                if at == to {
                    let mut path = vec![to];
                    let mut cur = to;
                    while cur != from {
                        cur = came_from.iter().find(|(n, _)| *n == cur)?.1;
                        path.push(cur);
                    }
                    path.reverse();
                    return Some(path);
                }
                for (a, b) in self.edge_list() {
                    let other = if a == at { b } else if b == at { a } else { continue };
                    if !came_from.iter().any(|(n, _)| *n == other) {
                        came_from.push((other, at));
                        next.push(other);
                    }
                }
            }
            frontier = next;
        }
        None
    }

    /// The trail moves into `cell`. Returns the cycle it closed (its cells,
    /// from `cell` round to the cell before), if it closed one.
    pub fn step(&mut self, cell: u32) -> Option<Vec<u32>> {
        let Some(last) = self.last else {
            self.restart(cell);
            return None;
        };
        if cell == last {
            return None;
        }
        self.last = Some(cell);
        if self.has_edge(last, cell) {
            return None;
        }
        let known = self.nodes.contains(&cell);
        if known {
            if let Some(cycle) = self.path(cell, last) {
                // The cycle's edges leave the graph, and so does a node no
                // edge holds any more, but the body's own cell stays.
                for pair in cycle.windows(2) {
                    let e = edge(pair[0], pair[1]);
                    let found = self.edge_list().position(|x| x == e);
                    if let Some(i) = found {
                        self.edges.drain(2 * i..2 * i + 2);
                    }
                }
                let edges: Vec<(u32, u32)> = self.edge_list().collect();
                self.nodes
                    .retain(|n| *n == cell || edges.iter().any(|(a, b)| a == n || b == n));
                self.loops += 1;
                return Some(cycle);
            }
        }
        if (!known && self.nodes.len() >= MAX_NODES) || self.edges.len() >= 2 * MAX_EDGES {
            self.restart(cell);
            return None;
        }
        if !known {
            self.nodes.push(cell);
        }
        let (a, b) = edge(last, cell);
        self.edges.extend([a, b]);
        None
    }

    fn restart(&mut self, cell: u32) {
        self.last = Some(cell);
        self.nodes = vec![cell];
        self.edges.clear();
    }
}

/// The damage box that covers a cycle's cells.
pub fn box_over(cycle: &[u32]) -> DamageBox {
    let centres: Vec<[f32; 2]> = cycle.iter().map(|c| centre_of(*c)).collect();
    let (mut lo, mut hi) = (centres[0], centres[0]);
    for c in &centres {
        lo = [lo[0].min(c[0]), lo[1].min(c[1])];
        hi = [hi[0].max(c[0]), hi[1].max(c[1])];
    }
    DamageBox {
        center: [(lo[0] + hi[0]) * 0.5, (lo[1] + hi[1]) * 0.5],
        half_extent: [(hi[0] - lo[0]) * 0.5 + CELL * 0.5, (hi[1] - lo[1]) * 0.5 + CELL * 0.5],
        damage: 1,
        knockback: 1.0,
        lifetime_s: 0.2,
    }
}

pub fn trail_loop() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(PROVIDER, "trail_loop"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![Trail::schema()],
        entries: vec![EntryDescriptor {
            key: "walk".into(),
            phase: WIELDED_USE,
            trigger: TriggerBinding {
                port: WieldedUsePort::KEY,
                selector: "shockwave".into(),
            },
            reads: Vec::new(),
            writes: vec![Trail::KEY],
            requests: vec![DamageBoxPort::KEY],
            after: Vec::new(),
            limits: Limits { max_requests: 1 },
            // Every tick the body holds the item, pressed or not: the trail
            // is where the body WENT.
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(walk),
        }],
    }
}

fn walk(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let w: Wielder = inv.trigger::<WieldedUsePort>()?.clone();
    let mut trail = Trail::load(inv)?;
    if let Some(cycle) = trail.step(cell_of(w.position)) {
        inv.submit::<DamageBoxPort>(box_over(&cycle))?;
    }
    trail.store(inv)
}

/// Every module this crate provides, as constructors.
pub const MODULES: &[fn() -> ModuleDescriptor] = &[session_tally, trail_loop];

#[cfg(test)]
mod tests {
    use super::*;

    fn c(x: i32, y: i32) -> u32 {
        cell_of([x as f32 * CELL + 1.0, y as f32 * CELL + 1.0])
    }

    #[test]
    fn a_straight_walk_and_its_way_back_close_no_cycle() {
        let mut t = Trail::default();
        for x in (0..6).chain((0..6).rev()) {
            assert_eq!(t.step(c(x, 0)), None);
        }
        assert_eq!(t.loops, 0);
        assert_eq!(t.nodes.len(), 6);
        assert_eq!(t.edges.len(), 2 * 5);
    }

    /// Walk right on the ground, jump back left over it, land on a cell
    /// already walked: the landing closes the loop.
    #[test]
    fn a_jump_back_over_the_walked_ground_closes_a_cycle() {
        let mut t = Trail::default();
        for x in 0..6 {
            assert_eq!(t.step(c(x, 0)), None);
        }
        assert_eq!(t.step(c(4, -1)), None);
        assert_eq!(t.step(c(3, -1)), None);
        let cycle = t.step(c(2, 0)).expect("the landing closes the loop");
        assert_eq!(cycle, vec![c(2, 0), c(3, 0), c(4, 0), c(5, 0), c(4, -1), c(3, -1)]);
        assert_eq!(t.loops, 1);
        // The cycle's edges are gone; the walk before it stays.
        assert!(t.has_edge(c(0, 0), c(1, 0)) && t.has_edge(c(1, 0), c(2, 0)));
        assert!(!t.has_edge(c(2, 0), c(3, 0)));
        assert!(!t.nodes.contains(&c(4, -1)));
        let b = box_over(&cycle);
        assert_eq!(b.center, [4.0 * CELL, -0.0 * CELL]);
        assert_eq!(b.half_extent, [2.0 * CELL, CELL]);
    }

    #[test]
    fn the_graph_restarts_at_its_bound() {
        let mut t = Trail::default();
        for x in 0..(MAX_NODES as i32 + 3) {
            t.step(c(x, 0));
        }
        assert!(t.nodes.len() <= MAX_NODES);
        assert_eq!(t.loops, 0);
    }

    #[test]
    fn a_cell_survives_the_packing() {
        for (x, y) in [(0, 0), (-3, 7), (100, -100)] {
            let cell = c(x, y);
            assert_eq!(cell_of(centre_of(cell)), cell);
        }
    }
}

ambition_extension_sdk::export_modules!(list: crate::MODULES);
