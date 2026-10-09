//! The navigation advisor: what a body that navigates can know of its room,
//! this tick.
//!
//! PERCEPTION, not decision and not state. For each body whose brain
//! navigates, [`advise_navigation`] reads the room's surface graph for that
//! body's tuning and writes an [`ae::navigation::NavAdvice`]: some places the
//! body can reach, and the next leg toward the goal its brain holds. The brain
//! reads it in its `BrainSnapshot` and decides. No brain reads a room.
//!
//! The graph ([`ambition_platformer2d_world::navigation::NavGraph`]) is a pure
//! function of the room's authored geometry, the body's tuning and its motion
//! frame. [`RoomNavigation`] keeps it, built whole on first use, so it is the
//! same on each peer and after each rewind, and no part of a snapshot.
//!
//! NOT SEEN: geometry that is not authored in the room (a gate, a breakable,
//! a sandbox solid) and a platform that moves. A leg such a thing stops fails,
//! and the brain plans again.

use std::collections::BTreeMap;

use ambition_platformer2d_core as ae;
use ae::navigation::{NavAdvice, NavNext};
use ambition_platformer2d_world::navigation::NavGraph;
use bevy::prelude::{Entity, Query, ResMut, Resource};

/// How far beside its target a body stands, past its own width (px).
const TARGET_ROOM: f32 = 12.0;
/// A target this far above a surface, or less, stands over it (px).
const TARGET_DEPTH: f32 = 160.0;
/// The most graphs kept: rooms times body tunings that navigate in them.
const GRAPHS_KEPT: usize = 16;

/// This tick's advice, by body. Written again each tick, before the brains
/// decide; a body with no entry has the default (no place, no answer).
#[derive(Resource, Default)]
pub struct NavigationAdvice {
    // Ordered: an instrument that reads each row must read them in one order.
    by_body: BTreeMap<Entity, NavAdvice>,
}

impl NavigationAdvice {
    pub fn of(&self, body: Entity) -> NavAdvice {
        self.by_body.get(&body).copied().unwrap_or_default()
    }

    /// Each body advised this tick, in entity order. For instruments.
    pub fn iter(&self) -> impl Iterator<Item = (Entity, &NavAdvice)> {
        self.by_body.iter().map(|(body, advice)| (*body, advice))
    }
}

/// What a graph is a function of.
#[derive(Clone, Copy, PartialEq)]
struct GraphKey {
    /// The room's authored geometry, as a number ([`geometry_stamp`]).
    room: u64,
    spec: ae::movement::MotionModelSpec,
    abilities: ae::AbilitySet,
    size: ae::Vec2,
    gravity: ae::Vec2,
}

/// The surface graphs in use: a derived cache, never rollback state.
#[derive(Resource, Default)]
pub struct RoomNavigation {
    graphs: Vec<(GraphKey, Option<NavGraph>)>,
    /// The graphs a body was advised from this tick. For instruments: a
    /// graph that is kept and not in use is no fact about the room now.
    in_use: Vec<GraphKey>,
}

impl RoomNavigation {
    /// How many graphs are kept. For tests and instruments.
    pub fn len(&self) -> usize {
        self.graphs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.graphs.is_empty()
    }

    /// The graphs a body was advised from on the last tick.
    pub fn graphs_in_use(&self) -> impl Iterator<Item = &NavGraph> {
        self.graphs
            .iter()
            .filter(|(key, _)| self.in_use.contains(key))
            .filter_map(|(_, graph)| graph.as_ref())
    }

    /// The graphs that were built. For tests and instruments.
    pub fn graphs(&self) -> impl Iterator<Item = &NavGraph> {
        self.graphs.iter().filter_map(|(_, graph)| graph.as_ref())
    }

    fn graph(&mut self, key: GraphKey, build: impl FnOnce() -> Option<NavGraph>) -> Option<&NavGraph> {
        let index = match self.graphs.iter().position(|(known, _)| *known == key) {
            Some(index) => index,
            None => {
                if self.graphs.len() >= GRAPHS_KEPT {
                    self.graphs.remove(0);
                }
                self.graphs.push((key, build()));
                self.graphs.len() - 1
            }
        };
        self.graphs[index].1.as_ref()
    }
}

