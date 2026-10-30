//! Walk the player out of its room with its own inputs: the surface graph of
//! the room for the player's own body, the input `follow_leg` gives for each
//! leg, Interact at a door, and the shipped room transition. No step puts the
//! body anywhere.
//!
//! The graph is built over the room as the body collides with it (the
//! authored blocks and each standing gate solid), with the other exits that
//! fire on overlap avoided: a leg through one takes the body to another room.
//!
//! Two drivers step a frame: the sim harness, by an `AgentAction`, and the
//! shipped App, by the keys of the arrows-and-ZXC preset. While a cutscene
//! plays, the walker presses confirm to dismiss its beats.

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::navigation::{follow_leg, LegFacts, LegPhase, LegProgress, NavLeg, NavNext};
use ambition_platformer2d::engine_core::AabbExt as _;
use ambition_platformer2d::world::navigation::NavGraph;
use bevy::prelude::{Entity, World};

/// The most frames one room may take to walk (60 s).
pub const FRAMES_FOR_A_ROOM: usize = 3600;

/// One frame of a player's input.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Press {
    /// The stick along the side axis, -1 to 1.
    pub axis: f32,
    /// Hold up: climb a wall, pull up from a ledge.
    pub up: bool,
    /// Hold down: with a jump press, fall through a one-way surface.
    pub down: bool,
    pub jump_pressed: bool,
    pub jump_held: bool,
    /// Hold Interact.
    pub interact: bool,
    /// Press the flight toggle.
    pub fly_toggle: bool,
    /// Press the dash (the burst).
    pub dash: bool,
    /// Press confirm: dismiss a beat of a cutscene.
    pub confirm: bool,
    /// Press Attack.
    pub attack: bool,
}

/// A session that a player's input steps one frame at a time.
pub trait Walker {
    fn world(&self) -> &World;
    fn world_mut(&mut self) -> &mut World;
    fn step(&mut self, press: Press);
}

/// The sim harness, stepped by an `AgentAction` with an analog stick.
pub struct Pad<'a> {
    pub sim: &'a mut ambition_app::Platformer2dSimHarness,
    interact_was_held: bool,
}

impl<'a> Pad<'a> {
    pub fn new(sim: &'a mut ambition_app::Platformer2dSimHarness) -> Self {
        Self { sim, interact_was_held: false }
    }
}

impl Walker for Pad<'_> {
    fn world(&self) -> &World {
        self.sim.world()
    }

    fn world_mut(&mut self) -> &mut World {
        self.sim.world_mut()
    }

    fn step(&mut self, press: Press) {
        let interact = press.interact && !self.interact_was_held;
        self.interact_was_held = press.interact;
        self.sim.step(ambition_app::AgentAction {
            move_x: press.axis,
            // Up is toward -y in the body's local axes.
            move_y: match (press.up, press.down) {
                (true, false) => -1.0,
                (false, true) => 1.0,
                _ => 0.0,
            },
            jump: press.jump_pressed,
            jump_held: press.jump_held,
            interact,
            interact_held: press.interact,
            fly_toggle: press.fly_toggle,
            dash: press.dash,
            confirm: press.confirm,
            attack: press.attack,
            ..crate::common::base()
        });
    }
}

/// The shipped App, stepped by the keys of the arrows-and-ZXC preset. A key
/// is a button, so a part of the stick is the arrow held on that part of the
/// frames, as a player taps it to creep.
pub struct Keys<'a> {
    pub app: &'a mut bevy::app::App,
    held: Vec<bevy::input::keyboard::KeyCode>,
    /// The part of a frame of arrow owed, from the stick of the frames before.
    duty: f32,
    /// A jump pressed while Jump is held is pressed on the next frame: a
    /// press is an edge, so the key is up for one frame first.
    jump_owed: bool,
}

impl<'a> Keys<'a> {
    pub fn new(app: &'a mut bevy::app::App) -> Self {
        Self { app, held: Vec::new(), duty: 0.0, jump_owed: false }
    }

    fn hold(&mut self, key: bevy::input::keyboard::KeyCode, down: bool) {
        use leafwing_input_manager::prelude::Buttonlike;
        let held = self.held.contains(&key);
        if down && !held {
            Buttonlike::press(&key, self.app.world_mut());
            self.held.push(key);
        } else if !down && held {
            Buttonlike::release(&key, self.app.world_mut());
            self.held.retain(|other| *other != key);
        }
    }
}

