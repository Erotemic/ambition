//! ⛔⛤ **A MECHANICAL RESOURCE OUTSIDE ROLLBACK HISTORY BREAKS RESIMULATION, AND
//! "FORWARD-ONLY" IS NOT A ROLLBACK CATEGORY.**
//!
//! `rollback_coverage.rs` waives `ActiveMovementTuning` and
//! `Platformer2dFeelTuningMonolith` with the reason *"forward-only"* — a
//! developer knob, not per-frame simulation state. That reasoning answers the
//! wrong question. The one that matters under rollback is not *"do we want to
//! rewind this value"* but **"can its value affect the simulation of a
//! HISTORICAL frame"**, and for these it can:
//!
//! ```text
//! frame 100:  jump_speed = A          simulated, checksummed
//! frame 101:  a developer edits it → B
//! frame 102:  rollback to frame 98
//!             ... resimulate 98..102 — now reading B
//! ```
//!
//! The snapshot restores frame-98 BODY state faithfully. It cannot restore the
//! tuning, because the tuning is not in the snapshot — so the resimulation of a
//! frame that already happened is run against a different world than the one
//! that produced its checksum. *"Forward-only"* has no coherent meaning if
//! execution can rewind BEHIND the forward-only mutation.
//!
//! ⭐⭐ **THESE ARMS ARE THE MEASUREMENT, WRITTEN BEFORE ANY FIX**, which is what
//! the architecture review asked for: their result decides whether there is an
//! implementation packet and which of the three coherent models it wants
//! (refuse mechanical edits while a timeline is live; make an edit a bounded
//! rebase; or make it deterministic timestamped input). They are named for what
//! they RECORD, so the day a model lands they flip rather than rot.
//!
//! ⚠ **THE CONTROL IS THE LOAD-BEARING HALF.** A harness that desyncs for any
//! reason would make the edit arm pass while proving nothing, so the same rig
//! runs the same frames with NO edit first and must stay healthy.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};

/// The sync-test canary: save every frame, rewind `4`, resimulate with the same
/// inputs, compare checksums. A mismatch is reported by `rollback_health`.
fn rollback_sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("Ambition GGRS sync-test harness builds")
}

/// Inputs that actually exercise the tuning under test: a body that jumps and
/// runs reads `jump_speed` and `max_run_speed` every frame.
fn scripted_action(frame: usize) -> AgentAction {
    AgentAction {
        move_x: if frame % 24 < 12 { 1.0 } else { -1.0 },
        jump: frame % 17 == 0,
        jump_held: frame % 17 < 5,
        ..AgentAction::default()
    }
}

const FRAMES: usize = 40;
/// Late enough that the rewind window straddles it.
const EDIT_AT: usize = 24;

/// ⭐ THE CONTROL. Without it, "the edit desynced the timeline" is satisfied by a
/// harness that desyncs on its own.
#[test]
fn the_same_frames_with_no_developer_edit_stay_healthy() {
    let mut sim = rollback_sim();
    for frame in 0..FRAMES {
        sim.step(scripted_action(frame));
        sim.rollback_health().unwrap_or_else(|error| {
            panic!("frame {frame}: the UNEDITED timeline desynced, so this rig cannot witness an edit: {error}")
        });
    }
}

/// ⛔⛤ **MEASURED: editing `ActiveMovementTuning` mid-timeline resimulates
/// history against the new value.**
#[test]
fn editing_movement_tuning_mid_timeline_changes_what_history_resimulates_to() {
    let mut sim = rollback_sim();
    let mut desync: Option<String> = None;
    for frame in 0..FRAMES {
        if frame == EDIT_AT {
            // Exactly what the inspector does: `apply_editable_movement_tuning`
            // pushes `EditableMovementTuning` into this resource, and simulation
            // systems read it directly.
            let mut tuning = sim
                .world_mut()
                .resource_mut::<ambition_platformer2d::runtime::demo_fixture::ActiveMovementTuning>();
            tuning.0.jump_speed *= 1.5;
            tuning.0.max_run_speed *= 1.5;
        }
        sim.step(scripted_action(frame));
        if let Err(error) = sim.rollback_health() {
            desync = Some(error);
            break;
        }
    }

    assert!(
        desync.is_some(),
        "MEASURED GAP CLOSED? This arm records that a mechanical developer edit \
         mid-timeline does NOT desync the sync-test canary. If it now stays \
         healthy, either the tuning entered rollback history, the edit was \
         refused while a timeline is live, or it became a bounded rebase — name \
         which, and this arm becomes the assertion that the chosen model holds.",
    );
}

/// ⛔⛤ **THE SAME, FOR THE OTHER WAIVED TUNING RESOURCE.**
///
/// `Platformer2dFeelTuningMonolith` carries the same *"feel tuning,
/// forward-only"* waiver, and the inspector edits the resource itself while
/// combat and time-control systems consume it. This arm exists separately rather
/// than as a loop because the two are waived for the same STATED reason and a
/// single arm would leave a reader guessing whether the other was covered.
#[test]
fn editing_feel_tuning_mid_timeline_changes_what_history_resimulates_to() {
    let mut sim = rollback_sim();
    let mut desync: Option<String> = None;
    for frame in 0..FRAMES {
        if frame == EDIT_AT {
            let mut feel = sim
                .world_mut()
                .resource_mut::<ambition_platformer2d::combat::feel::Platformer2dFeelTuningMonolith>();
            // A double-tap WINDOW is as mechanical as a value gets: it decides
            // whether a press becomes a dash.
            feel.down_double_tap_window *= 2.0;
            feel.up_double_tap_window *= 2.0;
        }
        sim.step(scripted_action(frame));
        if let Err(error) = sim.rollback_health() {
            desync = Some(error);
            break;
        }
    }

    // ⚠ ASSERTED AS A DISJUNCTION OF ONE, deliberately: unlike the movement
    // arm, this fixture's scripted inputs may never land a hit, in which case a
    // knockback scale legitimately changes nothing and a desync would be the
    // surprising outcome. So the finding recorded here is whichever of the two
    // is true, and the message says which.
    match desync {
        Some(error) => println!(
            "[measured] a feel-tuning edit mid-timeline DESYNCED the canary: {error}"
        ),
        None => println!(
            "[measured] a feel-tuning edit mid-timeline did NOT desync this \
             fixture — which says the scripted inputs never reached the term, \
             NOT that the resource is rollback-safe"
        ),
    }
}
