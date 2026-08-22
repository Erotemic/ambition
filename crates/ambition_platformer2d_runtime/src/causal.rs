//! Bevy host adapter for `ambition_causal` recording and frame stamps.
//!
//! Domain code depends on the Bevy-free causal vocabulary. ECS publishers write
//! the shared [`CausalRecording`] resource because the scoped causal sink is
//! thread-local and cannot safely receive facts from Bevy worker threads.
//! [`assert_no_offthread_loss`] exposes any accidental scoped-sink use.

use ambition_causal::{
    CausalFact, CausalRecording, Execution, FactDetail, RecordingPolicy, domains,
};
use bevy::prelude::*;

/// Publishers ordered after the causal frame stamp.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RecordingSet {
    Publish,
}

/// Install causal recording infrastructure without enabling any recording domain.
pub struct CausalPlugin;

impl Plugin for CausalPlugin {
    fn build(&self, app: &mut App) {
        // `First` provides a host frame stamp; the simulation schedule stamps
        // again when replay state is available. The writes are idempotent.
        app.init_resource::<CausalRecording>()
            .add_systems(bevy::app::First, stamp_causal_frame);
    }
}

/// Stamp the frame every subsequent fact belongs to: its tick, whether the host
/// is replaying it, and which lifecycle generation it is in.
///
/// Runs at the HEAD of the sim schedule, before any publisher. The host is
/// the only thing that knows any of these: a domain five hops down does not
/// know the world's clock, and it certainly does not know whether the host is
/// resimulating — a movement fact that guessed `Original` would make a replayed
/// tick indistinguishable from its original, which is the one distinction the
/// inspector must never lose.
pub fn stamp_causal_frame(
    time: Option<Res<ambition_time::SimTick>>,
    replay: Option<Res<ambition_platformer2d_shared_tangle::schedule::SimulationReplayState>>,
    boundary: Option<Res<ambition_platformer2d_core::confirmed_frame::ConfirmedFrameBoundary>>,
    // `Option`, because the FEATURE and the PLUGIN are two switches. Turning
    // `causal` on registers these systems; only `CausalPlugin` creates the
    // resource — and a host may legitimately compile the publishers without
    // installing an inspector. This was `ResMut` and killed six tests in
    // `ambition_demo_smash_app` the day that crate gained the feature:
    // "Resource does not exist", from a system nobody asked to run.
    log: Option<ResMut<CausalRecording>>,
    // The ROLLBACK EPOCH, kept here because only the host can see it.
    //
    // rollback can execute one tick more than once inside a generation, and
    // two attempts can produce DIFFERENT facts — which is precisely when
    // somebody opens an inspector. Grouped by `(generation, execution)` alone
    // they merged into one explanation and no query could say which attempt
    // produced a result.
    //
    // Bumped on the RISING EDGE of `replaying_history`: one rollback request
    // batch is one attempt, however many ticks it replays. A per-tick counter
    // would number ticks rather than attempts, which is the same information
    // the tick already carries.
    mut epoch: Local<RollbackEpoch>,
) {
    let Some(mut log) = log else {
        return;
    };
    if let Some(tick) = time {
        log.set_tick(tick.get());
    }
    let replaying = replay.is_some_and(|replay| replay.replaying_history);
    if replaying && !epoch.was_replaying {
        epoch.attempt = epoch.attempt.saturating_add(1);
    }
    epoch.was_replaying = replaying;
    let execution = if replaying {
        Execution::Resimulated
    } else {
        Execution::Original
    };
    let generation = boundary.map(|boundary| boundary.session).unwrap_or(0) as u32;
    // An ORIGINAL execution is always attempt 0: it happened once, and
    // numbering it by how many rollbacks preceded it would make the same
    // original tick answer to a different key depending on unrelated history.
    let attempt = if replaying { epoch.attempt } else { 0 };
    log.set_frame_attempt(execution, generation, attempt);
}

/// How many rollback batches this host has serviced. See
/// [`stamp_causal_frame`]'s `epoch` parameter.
#[derive(Default)]
pub struct RollbackEpoch {
    attempt: u32,
    was_replaying: bool,
}

/// Was this tick original execution or rollback resimulation?
///
/// One of the inspector's required questions, and the one no domain below the
/// host can answer: `SimulationReplayState` and the session generation are
/// facts about the HOST's relationship to time, not about any body.
///
/// Published with no subject, so it explains every body on that tick —
/// a resimulated frame is resimulated for all of them.
///
/// the fact records the generation as well as the flag. Frames restart at
/// zero on every session, so a tick number alone cannot tell a restart from a
/// rewind — the same reason `RollbackHealth` had to start carrying one.
pub fn record_execution_identity(log: Option<ResMut<CausalRecording>>) {
    // Same reason as `stamp_causal_frame`: the feature registers this, the
    // plugin creates the resource, and they are not the same switch.
    let Some(mut log) = log else {
        return;
    };
    if !log.is_recording() {
        return;
    }
    // Read back what `stamp_causal_frame` just decided, so this fact and every
    // other fact this frame agree by construction rather than by two systems
    // reading the same resources and hoping.
    let (execution, generation) = log.frame().unwrap_or((Execution::Original, 0));
    log.record(
        CausalFact::new(
            domains::ROLLBACK,
            0,
            FactDetail::new(
                "tick_execution",
                match execution {
                    Execution::Original => "original execution",
                    Execution::Resimulated => "rollback resimulation",
                },
            ),
        )
        .executed(execution)
        .in_generation(generation)
        .field("resimulated", execution == Execution::Resimulated)
        .field("generation", i64::from(generation)),
    );
}

/// The movement-intent observer, re-exported so a host installs the plugin and
/// the publishers through ONE path and cannot get half of them.
pub use ambition_platformer2d_actor_monolith::causal::{record_body_control_frame, record_player_movement_intent};

/// Turn the off-thread loss counter into a failure.
///
/// Call it after driving an app with recording on. A non-zero count means some
/// domain published through the thread-local sink from a worker thread and the
/// fact is gone — which would otherwise read as "that domain did not act".
pub fn assert_no_offthread_loss() {
    let lost = ambition_causal::facts_lost_offthread();
    assert_eq!(
        lost, 0,
        "{lost} causal fact(s) were published on a thread with no sink. A domain publishing \
         from an ECS system must use `ResMut<CausalRecording>`, not the thread-local sink."
    );
}

/// Turn recording on for a set of domains.
pub fn record_domains(app: &mut App, policy: RecordingPolicy) {
    app.world_mut()
        .resource_mut::<CausalRecording>()
        .set_policy(policy);
}