impl Drop for Keys<'_> {
    fn drop(&mut self) {
        for key in std::mem::take(&mut self.held) {
            leafwing_input_manager::prelude::Buttonlike::release(&key, self.app.world_mut());
        }
    }
}

impl Walker for Keys<'_> {
    fn world(&self) -> &World {
        self.app.world()
    }

    fn world_mut(&mut self) -> &mut World {
        self.app.world_mut()
    }

    fn step(&mut self, press: Press) {
        use bevy::input::keyboard::KeyCode;
        self.duty += press.axis.abs().min(1.0);
        let arrow = self.duty >= 1.0;
        if arrow {
            self.duty -= 1.0;
        }
        if press.axis == 0.0 {
            self.duty = 0.0;
        }
        self.hold(KeyCode::ArrowLeft, arrow && press.axis < 0.0);
        self.hold(KeyCode::ArrowRight, arrow && press.axis > 0.0);
        self.hold(KeyCode::ArrowUp, press.up);
        self.hold(KeyCode::ArrowDown, press.down);
        self.hold(KeyCode::KeyF, press.interact);
        // Utility toggles flight; a toggle is a press, so the key is up again
        // on the next frame.
        self.hold(KeyCode::KeyD, press.fly_toggle);
        // Burst dashes; a press, so the key is up again on the next frame.
        self.hold(KeyCode::KeyC, press.dash);
        self.hold(KeyCode::Enter, press.confirm);
        // Attack is a press, so the key is up again on the next frame.
        self.hold(KeyCode::KeyX, press.attack);
        let jump_is_held = self.held.contains(&KeyCode::KeyZ);
        if self.jump_owed {
            self.jump_owed = false;
            self.hold(KeyCode::KeyZ, true);
        } else if press.jump_pressed && jump_is_held {
            self.jump_owed = true;
            self.hold(KeyCode::KeyZ, false);
        } else {
            self.hold(KeyCode::KeyZ, press.jump_pressed || press.jump_held);
        }
        self.app.update();
    }
}

/// The player's body entity.
pub fn the_player(world: &mut World) -> Entity {
    world
        .query_filtered::<Entity, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
        .single(world)
        .expect("one player")
}

/// The live room the player's body is in, by its room stamp: with a second
/// participant, two rooms can be live.
fn the_players_room(world: &World, player: Entity) -> Option<ambition_platformer2d::world::rooms::LiveRoomInstance> {
    world.get::<ambition_platformer2d::platformer::lifecycle::InRoomInstance>(player).map(|stamp| stamp.0)
}

/// The id of the live room the player's body is in.
pub fn the_players_room_id(world: &World, player: Entity) -> String {
    let rooms = ambition_platformer2d::platformer::lifecycle::session_world_component::<
        ambition_platformer2d::world::rooms::RoomSet,
    >(world)
    .expect("the session keeps its room set");
    let live = ambition_platformer2d::world::rooms::live_room_definition_in(world, the_players_room(world, player))
        .expect("the player's room is live");
    rooms.rooms[live.index()].id.clone()
}

/// The collision overlay (the gate solids) of the live room the player is in.
fn the_players_overlay(world: &mut World, player: Entity) -> ambition_platformer2d::world::FeatureEcsWorldOverlay {
    let Some(room) = the_players_room(world, player) else {
        return ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<
            ambition_platformer2d::world::FeatureEcsWorldOverlay,
        >(world)
        .expect("the live room has a collision overlay")
        .clone();
    };
    world
        .query::<(&ambition_platformer2d::world::rooms::LiveRoomInstance, &ambition_platformer2d::world::FeatureEcsWorldOverlay)>()
        .iter(world)
        .find(|(instance, _)| **instance == room)
        .map(|(_, overlay)| overlay.clone())
        .expect("the player's room has a collision overlay")
}

