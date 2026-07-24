//! GGRS session/input bridge shared by the harness and future network hosts.

use bevy::prelude::*;
use bevy_ggrs::ggrs::{self, PlayerType, SessionBuilder};
use bevy_ggrs::{
    ConfirmedFrameCount, GgrsConfig, GgrsSchedule, GgrsTime, LoadWorld, LocalInputs, LocalPlayers,
    PlayerInputs, ReadInputs, RollbackFrameCount, RunGgrsSystems, Session, SyncTestMismatch,
};

use ambition_engine_core::{ConfirmedFrameBoundary, ControlFrame, ControlFrameLatch};

use super::RollbackRegistry;
use crate::{PreparedContentIdentity, SnapshotSchemaFingerprint};

pub type AmbitionGgrsConfig = GgrsConfig<ControlFrame>;
pub type AmbitionGgrsSession = Session<AmbitionGgrsConfig>;

#[derive(SystemSet, Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum AmbitionReadInputsSet {
    CaptureDeviceLatch,
    PublishLocalInputs,
}

/// External input waiting to be submitted to GGRS for the next frame. This is
/// intentionally not rollback state: prediction/session logic owns the input
/// stream, while simulation state is rewound beneath it.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct PendingLocalInput(pub ControlFrame);

/// Counts actual GGRS operations. It is intentionally outside rollback state so
/// tests can prove that a single harness step performed load/resimulation work.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RollbackExecutionStats {
    pub advance_runs: u64,
    pub load_runs: u64,
    /// The frame of the most recent advance, replay or not.
    pub last_simulated_frame: i32,
    /// High-water mark across every advance. A frame at or below it is being
    /// re-simulated, which is how [`count_advance_run`] tells a replay pass from
    /// a first-time one. `None` until the first advance, so frame 0 is not
    /// mistaken for a replay of itself.
    pub highest_simulated_frame: Option<i32>,
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct RollbackSessionStatus {
    pub mismatch_frames: Vec<i32>,
    pub invalidation: Option<String>,
}

/// Monotonic identity for rollback timelines.
///
/// This resource deliberately survives session teardown. Frame numbers restart
/// at zero for every GGRS session, so deriving a generation from the optional
/// [`ConfirmedFrameBoundary`] aliases a stopped-and-restarted session with the
/// one that preceded it. Host-side journals and traces use this generation to
/// discard work from timelines that no longer exist.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RollbackSessionGeneration(u64);

#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct RollbackSessionContract {
    pub content: Option<PreparedContentIdentity>,
    pub schema: SnapshotSchemaFingerprint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyncTestSettings {
    pub check_distance: usize,
    pub max_prediction_window: usize,
}

/// Who owns the currently installed GGRS session.
///
/// Local sync-test sessions may be stopped and recreated around a developer
/// content reload. External/P2P sessions require a coordinated peer barrier and
/// must never be replaced unilaterally by the local host.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub enum RollbackSessionOwnership {
    LocalSyncTest(SyncTestSettings),
    External,
}

impl Default for SyncTestSettings {
    fn default() -> Self {
        Self {
            check_distance: 7,
            max_prediction_window: 12,
        }
    }
}

pub fn start_sync_test_session(
    world: &mut World,
    settings: SyncTestSettings,
) -> Result<(), ggrs::GgrsError> {
    // A newly installed GGRS session always starts from the current live world
    // as frame zero. Snapshot stores are intentionally retained here: the first
    // SaveWorld request at frame zero replaces every non-negative frame in each
    // bevy_ggrs ring, while resetting these frame resources prevents that save
    // from being mislabeled with the previous session's frame number.
    world.insert_resource(RollbackFrameCount(0));
    world.insert_resource(ConfirmedFrameCount(-1));
    world.insert_resource(PendingLocalInput::default());
    if world.contains_resource::<ControlFrameLatch>() {
        world.insert_resource(ControlFrameLatch::default());
    }

    // GgrsTimePlugin derives deterministic elapsed time from RollbackFrameCount
    // by calling Time::advance_to. Replacing a running session resets the frame
    // counter to zero, so retaining the previous session's elapsed GGRS clock
    // would ask Bevy to move time backwards on the first AdvanceWorld and panic.
    // A session rebase is a new deterministic timeline: reset its clock along
    // with its frame identity before the first frame-zero snapshot is saved.
    world.insert_resource(Time::<GgrsTime>::new_with(GgrsTime));

    let session = SessionBuilder::<AmbitionGgrsConfig>::new()
        .with_num_players(1)?
        .with_fps(crate::SIM_TICK_HZ as usize)?
        .with_max_prediction_window(settings.max_prediction_window)
        .with_check_distance(settings.check_distance)
        .add_player(PlayerType::Local, 0)?
        .start_synctest_session()?;
    install_session_with_ownership(
        world,
        AmbitionGgrsSession::SyncTest(session),
        RollbackSessionOwnership::LocalSyncTest(settings),
    );
    Ok(())
}

