//! Ambition plays itself, for unattended profiling: `AMBITION_AUTOPLAY=ambition`.
//!
//! The policy is one fixed itinerary, so two runs load the same rooms in the
//! same order and their profiles compare. From the hub the player walks into
//! the hall of characters and across it, walks out and to the primeval trail,
//! flies through the trail over its dinosaurs and across the tyrant's hall,
//! and flies back to the hub; then again.
//!
//! Each stop is a room. The way there is the room graph's fewest-rooms route
//! ([`RoomSet::route`]); in each room on it the engine's autopilot takes the
//! body to the door the route leaves by (on foot by the room's surface graph,
//! or in the air by its flight map), with the input a person would give, and
//! the policy presses Interact there.
//!
//! A cutscene that plays on a room's entry (the hub's boot line from the
//! Warden) holds the body until its line is dismissed: the policy presses
//! Confirm, as a person would.
//!
//! What it does not do: talk (a conversation reads the menu input, which no
//! script drives), fight, or pick up. It is a load, not a playthrough.

use bevy::prelude::*;

use ambition_platformer2d::autopilot::{Autopilot, AutopilotOrder, AutopilotStatus, NavGoal};
use ambition_platformer2d::engine_core as ae;
use ae::AabbExt;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::world::rooms::{LiveRoomSpecs, LoadingZoneActivation, RoomSet, RoomSpec};

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
/// Frames the body may spend in one room on its way before the policy gives
/// that stop up and takes the next.
const GIVE_UP_FRAMES: u32 = 3600;
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
}

#[derive(Clone, Debug, PartialEq)]
enum Goal {
    /// The zone this room is left by on the way to the stop.
    Leave { zone: ae::Aabb, door: bool },
    /// Across the stop's room, to this point.
    Cross(ae::Vec2),
    /// Nothing left to do in this room: the stop is done.
    Done,
}

/// The policy: this frame's order to the autopilot.
pub fn play_ambition(
    mut order: ResMut<AutopilotOrder>,
    mut itinerary: Local<Itinerary>,
    autopilot: Option<Res<Autopilot>>,
    rooms: Option<LiveRoomSpecs>,
    players: Query<(Entity, &ae::BodyKinematics), With<PrimaryPlayer>>,
    cutscene: Option<Res<ambition_cutscene::ActiveCutscene>>,
) {
    let Ok((entity, kin)) = players.single() else {
        *order = AutopilotOrder::Idle;
        return;
    };
    let Some((rooms, room)) = rooms.as_ref().and_then(|rooms| Some((rooms.rooms(), rooms.spec_of(entity)?))) else {
        *order = AutopilotOrder::Idle;
        return;
    };
    let it = &mut *itinerary;
    it.frames_since_press = it.frames_since_press.saturating_add(1);
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
    }
    let status = autopilot.as_deref().map(Autopilot::status);
    if it.frames_in_room > GIVE_UP_FRAMES {
        it.advance();
    }
    if it.goal == Some(Goal::Done) || (matches!(it.goal, Some(Goal::Cross(_))) && status == Some(AutopilotStatus::Arrived)) {
        it.advance();
    }
    if it.goal.is_none() {
        it.goal = Some(goal_in(rooms, room, ITINERARY[it.stop], it.arrival));
    }
    let way = ITINERARY[it.stop].way;
    *order = match it.goal.clone() {
        Some(Goal::Leave { zone, door }) => {
            let in_it = contains(zone, kin.pos);
            if door && in_it && it.frames_since_press >= PRESS_EVERY {
                it.frames_since_press = 0;
                AutopilotOrder::Press(ControlFrame { interact_pressed: true, ..ControlFrame::default() })
            } else {
                match way {
                    // On the floor in the middle of the door.
                    Way::Walk => AutopilotOrder::GoTo(NavGoal::Stand(ae::Vec2::new(zone.center().x, zone.max.y - 1.0))),
                    Way::Fly => AutopilotOrder::GoTo(NavGoal::Fly(zone.center())),
                }
            }
        }
        Some(Goal::Cross(at)) => match way {
            Way::Walk => AutopilotOrder::GoTo(NavGoal::Stand(at)),
            Way::Fly => AutopilotOrder::GoTo(NavGoal::Fly(at)),
        },
        Some(Goal::Done) | None => AutopilotOrder::Idle,
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
fn goal_in(rooms: &RoomSet, room: &RoomSpec, stop: Stop, arrival: ae::Vec2) -> Goal {
    if room.id == stop.room {
        return if stop.cross { Goal::Cross(cross_point(room.world.size, arrival, stop.way)) } else { Goal::Done };
    }
    let first_hop = rooms
        .room_index_by_id(&room.id)
        .zip(rooms.room_index_by_id(stop.room))
        .and_then(|(from, to)| rooms.route(from, to))
        .and_then(|hops| hops.into_iter().next());
    match first_hop.and_then(|hop| room.loading_zones.iter().find(|zone| zone.id == hop.zone)) {
        Some(zone) => Goal::Leave { zone: zone.aabb, door: zone.activation == LoadingZoneActivation::Door },
        // No way from here: the stop is given up.
        None => Goal::Done,
    }
}

fn contains(zone: ae::Aabb, at: ae::Vec2) -> bool {
    at.x >= zone.min.x && at.x <= zone.max.x && at.y >= zone.min.y && at.y <= zone.max.y
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
}
