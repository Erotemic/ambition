//! Checkpoint reset horizon shared across lifecycle domains.
//!
//! Current world state, checkpoint state, durable save state, and authored source state are
//! distinct reconstruction horizons.

use bevy::prelude::{App, IntoScheduleConfigs, Message, Plugin, Resource, SystemSet};

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

use crate::schedule::SimScheduleExt;
use crate::sim_id::SimId;

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

/// Where a domain writes its baseline back, reading
/// [`AdmittedCheckpointRestore`].
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CheckpointRestore;

/// The three phases of one restore, in order.
///
/// ⛔⛔ THE SPLIT IS THE WHOLE POINT, AND IT IS NOT A TIDINESS EDGE. Every
/// member of [`CheckpointRestore`] used to read [`ResetToCheckpoint`] on its
/// own, so a request the lifecycle slot REFUSED still had its occurrence,
/// custody and owned-item consequences applied — measured: it rolled the
/// entitlement ledger back and DESTROYED an object acquired after the
/// checkpoint, because the room reconstruction that would re-author the object
/// is exactly what the refusal cancelled. Ordering the reads differently
/// repairs none of that; what the domains need is an ANSWER, which is why
/// [`Admit`](Self::Admit) runs first and publishes one.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CheckpointRestoreStep {
    /// The session coordinator takes the request to the lifecycle slot and, if
    /// it is accepted, publishes [`AdmittedCheckpointRestore`]. Sole writer.
    Admit,
    /// Domains reduce their own live state toward their own baseline — but only
    /// for an operation that was admitted.
    Apply,
    /// The session retires the accepted operation.
    Retire,
}

/// The checkpoint restore the lifecycle slot ADMITTED.
///
/// ⭐ ONE WRITER, MANY READERS, and that is the shape the defect needed. The
/// session coordinator is the only thing that may admit or retire; every domain
/// reducer reads it and does nothing without it. A refused request produces no
/// value here, so a refused request changes no domain state — which no amount of
/// re-ordering the old raw-message reads could achieve.
///
/// ⚠ IT DOES NOT OUTLIVE ITS FRAME, TODAY: [`CheckpointRestoreStep`] admits,
/// applies and retires inside one run of the restore set, and
/// `the_admitted_restore_never_survives_its_own_frame` says so. It is
/// nonetheless rollback state, because the day application moves to the
/// confirmed commit boundary that lifetime changes and the registration must
/// not be the thing anyone remembers to add.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct AdmittedCheckpointRestore(Option<AdmittedRestore>);

/// One admitted restore operation.
#[derive(Clone, Debug, PartialEq)]
pub struct AdmittedRestore {
    /// The sim frame the admission happened on.
    pub frame: i32,
    /// The body the operation restores around, by rollback-stable identity.
    ///
    /// Resolved BEFORE admission and kept: possession can change while the
    /// operation is in flight, and re-asking "who is controlled now" at apply
    /// time is how a restore acquires the wrong subject.
    pub subject: SimId,
}

impl AdmittedCheckpointRestore {
    /// The admitted operation, if the slot accepted one. `None` means every
    /// domain reducer must do nothing.
    pub fn admitted(&self) -> Option<&AdmittedRestore> {
        self.0.as_ref()
    }

    /// Publish an accepted operation.
    ///
    /// ⛔ CALLED ONLY BY THE SESSION COORDINATOR, and only with an `Admission`
    /// in hand. There is no path from a raw request to this function.
    pub fn admit(&mut self, restore: AdmittedRestore) {
        self.0 = Some(restore);
    }

    /// Retire the operation once its domains have applied it.
    pub fn retire(&mut self) -> Option<AdmittedRestore> {
        self.0.take()
    }

    /// ⭐ WHICH OPERATION, not merely "one is admitted". A presence probe would
    /// satisfy the coverage oracle while seeing neither the frame nor the
    /// subject — and the subject is the whole reason this value is carried
    /// rather than re-derived: a restore that came back naming a different body
    /// restores the wrong one.
    pub fn checksum(&self) -> u64 {
        match &self.0 {
            None => 0,
            Some(restore) => {
                let mut hash = (restore.frame as i64 as u64) ^ 0x9e37_79b9_7f4a_7c15;
                for byte in restore.subject.as_str().as_bytes() {
                    hash = hash.rotate_left(5) ^ u64::from(*byte);
                }
                hash | 1
            }
        }
    }
}

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
        // ⭐ THE STEP CHAIN AND THE TOKEN BELONG TO THE ADMISSION AUTHORITY, not
        // here. A domain offer names `CheckpointRestoreStep::Apply` as a label
        // and gets its ordering from whoever owns the phases; installing this
        // plugin alone leaves `Apply` unconfigured, which is the honest outcome
        // — with no session coordinator nothing can be admitted, so nothing in
        // it may run anyway.
        app.init_resource::<OccurrenceBaseline>()
            .init_resource::<CustodyBaseline>()
            .add_systems(
                sim,
                (capture_occurrence_baseline, capture_custody_baseline)
                    .in_set(CheckpointCapture),
            )
            .add_systems(
                sim,
                restore_occurrence_baseline.in_set(CheckpointRestoreStep::Apply),
            );
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

    registrar.rollback_resource_clone_checksum::<AdmittedCheckpointRestore>(
        OWNER,
        "resource.admitted_checkpoint_restore",
        "which operation the lifecycle slot admitted, by frame and subject",
        AdmittedCheckpointRestore::checksum,
    );
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
