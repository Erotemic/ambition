//! An authored room cutscene starts under a world that resimulates every frame.
//!
//! ⛔⛔ **READ THIS BEFORE CITING THIS FILE: IT IS NOT THE ARM THAT REPLACES
//! `check_sim_consumed_request_writers.py`, AND FINDING THAT OUT IS WHAT THIS
//! FILE MEASURED.** That ratchet's docstring names its own replacement — *"drive
//! a cutscene trigger across a rewind in a sync-test world and assert the
//! cutscene still starts"* — so this was built to be it, and then poisoned two
//! ways. **Both poisons PASSED**, 2026-09-17:
//!
//! | poison | prediction | result |
//! |---|---|---|
//! | `auto_trigger_room_cutscenes` moved from the sim schedule into `Update` | the ratchet's stated mechanism says the request is lost | **still green** |
//! | `cutscene.last_room` registration deleted | the edge latch's restoration is what re-queues | **still green** |
//!
//! ⇒ Neither explanation is what keeps this green, and a deeper layer owns it:
//! the room this arm boots into fires its binding at the FIRST tick, before any
//! rewind window has opened, and `ActiveCutscene` is itself rollback state
//! (`cutscene.playback`). Once playing, every rewind restores it playing and
//! `drain_cutscene_triggers` returns early on `is_playing()`. The queue's
//! cross-frame life in this fixture is about one frame, at boot, outside the
//! window. ⇒ **The property the ratchet wants needs a room TRANSITION taken
//! mid-session, well inside the check distance**, and that is the price of its
//! replacement. The ratchet stays until then.
//!
//! ⛔⛤ **AND THE RATCHET'S STATED MECHANISM IS WRONG FOR ITS OWN SUBJECT
//! ANYWAY.** Its premise is *"the restore puts the resource back and nothing
//! re-produces the request"*, which is the failure mode of a REGISTERED resource
//! drained in-sim. Measured against
//! `game/ambition_app/tests/rollback_schema_baseline.txt`:
//! `CutsceneTriggerQueue` **is not registered at all** — only `cutscene.playback`
//! and `cutscene.last_room` are. A rewind does not put the queue back; it leaves
//! the write AND the drain where the rolled-forward frames left them.
//!
//! ⭐ WHAT THIS ARM DOES PIN, which is worth having: the authored binding
//! resolves, the library holds the script, and the cutscene is playing under a
//! sync-test session — a boot-path regression net for the rollback composition,
//! not a statement about a request crossing a rewind.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions};
use ambition_app::AmbitionSim;
use ambition_app::TimestepMode;

/// The authored binding this arm drives: `INTRO_ROOM_CUTSCENE_BINDINGS`'s first
/// row, installed unconditionally by `IntroPlugin`.
const ROOM: &str = "intro_wake_room";
const SCRIPT: &str = "intro_wake";

fn arena(rollback: bool) -> Platformer2dSimHarness {
    let mut options = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        // ⚠ REQUIRED, not tolerant: the fallback would run this whole arm in
        // the authored start room, which has no cutscene binding, and report a
        // clean absence.
        .with_required_start_room(ROOM);
    if rollback {
        options = options.with_sync_test_rollback_settings(4, 10);
    }
    Platformer2dSimHarness::new_with_options(options)
        .expect("the intro wake room builds headlessly")
}

/// The id of the cutscene currently playing, or `None`.
fn playing(sim: &mut Platformer2dSimHarness) -> Option<String> {
    sim.world()
        .get_resource::<ambition_platformer2d::cutscene::ActiveCutscene>()
        .and_then(|active| active.runtime.as_ref())
        .map(|runtime| runtime.script.id.clone())
}

/// What the trigger queue holds right now. Read to distinguish "never queued"
/// from "queued and lost".
fn queued(sim: &mut Platformer2dSimHarness) -> Vec<String> {
    sim.world()
        .get_resource::<ambition_platformer2d::cutscene::CutsceneTriggerQueue>()
        .map(|queue| queue.0.clone())
        .unwrap_or_default()
}

fn sim_ticks(sim: &mut Platformer2dSimHarness) -> u64 {
    sim.world()
        .get_resource::<ambition_platformer2d::time::SimTick>()
        .map(|tick| tick.0)
        .unwrap_or(0)
}

/// ⭐ **AN AUTHORED ROOM CUTSCENE IS PLAYING UNDER A SYNC-TEST SESSION, AND THE
/// FIXED-TICK HOST IS THE CONTROL.**
///
/// ⛔ **NOT "the request survived a rewind" — see this file's header for the two
/// poisons that establish it does not say that.** What it says is that the
/// binding resolves and the script starts in a rollback composition, which is
/// the thing a schedule or registration change in this area would most cheaply
/// break.
///
/// ⚠ It asserts the playing script's id and nothing about its beats —
/// `a_fade_beat_projects_its_target_at_every_instant_rather_than_a_ramp` owns
/// what a beat shows.
#[test]
fn an_authored_room_cutscene_starts_with_and_without_a_rewind() {
    let mut fixed = arena(false);
    let mut rewinding = arena(true);

    const STEPS: usize = 90;
    for _ in 0..STEPS {
        fixed.step(AgentAction::default());
        rewinding.step(AgentAction::default());
    }

    // ⛔ THE LIVENESS FLOOR, BEFORE ANY COMPARISON. A rollback world that never
    // advanced reports the same "no cutscene" as one that lost the request, and
    // a world that never advanced also never triggered anything.
    let ticks = sim_ticks(&mut rewinding);
    assert!(
        ticks >= STEPS as u64 - 2,
        "the rewinding arena reached tick {ticks} in {STEPS} steps, so its \
         timeline is not advancing and nothing here was exercised"
    );

    // THE CONTROL, and it is the absence of the subject: no rewind window.
    assert_eq!(
        playing(&mut fixed).as_deref(),
        Some(SCRIPT),
        "the fixed-tick arena never started the authored cutscene, so the room \
         binding or the library is not what this arm assumes and the rollback \
         reading below says nothing. Queue holds {:?}",
        queued(&mut fixed)
    );

    assert_eq!(
        playing(&mut rewinding).as_deref(),
        Some(SCRIPT),
        "the rewinding arena never started the authored cutscene. `None` with an \
         EMPTY queue means the request was drained on a frame that was then \
         rolled back and nothing re-produced it; `None` with the script still \
         QUEUED means the drain is not running. Queue holds {:?}",
        queued(&mut rewinding)
    );
}
