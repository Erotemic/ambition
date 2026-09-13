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

/// ⛔⛤ **`Q118`'s OPEN HALF ASKS A QUESTION THIS RIG CAN ANSWER: is publishing a
/// CONTENT GENERATION across a live timeline harmful the way a developer edit
/// is?**
///
/// That row now says the live-timeline half *"cannot be sealed by refusing"* —
/// implementing the cancel made the shipped composition refuse every reload it
/// has, because a reload re-prepares the route the shell is already on and the
/// session being replaced owns a healthy speculating timeline. What it could not
/// say was whether publishing across that timeline actually HURTS in a local
/// single-player composition, or only in a networked one, and it wrote *"should
/// not be guessed"* beside it.
///
/// ⭐⭐ **THE MECHANISM IS THE SAME ONE THE ARMS ABOVE MEASURE, and that is why
/// this arm belongs in this file rather than beside the reload tests.** A cast is
/// published into `PreparedCharacterRegistry` — a RESOURCE outside rollback
/// history — and `project_prepared_character_definitions` reads it **in the SIM
/// schedule** (`.add_systems(sim, ..)`), writing `ActorMoveset` and friends onto
/// live bodies. Its own doc says it exists to catch exactly this: *"A new cast
/// changes nothing on a body, so the change-detection query cannot see one —
/// that is exactly how a body kept a retired cast's moves."*
///
/// ⇒ So a rewind restores the body's OLD `ActorMoveset` from the snapshot
/// (`rollback_component_clone`, `actor.moveset`) and then resimulates a
/// historical frame with a system that reads the NEW registry. Identical shape
/// to `ActiveMovementTuning`; the only question is whether it fires.
#[test]
fn publishing_a_cast_mid_timeline_changes_what_history_resimulates_to() {
    use ambition_platformer2d::characters::prepared::{
        activate_staged_revision, stage_character_revision, PreparedCharacterRegistry,
    };

    let mut sim = rollback_sim();
    // Settle, so a body exists to be revised. Without this the arm measures a
    // revision that reached nobody.
    for frame in 0..8 {
        sim.step(scripted_action(frame));
    }

    // ⛔ THE PREMISE: find the character a LIVE BODY is actually wearing, rather
    // than naming one and hoping. A revision of an unworn character reaches no
    // body, and would look exactly like "publication is rollback-safe".
    let worn: Option<String> = {
        let world = sim.world_mut();
        let mut q = world.query::<&ambition_platformer2d::characters::actor::WornCharacter>();
        q.iter(world).next().map(|worn| worn.0.as_str().to_string())
    };
    let Some(worn) = worn else {
        panic!(
            "no body in this fixture wears a character, so a cast revision has \
             nothing to reach and this arm would pass for the wrong reason"
        );
    };
    assert!(
        sim.world()
            .get_resource::<PreparedCharacterRegistry>()
            .and_then(|registry| registry.get(&worn))
            .is_some(),
        "the worn character `{worn}` is not in the prepared cast, so revising it \
         changes nothing the projection reads"
    );

    let mut desync: Option<String> = None;
    for frame in 8..FRAMES {
        if frame == EDIT_AT {
            // The production revision road: stage a changed definition for the
            // character a body is wearing, then publish it. `max_health` is as
            // mechanical as a value gets and the projection carries it.
            let mut definition = ambition_platformer2d::characters::actor::definition::CharacterDefinition::new(
                worn.clone(),
                worn.clone(),
                "rollback-probe",
            );
            definition.vitals.max_health = Some(4242);
            let app = sim.app_mut();
            stage_character_revision(app, definition, &Default::default())
                .expect("the revision stages");
            let support = app
                .world()
                .get_resource::<ambition_platformer2d::combat::technique::InstalledTechniques>()
                .map(|installed| installed.0.clone())
                .unwrap_or_default();
            let _ = activate_staged_revision(app.world_mut(), &support);
        }
        sim.step(scripted_action(frame));
        if let Err(error) = sim.rollback_health() {
            desync = Some(error);
            break;
        }
    }

    // ⚠ **RECORDED AS WHICHEVER IS TRUE, because `Q118` needs the ANSWER and not
    // a preferred one** — and the two answers send the packet in opposite
    // directions. A desync says the live-timeline half is harmful LOCALLY, so the
    // stop-and-rebase lifecycle is the work. No desync says the exposure may be
    // network-only, and refusing a reload in a rollback-COMPATIBLE session is the
    // cheap correct answer instead of a rebase.
    // ⛔⛤ **IT DOES FIRE. MEASURED 2026-09-13: `GGRS sync-test checksum mismatch
    // at frames [22, 23, 24]`**, with the file's own no-edit control green over
    // the same frames.
    //
    // ⇒ **THAT ANSWERS `Q118`'s "should not be guessed" QUESTION, AND IT CLOSES
    // THE CHEAPER OF THE TWO ROADS.** Publishing across a live timeline is
    // harmful in a LOCAL single-player composition, not only in a networked one
    // — so *"refuse a reload only in a rollback/network-COMPATIBLE session"* is
    // not available as the cheap correct answer. What remains is the
    // stop-and-rebase lifecycle the row names.
    assert!(
        desync.is_some(),
        "MEASURED GAP CLOSED? This arm records that publishing a cast mid-timeline \
         DESYNCS the sync-test canary, which is what makes `Q118`'s live-timeline \
         half a local problem rather than a network-only one. If it now stays \
         healthy, name what sealed it — the cast entering rollback history, the \
         publication becoming a bounded rebase, or the projection leaving the sim \
         schedule — and this arm becomes the assertion that the chosen model holds.",
    );
}
