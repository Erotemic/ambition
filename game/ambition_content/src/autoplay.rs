//! Ambition plays itself, for unattended profiling: `AMBITION_AUTOPLAY=ambition`.
//!
//! The policy is a tour of the world by its doors. In each room the player
//! walks to the door it has been through least (the engine's autopilot takes
//! it there by the room's surface graph, with the input a person would give)
//! and goes through it. So a run loads room after room, as a player does, and
//! a profile sees the rooms, their transitions and their streaming, not one
//! room standing still. A room with no door is crossed end to end.
//!
//! What it does not do: talk (a conversation reads the menu input, which no
//! script drives), fight, or pick up. It is a load, not a playthrough.

use std::collections::HashMap;

use bevy::prelude::*;

use ambition_platformer2d::autopilot::{Autopilot, AutopilotOrder, AutopilotStatus, NavGoal};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::world::rooms::{LiveRoomSpecs, LoadingZoneActivation, RoomSpec};

/// This game's id in the hub, the value of `AMBITION_AUTOPLAY` that selects
/// this policy.
pub const AUTOPLAY_GAME: &str = "ambition";

/// Frames between two presses of Interact at a door.
const PRESS_EVERY: u32 = 20;
/// Frames a goal may go unreached (no route, or no arrival) before the tour
/// takes another.
const GIVE_UP_FRAMES: u32 = 600;

#[derive(Default)]
pub struct Tour {
    /// How often each door of each room was taken.
    visits: HashMap<(String, String), u32>,
    room: Option<String>,
    /// The door being walked to, or the end of a doorless room.
    goal: Option<Goal>,
    frames_on_goal: u32,
    frames_since_press: u32,
}

#[derive(Clone, Debug, PartialEq)]
enum Goal {
    Door { id: String, zone: ae::Aabb },
    /// A room with no door: walk to one end, then the other.
    Across { right: bool },
}

/// The policy: this frame's order to the autopilot.
pub fn play_ambition(
    mut order: ResMut<AutopilotOrder>,
    mut tour: Local<Tour>,
    autopilot: Option<Res<Autopilot>>,
    rooms: Option<LiveRoomSpecs>,
    players: Query<(Entity, &ae::BodyKinematics), With<PrimaryPlayer>>,
) {
    let Ok((entity, kin)) = players.single() else {
        *order = AutopilotOrder::Idle;
        return;
    };
    let Some(room) = rooms.as_ref().and_then(|rooms| rooms.spec_of(entity)) else {
        *order = AutopilotOrder::Idle;
        return;
    };
    let tour = &mut *tour;
    tour.frames_since_press = tour.frames_since_press.saturating_add(1);
    tour.frames_on_goal = tour.frames_on_goal.saturating_add(1);
    if tour.room.as_deref() != Some(room.id.as_str()) {
        // A new room: the door it came in by is the one it stands in.
        tour.room = Some(room.id.clone());
        if let Some(arrival) = doors(room).find(|(_, zone)| contains(*zone, kin.pos)) {
            *tour.visits.entry((room.id.clone(), arrival.0.to_owned())).or_default() += 1;
        }
        tour.goal = None;
    }
    let status = autopilot.as_deref().map(Autopilot::status);
    let stuck = tour.frames_on_goal > GIVE_UP_FRAMES || status == Some(AutopilotStatus::NoGraph);
    if tour.goal.is_none() || stuck {
        if let (true, Some(Goal::Door { id, .. })) = (stuck, &tour.goal) {
            // A door it cannot reach is the last it tries again.
            *tour.visits.entry((room.id.clone(), id.clone())).or_default() += 8;
        }
        tour.goal = Some(next_goal(room, &tour.visits, &tour.goal));
        tour.frames_on_goal = 0;
    }
    let Some(goal) = tour.goal.clone() else {
        *order = AutopilotOrder::Idle;
        return;
    };
    match goal {
        Goal::Door { id, zone } => {
            let at_door = contains(zone, kin.pos) || status == Some(AutopilotStatus::Arrived);
            if at_door && tour.frames_since_press >= PRESS_EVERY {
                tour.frames_since_press = 0;
                *tour.visits.entry((room.id.clone(), id)).or_default() += 1;
                *order = AutopilotOrder::Press(ControlFrame { interact_pressed: true, ..ControlFrame::default() });
            } else {
                // The middle of the door, on the floor it stands on.
                let foot = ae::Vec2::new((zone.min.x + zone.max.x) * 0.5, zone.max.y - 1.0);
                *order = AutopilotOrder::GoTo(NavGoal::Stand(foot));
            }
        }
        Goal::Across { right } => {
            if status == Some(AutopilotStatus::Arrived) {
                tour.goal = Some(Goal::Across { right: !right });
                tour.frames_on_goal = 0;
            }
            let size = room.world.size;
            let x = if right { size.x - 48.0 } else { 48.0 };
            *order = AutopilotOrder::GoTo(NavGoal::Stand(ae::Vec2::new(x, size.y * 0.5)));
        }
    }
}

