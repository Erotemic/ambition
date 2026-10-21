//! The slow-motion finish: the killing blow of a boss that ends a fight slows
//! time (Jon, 2026-10-20: "we need an effect to slow down time when you kill
//! the boss").
//!
//! The beat, in order:
//!
//! 1. The defeat freeze every boss has (`boss_phase_transition_feedback`): the
//!    world stops for a moment on the blow, the camera pulls in, the sting.
//! 2. Then the world hangs at [`HANG_SCALE`] of its pace while the boss
//!    falls, and comes back up to speed. The read model publishes how deep
//!    in it the finish is ([`intensity`]), and presentation drains the
//!    picture of its colour but the boss and draws cinema bars
//!    (`ambition_render::rendering::boss_finish`).
//!
//! A boss opts in with its encounter's `slow_motion_finish`. The clock is the
//! boss's own seconds in `Death` (sim seconds, so they pass slowly while the
//! world hangs), which the encounter driver counts on for a death the room
//! saw and not for a corpse the room was built with. Nothing here is state:
//! a request is written each tick the finish is on, and an over finish
//! restores the pace by ceasing to ask (`apply_clock_scale_requests` reduces
//! by `min`).

use ambition_time::time_control::{ClockRequester, ClockScaleRequest};
use bevy::prelude::*;

use crate::{BossConfig, BossEncounter, BossEncounterPhase};

/// The pace the world hangs at.
pub const HANG_SCALE: f32 = 0.15;
/// Seconds of the boss's death (sim seconds) it hangs, and when it is back at
/// full speed. At [`HANG_SCALE`] the hang is about 1.5 s of a player's time.
pub const HANG_S: f32 = 0.22;
pub const END_S: f32 = 0.5;

/// The pace of the world `elapsed` seconds into a finish, or `None` when no
/// finish is on (before its first second is counted, and after it).
pub fn scale(elapsed: f32) -> Option<f32> {
    if !(elapsed > 0.0 && elapsed < END_S) {
        return None;
    }
    let back = ((elapsed - HANG_S) / (END_S - HANG_S)).clamp(0.0, 1.0);
    Some(HANG_SCALE + (1.0 - HANG_SCALE) * back * back * (3.0 - 2.0 * back))
}

/// How deep in its finish a boss is, 0 to 1, for presentation: all the way
/// while the world hangs, and less as it comes back up to speed.
pub fn intensity(elapsed: f32) -> f32 {
    scale(elapsed).map_or(0.0, |s| ((1.0 - s) / (1.0 - HANG_SCALE)).clamp(0.0, 1.0))
}

/// How deep in its finish this boss is now (see [`intensity`]).
pub fn finish_of(config: &BossConfig, status: &BossEncounter) -> f32 {
    match &status.encounter {
        Some(state) if config.seed.encounter.slow_motion_finish && matches!(state.phase, BossEncounterPhase::Death) => {
            intensity(state.phase_elapsed)
        }
        _ => 0.0,
    }
}

/// Ask for the hang while a boss's finish is on.
pub fn request_boss_finish_slow_motion(
    bosses: Query<(&BossConfig, &BossEncounter)>,
    mut writer: MessageWriter<ClockScaleRequest>,
) {
    let slowest = bosses
        .iter()
        .filter(|(config, _)| config.seed.encounter.slow_motion_finish)
        .filter_map(|(_, status)| match &status.encounter {
            Some(state) if matches!(state.phase, BossEncounterPhase::Death) => scale(state.phase_elapsed),
            _ => None,
        })
        .reduce(f32::min);
    if let Some(scale) = slowest {
        writer.write(ClockScaleRequest {
            domain: ambition_time::ClockDomain::SimClock,
            scale,
            requester: ClockRequester::Engine,
            reason: "boss_finish",
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// It hangs, comes back up to full speed, and is over; nothing before the
    /// first second is counted (a corpse the room was built with).
    #[test]
    fn the_finish_hangs_then_comes_back_to_speed() {
        assert_eq!(scale(0.0), None, "a corpse built dead stays at zero");
        assert_eq!(scale(0.1), Some(HANG_SCALE));
        let back = scale(0.4).expect("still on");
        assert!(back > HANG_SCALE && back < 1.0, "{back}");
        assert_eq!(scale(END_S), None);
        assert!((intensity(0.1) - 1.0).abs() < 1e-6 && intensity(END_S) == 0.0);
    }
}
