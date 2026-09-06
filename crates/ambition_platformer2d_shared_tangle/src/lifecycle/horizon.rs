//! Checkpoint reset horizon shared across lifecycle domains.
//!
//! Current world state, checkpoint state, durable save state, and authored source state are
//! distinct reconstruction horizons.

use bevy::prelude::{App, IntoScheduleConfigs, Message, Plugin, SystemSet};

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

use crate::schedule::SimScheduleExt;

use super::{
    capture_custody_baseline, capture_occurrence_baseline, restore_occurrence_baseline,
    CustodyBaseline, OccurrenceBaseline,
};

/// A checkpoint was committed: every contributing domain records its baseline
/// now.
///
///  a world EVENT, not a body position. The save shrine already writes a
/// `PersistedCheckpoint { room, x, y }`, and that value answers *where the body comes back*,
/// which is the smallest part of the question.
///
///  emitted by whatever a game decides a checkpoint is. The engine does not
/// decide: a shrine, a flag, a room entry and an autosave are all legitimate,
/// and a game with no checkpoints at all simply never writes this and gets a
/// death that restores the empty baseline — which is the sandbox reset, and is
/// the degenerate case rather than a special one.
#[derive(Message, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CheckpointCommitted;

/// Put the world back to the last committed checkpoint.
///
///  this is the HORIZON RESTORE and nothing else. Not a room unload and not
/// a room transition: each of those preserves or replaces current truth by its
/// own rule, while this one rewinds the world to a recorded baseline.
///
///  A SAVE LOAD IS ONE OF THESE, and the exclusion this comment used to
/// carry was wrong about its own producer. `complete_durable_restore` adopts
/// the file's occurrence/custody/minted rows into the live baselines and then
/// writes this — because putting the world at the horizon the file records is
/// the same operation a death performs against the horizon a checkpoint
/// recorded. Two producers, one meaning; a death and a load differ in where the
/// baseline came from, not in what happens to the world.
///
///  it is a request, not a report. Writing it asks the horizon to be
/// restored; the restoring happens in [`CheckpointRestore`], and a host that
/// registers no domain systems there gets a no-op rather than a half-restore.
#[derive(Message, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResetToCheckpoint;

/// Where a domain records its baseline, reading [`CheckpointCommitted`].
///
/// Every member runs in the same frame and none may read another domain's
/// baseline: a capture reads LIVE state and writes its own snapshot, so the
/// order within this set never matters. That independence is the property that
/// makes the domains genuinely separable rather than nominally so — the moment
/// one capture wants another's output, they are one domain wearing two names.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CheckpointCapture;

/// Where a domain writes its baseline back, reading [`ResetToCheckpoint`].
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CheckpointRestore;

/// The lifecycle domain's checkpoint contribution.
///
/// This plugin owns the concrete baseline types that live in the reusable
/// lifecycle layer. A host composes the contribution; it does not enumerate
/// `OccurrenceBaseline` / `CustodyBaseline` or their capture systems itself.
///
/// `CustodyBaseline` is captured here because the relation vocabulary is
/// lifecycle-owned. Its materializing restore is item policy and therefore
/// joins [`CheckpointRestore`] from the item-domain contribution instead.
pub struct LifecycleCheckpointHorizonPlugin;

impl Plugin for LifecycleCheckpointHorizonPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.init_resource::<OccurrenceBaseline>()
            .init_resource::<CustodyBaseline>()
            .add_systems(
                sim,
                (capture_occurrence_baseline, capture_custody_baseline)
                    .in_set(CheckpointCapture),
            )
            .add_systems(sim, restore_occurrence_baseline.in_set(CheckpointRestore));
    }
}

/// Register the rollback obligations of the lifecycle checkpoint horizon beside
/// the horizon vocabulary rather than in the crate-wide rollback census.
///
/// The host still supplies the backend-neutral registrar. This is the same
/// ownership inversion used by the rest of rollback federation: a domain names
/// its concrete state; composition only invokes the domain offer.
pub(crate) fn register_checkpoint_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    const OWNER: &str = env!("CARGO_PKG_NAME");

    registrar.rollback_resource_clone_checksum::<OccurrenceBaseline>(
        OWNER,
        "resource.occurrence_baseline",
        "entity-free remembered-whereabouts checksum projection",
        OccurrenceBaseline::checksum,
    );
    registrar.rollback_resource_clone_checksum::<CustodyBaseline>(
        OWNER,
        "resource.custody_baseline",
        "entity-free remembered-custody checksum projection",
        CustodyBaseline::checksum,
    );
    registrar.clear_message_on_rollback::<CheckpointCommitted>(
        OWNER,
        "message.checkpoint_committed",
    );
    registrar.clear_message_on_rollback::<ResetToCheckpoint>(
        OWNER,
        "message.reset_to_checkpoint",
    );
}

#[cfg(test)]
mod participant_tests {
    use bevy::prelude::App;

    use super::{
        CustodyBaseline, LifecycleCheckpointHorizonPlugin, OccurrenceBaseline,
    };

    #[test]
    fn lifecycle_checkpoint_offer_installs_its_baselines() {
        let mut app = App::new();
        app.add_plugins(LifecycleCheckpointHorizonPlugin);
        assert!(app.world().contains_resource::<OccurrenceBaseline>());
        assert!(app.world().contains_resource::<CustodyBaseline>());
    }
}