/// Install any already-constructed GGRS session behind Ambition's exact
/// content/schema contract. Matchbox will eventually construct a P2P session
/// and hand it to this same seam; the harness uses [`start_sync_test_session`].
pub fn install_session(world: &mut World, session: AmbitionGgrsSession) {
    install_session_with_ownership(world, session, RollbackSessionOwnership::External);
}

fn install_session_with_ownership(
    world: &mut World,
    session: AmbitionGgrsSession,
    ownership: RollbackSessionOwnership,
) {
    let schema = world
        .get_resource::<RollbackRegistry>()
        .cloned()
        .unwrap_or_default()
        .schema_fingerprint();
    let content = live_content_identity(world);
    world.insert_resource(RollbackSessionContract { content, schema });
    world.insert_resource(RollbackSessionStatus::default());
    world.insert_resource(RollbackExecutionStats::default());
    world.insert_resource(ownership);
    world.insert_resource(session);

    // A new session is a new timeline. The counter lives independently of the
    // boundary because teardown removes the boundary; deriving from that optional
    // resource would make every stop/restart cycle reuse generation zero.
    let generation = {
        let mut generation =
            world.get_resource_or_insert_with::<RollbackSessionGeneration>(Default::default);
        generation.0 = generation.0.wrapping_add(1);
        generation.0
    };
    world.insert_resource(ConfirmedFrameBoundary {
        current: 0,
        confirmed: -1,
        session: generation,
    });
}

/// Remove every resource whose presence means a rollback session is active.
///
/// The generation counter intentionally survives: the next installation must
/// receive a different identity even after the boundary itself is removed.
pub fn stop_session(world: &mut World) {
    world.remove_resource::<AmbitionGgrsSession>();
    world.remove_resource::<RollbackSessionContract>();
    world.remove_resource::<RollbackSessionOwnership>();
    // Nothing speculates any more, so external effects and persistence return
    // to their non-rollback behavior immediately. Leaving this installed would
    // strand pending effects and keep confirmed-state save gates closed forever.
    world.remove_resource::<ConfirmedFrameBoundary>();
}

/// Queue the exact same teardown from a regular Bevy system.
pub fn stop_session_deferred(commands: &mut Commands) {
    commands.queue(|world: &mut World| stop_session(world));
}

/// Return a diagnostic error when GGRS invalidated the session contract or a
/// sync-test checksum mismatch was observed.
pub fn session_health(world: &World) -> Result<(), String> {
    let Some(status) = world.get_resource::<RollbackSessionStatus>() else {
        return Ok(());
    };
    if let Some(reason) = &status.invalidation {
        return Err(reason.clone());
    }
    if !status.mismatch_frames.is_empty() {
        return Err(format!(
            "GGRS sync-test checksum mismatch at frames {:?}",
            status.mismatch_frames
        ));
    }
    Ok(())
}

pub fn session_is_active(world: &World) -> bool {
    world.contains_resource::<AmbitionGgrsSession>()
}

