//! The recording is the SAME under a rollback host as under an eager one.
//!
//! ⛔⛔ THE RECORDER USED TO BE ROLLBACK STATE, AND THAT WAS A QUADRATIC COST.
//! `InputStream::push` was append-only, so a resimulated tick recorded itself a
//! second time; the stream stayed contiguous only because the whole growing
//! `Vec` of frames was cloned into every GGRS save. Saving frame N therefore
//! cost N, and a session paid for its own length twice over.
//!
//! ⭐ `push` IS TICK-ADDRESSED NOW. Re-recording a tick the stream has already
//! passed discards the abandoned tail and rewrites from there — which is what a
//! rewind means — so the recorder reproduces its own correct state from the
//! resimulation and needs no restore at all. The registration is gone.
//!
//! ⛔ THE EAGER ARM IS NOT DECORATION. Without it, "the sync-test recording
//! validates and is 90 frames long" would also hold for a recorder that dropped
//! every resimulated tick on the floor. What makes this a measurement is that
//! the two hosts produce the SAME BYTES for the same script.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use ambition_platformer2d::engine_core::{ControlFrame, InputStream};
use ambition_platformer2d::runtime::InputStreamRecorder;

const TICK_HZ: u32 = 60;
const TICKS: usize = 90;

/// Scripted input with enough shape that a dropped or duplicated tick shows up:
/// run, jump, reverse, dash, settle.
fn scripted_input(tick: usize) -> ControlFrame {
    let mut frame = ControlFrame::default();
    match tick {
        0..=19 => {
            frame.axis_x = 1.0;
            frame.right_pressed = tick == 0;
        }
        20 => {
            frame.axis_x = 1.0;
            frame.jump_pressed = true;
            frame.jump_held = true;
        }
        21..=34 => {
            frame.axis_x = 1.0;
            frame.jump_held = true;
        }
        35 => {
            frame.axis_x = 1.0;
            frame.jump_released = true;
        }
        36..=49 => {
            frame.axis_x = -1.0;
            frame.left_pressed = tick == 36;
        }
        50 => {
            frame.axis_x = -1.0;
            frame.burst_pressed = true;
        }
        _ => frame.axis_x = -0.35,
    }
    frame
}

fn record(sim: &mut Platformer2dSimHarness) -> InputStream {
    sim.world_mut()
        .resource_mut::<InputStreamRecorder>()
        .arm_single_player(TICK_HZ);
    for tick in 0..TICKS {
        sim.step_frame(scripted_input(tick));
    }
    sim.world_mut()
        .resource_mut::<InputStreamRecorder>()
        .finish()
        .expect("the recorder was armed")
}

fn eager() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("the eager harness builds")
}

/// `check_distance: 4` makes GGRS re-simulate the last four frames every step,
/// so every tick past the fourth is recorded more than once.
fn rewinding() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("the GGRS sync-test harness builds")
}

#[test]
fn a_rewinding_host_records_the_same_stream_an_eager_one_does() {
    let eager_stream = record(&mut eager());
    let rewound_stream = record(&mut rewinding());

    // ⛔ THE PREMISE, first: an eager recording that was itself broken would
    // make the comparison below agree about nothing.
    assert_eq!(eager_stream.validate(), Ok(()));
    assert_eq!(
        eager_stream.len(),
        TICKS,
        "the eager arm records exactly one frame per step"
    );

    assert_eq!(
        rewound_stream.validate(),
        Ok(()),
        "a resimulated tick was recorded twice, so the stream is no longer \
         contiguous"
    );
    assert_eq!(
        rewound_stream.len(),
        TICKS,
        "the rewinding host recorded {} frames for {TICKS} ticks",
        rewound_stream.len()
    );
    // ⛔ THE TWO HOSTS DO NOT START ON THE SAME `SimTick`, and that is not what
    // this test is about. The eager harness records from tick 2 and the GGRS one
    // from tick 1 — an arming-vs-session-start offset that exists with or
    // without a rewind. What must match is the SEQUENCE of recorded input; the
    // absolute tick each host happened to be on when the recorder armed is the
    // harness's business.
    assert_eq!(
        rewound_stream.len(),
        eager_stream.len(),
        "the two hosts recorded different numbers of ticks"
    );

    // ⛔ COMPARE A COMPACT PROJECTION, not the whole `Debug`. Ninety frames of
    // thirty-odd fields makes a failure unreadable, which is a failure that
    // teaches nothing — and the fields below are the ones the script actually
    // varies.
    let shape = |stream: &InputStream| -> Vec<(i32, bool, bool, bool)> {
        stream
            .primary_frames()
            .map(|frame| {
                (
                    (frame.axis_x * 100.0) as i32,
                    frame.jump_pressed || frame.jump_held || frame.jump_released,
                    frame.burst_pressed,
                    frame.left_pressed || frame.right_pressed,
                )
            })
            .collect()
    };
    let (rewound, eager) = (shape(&rewound_stream), shape(&eager_stream));
    // ⛔ THE PREMISE AGAIN: a script that never varies would make the comparison
    // hold for any recorder at all.
    assert!(
        eager.iter().collect::<std::collections::BTreeSet<_>>().len() > 3,
        "the script has to vary or this comparison proves nothing: {eager:?}"
    );
    assert_eq!(
        rewound, eager,
        "the same script through two hosts must produce the same recording"
    );
}

/// ⭐ AND THE RECORDER IS NOT IN THE SNAPSHOT. The behavioural test above would
/// also pass with the registration still in place — restoring the stream is one
/// way to keep it contiguous, it is just the expensive way. This is the arm that
/// says the cost is actually gone.
#[test]
fn the_recorder_is_not_snapshotted_by_the_rollback_host() {
    let sim = rewinding();
    let registry = sim
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackRegistry>();
    let rows: Vec<(&str, ambition_platformer2d::rollback::RollbackEntryKind)> = registry
        .descriptors()
        .filter(|d| d.type_name.contains("InputStreamRecorder"))
        .map(|d| (d.type_name.as_str(), d.kind))
        .collect();
    assert!(
        !rows.is_empty(),
        "the recorder must still be DECLARED — an unregistered type is an \
         unanswered question, not an answered one"
    );
    assert!(
        rows.iter().all(|(_, kind)| *kind
            == ambition_platformer2d::rollback::RollbackEntryKind::Derived),
        "the recorded input history is being cloned into every GGRS save, so \
         saving frame N costs N: {rows:?}"
    );
}

/// The eager arm alone, so a failure above can be attributed.
#[test]
fn the_eager_recording_is_the_control() {
    let stream = record(&mut eager());
    assert_eq!(stream.validate(), Ok(()));
    assert_eq!(stream.slot_count(), Some(1));
    assert_eq!(stream.tick_hz, TICK_HZ);
    let _ = AgentAction::default();
}