/// The doors of a room: its zones a body takes by Interact.
fn doors(room: &RoomSpec) -> impl Iterator<Item = (&str, ae::Aabb)> {
    room.loading_zones
        .iter()
        .filter(|zone| zone.activation == LoadingZoneActivation::Door)
        .map(|zone| (zone.id.as_str(), zone.aabb))
}

fn contains(zone: ae::Aabb, at: ae::Vec2) -> bool {
    at.x >= zone.min.x && at.x <= zone.max.x && at.y >= zone.min.y && at.y <= zone.max.y
}

/// The door of `room` taken least (the lowest id of those), or across a room
/// with no door.
fn next_goal(room: &RoomSpec, visits: &HashMap<(String, String), u32>, last: &Option<Goal>) -> Goal {
    let taken = |id: &str| visits.get(&(room.id.clone(), id.to_owned())).copied().unwrap_or(0);
    doors(room)
        .min_by(|(a, _), (b, _)| taken(a).cmp(&taken(b)).then(a.cmp(b)))
        .map(|(id, zone)| Goal::Door { id: id.to_owned(), zone })
        .unwrap_or(match last {
            Some(Goal::Across { right }) => Goal::Across { right: !right },
            _ => Goal::Across { right: true },
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::world::rooms::LoadingZone;

    fn room_with(doors: &[&str]) -> RoomSpec {
        let mut room = RoomSpec::new("hall", ae::World::new("hall", ae::Vec2::new(800.0, 600.0), ae::Vec2::ZERO, vec![]));
        room.loading_zones = doors
            .iter()
            .enumerate()
            .map(|(i, id)| LoadingZone {
                id: (*id).into(),
                name: (*id).into(),
                activation: LoadingZoneActivation::Door,
                aabb: ae::Aabb::new(ae::Vec2::new(100.0 + 200.0 * i as f32, 400.0), ae::Vec2::new(24.0, 48.0)),
            })
            .collect();
        room
    }

    #[test]
    fn the_tour_takes_the_door_taken_least() {
        let room = room_with(&["a", "b", "c"]);
        let mut visits = HashMap::new();
        visits.insert(("hall".to_owned(), "a".to_owned()), 2);
        visits.insert(("hall".to_owned(), "b".to_owned()), 1);
        visits.insert(("hall".to_owned(), "c".to_owned()), 1);
        assert!(matches!(next_goal(&room, &visits, &None), Goal::Door { ref id, .. } if id == "b"));
    }

    #[test]
    fn a_room_with_no_door_is_crossed_end_to_end() {
        let room = room_with(&[]);
        let visits = HashMap::new();
        let first = next_goal(&room, &visits, &None);
        assert_eq!(first, Goal::Across { right: true });
        assert_eq!(next_goal(&room, &visits, &Some(first)), Goal::Across { right: false });
    }
}