pub(crate) fn install_session_bridge(app: &mut App) {
    // Only a speculating host quarantines external effects, so the whole
    // mechanism is installed HERE rather than in the engine group: a fixed-tick
    // or render-frame game carries none of these systems at all.
    crate::external_effects::quarantine_presentation_effects(app, LoadWorld);

    app.init_resource::<PendingLocalInput>()
        .init_resource::<ambition_platformer_primitives::schedule::SimulationReplayState>()
        .init_resource::<RollbackExecutionStats>()
        .init_resource::<RollbackSessionStatus>()
        .configure_sets(
            ReadInputs,
            (
                AmbitionReadInputsSet::CaptureDeviceLatch,
                AmbitionReadInputsSet::PublishLocalInputs,
            )
                .chain(),
        )
        .add_systems(
            ReadInputs,
            capture_latched_local_input.in_set(AmbitionReadInputsSet::CaptureDeviceLatch),
        )
        .add_systems(
            ReadInputs,
            publish_local_inputs.in_set(AmbitionReadInputsSet::PublishLocalInputs),
        )
        .add_systems(
            GgrsSchedule,
            (publish_ggrs_input, count_advance_run)
                .chain()
                .before(ambition_platformer_primitives::schedule::SandboxSet::CoreSimulation),
        )
        .add_systems(
            LoadWorld,
            (
                // Publishes the restored frame, which the abandoned-branch
                // discard reads. The edge is required, not incidental.
                mark_historical_replay
                    .before(crate::external_effects::ExternalEffectSet::DiscardAbandoned),
                count_load_run.in_set(super::AmbitionLoadWorldSet::Reconcile),
            ),
        )
        .add_systems(
            PreUpdate,
            (
                enforce_session_contract.before(RunGgrsSystems),
                clear_historical_replay.after(RunGgrsSystems),
                // Track B: execute a confirmed deferred lifecycle op in the
                // exclusive world and rebase, after the advance batch is done.
                // Ordered AFTER the external-effect Release: the rebase bumps the
                // session generation, and the effect journal discards any pending
                // confirmed effects stamped with the OLD generation — so they must
                // be released to presentation first, or transition-adjacent
                // SFX/VFX/debris confirmed before the rebase would be dropped.
                crate::lifecycle_commit::commit_confirmed_lifecycle
                    .after(RunGgrsSystems)
                    .after(clear_historical_replay)
                    .after(crate::external_effects::ExternalEffectSet::Release),
            ),
        )
        // Effects may only be released once this render frame's advances are
        // done. Without this edge Bevy is free to release first, and the next
        // advance's outbox clear then wipes what was just handed to
        // presentation — silently, since the journal has already counted it.
        .configure_sets(
            PreUpdate,
            crate::external_effects::ExternalEffectSet::Release.after(RunGgrsSystems),
        )
        .add_observer(record_sync_test_mismatch);
}

/// Consume device input only when GGRS actually asks for the next local
/// input. Draining the latch once per rendered frame is incorrect: several
/// rendered frames may pass before a simulation tick, and a later level-only
/// sample would overwrite a short press before GGRS observed it.
fn capture_latched_local_input(
    latch: Option<ResMut<ControlFrameLatch>>,
    mut pending: ResMut<PendingLocalInput>,
) {
    if let Some(mut latch) = latch {
        pending.0 = latch.take();
    }
}

fn publish_local_inputs(
    pending: Res<PendingLocalInput>,
    local_players: Res<LocalPlayers>,
    mut commands: Commands,
) {
    let mut inputs = bevy::platform::collections::HashMap::default();
    for &handle in &local_players.0 {
        inputs.insert(handle, pending.0);
    }
    commands.insert_resource(LocalInputs::<AmbitionGgrsConfig>(inputs));
}

fn publish_ggrs_input(
    inputs: Res<PlayerInputs<AmbitionGgrsConfig>>,
    mut control: ResMut<ControlFrame>,
) {
    *control = inputs.first().map(|(input, _)| *input).unwrap_or_default();
}

/// Publish the FACT "this frame number has been simulated before".
///
/// Deliberately a fact, not a policy — but note how few consumers it has left.
/// External effects (audio, VFX) no longer read it at all: "ran before" is not
/// "is settled", and answering the wrong question is what made the old
/// `SfxEmissionGate` keep phantoms and drop corrections. They now go through
/// [`crate::external_effects`], which defers rather than suppresses.
///
/// What remains are consumers that genuinely need to know a frame is being
/// revisited: the forensic trace uses it to avoid consuming per-logical-frame
/// suppression windows twice, and the falling-sand grid uses it as a step guard.
fn publish_replay_pass(
    replay: &mut ambition_platformer_primitives::schedule::SimulationReplayState,
    simulated_before: bool,
) {
    replay.replaying_history = simulated_before;
}

