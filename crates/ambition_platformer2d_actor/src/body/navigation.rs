//! The navigation advisor: what a body that navigates can know of its room,
//! this tick.
//!
//! [`advise_navigation`] answers the current navigation questions. It sends
//! the live position of the feet for each retained leg. The brain gets the
//! advice in its `BrainSnapshot`.
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
use ae::navigation::{ErrandSight, NavAdvice, NavNext, NavigationMethod, NavigationTask, RouteWait};
use ae::Vec2;
use ambition_characters::brain::ApproachDemand;
use ambition_platformer2d_world::navigation::{GroundMap, NavGraph, NavGraphCache, NavGraphKey, RefinementWork};
use bevy::prelude::{Entity, Query, ResMut, Resource};

/// How far beside its target a body stands, past its own width (px).
const TARGET_ROOM: f32 = 12.0;
/// A target this far above a surface, or less, stands over it (px).
const TARGET_DEPTH: f32 = 160.0;
/// The most graphs kept: rooms times body tunings that navigate in them.
const GRAPHS_KEPT: usize = 16;
const REFINEMENTS_PER_LEG: usize = 4;

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
    by_body: BTreeMap<Entity, NavGraphKey>,
    /// Each live room's geometry stamp, folded again only when the room's
    /// geometry is written.
    stamps: ambition_platformer2d_world::navigation::GeometryStamps,
    questions: u64,
    refinement: RefinementWork,
}

impl Default for RoomNavigation {
    fn default() -> Self {
        Self {
            graphs: NavGraphCache::with_capacity(GRAPHS_KEPT),
            in_use: Vec::new(),
            by_body: BTreeMap::new(),
            stamps: Default::default(),
            questions: 0,
            refinement: RefinementWork::default(),
        }
    }
}

impl RoomNavigation {
    pub fn questions(&self) -> u64 {
        self.questions
    }

    pub fn ground_preparations(&self) -> u64 {
        self.graphs.ground_preparations()
    }

    pub fn motion_measures(&self) -> u64 {
        self.graphs.motion_measures()
    }

    pub fn refinement_work(&self) -> RefinementWork {
        self.refinement
    }

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

    /// The graphs in use built from a world and exits with the stamp
    /// `room` ([`ambition_platformer2d_world::navigation::geometry_stamp`]).
    pub fn graphs_in_use_stamped(&self, room: u64) -> impl Iterator<Item = &NavGraph> {
        self.graphs
            .iter()
            .filter(move |(key, _)| key.room_stamp() == room && self.in_use.contains(key))
            .filter_map(|(_, graph)| graph)
    }

    /// The graphs that were built. For tests and instruments.
    pub fn graphs(&self) -> impl Iterator<Item = &NavGraph> {
        self.graphs.iter().filter_map(|(_, graph)| graph)
    }

    /// The graph for a body of this tuning, in this room and this motion
    /// frame: kept, or built whole now. `None`: the body has no graph there.
    /// A body no slot drives crosses no room by a zone: its graph has no exits.
    fn graph_of(&mut self, world: &ae::World, key: NavGraphKey) -> Option<&NavGraph> {
        if !self.in_use.contains(&key) {
            self.in_use.push(key);
        }
        self.graphs.get_or_build_measured(key, world, &[],
            bevy::tasks::ComputeTaskPool::try_get().map(|pool| &**pool))
    }
}

#[derive(Clone, Copy)]
enum TaskPlace {
    Result(ErrandSight),
    InRoom(Vec2),
    Door { point: Vec2, depth: f32 },
}

impl Default for TaskPlace {
    fn default() -> Self {
        Self::Result(ErrandSight::NoRequest)
    }
}

