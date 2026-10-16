//! The surface graphs in use, kept by what each one is a function of.
//!
//! A [`NavGraph`] is a pure function of the geometry it is built over, the
//! exits it avoids, the body's tuning, abilities and size, and its motion
//! frame. [`NavGraphKey`] is that function's argument as a value, and
//! [`NavGraphCache`] keeps a bounded number of graphs by it, each built whole.
//! The simulation's navigation advisor keeps one cache, and the autopilot of a
//! player's seat keeps another: one keying rule, so two graphs for the same
//! argument are the same graph.

use ambition_platformer2d_core as ae;
use ae::movement::MotionModelSpec;
use ae::{AbilitySet, Aabb, BodyBaseSize, BodyClusterScratch, MotionFrame, Vec2, World};

use super::NavGraph;

/// What one graph is a function of.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NavGraphKey {
    /// The geometry and the exits, as a number ([`geometry_stamp`]).
    room: u64,
    spec: MotionModelSpec,
    abilities: AbilitySet,
    size: Vec2,
    base_size: BodyBaseSize,
    /// The whole frame, and not its net acceleration: the graph is built from
    /// which way is down, from the gravity a jump law can scale, and from the
    /// external acceleration it cannot. Two frames with one sum are two rooms
    /// to a body (found in review, 2026-10-09).
    frame: MotionFrame,
}

impl NavGraphKey {
    /// The key of the graph of `world`, avoiding `exits`, for a body of this
    /// tuning under `frame`.
    pub fn new(
        world: &World,
        exits: &[Aabb],
        spec: MotionModelSpec,
        abilities: AbilitySet,
        size: Vec2,
        base_size: BodyBaseSize,
        frame: MotionFrame,
    ) -> Self {
        Self { room: geometry_stamp(world, exits), spec, abilities, size, base_size, frame }
    }

    /// The probe body the graph is built for: the tuning with no history. A
    /// graph is not a fact about what a body was doing when it was asked for.
    pub fn probe_body(&self) -> BodyClusterScratch {
        let mut body = BodyClusterScratch::new_with_abilities(Vec2::ZERO, self.abilities);
        ae::movement::switch_motion_model(&mut body.model, self.spec);
        body.kinematics.size = self.size;
        body.base_size = self.base_size;
        body
    }
}

/// Graphs by [`NavGraphKey`], at most `capacity`; the oldest goes first. A
/// key with no graph (the body has none there) is kept too, so it is not
/// built again.
pub struct NavGraphCache {
    graphs: Vec<(NavGraphKey, Option<NavGraph>)>,
    capacity: usize,
}

impl NavGraphCache {
    pub fn with_capacity(capacity: usize) -> Self {
        Self { graphs: Vec::with_capacity(capacity), capacity: capacity.max(1) }
    }

    pub fn len(&self) -> usize {
        self.graphs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.graphs.is_empty()
    }

    /// The kept answer for `key`: `Some(None)` when it has no graph, `None`
    /// when it was never built.
    pub fn get(&self, key: &NavGraphKey) -> Option<Option<&NavGraph>> {
        self.graphs.iter().find(|(known, _)| known == key).map(|(_, graph)| graph.as_ref())
    }

    /// Keep `graph` as the answer for `key`.
    pub fn insert(&mut self, key: NavGraphKey, graph: Option<NavGraph>) {
        if let Some(slot) = self.graphs.iter_mut().find(|(known, _)| *known == key) {
            slot.1 = graph;
            return;
        }
        if self.graphs.len() >= self.capacity {
            self.graphs.remove(0);
        }
        self.graphs.push((key, graph));
    }

    /// The kept answer for `key`, or `build()` kept and returned.
    pub fn get_or_build(&mut self, key: NavGraphKey, build: impl FnOnce() -> Option<NavGraph>) -> Option<&NavGraph> {
        let index = match self.graphs.iter().position(|(known, _)| *known == key) {
            Some(index) => index,
            None => {
                self.insert(key, build());
                self.graphs.len() - 1
            }
        };
        self.graphs[index].1.as_ref()
    }

    /// Each kept key and its graph.
    pub fn iter(&self) -> impl Iterator<Item = (&NavGraphKey, Option<&NavGraph>)> {
        self.graphs.iter().map(|(key, graph)| (key, graph.as_ref()))
    }
}

/// One number for the geometry a graph is built from: each block's box, and
/// what the graph reads of its kind (a support, a wall, a hazard, a one-way
/// surface, a block that moves), and each exit the graph avoids. Two rooms
/// with the same blocks and exits have the same graph.
pub fn geometry_stamp(world: &World, exits: &[Aabb]) -> u64 {
    use ae::collision_semantics::{is_full_collision_surface, is_support_surface};
    let mut stamp = ae::navigation::mix(world.blocks.len() as u64);
    let mut fold = |value: u64| stamp = ae::navigation::mix(stamp ^ value);
    fold(((world.size.x.to_bits() as u64) << 32) | world.size.y.to_bits() as u64);
    for block in &world.blocks {
        fold(((block.aabb.min.x.to_bits() as u64) << 32) | block.aabb.min.y.to_bits() as u64);
        fold(((block.aabb.max.x.to_bits() as u64) << 32) | block.aabb.max.y.to_bits() as u64);
        fold(
            u64::from(is_support_surface(block.kind))
                | u64::from(is_full_collision_surface(block.kind)) << 1
                | u64::from(block.kind == ae::BlockKind::Hazard) << 2
                | u64::from(block.kind == ae::BlockKind::OneWay) << 3
                | u64::from(block.velocity != Vec2::ZERO) << 4,
        );
    }
    fold(exits.len() as u64);
    for exit in exits {
        fold(((exit.min.x.to_bits() as u64) << 32) | exit.min.y.to_bits() as u64);
        fold(((exit.max.x.to_bits() as u64) << 32) | exit.max.y.to_bits() as u64);
    }
    stamp
}