/// Decide, per advance, whether GGRS is re-simulating a frame it already ran,
/// and publish where the confirmed boundary sits.
///
/// The frame number is the exact test for the first: at or below the high-water
/// mark means this frame was simulated before. Bracketing on "a rollback
/// happened this render frame" is NOT equivalent — `clear_historical_replay`
/// runs after the whole GGRS batch, so the coarse window also covers the
/// brand-new frame at the end of a rollback.
///
/// [`ConfirmedFrameBoundary`] is the separate, stronger fact: which frames can
/// never be simulated again. `ConfirmedFrameCount` is maintained by `bevy_ggrs`
/// for both session kinds (a P2P session's confirmed frame; `current -
/// check_distance` under sync test), so this works in the harness and online.
fn count_advance_run(
    frame: Res<RollbackFrameCount>,
    confirmed: Option<Res<ConfirmedFrameCount>>,
    mut stats: ResMut<RollbackExecutionStats>,
    mut replay: ResMut<ambition_platformer_primitives::schedule::SimulationReplayState>,
    boundary: Option<ResMut<ConfirmedFrameBoundary>>,
) {
    stats.advance_runs = stats.advance_runs.saturating_add(1);
    stats.last_simulated_frame = frame.0;
    let simulated_before = stats
        .highest_simulated_frame
        .is_some_and(|highest| frame.0 <= highest);
    stats.highest_simulated_frame = Some(
        stats
            .highest_simulated_frame
            .map_or(frame.0, |highest| highest.max(frame.0)),
    );
    publish_replay_pass(&mut replay, simulated_before);
    if let Some(mut boundary) = boundary {
        boundary.current = frame.0;
        boundary.confirmed = confirmed.map_or(-1, |confirmed| confirmed.0);
    }
}

/// `LoadWorld`: the host has restored `frame`, so the simulation now sits
/// there. Republishing it is what lets `discard_abandoned_predictions` drop the
/// branch that was just walked away from without naming a GGRS type.
fn mark_historical_replay(
    frame: Res<RollbackFrameCount>,
    mut replay: ResMut<ambition_platformer_primitives::schedule::SimulationReplayState>,
    boundary: Option<ResMut<ConfirmedFrameBoundary>>,
) {
    publish_replay_pass(&mut replay, true);
    if let Some(mut boundary) = boundary {
        boundary.current = frame.0;
    }
}

fn clear_historical_replay(
    mut replay: ResMut<ambition_platformer_primitives::schedule::SimulationReplayState>,
) {
    publish_replay_pass(&mut replay, false);
}

fn count_load_run(mut stats: ResMut<RollbackExecutionStats>) {
    stats.load_runs = stats.load_runs.saturating_add(1);
}

fn record_sync_test_mismatch(
    trigger: On<SyncTestMismatch>,
    mut status: ResMut<RollbackSessionStatus>,
) {
    status
        .mismatch_frames
        .extend(trigger.event().mismatched_frames.iter().copied());
}

fn enforce_session_contract(world: &mut World) {
    if !session_is_active(world) {
        return;
    }

    let current_schema = world
        .get_resource::<RollbackRegistry>()
        .cloned()
        .unwrap_or_default()
        .schema_fingerprint();
    let current_content = live_content_identity(world);

    let Some(contract) = world.get_resource::<RollbackSessionContract>().cloned() else {
        world.insert_resource(RollbackSessionContract {
            content: current_content,
            schema: current_schema,
        });
        return;
    };

    if contract.schema != current_schema {
        invalidate_session(
            world,
            format!(
                "GGRS rollback schema changed while the session was active: expected {}, observed {}",
                contract.schema, current_schema
            ),
        );
        return;
    }

    match (contract.content, current_content) {
        (None, Some(identity)) => {
            world.resource_mut::<RollbackSessionContract>().content = Some(identity);
        }
        (Some(expected), Some(observed)) if expected != observed => {
            invalidate_session(
                world,
                format!(
                    "prepared content changed while the GGRS session was active: expected {:?}, observed {:?}",
                    expected, observed
                ),
            );
        }
        (Some(expected), None) => {
            invalidate_session(
                world,
                format!(
                    "canonical prepared content {:?} disappeared while the GGRS session was active",
                    expected
                ),
            );
        }
        _ => {}
    }
}

