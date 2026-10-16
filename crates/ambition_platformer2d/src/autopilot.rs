//! A seat that navigation drives: the player's own body goes to a goal by the
//! room's surface graph, with the input a person would give.
//!
//! A game's autoplay policy says what it wants this frame ([`AutopilotOrder`]):
//! nothing, one frame of its own input, or a goal. For a goal the autopilot
//! takes the surface graph of the player's room for the player's own body,
//! follows the route with [`RouteFollower`] (the follower every navigating
//! body uses, with `follow_leg`, the rule each leg of the graph was rolled out
//! with), and writes each step's input into [`ScriptedControls`]. So the
//! simulation receives what a hand on a pad gives it: the same seat, the same
//! control gate, the same kernel.
//!
//! The autopilot is an INPUT SOURCE, outside the simulation's timeline, as a
//! person is. Its graphs are built on the async compute pool, never on the
//! main thread: until a graph is there, the seat is at rest. They are kept in
//! a [`NavGraphCache`] by what each one is a function of, and the geometry is
//! composed again only when a cheap signature of its inputs changes.
//!
//! The graph is built over what the body collides with that does not move:
//! the room's authored blocks, its contributed solids and gate solids, less
//! the blocks taken out. Exits that take a body on overlap are avoided: a leg
//! through one would leave the room. A moving platform is not in it; a leg
//! that one stops fails, and the follower plans again.

use bevy::prelude::*;
use bevy::tasks::{futures::check_ready, AsyncComputeTaskPool, Task};

use ambition_platformer2d_core as ae;
use ae::navigation::{approach_point, leg_stick_y, FollowFacts, Followed, LegFacts, LegInput, RouteFollower};
use ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame;
use ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance;
use ambition_platformer2d_shared_tangle::markers::PrimaryPlayer;
use ambition_platformer2d_world::collision::{world_with_sandbox_solids, CollisionWorld};
use ambition_platformer2d_world::navigation::{NavGraph, NavGraphCache, NavGraphKey};
use ambition_platformer2d_world::rooms::LiveRoomSpecs;

use crate::scripted_input::ScriptedControls;

/// The graphs kept: rooms times the forms the player takes in them.
const GRAPHS_KEPT: usize = 8;
/// After a goal is given up, frames before the autopilot tries it again: the
/// room can change (a gate opens, a block breaks).
const RETRY_FRAMES: u32 = 60;
/// A stick past this is a press of that direction, for the `*_pressed` edges.
const PRESS_THRESHOLD: f32 = 0.5;
/// A point farther than this is "far" to the walk (px), as for a brain.
const FAR_BEYOND: f32 = 160.0;
/// The walk to a far point, as a part of the body's top speed.
const FAR_PACE_FRACTION: f32 = 0.5;
/// The walk to a near point (px/s): slow enough to stop on it.
const NEAR_PACE: f32 = 120.0;

/// Where a goal is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NavGoal {
    /// Stand on the surface under this point, as near to it as the body can.
    Stand(ae::Vec2),
}

/// What the autoplay policy wants of the seat this frame. Written by a game's
/// policy in `scripted_input::AutoplayPolicy`, read by [`drive_autopilot`].
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub enum AutopilotOrder {
    /// The seat is at rest.
    #[default]
    Idle,
    /// This input, this frame: a press of the game's own (a transformation).
    Press(ae::ControlFrame),
    /// Go to the goal by the room's graph.
    GoTo(NavGoal),
}

/// What the autopilot came to on its last frame. For tests and instruments.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AutopilotStatus {
    #[default]
    Idle,
    /// The graph of the room for this body is being built.
    Building,
    /// On the way to the goal.
    Going,
    /// At the goal.
    Arrived,
    /// No route to the goal, or its legs failed: tried again after a time.
    NoRoute,
    /// The body has no graph in this room (the envelope cannot measure it).
    NoGraph,
}

/// What a graph is a function of in the body: a form with another tuning has
/// another graph.
#[derive(Clone, Copy, Debug, PartialEq)]
struct BodyTuning {
    spec: ae::movement::MotionModelSpec,
    abilities: ae::AbilitySet,
    size: ae::Vec2,
    base_size: ae::BodyBaseSize,
    frame: ae::MotionFrame,
}

