//! Mary-O plays herself, for unattended profiling: `AMBITION_AUTOPLAY=mary_o`.
//!
//! The policy, from Jon (2026-10-16): take the cosmic form, then go to the
//! goal. In the form no enemy hurts her. This module says only what is hers:
//! the form, and where a level's goal is (the pole, or a castle's switch). The
//! engine's autopilot takes her there by the room's surface graph for her own
//! body, with the input a person would give, and keeps her out of the pits,
//! the lava and the room's exits as navigation does for any body.

use bevy::prelude::*;

use ambition_platformer2d::autopilot::{AutopilotOrder, NavGoal};
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::world::rooms::{LiveRoomSpecs, RoomSpec};

use crate::cosmic::COSMIC_CHARACTER_ID;
use crate::flag::FlagSequence;

/// This game's id in the hub, the value of `AMBITION_AUTOPLAY` that selects
/// this policy.
pub const AUTOPLAY_GAME: &str = "mary_o";

/// Frames between two presses of Utility while she is not in the form.
const UTILITY_RETRY_FRAMES: u32 = 45;

#[derive(Default)]
pub struct AutoplayState {
    frames_since_utility: u32,
}

/// The policy: this frame's order to the autopilot.
pub fn play_mary_o(
    mut order: ResMut<AutopilotOrder>,
    mut state: Local<AutoplayState>,
    rooms: Option<LiveRoomSpecs>,
    players: Query<(Entity, &WornCharacter), With<PrimaryPlayer>>,
    flags: Query<&FlagSequence>,
) {
    state.frames_since_utility = state.frames_since_utility.saturating_add(1);
    let Ok((entity, worn)) = players.single() else {
        *order = AutopilotOrder::Idle;
        return;
    };
    // The flag sequence holds her: let go.
    if flags.iter().any(FlagSequence::active) {
        *order = AutopilotOrder::Idle;
        return;
    }
    // The form before she moves. Its press is the Utility control.
    if worn.id() != COSMIC_CHARACTER_ID {
        *order = if state.frames_since_utility >= UTILITY_RETRY_FRAMES {
            state.frames_since_utility = 0;
            AutopilotOrder::Press(ControlFrame { fly_toggle_pressed: true, ..ControlFrame::default() })
        } else {
            AutopilotOrder::Idle
        };
        return;
    }
    *order = rooms
        .as_ref()
        .and_then(|rooms| rooms.spec_of(entity))
        .and_then(level_goal)
        .map_or(AutopilotOrder::Idle, |goal| AutopilotOrder::GoTo(NavGoal::Stand(goal)));
}

/// Where a level ends: on top of a castle's switch, or at the foot of the
/// pole (a body there is in the pole's grab).
fn level_goal(room: &RoomSpec) -> Option<ae::Vec2> {
    if let Some(switch) = crate::castle_end::room_switch(room) {
        return Some(ae::Vec2::new((switch.min.x + switch.max.x) * 0.5, switch.min.y));
    }
    crate::room_pole(room).map(|pole| ae::Vec2::new(pole.x, pole.base_y))
}
