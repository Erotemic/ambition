//! The sync test reports an effect that only the first run of a frame has.
//!
//! A GGRS sync test rewinds on each step and runs the last frames again. It
//! compares the checksum of each saved frame against the first checksum it
//! saved for that frame. But it never saves the state that the FIRST run of a
//! frame leaves: the next step rewinds before it saves. So each compare is
//! between two resimulations, and an effect that only the first run has is in
//! no saved state.
//!
//! That effect is a real defect. On a peer whose inputs for the frame were
//! confirmed, the first run is the only run, and the effect stays; on a peer
//! that rewound across the frame, it is gone. A `Local` or a static that the
//! first run of a system sets is this shape.
//!
//! The rollback host takes the checksum of the first-run state itself and
//! compares it with the first save of that frame.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim as _, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::rollback::{GgrsSchedule, RollbackFrameCount};
use bevy::prelude::*;

/// The rewind depth of the sync test of this file.
const CHECK_DISTANCE: usize = 4;
/// The frame whose advance has the effect.
const FRAME: i32 = 40;
/// How far the effect moves the body, in pixels.
const PUSH: f32 = 48.0;

/// Which executions of the advance of `FRAME` the effect acts on.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Acts {
    /// The control: the effect is a function of the frame, so each run has it.
    OnEachRun,
    /// Only the first run. No resimulation has the effect.
    OnTheFirstRunOnly,
    /// Only the first resimulation.
    OnTheSecondRunOnly,
}

/// How many times the advance of `FRAME` ran. It is host-local and is not
/// rollback state, so a rewind does not take it back: it is what a `Local`
/// or a static in a simulation system is.
#[derive(Resource)]
struct Latch {
    acts: Acts,
    runs: u32,
}

fn push_the_body_at_one_frame(
    mut latch: ResMut<Latch>,
    frame: Res<RollbackFrameCount>,
    mut bodies: Query<&mut BodyKinematics, With<PrimaryPlayer>>,
) {
    if frame.0 != FRAME {
        return;
    }
    latch.runs += 1;
    let acts = match latch.acts {
        Acts::OnEachRun => true,
        Acts::OnTheFirstRunOnly => latch.runs == 1,
        Acts::OnTheSecondRunOnly => latch.runs == 2,
    };
    if acts {
        for mut kin in &mut bodies {
            kin.pos.x += PUSH;
        }
    }
}

fn x_of_the_body(sim: &mut Platformer2dSimHarness) -> f32 {
    let world = sim.world_mut();
    world
        .query_filtered::<&BodyKinematics, With<PrimaryPlayer>>()
        .single(world)
        .expect("one primary body")
        .pos
        .x
}

/// What one run of the fixture found.
#[derive(Debug)]
struct Found {
    /// How many times the advance of `FRAME` ran before the run ended.
    runs: u32,
    /// How far the body is from where it stood before `FRAME`, in pixels.
    moved: i32,
    /// The first report of the rollback host, if the session did not stay
    /// healthy. The harness does not step a session that is not healthy, so
    /// the run ends there.
    report: Option<String>,
    /// The last reason in the diagnostic history of the host.
    reason: Option<String>,
    /// How many first saves the host compared with the first run of a frame.
    compared: u64,
}

fn run(acts: Acts) -> Found {
    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(CHECK_DISTANCE, 10),
    )
    .expect("the sync-test harness builds");
    sim.world_mut().insert_resource(Latch { acts, runs: 0 });
    sim.app_mut().add_systems(GgrsSchedule, push_the_body_at_one_frame);
    let mut before = None;
    let mut report = None;
    for _ in 0..(FRAME as usize + 40) {
        if sim.world().resource::<Latch>().runs == 0 {
            before = Some(x_of_the_body(&mut sim));
        }
        sim.step(AgentAction::default());
        report = sim.rollback_health().err();
        if report.is_some() {
            break;
        }
    }
    let moved = (x_of_the_body(&mut sim) - before.expect("the body stood somewhere before the frame")).round() as i32;
    Found {
        runs: sim.world().resource::<Latch>().runs,
        moved,
        report,
        reason: sim
            .world()
            .resource::<ambition_platformer2d::rollback::RollbackDiagnosticHistory>()
            .last()
            .map(|diagnostic| diagnostic.reason.clone()),
        compared: sim
            .world()
            .resource::<ambition_platformer2d::rollback::FirstRunWitness>()
            .compared(),
    }
}

/// THE CONTROL. An effect that each run of the frame has is deterministic:
/// the session stays healthy and the body is moved. The host compared a first
/// save with a first run on each step, so the other arms are not red for the
/// reason that the witness is red always.
#[test]
fn an_effect_that_each_run_of_a_frame_has_leaves_the_session_healthy() {
    let found = run(Acts::OnEachRun);
    assert_eq!(
        (found.report.as_deref(), found.moved, found.runs),
        (None, PUSH as i32, CHECK_DISTANCE as u32 + 1),
        "(the report, how far the body moved, the runs of the frame): {found:?}"
    );
    assert!(
        found.compared >= 40,
        "the premise: the host compared the first save of a frame with its first run on each step: {found:?}"
    );
}

/// THE PROPERTY. An effect that only the first run of a frame has is reported
/// at that frame, on the step after it: the first resimulation does not have
/// it.
///
/// Measured 2026-10-05 before the witness: the session stayed healthy for the
/// 40 frames after, the frame ran five times, and the body was not moved. GGRS
/// compares a saved frame with the first save of that frame, and the first
/// save comes from the first resimulation.
#[test]
fn an_effect_that_only_the_first_run_of_a_frame_has_is_reported() {
    let found = run(Acts::OnTheFirstRunOnly);
    assert!(
        found.report.as_deref().is_some_and(|report| report.contains(&format!("[{FRAME}]"))),
        "the session stayed healthy, or the report does not name frame {FRAME}: {found:?}"
    );
    assert!(
        found.reason.as_deref().is_some_and(|reason| reason.contains("only the first run")),
        "the diagnostic does not say that the first run differs from its resimulation: {found:?}"
    );
    assert_eq!(found.runs, 2, "the runs of the frame before the report: the first run and one resimulation");
}

/// An effect that only the first resimulation has is reported too. GGRS
/// reports this one without the witness, one run later: its first save has
/// the effect and its second save does not.
#[test]
fn an_effect_that_only_the_first_resimulation_has_is_reported() {
    let found = run(Acts::OnTheSecondRunOnly);
    assert!(
        found.report.as_deref().is_some_and(|report| report.contains(&format!("[{FRAME}]"))),
        "the session stayed healthy, or the report does not name frame {FRAME}: {found:?}"
    );
}
