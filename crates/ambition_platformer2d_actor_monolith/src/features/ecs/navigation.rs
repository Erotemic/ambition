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
use ae::navigation::{ErrandSight, NavAdvice, NavNext};
use ambition_platformer2d_world::navigation::{NavGraph, NavGraphCache, NavGraphKey};
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

/// The surface graphs in use: a derived cache, never rollback state. Each
/// graph is built whole on first use, so it is the same on each peer and
/// after each rewind.
#[derive(Resource)]
pub struct RoomNavigation {
    graphs: NavGraphCache,
    /// The graphs a body was advised from this tick. For instruments: a
    /// graph that is kept and not in use is no fact about the room now.
    in_use: Vec<NavGraphKey>,
    /// Each live room's geometry stamp, folded again only when the room's
    /// geometry is written.
    stamps: ambition_platformer2d_world::navigation::GeometryStamps,
}

impl Default for RoomNavigation {
    fn default() -> Self {
        Self { graphs: NavGraphCache::with_capacity(GRAPHS_KEPT), in_use: Vec::new(), stamps: Default::default() }
    }
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
            .filter_map(|(_, graph)| graph)
    }

    /// The graphs that were built. For tests and instruments.
    pub fn graphs(&self) -> impl Iterator<Item = &NavGraph> {
        self.graphs.iter().filter_map(|(_, graph)| graph)
    }

    /// The graph for a body of this tuning, in this room and this motion
    /// frame: kept, or built whole now. `None`: the body has no graph there.
    /// A body no slot drives crosses no room by a zone: its graph has no exits.
    fn graph_of(
        &mut self,
        world: &ae::World,
        // `geometry_stamp(world, &[])`, kept by `GeometryStamps`.
        stamp: u64,
        spec: ae::movement::MotionModelSpec,
        abilities: ae::AbilitySet,
        size: ae::Vec2,
        base_size: ae::BodyBaseSize,
        frame: ae::MotionFrame,
    ) -> Option<&NavGraph> {
        let key = NavGraphKey::from_stamp(stamp, spec, abilities, size, base_size, frame);
        if !self.in_use.contains(&key) {
            self.in_use.push(key);
        }
        self.graphs.get_or_build(key, || NavGraph::build(world, &key.probe_body(), frame))
    }
}