fn invalidate_session(world: &mut World, reason: String) {
    stop_session(world);
    world
        .get_resource_or_insert_with::<RollbackSessionStatus>(Default::default)
        .invalidation = Some(reason);
}

fn live_content_identity(world: &mut World) -> Option<PreparedContentIdentity> {
    let mut query = world.query::<&PreparedContentIdentity>();
    query.iter(world).next().copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restarting_a_sync_test_session_rebases_ggrs_time_to_frame_zero() {
        let mut world = World::new();
        let mut old_timeline = Time::<GgrsTime>::new_with(GgrsTime);
        old_timeline.advance_to(std::time::Duration::from_secs(9));
        world.insert_resource(old_timeline);
        world.insert_resource(RollbackFrameCount(540));

        start_sync_test_session(
            &mut world,
            SyncTestSettings {
                check_distance: 0,
                max_prediction_window: 8,
            },
        )
        .expect("a one-player baseline SyncTest session is valid");

        assert_eq!(world.resource::<RollbackFrameCount>().0, 0);
        assert_eq!(
            *world.resource::<RollbackSessionOwnership>(),
            RollbackSessionOwnership::LocalSyncTest(SyncTestSettings {
                check_distance: 0,
                max_prediction_window: 8,
            })
        );
        assert_eq!(
            world.resource::<Time<GgrsTime>>().elapsed(),
            std::time::Duration::ZERO,
            "a new frame-zero session must not retain elapsed time from the old timeline"
        );
    }

    #[test]
    fn stop_restart_uses_a_fresh_generation_after_the_boundary_was_removed() {
        let mut world = World::new();
        let settings = SyncTestSettings {
            check_distance: 0,
            max_prediction_window: 8,
        };

        start_sync_test_session(&mut world, settings).expect("first session starts");
        let first = world.resource::<ConfirmedFrameBoundary>().session;
        stop_session(&mut world);
        assert!(
            !world.contains_resource::<ConfirmedFrameBoundary>(),
            "teardown must immediately disable quarantine and confirmation gates"
        );

        start_sync_test_session(&mut world, settings).expect("second session starts");
        let second = world.resource::<ConfirmedFrameBoundary>().session;
        assert_ne!(
            first, second,
            "frame zero in the restarted session must not alias frame zero from the old timeline"
        );
    }

    #[test]
    fn deferred_stop_removes_the_confirmed_boundary_too() {
        fn queue_stop(mut commands: Commands) {
            stop_session_deferred(&mut commands);
        }

        let mut app = App::new();
        app.world_mut().insert_resource(ConfirmedFrameBoundary {
            current: 9,
            confirmed: 4,
            session: 3,
        });
        app.add_systems(Update, queue_stop);
        app.update();

        assert!(
            !app.world().contains_resource::<ConfirmedFrameBoundary>(),
            "the deferred path must execute the same complete teardown as stop_session"
        );
    }

    #[test]
    fn invalidation_removes_the_confirmed_boundary_but_preserves_the_reason() {
        let mut world = World::new();
        world.insert_resource(ConfirmedFrameBoundary {
            current: 7,
            confirmed: 2,
            session: 5,
        });

        invalidate_session(&mut world, "contract changed".into());

        assert!(!world.contains_resource::<ConfirmedFrameBoundary>());
        assert_eq!(
            world
                .resource::<RollbackSessionStatus>()
                .invalidation
                .as_deref(),
            Some("contract changed")
        );
    }

    #[test]
    fn device_edges_are_consumed_when_read_inputs_runs_not_each_render_frame() {
        let mut app = App::new();
        app.init_schedule(ReadInputs)
            .init_resource::<ControlFrameLatch>()
            .init_resource::<PendingLocalInput>()
            .init_resource::<LocalPlayers>();
        install_session_bridge(&mut app);

        {
            let mut latch = app.world_mut().resource_mut::<ControlFrameLatch>();
            latch.accumulate(ControlFrame {
                jump_pressed: true,
                jump_held: true,
                ..default()
            });
            // A later rendered frame sees the button released, but no GGRS
            // tick requested input between these samples.
            latch.accumulate(ControlFrame::default());
        }

        assert_eq!(
            app.world().resource::<PendingLocalInput>().0,
            ControlFrame::default(),
            "render-frame sampling must not consume the tick latch"
        );

        app.world_mut().run_schedule(ReadInputs);
        let first = app.world().resource::<PendingLocalInput>().0;
        assert!(
            first.jump_pressed,
            "the short press must reach the next GGRS tick"
        );
        assert!(!first.jump_held, "the latest button level is released");

        app.world_mut().run_schedule(ReadInputs);
        assert!(
            !app.world().resource::<PendingLocalInput>().0.jump_pressed,
            "the edge must be consumed exactly once"
        );
    }
}

