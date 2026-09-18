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
//! replacement.
//!
//! ⛔⛔ **THAT PRICE WAS PAID ON 2026-09-18 AND THE ARM STILL IS NOT THE
//! REPLACEMENT. THE TRANSITION ROAD REMOVES ITSELF FROM THE WINDOW.**
//! `a_room_cutscene_taken_mid_session_starts_under_a_rewind` below does exactly
//! what the paragraph above prescribes — 40 settling frames in
//! `central_hub_complex`, then a door crossing into `cutscene_lab` — and it was
//! poisoned SINGLY and TOGETHER:
//!
//! | poison | result |
//! |---|---|
//! | `auto_trigger_room_cutscenes` moved into `Update` | **still green**, and the cutscene starts on the very first step after arrival, with no delay to measure |
//! | that, AND the `cutscene.last_room` registration deleted | **still green** |
//!
//! ⇒ The self-healing latch was the obvious explanation and the second poison
//! rules it out. What is left is `detect_room_transition_system`'s Track B: under
//! a rollback host it does not cross rooms on a speculative frame at all, it
//! records a `PendingLifecycleCommit` that the host commits *once the recording
//! frame is CONFIRMED*. A room change therefore cannot happen inside the check
//! distance by construction, so no trigger keyed on one can be made to cross a
//! rewind. ⇒ **A room transition is the wrong subject for this property, and the
//! ratchet's own prescription cannot be built.** Its replacement, if there is
//! one, has to drive a producer that fires on a SPECULATIVE frame — which is
//! what item 1's `CutsceneAdvanceRequest` already is, and why that one has a
//! failing witness and this one does not. The ratchet stays.
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

// ─────────────────────────────────────────────────────────────────────────────
// THE TRANSITION ARM.

/// The arm below starts here, and it is a RUNTIME ROOM ID rather than an LDtk
/// level identifier: the `central_hub_main` and `central_hub_basement` levels
/// both declare `activeArea: central_hub_complex`, so they are one room, and
/// that room authors the only door in the world to [`LAB_ROOM`]. It has no
/// cutscene binding of its own, so nothing fires at boot — which is exactly what
/// defeated the arm above.
const BASEMENT: &str = "central_hub_complex";
/// Bound to [`LAB_SCRIPT`] by `default_room_cutscene_bindings`, and reached only
/// through `cutscene_lab_door`.
const LAB_ROOM: &str = "cutscene_lab";
const LAB_SCRIPT: &str = "cutscene_lab_intro";

fn basement_arena(rollback: bool) -> Platformer2dSimHarness {
    let mut options = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_required_start_room(BASEMENT);
    if rollback {
        options = options.with_sync_test_rollback_settings(4, 10);
    }
    Platformer2dSimHarness::new_with_options(options)
        .expect("the central hub complex builds headlessly")
}

/// The authored `Door` zone of the ACTIVE room that leads to `target`, asked of
/// the same resolver the crossing itself uses.
fn door_to(
    sim: &mut Platformer2dSimHarness,
    target: &str,
) -> ambition_platformer2d::world::rooms::LoadingZone {
    let before = sim.observation().active_room.clone();
    let world = sim.world_mut();
    let mut query = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
    let room_set = query
        .iter(world)
        .next()
        .expect("the session has an active room set");
    let mut reachable: Vec<String> = Vec::new();
    let mut chosen = None;
    for zone in room_set.active_loading_zones() {
        if zone.activation != ambition_platformer2d::world::rooms::LoadingZoneActivation::Door {
            continue;
        }
        let Some(transition) = room_set.transition_for_player(
            zone.aabb,
            ambition_platformer2d::engine_core::Vec2::ZERO,
            true,
        ) else {
            continue;
        };
        let Some(destination) = room_set.rooms.get(transition.target_room) else {
            continue;
        };
        reachable.push(destination.id.clone());
        if destination.id == target {
            chosen = Some(zone.clone());
            break;
        }
    }
    chosen.unwrap_or_else(|| {
        panic!("'{before}' has no Door to '{target}'; its doors reach {reachable:?}")
    })
}

/// Stand in the door to `target` and hold interact until the active room
/// changes. Returns the room arrived in.
fn walk_through_the_door_to(sim: &mut Platformer2dSimHarness, target: &str) -> String {
    use ambition_platformer2d::engine_core::AabbExt as _;
    let before = sim.observation().active_room.clone();
    let door = door_to(sim, target);
    let center = door.aabb.center();
    sim.teleport_player((center.x, center.y));
    for _ in 0..120 {
        let room = sim
            .step(AgentAction {
                interact: true,
                interact_held: true,
                ..crate::common::base()
            })
            .active_room;
        if room != before {
            return room;
        }
    }
    panic!("held interact inside '{}' for 120 frames and '{before}' never changed", door.name);
}

/// ⭐ **A ROOM CUTSCENE BOUND TO A ROOM ENTERED MID-SESSION STARTS UNDER A
/// SYNC-TEST HOST**, with the fixed-tick arena as the control.
///
/// ⛔ **NOT "the trigger survived a rewind" either — read this file's header for
/// the three poisons that establish it cannot say that.** A room transition
/// commits only on a confirmed frame, so this crossing is never speculative.
/// What it pins is the composition: a mid-session crossing resolves its binding,
/// the queue reaches the drain, and the bound script starts. That is strictly
/// more than the boot arm above, which never leaves its first room.
#[test]
fn a_room_cutscene_taken_mid_session_starts_under_a_rewind() {
    let mut fixed = basement_arena(false);
    let mut rewinding = basement_arena(true);

    // Get well past the check distance before the transition, so the trigger
    // frame is one the sync-test host actually resimulates.
    const SETTLE: usize = 40;
    for _ in 0..SETTLE {
        fixed.step(crate::common::base());
        rewinding.step(crate::common::base());
    }

    let ticks = sim_ticks(&mut rewinding);
    assert!(
        ticks >= SETTLE as u64 - 2,
        "the rewinding arena reached tick {ticks} in {SETTLE} steps, so its timeline \
         is not advancing and nothing below was exercised"
    );
    assert_eq!(
        playing(&mut rewinding),
        None,
        "the basement fired a cutscene at boot, which would make the transition \
         below unobservable"
    );

    let arrived_fixed = walk_through_the_door_to(&mut fixed, LAB_ROOM);
    let arrived_rewinding = walk_through_the_door_to(&mut rewinding, LAB_ROOM);
    assert_eq!(arrived_fixed, LAB_ROOM);
    assert_eq!(arrived_rewinding, LAB_ROOM);

    for _ in 0..20 {
        fixed.step(crate::common::base());
        rewinding.step(crate::common::base());
    }

    assert_eq!(
        playing(&mut fixed).as_deref(),
        Some(LAB_SCRIPT),
        "the fixed-tick arena took the transition and never started the bound \
         cutscene, so the rollback reading below says nothing. Queue holds {:?}",
        queued(&mut fixed)
    );
    assert_eq!(
        playing(&mut rewinding).as_deref(),
        Some(LAB_SCRIPT),
        "the rewinding arena took the same transition and the cutscene did not \
         start. Queue holds {:?}",
        queued(&mut rewinding)
    );
}
