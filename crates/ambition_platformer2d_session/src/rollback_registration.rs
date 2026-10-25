//! Rollback state owned by the lifecycle coordinator.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

const OWNER: &str = env!("CARGO_PKG_NAME");

pub fn register_rollback_state<R: RollbackRegistrar>(registrar: &mut R) {
    registrar
        .rollback_resource_canonical::<crate::lifecycle_commit::PendingLifecycleCommit>(
            OWNER,
            "resource.pending_lifecycle_commit",
        );
    registrar.rollback_component_clone_probed::<crate::rooms::passage::ZonePassage>(
        OWNER,
        "actor.zone_passage",
        crate::rooms::passage::ZonePassage::checksum,
    );
    registrar.clear_message_on_rollback::<crate::reset::NewGameRequested>(
        OWNER,
        "message.new_game_requested",
    );
    registrar.clear_message_on_rollback::<crate::reset::RoomReplayRequested>(
        OWNER,
        "message.room_replay_requested",
    );
    registrar.rollback_component_clone_checksum::<crate::checkpoint::SessionCheckpointOperations>(
        OWNER,
        "resource.session_checkpoint_operations",
        "how many checkpoint restores this session has admitted",
        crate::checkpoint::SessionCheckpointOperations::checksum,
    );
    registrar.rollback_component_clone_checksum::<crate::checkpoint::SessionCheckpointOutcomes>(
        OWNER,
        "resource.session_checkpoint_outcomes",
        "the terminal outcome of the last answered checkpoint operation",
        crate::checkpoint::SessionCheckpointOutcomes::checksum,
    );
    registrar.rollback_component_clone_checksum::<crate::checkpoint::AcceptedCheckpointRestore>(
        OWNER,
        "resource.accepted_checkpoint_restore",
        "which operation was accepted, the occurrence population it reconstructs, and the replay whose consequences run at its publication",
        crate::checkpoint::AcceptedCheckpointRestore::checksum,
    );
    registrar.rollback_component_clone_checksum::<crate::checkpoint::OutstandingCheckpointRequest>(
        OWNER,
        "resource.outstanding_checkpoint_request",
        "whether the session is still owed a checkpoint restore",
        crate::checkpoint::OutstandingCheckpointRequest::checksum,
    );
    registrar.rollback_component_clone_checksum::<crate::checkpoint::SessionStartupResume>(
        OWNER,
        "resource.session_startup_resume",
        "which session generation startup resolution reached, and which operation it routed",
        crate::checkpoint::SessionStartupResume::checksum,
    );
}
