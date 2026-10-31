//! Ambition plays itself, for unattended profiling: `AMBITION_AUTOPLAY=ambition`.
//!
//! The policy is one fixed itinerary, so two runs load the same rooms in the
//! same order and their profiles compare. From the hub the player walks into
//! the hall of characters and across it, walks out and to the primeval trail,
//! flies through the trail over its dinosaurs and across the tyrant's hall,
//! and flies back to the hub; then again.
//!
//! Each stop is a room. Doors and available dimension gates share one room
//! route. A gate trip uses its terminal, dials through the host intent ledger,
//! and enters the open ring. In each room the engine's autopilot takes the
//! body to the door the route leaves by (on foot by the room's surface graph,
//! or in the air by its flight map), with the input a person would give, and
//! the policy presses Interact there.
//!
//! A cutscene that plays on a room's entry (a level's `entry_cutscene`) holds
//! the body until its lines are dismissed: the policy presses Confirm, as a
//! person would.
//!
//! What it does not do: talk (a conversation reads the menu input, which no
//! script drives), fight, or pick up. It is a load, not a playthrough.

use bevy::prelude::*;

use ambition_platformer2d::autopilot::{Autopilot, AutopilotOrder, AutopilotStatus, NavGoal};
use ambition_platformer2d::engine_core as ae;
use ae::AabbExt;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::world::rooms::{
    GateFacts, GateNetworkCatalog, GateNetworkState, GatePortalPhases, GateRingPositions,
    GateTerminalAction, GateTerminalIntent, LiveRoomSpecs, LoadingZone, LoadingZoneActivation,
    RoomHop, RoomSet, RoomSpec,
};
use ambition_platformer2d::platformer::host_intents::HostIntentWriter;
use bevy::ecs::system::SystemParam;

/// This game's id in the hub, the value of `AMBITION_AUTOPLAY` that selects
/// this policy.
pub const AUTOPLAY_GAME: &str = "ambition";

/// How the body goes on its way to a stop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Way {
    Walk,
    Fly,
}

/// One stop of the itinerary: a room, how the body goes there, and whether it
/// crosses the room before it goes on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stop {
    pub room: &'static str,
    pub way: Way,
    pub cross: bool,
}

/// The itinerary, in order, and again from the first.
pub const ITINERARY: [Stop; 4] = [
    Stop { room: "hall_of_characters", way: Way::Walk, cross: true },
    Stop { room: "primeval_trailhead", way: Way::Walk, cross: false },
    Stop { room: "primeval_tyrant_hall", way: Way::Fly, cross: true },
    Stop { room: "central_hub_complex", way: Way::Fly, cross: false },
];

/// Frames between two presses of Interact at a door, or of Confirm in a
/// cutscene.
const PRESS_EVERY: u32 = 20;
/// Frames between route attempts while no route is available.
const RETRY_ROUTE_FRAMES: u32 = 60;
/// How far from the far wall a crossing ends (px).
const CROSS_INSET: f32 = 96.0;
/// How high over its arrival a flight across a room goes (px).
const CROSS_LIFT: f32 = 64.0;

/// Where the itinerary is.
#[derive(Default)]
pub struct Itinerary {
    stop: usize,
    /// The room the body is in, as far as this policy has seen.
    room: Option<String>,
    /// Where the body came into it.
    arrival: ae::Vec2,
    /// What the body goes to in this room.
    goal: Option<Goal>,
    frames_in_room: u32,
    frames_since_press: u32,
    pending_dial: Option<(String, u64)>,
}

#[derive(Clone, Debug, PartialEq)]
enum Goal {
    /// The zone this room is left by on the way to the stop.
    Leave(RoomHop),
    /// Across the stop's room, to this point.
    Cross(ae::Vec2),
    /// Nothing left to do in this room: the stop is done.
    Done,
    /// No available route yet. A closed gate can become available later.
    Waiting,
}

/// The live gate facts and the same request channel that the terminal uses.
#[derive(SystemParam)]
pub struct GateTravel<'w, 's> {
    catalog: Res<'w, GateNetworkCatalog>,
    state: Res<'w, GateNetworkState>,
    positions: Res<'w, GateRingPositions>,
    phases: Res<'w, GatePortalPhases>,
    save: Res<'w, ambition_platformer2d::persistence::save::AmbitionGameSave>,
    intents: HostIntentWriter<'w, 's, GateTerminalIntent>,
    terminals: Query<'w, 's, (Entity, &'static ambition_platformer2d::encounter::switches::SwitchFeature,
        &'static ae::geometry::CenteredAabb)>,
}

impl GateTravel<'_, '_> {
    fn route(&self, rooms: &RoomSet, from: usize, to: usize) -> Option<Vec<RoomHop>> {
        let data = self.save.data();
        let switch_on = |id: &str| data.switch(id);
        let flag = |id: &str| data.flag(id);
        let zone_exists = |room: &str, zone: &str| rooms.has_zone(room, zone);
        let distance = |a: &str, b: &str| self.positions.distance(a, b);
        self.catalog.route(rooms, &self.state, from, to, &GateFacts {
            switch_on: &switch_on, flag: &flag, zone_exists: &zone_exists, distance: &distance,
        })
    }