#[cfg(test)]
mod replay_pass_tests {
    use super::*;
    use ambition_platformer_primitives::schedule::SimulationReplayState;

    /// Runs the REAL `count_advance_run` for a frame, returning whether it
    /// judged the frame a re-simulation. Driving the actual system (not a
    /// reimplementation of its rule) is what makes this able to fail when the
    /// rule regresses.
    fn advance_to(world: &mut World, frame: i32) -> bool {
        world.insert_resource(RollbackFrameCount(frame));
        world
            .run_system_cached(count_advance_run)
            .expect("count_advance_run runs");
        world.resource::<SimulationReplayState>().replaying_history
    }

    fn rollback_world() -> World {
        let mut world = World::new();
        world.init_resource::<RollbackExecutionStats>();
        world.init_resource::<SimulationReplayState>();
        world
    }

    /// A rollback re-runs frames 3 and 4 and then simulates 5 for the first
    /// time. The distinction still matters for the consumers that legitimately
    /// want "don't do this twice" — the trace's duplicate append and the
    /// falling-sand step guard. External effects no longer read it at all.
    #[test]
    fn only_the_re_simulated_frames_are_marked_as_replay() {
        let mut world = rollback_world();
        for frame in 0..=4 {
            assert!(
                !advance_to(&mut world, frame),
                "frame {frame} is being simulated for the first time"
            );
        }

        // Rollback: GGRS reloads frame 2 and re-advances through 4.
        for frame in 3..=4 {
            assert!(advance_to(&mut world, frame), "frame {frame} ran before");
        }

        assert!(
            !advance_to(&mut world, 5),
            "frame 5 is new — the frame the player just caused"
        );
    }

    /// The confirmed boundary is the fact external effects key on, and it is
    /// republished every advance from GGRS's own counters.
    #[test]
    fn each_advance_publishes_where_the_confirmed_line_sits() {
        let mut world = rollback_world();
        world.insert_resource(ConfirmedFrameBoundary::default());
        world.insert_resource(ConfirmedFrameCount(2));

        advance_to(&mut world, 6);

        let boundary = *world.resource::<ConfirmedFrameBoundary>();
        assert_eq!(boundary.current, 6);
        assert_eq!(boundary.confirmed, 2);
        assert!(
            !boundary.fully_confirmed(),
            "frames 3..=6 are still predicted"
        );
    }

    /// `LoadWorld` moves the simulation back to the restored frame. The
    /// abandoned-branch discard reads exactly this, so it must be republished
    /// rather than left pointing at the frame the host walked away from.
    #[test]
    fn restoring_a_frame_moves_the_published_boundary_back_to_it() {
        let mut world = rollback_world();
        world.insert_resource(ConfirmedFrameBoundary::default());
        world.insert_resource(ConfirmedFrameCount(1));
        advance_to(&mut world, 9);

        world.insert_resource(RollbackFrameCount(4));
        world
            .run_system_cached(mark_historical_replay)
            .expect("mark_historical_replay runs");

        assert_eq!(
            world.resource::<ConfirmedFrameBoundary>().current,
            4,
            "the simulation now sits at the restored frame, not at 9"
        );
    }

    /// A host with no confirmed boundary installed (fixed-tick, headless, or a
    /// rollback host before its first session) must still work.
    #[test]
    fn a_host_without_a_boundary_still_tracks_replay() {
        let mut world = rollback_world();

        for frame in [0, 1, 1] {
            world.insert_resource(RollbackFrameCount(frame));
            world
                .run_system_cached(count_advance_run)
                .expect("count_advance_run runs without a boundary");
        }

        assert!(world.resource::<SimulationReplayState>().replaying_history);
    }
}