/// A graph that is being built, and the key it is for.
struct PendingGraph {
    key: NavGraphKey,
    task: Task<Option<NavGraph>>,
}

/// The autopilot's state: the follower, the graphs and the one being built.
#[derive(Resource)]
pub struct Autopilot {
    follower: RouteFollower,
    graphs: NavGraphCache,
    pending: Option<PendingGraph>,
    /// The geometry signature and the body on the frame `wanted` was made.
    signature: u64,
    body: Option<BodyTuning>,
    /// The graph of the room as it is now. It is followed once it is built
    /// and no leg is under way: until then the body goes on by `active`.
    wanted: Option<NavGraphKey>,
    /// The graph the follower's legs are from, and the room and body it is
    /// for: in another room, or for another body, it is no graph at all.
    active: Option<NavGraphKey>,
    active_for: Option<(Option<InRoomInstance>, BodyTuning)>,
    /// The goal the follower holds, as the order gave it.
    goal: Option<NavGoal>,
    retry_in: u32,
    /// Frames before a recovery may spend another air jump.
    air_jump_wait: u32,
    last: ae::ControlFrame,
    status: AutopilotStatus,
}

impl Default for Autopilot {
    fn default() -> Self {
        Self {
            follower: RouteFollower::default(),
            graphs: NavGraphCache::with_capacity(GRAPHS_KEPT),
            pending: None,
            signature: 0,
            body: None,
            wanted: None,
            active: None,
            active_for: None,
            goal: None,
            retry_in: 0,
            air_jump_wait: 0,
            last: ae::ControlFrame::default(),
            status: AutopilotStatus::Idle,
        }
    }
}

impl Autopilot {
    pub fn status(&self) -> AutopilotStatus {
        self.status
    }

    /// The graph the autopilot follows now, if it is built.
    pub fn graph(&self) -> Option<&NavGraph> {
        self.active.as_ref().and_then(|key| self.graphs.get(key)).flatten()
    }
}

/// Where the autopilot runs: after the game's policy, before the script is
/// written into the seat.
#[derive(bevy::ecs::schedule::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AutopilotDrive;

/// Install the autopilot. `scripted_input::install_autoplay` calls it.
pub fn install(app: &mut App) {
    app.init_resource::<AutopilotOrder>();
    app.init_resource::<Autopilot>();
    app.configure_sets(
        Update,
        AutopilotDrive.after(crate::scripted_input::AutoplayPolicy),
    );
    app.add_systems(Update, (drive_autopilot, report_autopilot).chain().in_set(AutopilotDrive));
}

type PlayerBody<'a> = (
    &'a ae::BodyKinematics,
    &'a ae::BodyGroundState,
    Option<&'a ae::BodyWallState>,
    &'a ae::movement::MotionModel,
    &'a ae::BodyAbilities,
    &'a ae::BodyBaseSize,
    &'a ResolvedMotionFrame,
    Option<&'a InRoomInstance>,
);

