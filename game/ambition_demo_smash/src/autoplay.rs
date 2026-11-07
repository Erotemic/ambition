//! An unattended *real* Smash match for phone/desktop profiling.
//!
//! `AMBITION_AUTOPLAY=smash` (2 CPU fighters) or `smash_4p` (4 CPUs), with
//! `AMBITION_BOOT_ROUTE=smash_select`. The host's one-shot Android launch
//! environment selects this without changing a normal launch from the icon.
//!
//! Unlike an Ambition/Mary-O navigation autopilot, this policy does not steer
//! a human seat: the Smash roster gives **every** fighter its shipped CPU brain.
//! This is the same select-settle -> publish-roster -> gameplay-route flow as
//! `smash_match_profile`, including the opening-ceremony/live-cast premise.

use bevy::prelude::*;

use ambition_platformer2d::actor::{MatchParticipantRoster, MatchSeat};
use ambition_platformer2d::characters::control::ControlHolds;
use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId, ShellRouter};
use ambition_platformer2d::scripted_input::Autoplay;

use crate::{
    apply_smash_match_rules, select::SmashRoster, smash_roster_at_levels,
    SMASH_GAMEPLAY_ROUTE, SMASH_SELECT_ROUTE,
};

pub const TWO_CPU_PROFILE: &str = "smash";
pub const FOUR_CPU_PROFILE: &str = "smash_4p";

/// Match the profile names without enabling the driver for normal play.
pub fn profile_seat_count(name: &str) -> Option<usize> {
    match name {
        TWO_CPU_PROFILE => Some(2),
        FOUR_CPU_PROFILE => Some(4),
        _ => None,
    }
}

pub fn is_smash_autoplay(autoplay: Option<Res<Autoplay>>) -> bool {
    autoplay.is_some_and(|profile| profile_seat_count(&profile.0).is_some())
}

/// Wait until Startup has published the seatable roster and the select route
/// has installed its frontend state. Mirrors the desktop match profiler.
const SETTLE_FRAMES: u32 = 30;
const LIVE_DEADLINE_FRAMES: u32 = 600;
const PREMISE_CHECK_FRAMES: u32 = 120;
/// Keep the real match running through a profiling capture, including KOs.
/// This changes only stocks, not movement, AI, rendering, or match composition.
const PROFILE_STOCKS: u32 = 40;
/// The authored `duelist_l3` brain is a representative, non-idle CPU.
const CPU_LEVEL: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Settling(u32),
    WaitingForLive(u32),
    Live(u32),
    Failed,
}

impl Default for Stage {
    fn default() -> Self {
        Self::Settling(0)
    }
}

/// Diagnostic state: tests/inspection can tell whether a capture measured
/// gameplay rather than a select screen or ceremony.
#[derive(Resource, Debug, Default)]
pub struct SmashAutoplayDriver {
    stage: Stage,
    seats_expected: usize,
}

fn select_is_active(world: &World) -> bool {
    world.get_resource::<ShellRouter>().is_some_and(|router| {
        router.active.as_ref().is_some_and(|active| {
            active.route_id.as_str() == SMASH_SELECT_ROUTE
        })
    })
}

fn gameplay_is_active(world: &World) -> bool {
    world.get_resource::<ShellRouter>().is_some_and(|router| {
        router.active.as_ref().is_some_and(|active| {
            active.route_id.as_str() == SMASH_GAMEPLAY_ROUTE
        })
    })
}

fn cast_state(world: &mut World) -> (usize, usize) {
    let seats = world.query::<&MatchSeat>().iter(world).count();
    let held = world
        .query_filtered::<&MatchSeat, With<ControlHolds>>()
        .iter(world)
        .count();
    (seats, held)
}

/// Choose a reproducible cast from actually prepared character ids. If a
/// minimal Smash-only build has two fighters and the full hub has twenty,
/// both still give two (or four) live CPUs with no missing-identity fallback.
fn profile_roster(seatable: &SmashRoster, count: usize) -> Result<MatchParticipantRoster, String> {
    if seatable.0.len() < 2 {
        return Err(format!(
            "Smash autoplay needs two seatable fighters, got {:?}",
            seatable.0
        ));
    }
    let ids = (0..count)
        .map(|seat| seatable.0[seat % 2].clone())
        .collect::<Vec<_>>();
    let mut roster = smash_roster_at_levels(ids, &vec![CPU_LEVEL; count]);
    // A 60-90s phone capture should not turn into a results-screen profile
    // just because the CPUs exhausted three default stocks. Preserve all
    // other authored match rules, including the real countdown and respawns.
    apply_smash_match_rules(&mut roster, PROFILE_STOCKS);
    Ok(roster)
}

fn fail(world: &mut World, message: &str) {
    eprintln!("[smash-autoplay] FAILED: {message}");
    // Do not quietly measure a menu in a purported CPU-match profile.
    world.write_message(bevy::app::AppExit::from_code(3));
}