    fn ask(&mut self, ring: &str, action: GateTerminalAction, it: &mut Itinerary) {
        if action == GateTerminalAction::Close || it.frames_since_press >= PRESS_EVERY {
            it.frames_since_press = 0;
            if matches!(action, GateTerminalAction::Dial(_)) {
                it.pending_dial = Some((ring.to_owned(), self.state.next_serial));
            }
            self.intents.write(GateTerminalIntent { ring: ring.to_owned(), action });
        }
    }
}

/// The policy: this frame's order to the autopilot.
pub fn play_ambition(
    mut order: ResMut<AutopilotOrder>,
    mut itinerary: Local<Itinerary>,
    autopilot: Option<Res<Autopilot>>,
    rooms: Option<LiveRoomSpecs>,
    players: Query<(Entity, &ae::BodyKinematics, Option<&ae::SweepSample>), With<PrimaryPlayer>>,
    cutscene: Option<Res<ambition_cutscene::ActiveCutscene>>,
    mut gates: GateTravel,
) {
    let Ok((entity, kin, sweep)) = players.single() else {
        *order = AutopilotOrder::Idle;
        return;
    };
    let Some(specs) = rooms.as_ref() else {
        *order = AutopilotOrder::Idle;
        return;
    };
    let Some(room) = specs.spec_of(entity) else {
        *order = AutopilotOrder::Idle;
        return;
    };
    let rooms = specs.rooms();
    let it = &mut *itinerary;
    if specs.set_is_changed() || gates.catalog.is_changed() {
        it.goal = None;
    }
    it.frames_since_press = it.frames_since_press.saturating_add(1);
    if let Some((ring, serial)) = it.pending_dial.clone() {
        if let Some(report) = gates.state.reports.get(&ring).filter(|report| report.serial >= serial) {
            it.pending_dial = None;
            if report.outcome.is_err() {
                eprintln!("[autoplay] dial refused ring={ring} outcome={:?}; planning again", report.outcome);
                gates.ask(&ring, GateTerminalAction::Close, it);
                it.goal = None;
                *order = AutopilotOrder::Idle;
                return;
            }
        }
    }
    if cutscene.is_some_and(|cutscene| cutscene.is_playing()) {
        *order = if it.frames_since_press >= PRESS_EVERY {
            it.frames_since_press = 0;
            AutopilotOrder::Press(ControlFrame { confirm_pressed: true, ..ControlFrame::default() })
        } else {
            AutopilotOrder::Idle
        };
        return;
    }
    it.frames_in_room = it.frames_in_room.saturating_add(1);
    if it.room.as_deref() != Some(room.id.as_str()) {
        it.room = Some(room.id.clone());
        it.arrival = kin.pos;
        it.frames_in_room = 0;
        it.goal = None;
        it.pending_dial = None;
    }
    let status = autopilot.as_deref().map(Autopilot::status);
    if it.goal == Some(Goal::Waiting) && it.frames_in_room % RETRY_ROUTE_FRAMES == 0 {
        it.goal = None;
    }
    if it.goal == Some(Goal::Done) || (matches!(it.goal, Some(Goal::Cross(_))) && status == Some(AutopilotStatus::Arrived)) {
        it.advance();
    }
    if it.goal.is_none() {
        it.goal = Some(goal_in(rooms, room, ITINERARY[it.stop], it.arrival, &gates));
        eprintln!("[autoplay] room={} stop={} goal={:?}", room.id, ITINERARY[it.stop].room, it.goal);
    }
    let way = ITINERARY[it.stop].way;
    *order = match it.goal.clone() {
        Some(Goal::Leave(hop)) => {
            let Some(zone) = room.loading_zones.iter().find(|zone| zone.id == hop.zone) else {
                it.goal = None;
                *order = AutopilotOrder::Idle;
                return;
            };
            let body = kin.collision_box(sweep);
            if let Some(address) = &hop.dial {
                let linked = gates.state.route_from(&zone.id).is_some_and(|link| link.address == *address);
                if gates.state.terminal.as_deref() == Some(&zone.id) {
                    let action = if linked { GateTerminalAction::Close } else { GateTerminalAction::Dial(address.clone()) };
                    gates.ask(&zone.id, action, it);
                    *order = AutopilotOrder::Idle;
                    return;
                }
                if !linked {
                    if let Some(open) = gates.state.terminal.clone() {
                        gates.ask(&open, GateTerminalAction::Close, it);
                        *order = AutopilotOrder::Idle;
                        return;
                    }
                    let terminal = gates.catalog.ring(&zone.id).and_then(|ring| ring.terminal.as_deref());
                    let at = gates.terminals.iter().find(|(switch, feature, _)| {
                        Some(feature.activation.id.as_str()) == terminal && specs.definition_of(*switch) == specs.definition_of(entity)
                    }).map(|(_, _, at)| *at);
                    *order = match at {
                        Some(at) if body.strict_intersects(at.aabb()) => press_interact(it),
                        Some(at) => go_to(at.center, way),
                        None => {
                            it.goal = None;
                            AutopilotOrder::Idle
                        }
                    };
                    return;
                }
                if !gates.phases.allows_traversal(&zone.id) {
                    *order = AutopilotOrder::Idle;
                    return;
                }
            }
            enter(zone, body, way, it)
        }
        Some(Goal::Cross(at)) => match way {
            Way::Walk => AutopilotOrder::GoTo(NavGoal::Stand(at)),
            Way::Fly => AutopilotOrder::GoTo(NavGoal::Fly(at)),
        },
        Some(Goal::Done | Goal::Waiting) | None => AutopilotOrder::Idle,
    };
}

impl Itinerary {
    /// On to the next stop, from this room.
    fn advance(&mut self) {
        self.stop = (self.stop + 1) % ITINERARY.len();
        self.goal = None;
        self.frames_in_room = 0;
    }
}

/// What the body goes to in `room` on its way to `stop`, having come in at
/// `arrival`.
fn goal_in(rooms: &RoomSet, room: &RoomSpec, stop: Stop, arrival: ae::Vec2, gates: &GateTravel) -> Goal {
    if room.id == stop.room {
        return if stop.cross { Goal::Cross(cross_point(room.world.size, arrival, stop.way)) } else { Goal::Done };
    }
    let first_hop = rooms
        .room_index_by_id(&room.id)
        .zip(rooms.room_index_by_id(stop.room))
        .and_then(|(from, to)| gates.route(rooms, from, to))
        .and_then(|hops| hops.into_iter().next());
    match first_hop {
        Some(hop) => Goal::Leave(hop),
        None => Goal::Waiting,
    }
}

fn go_to(at: ae::Vec2, way: Way) -> AutopilotOrder {
    AutopilotOrder::GoTo(match way { Way::Walk => NavGoal::Stand(at), Way::Fly => NavGoal::Fly(at) })
}

fn press_interact(it: &mut Itinerary) -> AutopilotOrder {
    if it.frames_since_press < PRESS_EVERY { return AutopilotOrder::Idle; }
    it.frames_since_press = 0;
    AutopilotOrder::Press(ControlFrame { interact_pressed: true, ..ControlFrame::default() })
}

fn enter(zone: &LoadingZone, body: ae::Aabb, way: Way, it: &mut Itinerary) -> AutopilotOrder {
    if zone.activation == LoadingZoneActivation::Door && zone.reached_by(body, ae::Vec2::ZERO) {
        return press_interact(it);
    }
    let at = match way {
        Way::Walk => ae::Vec2::new(zone.aabb.center().x, zone.aabb.max.y - 1.0),
        Way::Fly => return AutopilotOrder::GoTo(NavGoal::FlyInto(zone.aabb)),
    };
    go_to(at, way)
}

/// The far side of a room of `size` from `arrival`: on foot at the height it
/// came in at, in the air a little over it.
fn cross_point(size: ae::Vec2, arrival: ae::Vec2, way: Way) -> ae::Vec2 {
    let x = if arrival.x < size.x * 0.5 { size.x - CROSS_INSET } else { CROSS_INSET };
    match way {
        Way::Walk => ae::Vec2::new(x, arrival.y),
        Way::Fly => ae::Vec2::new(x, arrival.y - CROSS_LIFT),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_crossing_goes_to_the_far_side_from_where_the_body_came_in() {
        let size = ae::Vec2::new(2000.0, 700.0);
        assert_eq!(cross_point(size, ae::Vec2::new(100.0, 600.0), Way::Walk), ae::Vec2::new(1904.0, 600.0));
        assert_eq!(cross_point(size, ae::Vec2::new(1900.0, 600.0), Way::Fly), ae::Vec2::new(96.0, 536.0));
    }

    #[test]
    fn a_flying_body_opens_an_edge_door_without_putting_its_center_outside_the_room() {
        let zone = LoadingZone {
            id: "west".into(), name: "west".into(), activation: LoadingZoneActivation::Door,
            aabb: ae::Aabb { min: ae::Vec2::new(0.0, 384.0), max: ae::Vec2::new(16.0, 480.0) },
        };
        let body = ae::Aabb::new(ae::Vec2::new(36.0, 426.0), ae::Vec2::new(28.0, 24.0));
        let mut it = Itinerary { frames_since_press: PRESS_EVERY, ..default() };
        assert!(body.center().x > zone.aabb.max.x);
        assert!(matches!(enter(&zone, body, Way::Fly, &mut it), AutopilotOrder::Press(frame) if frame.interact_pressed));
        let far = ae::Aabb::new(ae::Vec2::new(100.0, 426.0), body.half_size());
        assert!(matches!(enter(&zone, far, Way::Fly, &mut it), AutopilotOrder::GoTo(_)));
    }
}