/// One frame of the autopilot: the order in, the seat's input out.
pub fn drive_autopilot(
    order: Res<AutopilotOrder>,
    mut pilot: ResMut<Autopilot>,
    mut script: ResMut<ScriptedControls>,
    time: Res<Time>,
    collision: CollisionWorld,
    specs: Option<LiveRoomSpecs>,
    players: Query<(Entity, PlayerBody), With<PrimaryPlayer>>,
) {
    let pilot = &mut *pilot;
    let goal = match *order {
        AutopilotOrder::Idle => {
            pilot.follower.stop();
            pilot.goal = None;
            pilot.status = AutopilotStatus::Idle;
            script.0 = pilot.edges(LegInput::default());
            return;
        }
        AutopilotOrder::Press(frame) => {
            script.0 = frame;
            pilot.last = frame;
            return;
        }
        AutopilotOrder::GoTo(goal) => goal,
    };
    let Ok((entity, (kin, ground, wall, model, abilities, base_size, frame, in_room))) = players.single() else {
        script.0 = pilot.edges(LegInput::default());
        return;
    };
    let Some(room) = collision.room(in_room) else {
        script.0 = pilot.edges(LegInput::default());
        return;
    };
    let motion = frame.get();
    let body = BodyTuning {
        spec: model.spec(),
        abilities: abilities.abilities,
        size: kin.size,
        base_size: *base_size,
        frame: motion,
    };

    // The graph's inputs: one number for the geometry, with no allocation,
    // and the body. When both are the last frame's, so is the key.
    let overlay = room.overlay();
    let signature = geometry_signature(in_room, room.base(), overlay);
    if pilot.wanted.is_none() || signature != pilot.signature || pilot.body != Some(body) {
        let exits: Vec<ae::Aabb> = specs
            .as_ref()
            .and_then(|specs| specs.spec_of(entity))
            .map(|spec| spec.loading_zones.iter().filter(|zone| zone.is_ready(false)).map(|zone| zone.aabb).collect())
            .unwrap_or_default();
        let world = match overlay {
            Some(overlay) => world_with_sandbox_solids(room.base(), &[], overlay),
            None => room.base().clone(),
        };
        let key = NavGraphKey::new(&world, &exits, body.spec, body.abilities, body.size, body.base_size, motion);
        // A graph not kept and not on the way is built. A build for an
        // older geometry is dropped, which cancels it.
        if pilot.graphs.get(&key).is_none() && pilot.pending.as_ref().is_none_or(|pending| pending.key != key) {
            let task = AsyncComputeTaskPool::get()
                .spawn(async move { NavGraph::build_avoiding(&world, &exits, &key.probe_body(), motion) });
            pilot.pending = Some(PendingGraph { key, task });
        }
        pilot.wanted = Some(key);
        pilot.signature = signature;
        pilot.body = Some(body);
    }
    if let Some(pending) = pilot.pending.as_mut() {
        if let Some(graph) = check_ready(&mut pending.task) {
            let key = pending.key;
            pilot.pending = None;
            pilot.graphs.insert(key, graph);
        }
    }
    // The room changed (a block broke, a gate opened): the new graph is
    // followed once it is built, and not in the middle of a leg. A leg under
    // way goes on, and the follower plans again from the new graph after it.
    // In another room, or in another body (a form), the active graph is no
    // graph: drop it at once, and wait for the room's own.
    if pilot.active_for.is_some_and(|active_for| active_for != (in_room.copied(), body)) {
        pilot.active = None;
        pilot.active_for = None;
        pilot.follower.stop();
    }
    if pilot.active != pilot.wanted
        && pilot.wanted.is_some_and(|wanted| pilot.graphs.get(&wanted).is_some())
        && !pilot.follower.committed()
    {
        pilot.active = pilot.wanted;
        pilot.active_for = Some((in_room.copied(), body));
        pilot.follower.stop();
    }
    let Some(key) = pilot.active else {
        pilot.status = AutopilotStatus::Building;
        script.0 = pilot.edges(LegInput::default());
        return;
    };
    let graph = match pilot.graphs.get(&key) {
        None => {
            pilot.status = AutopilotStatus::Building;
            script.0 = pilot.edges(LegInput::default());
            return;
        }
        Some(None) => {
            pilot.status = AutopilotStatus::NoGraph;
            script.0 = pilot.edges(LegInput::default());
            return;
        }
        Some(Some(graph)) => graph,
    };

    let feet = kin.pos + graph.frame.down * graph.half.y;
    // A new goal, or the last one was reached or given up and its wait is
    // over. A leg past its start is not dropped for a new goal: the body is
    // in its run-up or in the air.
    let renew = pilot.goal != Some(goal) || (pilot.follower.goal.is_none() && pilot.retry_in == 0);
    if renew && !pilot.follower.committed() {
        match stand_point(graph, goal) {
            Some(point) => pilot.follower.go_to(point),
            None => pilot.follower.stop(),
        }
        pilot.goal = Some(goal);
    }
    pilot.retry_in = pilot.retry_in.saturating_sub(1);
    let facts = FollowFacts {
        now: time.elapsed_secs(),
        leg: LegFacts {
            feet,
            vel: kin.vel,
            on_ground: ground.on_ground,
            side: graph.frame.side,
            down: graph.frame.down,
            dt: time.delta_secs().max(1.0 / 240.0),
            wall: wall.and_then(|wall| wall.on_wall.then_some(wall.wall_normal_x.signum())),
            // A player runs at its top speed and stops by its own braking.
            approach: graph.approach_law(),
        },
        walk_speed: graph.run_speed,
        answer: pilot.follower.needs_route().map(|goal| (goal, graph.next(feet, goal))),
    };
    let step = pilot.follower.step(&facts);
    pilot.status = match step.followed {
        Followed::Idle if pilot.retry_in > 0 => AutopilotStatus::NoRoute,
        Followed::Idle => AutopilotStatus::Idle,
        Followed::Going => AutopilotStatus::Going,
        Followed::Arrived => AutopilotStatus::Arrived,
        Followed::GaveUp => {
            pilot.retry_in = RETRY_FRAMES;
            AutopilotStatus::NoRoute
        }
    };
    let input = match step.input {
        // A leg's input that is not at full speed is a fraction of the body's
        // walk, and the walk is the owner's to choose, as a brain's pace is: a
        // body that comes to a point at its top speed runs past it (Mary-O's
        // cosmic form runs at 620 px/s, and ran off the edge a hop starts at).
        Some((mut input, leg)) => {
            if !input.full_speed {
                let far = (approach_point(&leg) - feet).dot(graph.frame.side).abs() > FAR_BEYOND;
                let pace = if far { graph.run_speed * FAR_PACE_FRACTION } else { NEAR_PACE.min(graph.run_speed) };
                input.axis *= pace / graph.run_speed.max(1.0);
            }
            input
        }
        // In the air with no leg: a leg failed there. Come down where the
        // body can, and not where it falls.
        None if !ground.on_ground => {
            let rise = if abilities.abilities.double_jump { graph.apex_rise } else { 0.0 };
            let toward = pilot.follower.goal.unwrap_or(feet);
            match graph.landing_in_reach(feet, rise, toward) {
                Some(landing) => recover_to(graph, feet, kin.vel, landing, abilities.abilities.double_jump, &mut pilot.air_jump_wait),
                None => LegInput::default(),
            }
        }
        None => LegInput::default(),
    };
    script.0 = pilot.edges(input);
}

