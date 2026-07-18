//! GGRS session/input bridge shared by the harness and future network hosts.

use bevy::prelude::*;
use bevy_ggrs::ggrs::{self, PlayerType, SessionBuilder};
use bevy_ggrs::{
    ConfirmedFrameCount, GgrsConfig, GgrsSchedule, GgrsTime, LocalInputs, LocalPlayers,
    PlayerInputs, ReadInputs, RollbackFrameCount, RunGgrsSystems, Session, SyncTestMismatch,
};

use ambition_engine_core::{ControlFrame, ControlFrameLatch};

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
    pub last_simulated_frame: i32,
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct RollbackSessionStatus {
    pub mismatch_frames: Vec<i32>,
    pub invalidation: Option<String>,
}

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
    install_session(world, AmbitionGgrsSession::SyncTest(session));
    Ok(())
}

/// Install any already-constructed GGRS session behind Ambition's exact
/// content/schema contract. Matchbox will eventually construct a P2P session
/// and hand it to this same seam; the harness uses [`start_sync_test_session`].
pub fn install_session(world: &mut World, session: AmbitionGgrsSession) {
    let schema = world
        .get_resource::<RollbackRegistry>()
        .cloned()
        .unwrap_or_default()
        .schema_fingerprint();
    let content = live_content_identity(world);
    world.insert_resource(RollbackSessionContract { content, schema });
    world.insert_resource(RollbackSessionStatus::default());
    world.insert_resource(RollbackExecutionStats::default());
    world.insert_resource(session);
}

pub fn stop_session(world: &mut World) {
    world.remove_resource::<AmbitionGgrsSession>();
    world.remove_resource::<RollbackSessionContract>();
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
    app.init_resource::<PendingLocalInput>()
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
            bevy_ggrs::LoadWorld,
            count_load_run.in_set(super::AmbitionLoadWorldSet::Reconcile),
        )
        .add_systems(PreUpdate, enforce_session_contract.before(RunGgrsSystems))
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

fn count_advance_run(frame: Res<RollbackFrameCount>, mut stats: ResMut<RollbackExecutionStats>) {
    stats.advance_runs = stats.advance_runs.saturating_add(1);
    stats.last_simulated_frame = frame.0;
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
    world.remove_resource::<AmbitionGgrsSession>();
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
            world.resource::<Time<GgrsTime>>().elapsed(),
            std::time::Duration::ZERO,
            "a new frame-zero session must not retain elapsed time from the old timeline"
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