/// The player's body as a probe: its movement law, abilities and box.
fn the_players_body(world: &World, player: Entity) -> ae::BodyClusterScratch {
    let model = world.get::<ae::MotionModel>(player).expect("a movement law").clone();
    let abilities = world.get::<ae::BodyAbilities>(player).expect("abilities").abilities;
    let size = world.get::<ae::BodyKinematics>(player).expect("a body").size;
    let mut body = ae::BodyClusterScratch::new_with_abilities(ae::Vec2::ZERO, abilities);
    body.model = model;
    body.kinematics.size = size;
    body.base_size.base_size = size;
    body
}

/// The feet point on a surface where the body comes nearest to `zone`, an
/// overlap before a touch: a body that stands there is in an edge exit
/// beside the end of a floor.
fn nearest_the_zone(graph: &NavGraph, zone: ae::Aabb) -> Option<ae::Vec2> {
    let gap = |feet: ae::Vec2| {
        let body = ae::Aabb::new(feet - graph.frame.down * graph.half.y, graph.half);
        let dx = (zone.min.x - body.max.x).max(body.min.x - zone.max.x);
        let dy = (zone.min.y - body.max.y).max(body.min.y - zone.max.y);
        if dx > 0.0 || dy > 0.0 {
            dx.max(0.0).hypot(dy.max(0.0))
        } else {
            -(dx * dy)
        }
    };
    (0..graph.surfaces.len())
        .map(|index| graph.point_on(index, graph.frame.along(zone.center())))
        .min_by(|a, b| gap(*a).total_cmp(&gap(*b)))
}

/// The surface graph of the player's room for the player's own body. It is
/// built over the room as the body collides with it (the authored blocks and
/// each gate solid, a lock wall, that stands now), and each exit that fires
/// on overlap is avoided but `keep`: a leg through one takes the body to
/// another room.
fn the_players_graph(walker: &mut impl Walker, player: Entity, keep: Option<ae::Aabb>) -> Result<NavGraph, String> {
    let room = the_players_room_id(walker.world(), player);
    let rooms = ambition_platformer2d::platformer::lifecycle::session_world_component::<
        ambition_platformer2d::world::rooms::RoomSet,
    >(walker.world())
    .expect("the session keeps its room set")
    .clone();
    let definition = rooms.definition_by_id(&room).expect("the live room is authored");
    let exits: Vec<ae::Aabb> = rooms
        .spec(definition)
        .loading_zones
        .iter()
        .filter(|zone| zone.is_ready(false) && Some(zone.aabb) != keep)
        .map(|zone| zone.aabb)
        .collect();
    let overlay = the_players_overlay(walker.world_mut(), player);
    let world = ambition_platformer2d::world::collision::world_with_gate_solids_and_carves(
        &rooms.spec(definition).world,
        &overlay.gate_solids,
        &[],
        &[],
    )
    .into_owned();
    let frame = ae::MotionFrame::from_acceleration(ae::Vec2::new(0.0, ae::movement::GRAVITY)).expect("gravity");
    let body = the_players_body(walker.world(), player);
    NavGraph::build_avoiding(&world, &exits, &body, frame).ok_or_else(|| format!("`{room}` has no graph for the player"))
}

/// An exit a walk leaves its room by.
struct Exit<'a> {
    zone: ae::Aabb,
    door: bool,
    target: &'a str,
}

/// Walk from the live room through its exit to `target`, with the player's
/// own inputs. Returns the frames it took.
pub fn walk_through(walker: &mut impl Walker, target: &str) -> Result<usize, String> {
    walk_through_the_first_exit(walker, target, None)
}

/// [`walk_through`] by the exit whose zone id is `zone`: two exits of one
/// room can lead to the same room (a low road and a high road).
pub fn walk_through_zone(walker: &mut impl Walker, target: &str, zone: &str) -> Result<usize, String> {
    walk_through_the_first_exit(walker, target, Some(zone))
}

