//! Rollback state owned by the lifecycle coordinator.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

const OWNER: &str = env!("CARGO_PKG_NAME");

pub fn register_rollback_state<R: RollbackRegistrar>(registrar: &mut R) {
    registrar.rollback_component_clone_checksum::<crate::room_preparation::CapturedRoomPreparation>(
        OWNER,
        "session.room_preparation",
        "the admitted room operation and its immutable continuity inputs",
        crate::room_preparation::CapturedRoomPreparation::checksum,
    );
    registrar.rollback_component_clone_checksum::<crate::party_travel::PartyTravel>(
        OWNER, "session.party_travel", "party travel rules and pending zone agreements",
        crate::party_travel::PartyTravel::checksum,
    );
    registrar.rollback_component_canonical::<crate::participants::SessionParticipants>(
        OWNER,
        "session.participants",
    );
    registrar
        .rollback_resource_clone_checksum::<crate::lifecycle_commit::PendingLifecycleCommit>(
            OWNER,
            "resource.pending_lifecycle_commit",
            "the pending, withdrawn, and applied intents with their restore admissions",
            crate::lifecycle_commit::PendingLifecycleCommit::checksum,
        );
    registrar.rollback_component_clone_probed::<crate::rooms::passage::ZonePassage>(
        OWNER,
        "actor.zone_passage",
        crate::rooms::passage::ZonePassage::checksum,
    );
    registrar.clear_message_on_rollback::<crate::leave::ParticipantLeaveRequested>(
        OWNER,
        "message.participant_leave_requested",
    );
    registrar.clear_message_on_rollback::<crate::reset::NewGameRequested>(
        OWNER,
        "message.new_game_requested",
    );
    registrar.clear_message_on_rollback::<crate::reset::RoomReplayRequested>(
        OWNER,
        "message.room_replay_requested",
    );
    registrar.clear_message_on_rollback::<crate::checkpoint::RecoverAtCheckpoint>(
        OWNER,
        "message.recover_at_checkpoint",
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
    registrar.rollback_component_clone_checksum::<crate::fact_horizon::FactsBaseline>(
        OWNER,
        "resource.facts_baseline",
        "the save's flags and quest steps at the last checkpoint, which a death puts back",
        crate::fact_horizon::FactsBaseline::checksum,
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