/// This is an exclusive Update system because the roster must be published
/// before the shell route request, without a deferred-Commands race. The
/// algorithm is inactive during normal play (`is_smash_autoplay`).
pub fn drive_smash_autoplay(world: &mut World) {
    let Some(mut state) = world.remove_resource::<SmashAutoplayDriver>() else {
        return;
    };
    let requested = world
        .get_resource::<Autoplay>()
        .and_then(|profile| profile_seat_count(&profile.0))
        .unwrap_or(2);

    state.stage = match state.stage {
        Stage::Settling(_) if !select_is_active(world) => Stage::Settling(0),
        Stage::Settling(frames) if frames < SETTLE_FRAMES => Stage::Settling(frames + 1),
        Stage::Settling(_) => {
            let seatable = world.get_resource::<SmashRoster>().cloned().unwrap_or_default();
            match profile_roster(&seatable, requested) {
                Ok(roster) => {
                    let cast = roster
                        .participants
                        .iter()
                        .map(|p| p.character.to_string())
                        .collect::<Vec<_>>();
                    eprintln!(
                        "[smash-autoplay] starting CPUs={requested} brain=duelist_l{CPU_LEVEL} \
                         stocks={PROFILE_STOCKS} cast={cast:?}"
                    );
                    state.seats_expected = requested;
                    // The select route announces Pending seating. As in the
                    // real lobby, replace that with a decided channel plan
                    // BEFORE activating gameplay. All-CPU => zero local
                    // device channels, not an accidental human primary seat.
                    world.insert_resource(
                        ambition_platformer2d::input::SessionSeatingSource::decided(
                            crate::SMASH_EXPERIENCE,
                            roster.local_channel_plan(),
                        ),
                    );
                    world.insert_resource(
                        ambition_platformer2d::input::DeclaredBindingLayout::new(
                            crate::SMASH_EXPERIENCE,
                            ambition_platformer2d::input::BindingLayout::Smash,
                        ),
                    );
                    world.insert_resource(roster);
                    world.write_message(ShellCommand::GoTo(ShellRouteId::new(
                        SMASH_GAMEPLAY_ROUTE,
                    )));
                    Stage::WaitingForLive(0)
                }
                Err(problem) => {
                    fail(world, &problem);
                    Stage::Failed
                }
            }
        }
        Stage::WaitingForLive(frames) => {
            let (seats, held) = cast_state(world);
            if gameplay_is_active(world) && seats == state.seats_expected && held == 0 {
                eprintln!(
                    "[smash-autoplay] LIVE: seats={seats}, ceremony_released=true, \
                     all_cpu=true"
                );
                Stage::Live(0)
            } else if frames >= LIVE_DEADLINE_FRAMES {
                fail(world, &format!(
                    "match did not go live within {frames} frames: \
                     seats={seats}/{} held={held} route_gameplay={}",
                    state.seats_expected,
                    gameplay_is_active(world)
                ));
                Stage::Failed
            } else {
                Stage::WaitingForLive(frames + 1)
            }
        }
        Stage::Live(frames) => {
            if frames % PREMISE_CHECK_FRAMES == 0 {
                let (seats, held) = cast_state(world);
                if !gameplay_is_active(world) || seats == 0 {
                    fail(world, "match ended or left gameplay during the measurement");
                    Stage::Failed
                } else {
                    if seats != state.seats_expected || held != 0 {
                        eprintln!(
                            "[smash-autoplay] WARNING: population/ceremony changed: \
                             seats={seats}/{} held={held}",
                            state.seats_expected
                        );
                    }
                    Stage::Live(frames + 1)
                }
            } else {
                Stage::Live(frames + 1)
            }
        }
        Stage::Failed => Stage::Failed,
    };
    world.insert_resource(state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::actor::ControllerBinding;

    #[test]
    fn only_smash_profiles_select_the_cpu_match_driver() {
        assert_eq!(profile_seat_count(TWO_CPU_PROFILE), Some(2));
        assert_eq!(profile_seat_count(FOUR_CPU_PROFILE), Some(4));
        assert_eq!(profile_seat_count("mary_o"), None);
        assert_eq!(profile_seat_count(""), None);
    }

    #[test]
    fn the_profile_does_not_leave_a_human_primary_seat() {
        let available = SmashRoster(vec!["smash_duelist_a".into(), "smash_duelist_b".into()]);
        for expected in [2, 4] {
            let roster = profile_roster(&available, expected).expect("seatable cast");
            assert_eq!(roster.participants.len(), expected);
            assert_eq!(roster.local_channel_plan().channels(), 0);
            assert!(roster.participants.iter().all(|participant| {
                matches!(
                    &participant.controller,
                    ControllerBinding::Cpu { brain_profile }
                        if brain_profile.as_deref() == Some("duelist_l3")
                )
            }));
        }
    }

    #[test]
    fn a_missing_second_fighter_is_an_error_not_a_zero_load_profile() {
        assert!(profile_roster(&SmashRoster(vec!["one".into()]), 2).is_err());
    }
}