impl TaskPlace {
    fn sight(
        self,
        map: &GroundMap,
        feet: Vec2,
        failed_goal: Option<Vec2>,
        check: impl FnOnce(usize, usize, Vec2) -> Result<(), ErrandSight>,
    ) -> ErrandSight {
        let (target, depth) = match self {
            Self::Result(result) => return result,
            Self::InRoom(point) => (point, TARGET_DEPTH),
            Self::Door { point, depth } => (point, depth),
        };
        let Some(from) = map.surface_at(feet) else {
            return ErrandSight::Waiting(RouteWait::Unsupported);
        };
        let Some(to) = map.surface_under(target, depth) else {
            return ErrandSight::Waiting(RouteWait::Unsupported);
        };
        let place = map.point_on(to, map.frame.along(target));
        if failed_goal == Some(place) {
            return ErrandSight::ExecutionFailed;
        }
        if let Err(result) = check(from, to, place) {
            return result;
        }
        match self {
            Self::Door { .. } => ErrandSight::Door(place),
            _ => ErrandSight::At(place),
        }
    }
}

/// OBSERVE: write this tick's [`NavAdvice`] for each body whose brain
/// navigates.
pub fn advise_navigation(
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    freeze: ambition_platformer2d_shared_tangle::lifecycle::RoomFreeze,
    time: bevy::prelude::Res<ambition_time::WorldTime>,
    elapsed: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<ambition_time::GameplayElapsed>,
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
            Option<&super::visit::Visit>,
            // Whether the body may leave the live rooms for its errand: a
            // body with durable whereabouts.
            Option<&ambition_combat::actor_tuning::ActorConfig>,
            Option<&ambition_platformer2d_shared_tangle::construction::SpawnOrigin>,
            (bevy::prelude::Has<super::dormancy::Dormant>, bevy::prelude::Has<ambition_characters::control::DrivingParticipant>),
        ),
    >,
    // Where an errand item lies when it lies in no live room.
    ledger: Option<bevy::prelude::Res<ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences>>,
    // The items an errand can name: in the world, by their stable identity.
    items: Query<(
        Entity,
        &ambition_platformer2d_shared_tangle::sim_id::SimId,
        &ae::BodyKinematics,
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
    cache.graphs.begin_demand();
    cache.by_body.clear();
    for (entity, brain, model, abilities, kinematics, ground, base_size, frame, target, errand, visit, config, origin, (dormant, driven)) in &bodies {
        let Some(mut request) = brain.navigation_request(elapsed.0, ground.on_ground) else {
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
        if time.sim_dt() <= 0.0 || freeze.scale_of(entity) <= 0.0 {
            continue;
        }
        if dormant || driven {
            // A task owner can still need an answer while the brain cannot decide.
            request = Default::default();
        }
        let pending_errand = errand.filter(|errand| errand.outcome == super::errand::ErrandOutcome::Pending);
        let pending_visit = visit.filter(|visit| visit.outcome == super::visit::VisitOutcome::Pending);
        let live_room = rooms.of(entity);
        let selected = target.filter(|target| target.entity.is_some()).map(|target| target.pos);
        let attended = match request.approach {
            ApproachDemand::None => None,
            ApproachDemand::CombatTarget => selected,
            ApproachDemand::Nearby(_) | ApproachDemand::Any => selected.or_else(|| {
                players.iter()
                    .filter(|(player, _)| *player != entity && rooms.of(*player) == live_room)
                    .map(|(_, player)| player.pos)
                    .min_by(|a, b| a.distance_squared(kinematics.pos).total_cmp(&b.distance_squared(kinematics.pos))
                        .then(a.x.total_cmp(&b.x)).then(a.y.total_cmp(&b.y)))
            }),
        };
        let down = frame.down();
        let size = kinematics.size();
        let physical_feet = kinematics.pos + down * ((size * down).length() * 0.5);
        let attended = attended.filter(|at| {
            let ApproachDemand::Nearby(radius) = request.approach else { return true; };
            // Allow for the projection to support and the space beside the target.
            let slack = TARGET_DEPTH + TARGET_ROOM + 3.0 * size.length();
            at.distance(physical_feet) <= radius + slack
        });
        let route_goal = request.next_leg.filter(|_| ground.on_ground);
        let task_kind = pending_errand.map(|_| NavigationTask::Fetch)
            .or_else(|| pending_visit.map(|_| NavigationTask::Visit));
        let method = if task_kind.is_some() { NavigationMethod::SimulationBacked } else { request.method };
        let task_needs_place = ground.on_ground && task_kind.is_some();
        let failed_task_goal = request.failed_task_goal
            .filter(|(kind, _)| task_kind == Some(*kind))
            .map(|(_, place)| place);
        let waiting = |reason| NavAdvice {
            feet: physical_feet,
            goal: request.next_leg,
            next: request.next_leg.map_or(NavNext::NoRequest, |_| NavNext::Waiting(reason)),
            errand: if task_kind.is_some() { ErrandSight::Waiting(reason) } else { ErrandSight::NoRequest },
            task: task_kind,
            ..NavAdvice::default()
        };
        if route_goal.is_none() && request.destination_seed.is_none() && attended.is_none() && !task_needs_place {
            if request.following_leg || request.next_leg.is_some() || task_kind.is_some() {
                advice.by_body.insert(entity, waiting(RouteWait::NotApplicable));
            }
            continue;
        }
        cache.questions += u64::from(route_goal.is_some())
            + u64::from(request.destination_seed.is_some())
            + u64::from(attended.is_some())
            + u64::from(task_needs_place);
        let stamp = rooms
            .stamped(entity)
            .map(ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance);
        let Some(room) = collision.room(stamp.as_ref()) else {
            advice.by_body.insert(entity, waiting(RouteWait::Deferred));
            continue;
        };
        let world = room.base();
        let stamp = cache.stamps.of(room);
        let task = if task_needs_place {
            pending_errand.map_or_else(
                || pending_visit.map_or(TaskPlace::default(), |visit| TaskPlace::InRoom(visit.place)),
                |errand| {
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
                                    items.iter().map(|(entity, id, _, custody)| (entity, id, custody)),
                                )
                            });
                            // An item that lies somewhere no route reaches is not
                            // gone: the errand says there is no route.
                            let crossing = match crossing {
                                Some(Ok(crossing)) => crossing,
                                Some(Err(super::errand::ErrandRefusal::NoRoute)) => return TaskPlace::Result(ErrandSight::NoRoute),
                                Some(Err(_)) | None => return TaskPlace::Result(ErrandSight::Gone),
                            };
                            use ae::AabbExt as _;
                            TaskPlace::Door {
                                point: crossing.zone.center(),
                                depth: crossing.zone.half_size().length() + TARGET_DEPTH,
                            }
                        }
                        Some(point) => TaskPlace::InRoom(point),
                    }
                },
            )
        } else {
            TaskPlace::Result(waiting(RouteWait::NotApplicable).errand)
        };
        let key = NavGraphKey::from_stamp(stamp, model.spec(), abilities.abilities,
            kinematics.canonical_size, kinematics.material, *base_size, frame.get());
        let key = match method {
            NavigationMethod::Approximate | NavigationMethod::Hybrid => key.estimated(),
            NavigationMethod::ConnectedGround => key.connected_ground(),
            _ => key,
        };
        let Some(map) = cache.graphs.ground(&key, world) else {
            advice.by_body.insert(entity, waiting(RouteWait::Unsupported));
            continue;
        };
        let feet = if ground.on_ground && map.surface_at(physical_feet).is_none() {
            // A ramp can rise from a flat support. Native routes can start at that support.
            map.surface_under(physical_feet, map.half.y * 2.0)
                .map_or(physical_feet, |under| map.point_on(under, map.frame.along(physical_feet)))
        } else {
            physical_feet
        };

        let mut next = route_goal.and_then(|goal| map.walk(physical_feet, goal));
        let mut waypoints = request.destination_seed.and_then(|seed| {
            if method == NavigationMethod::LocalGround {
                map.local_waypoints(physical_feet, seed)
            } else {
                map.only_support_waypoints(physical_feet, seed)
            }
        });
        let beside = size.x + TARGET_ROOM;
        let approach = attended.and_then(|at| map.approach(feet, at, beside, TARGET_DEPTH));
        let target_shares_surface = approach.is_some_and(|(from, to, _)| from == to);
        let mut target_place = approach.and_then(|(from, to, place)| {
            (from == to && map.walk(physical_feet, place).is_some()).then_some(place)
        });
        let mut task_needs_graph = false;
        let mut errand = task.sight(&map, feet, failed_task_goal, |_, _, place| {
            if map.walk(physical_feet, place).is_some() {
                Ok(())
            } else {
                task_needs_graph = true;
                Err(ErrandSight::Waiting(RouteWait::Unsupported))
            }
        });
        let route_needs_graph = next.is_none() && route_goal.is_some_and(|goal| map.surface_at(goal).is_some());
        let roaming_needs_graph = request.destination_seed.is_some() && waypoints.is_none();
        if method != NavigationMethod::LocalGround
            && ((map.surface_at(feet).is_some() && (route_needs_graph || roaming_needs_graph))
                || (approach.is_some() && target_place.is_none())
                || task_needs_graph)
        {
            cache.by_body.insert(entity, key);
            let mut refine = None;
            if let Some(graph) = cache.graph_of(world, key) {
                if let Some(goal) = route_goal.filter(|_| next.is_none()) {
                    if method == NavigationMethod::Hybrid {
                        refine = Some(goal);
                    } else {
                        next = Some(graph.next(feet, goal));
                    }
                }
                if let Some(seed) = request.destination_seed.filter(|_| waypoints.is_none()) {
                    waypoints = Some(graph.waypoints(feet, seed));
                }
                if let Some((from, to, place)) = approach {
                    if target_place.is_none() && graph.reaches(from, to) {
                        target_place = Some(place);
                    }
                }
                if task_needs_graph {
                    errand = task.sight(&map, feet, failed_task_goal, |from, to, _| {
                        graph.reaches(from, to).then_some(()).ok_or(ErrandSight::NoRoute)
                    });
                }
            }
            if let Some(goal) = refine {
                let (answer, work) = cache.graphs.refined_next(key, world, &[], feet, goal, REFINEMENTS_PER_LEG);
                next = Some(answer);
                cache.refinement.add(work);
            }
        }
        let (waypoints, waypoint_count) = waypoints.unwrap_or_default();
        advice.by_body.insert(entity, NavAdvice {
            errand,
            waypoints,
            waypoint_count,
            next: next.unwrap_or_else(|| waiting(if ground.on_ground { RouteWait::Unsupported } else { RouteWait::NotApplicable }).next),
            target_place,
            target_shares_surface,
            ..waiting(RouteWait::Unsupported)
        });
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
        graph.map(|graph| (graph.ground.frame.down, graph.ground.surfaces.len(), graph.links.len()))
    }

    fn ask(cache: &mut RoomNavigation, world: &World, frame: MotionFrame) -> Option<(Vec2, usize, usize)> {
        let abilities = AbilitySet { move_horizontal: true, jump: true, ..AbilitySet::NONE };
        let body = ae::BodyClusterScratch::new_with_abilities(Vec2::ZERO, abilities);
        let stamp = ambition_platformer2d_world::navigation::geometry_stamp(world, &[]);
        let key = NavGraphKey::from_stamp(stamp, body.model.spec(), abilities, body.kinematics.canonical_size,
            body.kinematics.material, body.base_size, frame);
        read(cache.graph_of(world, key))
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

/// The navigation of a room, for observation (GOBS-3,
/// `docs/planning/engine/geometry-observability-and-f1.md`): `navigation.graph`
/// reads what the navigation overlay reads. Each graph a body was advised
/// from this tick in the asked room (its standing surfaces, `nav_surface`;
/// its hops, `nav_hop`, and drops, `nav_drop`, from take-off to landing), and
/// each advised body in the room: its route through its waypoints
/// (`nav_route`), the place it is going to (`nav_goal`) and the place beside
/// its target (`nav_target_place`). Kept apart from the collision observers,
/// as a navigation view is apart from a collision view.
pub fn install_geometry_observer(app: &mut bevy::app::App) {
    use ambition_geometry_observation::{GeometryObserverAppExt as _, ObserverInfo};
    app.add_geometry_observer(
        ObserverInfo {
            name: "navigation.graph",
            description: "the surface graphs bodies were advised from this tick (surfaces, hops, drops) and each body's route and goal",
            needs_subject: false,
        },
        observe_navigation,
    );
}

fn observe_navigation(
    query: bevy::prelude::In<ambition_geometry_observation::ObservationQuery>,
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    roots: Query<&ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
    bodies: Query<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
    names: Query<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
    navigation: Option<bevy::prelude::Res<RoomNavigation>>,
    advice: Option<bevy::prelude::Res<NavigationAdvice>>,
) -> Vec<ambition_geometry_observation::GeometryObservation> {
    use ambition_geometry_observation::{GeometryObservation, ObservedShape, ObservedState};
    use std::borrow::Cow;
    let room = match (query.room, query.subject) {
        (Some(root), _) => roots.get(root).ok().map(|room| ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance(*room)),
        (None, Some(subject)) => bodies.get(subject).ok().copied(),
        (None, None) => None,
    };
    let Some(walls) = collision.room(room.as_ref()) else {
        return Vec::new();
    };
    let piece = |key: String, role: &'static str, shape: ObservedShape| GeometryObservation {
        key,
        roles: vec![Cow::Borrowed(role)],
        shape,
        state: ObservedState::Active,
    };
    let mut out = Vec::new();
    let stamp = ambition_platformer2d_world::navigation::geometry_stamp(walls.base(), &[]);
    let graphs: Vec<_> = navigation.iter().flat_map(|navigation| {
        navigation.graphs.iter().filter(|(key, _)| key.room_stamp() == stamp && navigation.in_use.contains(key)
            && query.subject.is_none_or(|body| navigation.by_body.get(&body) == Some(key)))
            .filter_map(|(_, graph)| graph)
    }).collect();
    for (g, graph) in graphs.into_iter().enumerate() {
        for (k, surface) in graph.ground.surfaces.iter().enumerate() {
            let (a, b) = (graph.ground.frame.point(surface.left, surface.top), graph.ground.frame.point(surface.right, surface.top));
            out.push(piece(format!("graph{g}/surface{k}"), "nav_surface", ObservedShape::Segment { a: a.to_array(), b: b.to_array() }));
        }
        for (k, link) in graph.links.iter().enumerate() {
            let role = if link.leg.kind == ae::navigation::NavLegKind::Drop { "nav_drop" } else { "nav_hop" };
            out.push(piece(
                format!("graph{g}/link{k}"),
                role,
                ObservedShape::Segment { a: link.leg.takeoff.to_array(), b: link.leg.land.to_array() },
            ));
        }
    }
    for (body, advised) in advice.iter().flat_map(|advice| advice.iter()) {
        if query.subject.is_some_and(|subject| subject != body) || (room.is_some() && bodies.get(body).ok() != room.as_ref()) {
            continue;
        }
        let name = names.get(body).map_or_else(|_| format!("{body}"), |id| id.as_str().to_owned());
        let mut at = advised.feet;
        for (k, waypoint) in advised.waypoints.iter().take(advised.waypoint_count as usize).enumerate() {
            out.push(piece(format!("{name}/route{k}"), "nav_route", ObservedShape::Segment { a: at.to_array(), b: waypoint.to_array() }));
            at = *waypoint;
        }
        if let Some(goal) = advised.goal {
            out.push(piece(format!("{name}/goal"), "nav_goal", ObservedShape::Circle { center: goal.to_array(), radius: 10.0 }));
        }
        if let Some(place) = advised.target_place {
            out.push(piece(format!("{name}/target_place"), "nav_target_place", ObservedShape::Circle { center: place.to_array(), radius: 10.0 }));
        }
    }
    out
}