/// Frames a going body stands still before the report says so.
const STALL_FRAMES: u32 = 120;

/// Frames between two air jumps of a recovery: a press is an edge, and the
/// jump it gives must rise before the next is judged.
const AIR_JUMP_FRAMES: u32 = 12;

/// The input that brings a body in the air at `feet` down on `landing`: steer
/// to it at full speed, and, with an air jump, spend one when the body falls
/// below the landing's height.
fn recover_to(graph: &NavGraph, feet: ae::Vec2, vel: ae::Vec2, landing: ae::Vec2, air_jump: bool, wait: &mut u32) -> LegInput {
    *wait = wait.saturating_sub(1);
    let along = (landing - feet).dot(graph.frame.side);
    let below = (landing - feet).dot(graph.frame.down);
    let falling = vel.dot(graph.frame.down) > 0.0;
    let jump = air_jump && falling && below < 0.0 && *wait == 0;
    if jump {
        *wait = AIR_JUMP_FRAMES;
    }
    LegInput {
        axis: (along / 16.0).clamp(-1.0, 1.0),
        full_speed: true,
        jump_pressed: jump,
        jump_held: jump || (air_jump && below < 0.0),
        ..LegInput::default()
    }
}

/// One log line for each change of the autopilot's status: what it came to,
/// where the body is, and the graph it follows. Changes are rare.
///
/// Also one line for each death of the driven body: where it was, and the leg
/// it was on. A death under the autopilot is a fact about navigation or about
/// the game's policy, and this line says which leg to look at.
pub fn report_autopilot(
    pilot: Res<Autopilot>,
    mut last: Local<Option<AutopilotStatus>>,
    mut still: Local<(ae::Vec2, u32)>,
    players: Query<(Entity, &ae::BodyKinematics), With<PrimaryPlayer>>,
    deaths: Option<Res<Messages<crate::combat::death_rules::ActorDiedMessage>>>,
) {
    if let (Some(deaths), Ok((player, kin))) = (deaths, players.single()) {
        if deaths.iter_current_update_messages().any(|death| death.victim == player) {
            eprintln!(
                "[autopilot] died at={:?} vel={:?} leg={:?} phase={:?} goal={:?}",
                kin.pos,
                kin.vel,
                pilot.follower.leg.map(|leg| (leg.kind, leg.start, leg.land)),
                pilot.follower.phase,
                pilot.follower.goal
            );
        }
    }
    // A body that does not move while it goes: say once what it is given.
    if let Ok((_, kin)) = players.single() {
        let (at, frames) = *still;
        if pilot.status == AutopilotStatus::Going && (kin.pos - at).length() < 1.0 {
            *still = (at, frames + 1);
            if frames + 1 == STALL_FRAMES {
                eprintln!(
                    "[autopilot] stalled at={:?} vel={:?} input={:?} leg={:?} phase={:?}",
                    kin.pos,
                    kin.vel,
                    pilot.last,
                    pilot.follower.leg.map(|leg| (leg.kind, leg.start, leg.takeoff, leg.land)),
                    pilot.follower.phase
                );
            }
        } else {
            *still = (kin.pos, 0);
        }
    }
    if *last == Some(pilot.status) {
        return;
    }
    *last = Some(pilot.status);
    let at = players.single().map(|(_, kin)| kin.pos).ok();
    let graph = pilot
        .graph()
        .map(|graph| format!("surfaces={} links={} rollouts={}", graph.surfaces.len(), graph.links.len(), graph.cost.rollouts))
        .unwrap_or_else(|| "no graph".to_owned());
    eprintln!(
        "[autopilot] status={:?} at={at:?} goal={:?} follower_goal={:?} {graph}",
        pilot.status, pilot.goal, pilot.follower.goal
    );
}