fn walk_through_the_first_exit(walker: &mut impl Walker, target: &str, zone_id: Option<&str>) -> Result<usize, String> {
    let player = the_player(walker.world_mut());
    let from = the_players_room_id(walker.world(), player);
    let rooms = ambition_platformer2d::platformer::lifecycle::session_world_component::<
        ambition_platformer2d::world::rooms::RoomSet,
    >(walker.world())
    .expect("the session keeps its room set")
    .clone();
    let definition = rooms.definition_by_id(&from).expect("the live room is authored");
    let (zone, door) = rooms
        .spec(definition)
        .loading_zones
        .iter()
        .find(|zone| {
            zone_id.is_none_or(|id| zone.id == id)
                && rooms
                .transition_for_player(definition, zone.aabb, ae::Vec2::ZERO, true)
                .is_some_and(|transition| rooms.rooms[transition.target_room].id == target)
        })
        .map(|zone| (zone.aabb, zone.activation == ambition_platformer2d::world::rooms::LoadingZoneActivation::Door))
        .ok_or_else(|| format!("`{from}` has no exit to `{target}` (zone {zone_id:?})"))?;
    let graph = the_players_graph(walker, player, Some(zone))?;
    let goal = nearest_the_zone(&graph, zone).ok_or("no surface near the exit")?;
    walk(walker, player, &graph, goal, Some(Exit { zone, door, target }))
}

/// Walk in the live room to stand on the surface under `at`, with the
/// player's own inputs. Returns the frames it took.
pub fn walk_to(walker: &mut impl Walker, at: ae::Vec2) -> Result<usize, String> {
    let player = the_player(walker.world_mut());
    let graph = the_players_graph(walker, player, None)?;
    let goal = (0..graph.surfaces.len())
        .map(|index| graph.point_on(index, graph.frame.along(at)))
        .filter(|feet| graph.frame.below(*feet) >= graph.frame.below(at) - 1.0)
        .min_by(|a, b| (*a - at).length().total_cmp(&(*b - at).length()))
        .ok_or("no surface under the place")?;
    walk(walker, player, &graph, goal, None)
}