/// OBSERVE: write this tick's [`NavAdvice`] for each body whose brain
/// navigates.
pub fn advise_navigation(
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    // The room graph and the live rooms, for an errand item in another room.
    specs: Option<ambition_platformer2d_world::rooms::LiveRoomSpecs>,
    mut cache: ResMut<RoomNavigation>,
    mut advice: ResMut<NavigationAdvice>,
    bodies: Query<
        (
            Entity,
            &ambition_characters::brain::Brain,
            &ae::movement::MotionModel,
            &ae::BodyAbilities,
            &ae::BodyKinematics,
            &ae::BodyGroundState,
            &ae::BodyBaseSize,
            &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
            // Whom the body attends to, when it has a foe: the same read-model
            // its brain's `target_pos` comes from.
            Option<&ambition_combat::components::ActorTarget>,
            Option<&super::errand::Errand>,
            // Whether the body may leave the live rooms for its errand: a
            // body with durable whereabouts.
            Option<&ambition_combat::actor_tuning::ActorConfig>,
            Option<&ambition_platformer2d_shared_tangle::construction::SpawnOrigin>,
        ),
    >,
    // Where an errand item lies when it lies in no live room.
    ledger: Option<bevy::prelude::Res<ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences>>,
    // The items an errand can name: in the world, by their stable identity.
    items: Query<(
        Entity,
        &ambition_platformer2d_shared_tangle::sim_id::SimId,
        &ambition_held_items::GroundItem,
        &ambition_held_items::ItemCustody,
    )>,
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
    for (entity, brain, model, abilities, kinematics, ground, base_size, frame, target, errand, config, origin) in &bodies {
        let Some(request) = brain.navigation_request() else {
            continue;
        };
        // A body the movement kernel has not stepped yet has the motion model
        // it was born with. The integrator writes the body's own tuning into
        // the model on each step (`step_body`), so before the first step the
        // model is of a body that does not exist, and a graph for it is a
        // build for nothing (measured: 54 to 112 ms for each dog). The kernel
        // marks the body's ground contact on that first step.
        if !ground.contact_initialized {
            continue;
        }
        let stamp = rooms
            .stamped(entity)
            .map(ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance);
        let Some(room) = collision.room(stamp.as_ref()) else {
            continue;
        };
        let world = room.base();
        let stamp = cache.stamps.of(room);
        let graph = cache.graph_of(
            world,
            stamp,
            model.spec(),
            abilities.abilities,
            kinematics.size,
            *base_size,
            frame.get(),
        );
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
                // The nearest; of two as near, the one at the lesser point,
                // so the choice is a fact of positions and of no query order.
                .min_by(|a, b| {
                    a.distance_squared(kinematics.pos)
                        .total_cmp(&b.distance_squared(kinematics.pos))
                        .then(a.x.total_cmp(&b.x))
                        .then(a.y.total_cmp(&b.y))
                })
        });
        let target_place = attended
            .and_then(|at| graph.place_beside(feet, at, kinematics.size.x + TARGET_ROOM, TARGET_DEPTH));
        let target_shares_surface = attended.is_some_and(|at| {
            let under = graph.surface_under(at, TARGET_DEPTH);
            under.is_some() && under == graph.surface_at(feet)
        });
        let errand = errand
            .filter(|errand| errand.outcome == super::errand::ErrandOutcome::Pending)
            .map_or(ErrandSight::None, |errand| {
                // A body in the air stands on no surface: no route is judged
                // from there, and the errand waits for the body to land.
                let Some(from) = graph.surface_at(feet) else {
                    return ErrandSight::None;
                };
                let reached = |at: ae::Vec2, depth: f32| {
                    graph.surface_under(at, depth).filter(|under| graph.reachable_from(from).contains(under))
                };
                match super::errand::errand_item_at(errand, live_room, &rooms, &items) {
                    // Not in this room: in another live room, through the zone
                    // where the route there starts, when the body gets to it.
                    None => {
                        let crossing = specs.as_ref().zip(live_room).map(|(specs, body_room)| {
                            let may_leave = config.is_some_and(|config| {
                                ambition_platformer2d_construction::whereabouts::keeps_durable_whereabouts(specs, config, origin)
                            });
                            super::errand::errand_crossing(
                                errand,
                                body_room,
                                may_leave,
                                specs,
                                ledger.as_deref(),
                                &items,
                            )
                        });
                        // An item that lies somewhere no route reaches is not
                        // gone: the errand says there is no route.
                        let crossing = match crossing {
                            Some(Ok(crossing)) => crossing,
                            Some(Err(super::errand::ErrandRefusal::NoRoute)) => return ErrandSight::NoRoute,
                            Some(Err(_)) | None => return ErrandSight::Gone,
                        };
                        use ae::AabbExt as _;
                        let door = crossing.zone.center();
                        match reached(door, crossing.zone.half_size().length() + TARGET_DEPTH) {
                            Some(under) => ErrandSight::Door(graph.point_on(under, graph.frame.along(door))),
                            None => ErrandSight::NoRoute,
                        }
                    }
                    Some(at) => match graph.surface_under(at, TARGET_DEPTH) {
                        Some(under) if graph.reachable_from(from).contains(&under) => {
                            ErrandSight::At(graph.point_on(under, graph.frame.along(at)))
                        }
                        _ => ErrandSight::NoRoute,
                    },
                }
            });
        advice.by_body.insert(
            entity,
            NavAdvice {
                errand,
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
    cache.stamps.retire_unused();
}

#[cfg(test)]
mod tests {
    use super::*;
    use ae::movement::GRAVITY;
    use ae::{AbilitySet, AccelerationFrame, Block, MotionFrame, Vec2, World};

    fn room() -> World {
        World::new(
            "advisor fixture",
            Vec2::new(2000.0, 1200.0),
            Vec2::ZERO,
            vec![
                Block::solid("floor", Vec2::new(0.0, 1000.0), Vec2::new(2000.0, 64.0)),
                Block::one_way("perch", Vec2::new(400.0, 940.0), Vec2::new(300.0, 16.0)),
                Block::solid("wall", Vec2::new(1936.0, 0.0), Vec2::new(64.0, 1000.0)),
            ],
        )
    }

    /// What a test reads of a graph: which way is down in it, and its size.
    fn read(graph: Option<&NavGraph>) -> Option<(Vec2, usize, usize)> {
        graph.map(|graph| (graph.frame.down, graph.surfaces.len(), graph.links.len()))
    }

    fn ask(cache: &mut RoomNavigation, world: &World, frame: MotionFrame) -> Option<(Vec2, usize, usize)> {
        let abilities = AbilitySet { move_horizontal: true, jump: true, ..AbilitySet::NONE };
        let body = ae::BodyClusterScratch::new_with_abilities(Vec2::ZERO, abilities);
        let stamp = ambition_platformer2d_world::navigation::geometry_stamp(world, &[]);
        read(cache.graph_of(world, stamp, body.model.spec(), abilities, body.kinematics.size, body.base_size, frame))
    }

    /// Two bodies whose frames have one net acceleration and are not one
    /// frame get two graphs, each its own, whichever asks first.
    ///
    /// `upright` and `sideways` pull the same way with the same strength; in
    /// `sideways` the feet point along +x (a basis that an acceleration does
    /// not turn). `split` is upright, with half of its pull an external
    /// acceleration, which a jump law does not scale.
    #[test]
    fn two_frames_with_one_net_acceleration_are_two_graphs_in_each_order() {
        let world = room();
        let pull = Vec2::new(0.0, GRAVITY);
        let upright = MotionFrame::from_acceleration(pull).expect("a pull");
        let sideways = MotionFrame::new(AccelerationFrame::new(Vec2::X), pull);
        let split = MotionFrame::with_accelerations(AccelerationFrame::new(Vec2::Y), pull * 0.5, pull * 0.5);
        // ⛔ THE PREMISE: one net acceleration, three frames.
        assert_eq!(upright.acceleration(), sideways.acceleration());
        assert_eq!(upright.acceleration(), split.acceleration());
        assert!(upright != sideways && upright != split);

        // What each frame's graph is with nothing kept.
        let alone = |frame| ask(&mut RoomNavigation::default(), &world, frame);
        let (of_upright, of_sideways) = (alone(upright), alone(sideways));
        assert!(of_upright.is_some(), "premise: an upright walker has a graph in the fixture");
        assert_ne!(of_upright, of_sideways, "premise: the two frames do not have one graph");

        for (first, second) in [(upright, sideways), (sideways, upright)] {
            let mut cache = RoomNavigation::default();
            assert_eq!(ask(&mut cache, &world, first), alone(first));
            assert_eq!(
                ask(&mut cache, &world, second),
                alone(second),
                "the second body was given the graph of the first"
            );
            assert_eq!(cache.len(), 2);
        }
        // The decomposition is in the key too.
        let mut cache = RoomNavigation::default();
        ask(&mut cache, &world, upright);
        ask(&mut cache, &world, split);
        assert_eq!(cache.len(), 2, "a frame with an external acceleration shared the upright graph");
    }
}