/// One number for the geometry a graph is built from: each block's box and
/// kind. Two rooms with the same blocks have the same graph.
fn geometry_stamp(world: &ae::World) -> u64 {
    let mut stamp = ae::navigation::mix(world.blocks.len() as u64);
    let mut fold = |value: u64| stamp = ae::navigation::mix(stamp ^ value);
    fold(((world.size.x.to_bits() as u64) << 32) | world.size.y.to_bits() as u64);
    for block in &world.blocks {
        fold(((block.aabb.min.x.to_bits() as u64) << 32) | block.aabb.min.y.to_bits() as u64);
        fold(((block.aabb.max.x.to_bits() as u64) << 32) | block.aabb.max.y.to_bits() as u64);
        fold(std::mem::discriminant(&block.kind).hash_u64());
    }
    stamp
}

trait DiscriminantNumber {
    fn hash_u64(&self) -> u64;
}

impl<T> DiscriminantNumber for std::mem::Discriminant<T> {
    fn hash_u64(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        // A fixed-key hasher: the same number in each process.
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

/// OBSERVE: write this tick's [`NavAdvice`] for each body whose brain
/// navigates.
pub fn advise_navigation(
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    mut cache: ResMut<RoomNavigation>,
    mut advice: ResMut<NavigationAdvice>,
    bodies: Query<
        (
            Entity,
            &ambition_characters::brain::Brain,
            &ae::movement::MotionModel,
            &ae::BodyAbilities,
            &ae::BodyKinematics,
            &ae::BodyBaseSize,
            &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
            // Whom the body attends to, when it has a foe: the same read-model
            // its brain's `target_pos` comes from.
            Option<&ambition_combat::components::ActorTarget>,
        ),
    >,
    // The players, for a body with no foe: it attends to the nearest one in
    // its room. A peaceful body has no combat target, and a companion keeps
    // near a friend.
    players: Query<
        (Entity, &ae::BodyKinematics),
        bevy::prelude::With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
    >,
) {
    advice.by_body.clear();
    cache.in_use.clear();
    for (entity, brain, model, abilities, kinematics, base_size, frame, target) in &bodies {
        let Some(request) = brain.navigation_request() else {
            continue;
        };
        let stamp = rooms
            .stamped(entity)
            .map(ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance);
        let Some(room) = collision.room(stamp.as_ref()) else {
            continue;
        };
        let world = room.base();
        let frame = frame.get();
        let key = GraphKey {
            room: geometry_stamp(world),
            spec: model.spec(),
            abilities: abilities.abilities,
            size: kinematics.size,
            gravity: frame.acceleration(),
        };
        if !cache.in_use.contains(&key) {
            cache.in_use.push(key);
        }
        let graph = cache.graph(key, || {
            // The body's tuning with no history: a graph is not a fact about
            // what this body was doing when it was first asked for.
            let mut body = ae::BodyClusterScratch::new_with_abilities(ae::Vec2::ZERO, abilities.abilities);
            ae::movement::switch_motion_model(&mut body.model, model.spec());
            body.kinematics.size = kinematics.size;
            body.base_size = *base_size;
            NavGraph::build(world, &body, frame)
        });
        let Some(graph) = graph else {
            continue;
        };
        let feet = kinematics.pos + graph.frame.down * graph.half.y;
        let (waypoints, waypoint_count) = graph.waypoints(feet, request.choice);
        let next = request.goal.map_or(NavNext::Unknown, |goal| graph.next(feet, goal));
        let live_room = rooms.of(entity);
        let attended = target.filter(|target| target.entity.is_some()).map(|target| target.pos).or_else(|| {
            players
                .iter()
                .filter(|(player, _)| *player != entity && rooms.of(*player) == live_room)
                .map(|(_, player)| player.pos)
                .min_by(|a, b| a.distance_squared(kinematics.pos).total_cmp(&b.distance_squared(kinematics.pos)))
        });
        let target_place = attended
            .and_then(|at| graph.place_beside(feet, at, kinematics.size.x + TARGET_ROOM, TARGET_DEPTH));
        let target_shares_surface = attended.is_some_and(|at| {
            let under = graph.surface_under(at, TARGET_DEPTH);
            under.is_some() && under == graph.surface_at(feet)
        });
        advice.by_body.insert(
            entity,
            NavAdvice {
                feet,
                waypoints,
                waypoint_count,
                goal: request.goal,
                next,
                target_place,
                target_shares_surface,
            },
        );
    }
}