/// Follow `graph` to `goal`, and through `exit` when there is one.
fn walk(walker: &mut impl Walker, player: Entity, graph: &NavGraph, goal: ae::Vec2, exit: Option<Exit<'_>>) -> Result<usize, String> {
    let from = the_players_room_id(walker.world(), player);
    let mut leg: Option<(NavLeg, LegPhase)> = None;
    // Arrived at an edge exit: the body walks on into it and does not turn
    // back to the goal. Past the end of a floor into a hole below, the body
    // that overhangs the edge is on no surface of the graph, and asking the
    // graph again sent it back to the goal: it rocked at the edge for good.
    let mut into_the_exit = false;
    for step in 0..FRAMES_FOR_A_ROOM {
        if exit.as_ref().is_some_and(|exit| the_players_room_id(walker.world(), player) == exit.target) {
            return Ok(step);
        }
        let world = walker.world();
        // A cutscene holds the seat's input, and a dialogue beat waits for a
        // dismiss: press confirm, as a player does, and start the leg again
        // after it.
        if world
            .get_resource::<ambition_platformer2d::cutscene::ActiveCutscene>()
            .is_some_and(|cutscene| cutscene.is_playing())
        {
            leg = None;
            walker.step(Press { confirm: step % 10 == 0, ..Press::default() });
            continue;
        }
        let kinematics = *world.get::<ae::BodyKinematics>(player).expect("the player has a body");
        let on_ground = world.get::<ae::BodyGroundState>(player).is_some_and(|ground| ground.on_ground);
        let wall = world
            .get::<ae::BodyWallState>(player)
            .and_then(|wall| wall.on_wall.then_some(wall.wall_normal_x.signum()));
        let feet = kinematics.pos + graph.frame.down * graph.half.y;
        let mut press = Press::default();
        // A flight that no leg steers (a fight can end in the air of a Fly
        // leg): turn it off, and land. A leg starts from the ground.
        if leg.is_none() && world.get::<ae::BodyFlightState>(player).is_some_and(|flight| flight.fly_enabled) {
            press.fly_toggle = true;
            walker.step(press);
            continue;
        }
        if into_the_exit {
            if let Some(exit) = &exit {
                // An exit over the body's head (a shaft up, a ladder's top):
                // hold up and climb, kept under it. One beside the body: walk
                // on into it.
                let head = graph.frame.below(feet) - 2.0 * graph.half.y;
                let overhead = graph.frame.below(exit.zone.max) <= head;
                let along = (exit.zone.center() - feet).dot(graph.frame.side);
                if overhead {
                    press.up = true;
                    press.axis = if along.abs() > 6.0 { along.signum() * 0.5 } else { 0.0 };
                    walker.step(press);
                    continue;
                }
                // One under the body's feet (a hole under a hatch): hold down
                // and jump, to drop through the one-way floor over it.
                let underfoot = graph.frame.below(exit.zone.min) >= graph.frame.below(feet) - 1.0;
                if on_ground && underfoot && along.abs() <= exit.zone.half_size().x {
                    press.down = true;
                    press.jump_pressed = step % 20 == 0;
                    walker.step(press);
                    continue;
                }
                if on_ground {
                    press.axis = along.signum();
                    walker.step(press);
                    continue;
                }
            }
        }
        if leg.is_none() && on_ground {
            match graph.next(feet, goal) {
                // At the exit. A door takes a fresh press of Interact. An edge
                // exit takes the body that is in it: the goal is the last
                // place to stand, so the body walks on into the zone, as a
                // player does (the zone can begin past the end of the floor).
                NavNext::Arrived => match &exit {
                    Some(exit) => {
                        press.interact = exit.door && step % 10 < 2;
                        if !exit.door {
                            press.axis = (exit.zone.center() - feet).dot(graph.frame.side).signum();
                            into_the_exit = true;
                        }
                    }
                    None => return Ok(step),
                },
                NavNext::Leg(next) => leg = Some((next, LegPhase::Approach)),
                NavNext::Unreachable => return Err(format!("no route from {feet:?} to {goal:?} in `{from}`")),
                // On no surface of the graph: a platform that moves. Walk
                // toward the goal, off it, to a surface the graph has.
                NavNext::Unknown => press.axis = (goal - feet).dot(graph.frame.side).signum(),
            }
        }
        if let Some((current, phase)) = leg {
            let facts = LegFacts {
                feet,
                vel: kinematics.vel,
                on_ground,
                side: graph.frame.side,
                down: graph.frame.down,
                dt: 1.0 / 60.0,
                wall,
                approach: None,
            };
            let (input, progress) = follow_leg(&current, phase, &facts);
            // A player walks at the top speed; the leg slows the stick
            // itself near a point it stops at.
            press.axis = input.axis;
            press.up = input.up;
            press.down = input.down;
            press.jump_pressed = input.jump_pressed;
            press.jump_held = input.jump_held;
            press.fly_toggle = input.fly_toggle;
            press.dash = input.dash_pressed;
            leg = match progress {
                LegProgress::Going(next) => Some((current, next)),
                LegProgress::Arrived | LegProgress::Failed => None,
            };
        }
        walker.step(press);
    }
    let at = walker.world().get::<ae::BodyKinematics>(player).map(|kinematics| kinematics.pos);
    Err(format!("still in `{from}` after {FRAMES_FOR_A_ROOM} frames, at {at:?}, aiming at {goal:?}"))
}

/// How near a foe is for the fight to swing at it: its edge within this
/// along the side axis of the player's edge, and its box within
/// `REACH_TALL` along DOWN. The robot's blade reaches about this far. It is
/// from the edge, so the body swings from beside a big foe and not from
/// inside it, where its touch does harm.
const BLADE_REACH: f32 = 24.0;
const REACH_TALL: f32 = 20.0;
/// Frames between two Attack presses: a press is an edge, and a swing must
/// play out before the next one starts.
const SWING_FRAMES: usize = 18;
/// Frames between two choices of the foe to go to: a foe moves, so the goal
/// is chosen again.
const RETARGET_FRAMES: usize = 30;