impl Autopilot {
    /// `input` as a seat's frame, with the press and release edges against
    /// the last frame, as a device gives them.
    fn edges(&mut self, input: LegInput) -> ae::ControlFrame {
        let last = self.last;
        let axis_y = leg_stick_y(input);
        let frame = ae::ControlFrame {
            axis_x: input.axis,
            axis_y,
            jump_pressed: input.jump_pressed,
            jump_held: input.jump_held || input.jump_pressed,
            jump_released: last.jump_held && !(input.jump_held || input.jump_pressed),
            burst_pressed: input.dash_pressed,
            left_pressed: input.axis < -PRESS_THRESHOLD && last.axis_x >= -PRESS_THRESHOLD,
            right_pressed: input.axis > PRESS_THRESHOLD && last.axis_x <= PRESS_THRESHOLD,
            up_pressed: axis_y < -PRESS_THRESHOLD && last.axis_y >= -PRESS_THRESHOLD,
            down_pressed: axis_y > PRESS_THRESHOLD && last.axis_y <= PRESS_THRESHOLD,
            fly_toggle_pressed: input.fly_toggle,
            ..ae::ControlFrame::default()
        };
        self.last = frame;
        frame
    }
}

/// The feet point of `goal` on the graph: on the surface under it, nearest it.
fn stand_point(graph: &NavGraph, goal: NavGoal) -> Option<ae::Vec2> {
    let NavGoal::Stand(at) = goal;
    (0..graph.surfaces.len())
        .map(|index| graph.point_on(index, graph.frame.along(at)))
        .filter(|feet| graph.frame.below(*feet) >= graph.frame.below(at) - 1.0)
        .min_by(|a, b| (*a - at).length_squared().total_cmp(&(*b - at).length_squared()))
}

/// One number for the geometry a graph of the player's room is made from,
/// folded from its parts with no allocation: the live room (its authored
/// blocks are fixed for it), each contributed block and gate solid that does
/// not move, the carves, and the names of the blocks taken out. Equal to the
/// last frame's: the same geometry.
fn geometry_signature(
    room: Option<&InRoomInstance>,
    base: &ae::World,
    overlay: Option<&ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay>,
) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::hash::DefaultHasher::new();
    room.hash(&mut hasher);
    base.blocks.len().hash(&mut hasher);
    let mut fold_box = |aabb: ae::Aabb| {
        for value in [aabb.min.x, aabb.min.y, aabb.max.x, aabb.max.y] {
            value.to_bits().hash(&mut hasher);
        }
    };
    if let Some(overlay) = overlay {
        for block in overlay.blocks.iter().chain(&overlay.gate_solids) {
            if block.velocity == ae::Vec2::ZERO {
                fold_box(block.aabb);
            }
        }
        // Not the portal holes: a hole is open only for the bodies that opened
        // it, and the graph is of the walls as a body meets them otherwise.
        for carve in &overlay.climbable_carves {
            fold_box(*carve);
        }
        overlay.removed_block_names.hash(&mut hasher);
    }
    hasher.finish()
}