/// Fight in the live room with the player's own inputs until `done` says
/// the fight is over: go to the nearest live foe that `foe` names, by the
/// room's graph, and press Attack facing it when it is in reach. While no
/// foe stands, go back to `rally` and wait there (a wave comes to a player
/// in its arena). Returns the frames it took.
///
/// The graph is built again when the gate solids change: a fight shuts the
/// room with its lock walls.
pub fn fight(
    walker: &mut impl Walker,
    foe: impl Fn(&World, Entity) -> bool,
    done: impl Fn(&World) -> bool,
    rally: ae::Vec2,
    frames: usize,
) -> Result<usize, String> {
    let player = the_player(walker.world_mut());
    let room = the_players_room_id(walker.world(), player);
    let mut solids = gate_solid_names(walker, player);
    let mut graph = the_players_graph(walker, player, None)?;
    let mut leg: Option<(NavLeg, LegPhase)> = None;
    let mut goal: Option<ae::Vec2> = None;
    let mut last_swing = 0usize;
    // Deaths in the fight: a death starts the attempt again, and its fight
    // with it. Counted by the death the game announces, because a hazard
    // kills a body at full health.
    let mut deaths = 0usize;
    for step in 0..frames {
        if done(walker.world()) {
            return Ok(step);
        }
        deaths += walker
            .world()
            .get_resource::<bevy::prelude::Messages<ambition_platformer2d::combat::death_rules::ActorDiedMessage>>()
            .map_or(0, |died| died.iter_current_update_messages().filter(|death| death.victim == player).count());
        let now = gate_solid_names(walker, player);
        if now != solids {
            solids = now;
            graph = the_players_graph(walker, player, None)?;
            leg = None;
            goal = None;
        }
        let world = walker.world_mut();
        let kinematics = *world.get::<ae::BodyKinematics>(player).expect("the player has a body");
        let on_ground = world.get::<ae::BodyGroundState>(player).is_some_and(|ground| ground.on_ground);
        let wall = world
            .get::<ae::BodyWallState>(player)
            .and_then(|wall| wall.on_wall.then_some(wall.wall_normal_x.signum()));
        let flying = world.get::<ae::BodyFlightState>(player).is_some_and(|flight| flight.fly_enabled);
        let player_room = the_players_room(world, player);
        let mut bodies = world.query::<(Entity, &ae::BodyKinematics, &ambition_platformer2d::characters::actor::BodyHealth)>();
        let candidates: Vec<(Entity, ae::Vec2, ae::Vec2)> = bodies
            .iter(world)
            .filter(|(entity, _, health)| *entity != player && health.alive())
            .map(|(entity, kin, _)| (entity, kin.pos, kin.size * 0.5))
            .collect();
        let nearest = candidates
            .into_iter()
            .filter(|(entity, ..)| foe(world, *entity))
            .filter(|(entity, ..)| {
                world.get::<ambition_platformer2d::platformer::lifecycle::InRoomInstance>(*entity).map(|room| room.0)
                    == player_room
            })
            .min_by(|a, b| (a.1 - kinematics.pos).length().total_cmp(&(b.1 - kinematics.pos).length()));
        let feet = kinematics.pos + graph.frame.down * graph.half.y;
        let mut press = Press::default();
        let at = match nearest {
            Some((_, at, _)) => at,
            // No foe stands: to the rally place, unless the body is there.
            // At the rally place: in a fight's strip, which is narrow.
            None if (rally - kinematics.pos).dot(graph.frame.side).abs() <= ae::navigation::ARRIVE_TOLERANCE * 2.0 => {
                leg = None;
                goal = None;
                walker.step(press);
                continue;
            }
            None => rally,
        };
        let offset = at - kinematics.pos;
        let (along, tall) = (offset.dot(graph.frame.side), offset.dot(graph.frame.down));
        // Reach and the jump under a foe are for a foe; the rally place is
        // walked to.
        let foe_stands = nearest.is_some();
        // The foe's half box on the two axes, and the reach to its edge.
        let foe_half = nearest.map_or(ae::Vec2::ZERO, |(_, _, half)| half);
        let foe_half_along = (foe_half * graph.frame.side.abs()).length();
        let reach_along = foe_half_along + graph.half.x + BLADE_REACH;
        let reach_tall = (foe_half * graph.frame.down.abs()).length() + REACH_TALL;
        if foe_stands && along.abs() <= reach_along && tall.abs() <= reach_tall {
            // In reach: face it, and swing on an edge once the last swing is
            // over. In the air, keep the jump held.
            leg = None;
            goal = None;
            let facing = kinematics.facing;
            press.jump_held = !on_ground;
            if facing * along < 0.0 {
                press.axis = along.signum();
            } else if step >= last_swing + SWING_FRAMES {
                press.attack = true;
                last_swing = step;
            }
            walker.step(press);
            continue;
        }
        // A jump brings the blade as high as the feet rise, and a little
        // more: a foe higher than that is on a ledge, which a leg gets to.
        let jump_tall = graph.apex_rise + reach_tall;
        if foe_stands
            && leg.is_none()
            && along.abs() <= reach_along
            && tall < -reach_tall
            && tall >= -jump_tall
        {
            // Above the blade: jump under it, as a player does, and swing
            // when the jump brings the blade level with it (the branch
            // above). A leg that is under way goes on: it is how the body
            // gets to a ledge, and a jump in its air drops it.
            goal = None;
            press.jump_pressed = on_ground;
            press.jump_held = true;
            walker.step(press);
            continue;
        }
        if leg.is_none() && flying {
            // A flight that no leg steers: turn it off, and land. A leg
            // starts from the ground.
            press.fly_toggle = true;
            walker.step(press);
            continue;
        }
        if on_ground && (goal.is_none() || step % RETARGET_FRAMES == 0) && leg.is_none() {
            goal = (0..graph.surfaces.len())
                .map(|index| graph.point_on(index, graph.frame.along(at)))
                .filter(|place| graph.frame.below(*place) >= graph.frame.below(at) - 1.0)
                .min_by(|a, b| (*a - at).length().total_cmp(&(*b - at).length()));
        }
        if leg.is_none() && on_ground {
            if let Some(goal) = goal {
                match graph.next(feet, goal) {
                    NavNext::Leg(next) => leg = Some((next, LegPhase::Approach)),
                    NavNext::Arrived | NavNext::Unknown => press.axis = along.signum(),
                    NavNext::Unreachable => return Err(format!("no route to a foe at {at:?} in `{room}` from {feet:?}")),
                }
            }
        }
        if let Some((current, phase)) = leg {
            let facts = LegFacts { feet, vel: kinematics.vel, on_ground, side: graph.frame.side, down: graph.frame.down, dt: 1.0 / 60.0, wall, approach: None };
            let (input, progress) = follow_leg(&current, phase, &facts);
            press.axis = input.axis;
            press.up = input.up;
            press.down = input.down;
            press.jump_pressed = input.jump_pressed;
            press.jump_held = input.jump_held;
            press.fly_toggle = input.fly_toggle;
            press.dash = input.dash_pressed;
            leg = match progress {
                LegProgress::Going(next) => Some((current, next)),
                LegProgress::Arrived | LegProgress::Failed => None,
            };
        }
        walker.step(press);
    }
    // What was left, so a fight that did not end says why.
    let world = walker.world_mut();
    let player_room = the_players_room(world, player);
    let mut bodies = world.query::<(Entity, &ae::BodyKinematics, &ambition_platformer2d::characters::actor::BodyHealth)>();
    let rows: Vec<(Entity, ae::Vec2, i32, bool)> =
        bodies.iter(world).map(|(entity, kin, health)| (entity, kin.pos, health.current(), health.alive())).collect();
    let left: Vec<String> = rows
        .iter()
        .filter(|(entity, _, _, alive)| *alive && *entity != player && foe(world, *entity))
        .filter(|(entity, ..)| {
            world.get::<ambition_platformer2d::platformer::lifecycle::InRoomInstance>(*entity).map(|room| room.0) == player_room
        })
        .map(|(entity, pos, health, _)| format!("{entity:?} at {pos:?} health {health}"))
        .collect();
    let me = rows.iter().find(|(entity, ..)| *entity == player).map(|(_, pos, health, _)| (*pos, *health));
    // Each body the fight names, in any room, alive or not.
    let named: Vec<String> = rows
        .iter()
        .filter(|(entity, ..)| *entity != player && foe(world, *entity))
        .map(|(entity, pos, health, alive)| {
            let room = world.get::<ambition_platformer2d::platformer::lifecycle::InRoomInstance>(*entity).map(|room| room.0);
            format!("{entity:?} at {pos:?} health {health} alive {alive} room {room:?}")
        })
        .collect();
    Err(format!(
        "the fight in `{room}` was not over after {frames} frames; the player (place, health) {me:?} in {player_room:?}, \
         {deaths} death(s); foes left: {left:?}; every body named: {named:?}"
    ))
}

/// The names of the gate solids that stand in the player's room now.
fn gate_solid_names(walker: &mut impl Walker, player: Entity) -> Vec<String> {
    the_players_overlay(walker.world_mut(), player).gate_solids.iter().map(|solid| solid.name.clone()).collect()
}
