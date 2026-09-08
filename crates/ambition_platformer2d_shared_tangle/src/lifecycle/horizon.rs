//! Checkpoint reset horizon shared across lifecycle domains.
//!
//! Current world state, checkpoint state, durable save state, and authored source state are
//! distinct reconstruction horizons.

use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::{App, IntoScheduleConfigs, Message, Plugin, Resource, Schedule, SystemSet};

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

/// Where the checkpoint request is taken to the lifecycle slot.
///
/// ⚠ IT NO LONGER CONTAINS THE RESTORE. Domain application moved to
/// [`CheckpointDomainApply`], which the commit executor runs; what remains in
/// this simulation set is the session's admission — the only part of a
/// reconstruction that belongs in ordinary speculative simulation. The set keeps
/// its name and its host-level edge because that edge is about admission order
/// (`before(RoomReplayAdmission)`), not about restoring anything.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CheckpointRestore;

/// The one schedule in which a checkpoint's domain state is put back.
///
/// ⛔⛔ **AUTHORIZATION IS STRUCTURAL HERE, NOT A FLAG.** This schedule is run
/// by the common commit executor and by nothing else — the eager host's
/// exclusive runner and the confirmed host's commit tail. A domain reducer that
/// lives in it therefore cannot act on an unadmitted request, an unprepared
/// room, or a crossing that was cancelled before its destructive application:
/// there is no path from a raw `ResetToCheckpoint` to running this.
///
/// ⭐ AND IT REPLACED A TOKEN. A1c/1-2 published a one-frame
/// `AdmittedCheckpointRestore` that every reducer had to remember to read; the
/// reducers ran in ordinary speculative simulation and consulted a value to
/// learn whether they were allowed to. Running them from the commit instead
/// makes the same guarantee out of WHEN they run, which no reducer can forget.
///
/// ⚠ ITS INPUTS ARE INSTALLED, NOT LOOKED UP. [`CheckpointRestoreInputs`] is
/// present only while this schedule runs; the executor removes it on every
/// success and every failure path. A reducer with no inputs does nothing, so an
/// accidental invocation is a no-op rather than a restore to an empty baseline.
#[derive(ScheduleLabel, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CheckpointDomainApply;

/// The pinned lifecycle-layer inputs one committed restore applies.
///
/// ⛔ INSTALLED FOR THE DURATION OF [`CheckpointDomainApply`] AND REMOVED AFTER.
/// It is not a resource domains may read at any other time: its absence is what
/// makes "this reducer cannot run outside an authorized commit" true by
/// construction rather than by convention.
///
/// ⚠ WHY IT IS NOT SIMPLY THE LIVE BASELINES. The values here were pinned when
/// the lifecycle slot ACCEPTED the operation. A capture that lands between
/// acceptance and commit belongs to the next operation, not this one; reading
/// the live baseline at commit would let a checkpoint taken during the load
/// retarget a restore already in flight.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct CheckpointRestoreInputs {
    /// The occurrence population this operation restores.
    pub occurrences: OccurrenceBaseline,
    /// The custody relation it restores. Applied by the item domain, which owns
    /// the materialization; the relation vocabulary is lifecycle's.
    pub custody: CustodyBaseline,
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
        // ⭐ THE RESTORE IS NOT IN THE SIM SCHEDULE AT ALL. This offer contributes
        // its CAPTURE to the simulation and its reducer to the commit executor's
        // schedule; a composition installing this plugin alone gets a schedule
        // nothing runs, which is the honest outcome — with no commit executor
        // there is no authorized moment to restore anything.
        app.add_schedule(Schedule::new(CheckpointDomainApply));
        app.init_resource::<OccurrenceBaseline>()
            .init_resource::<CustodyBaseline>()
            .add_systems(
                sim,
                (capture_occurrence_baseline, capture_custody_baseline)
                    .in_set(CheckpointCapture),
            )
            .add_systems(CheckpointDomainApply, restore_occurrence_baseline);
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

    use super::{CustodyBaseline, LifecycleCheckpointHorizonPlugin, OccurrenceBaseline};

    /// ⛔⛔ **THE INSTALLED INPUTS STAY LIFECYCLE-LAYER VALUES.** The exhaustive
    /// destructure is the guard: a field added to [`CheckpointRestoreInputs`]
    /// stops this compiling, which is the moment to ask which layer the new
    /// value belongs to. What may live here is what a `shared_tangle` reducer
    /// applies — an occurrence population, a custody relation. Item recipes and
    /// entitlement quantities are the ITEM domain's and travel in its own
    /// installed inputs; putting them here would make this crate the checkpoint
    /// coordinator and force every domain to read a value it does not own.
    #[test]
    fn the_installed_restore_inputs_carry_only_lifecycle_layer_values() {
        let inputs = super::CheckpointRestoreInputs {
            occurrences: Default::default(),
            custody: Default::default(),
        };
        let super::CheckpointRestoreInputs {
            occurrences,
            custody,
        } = &inputs;
        assert_eq!(*occurrences, Default::default());
        assert_eq!(*custody, Default::default());
    }

    #[test]
    fn lifecycle_checkpoint_offer_installs_its_baselines() {
        let mut app = App::new();
        app.add_plugins(LifecycleCheckpointHorizonPlugin);
        assert!(app.world().contains_resource::<OccurrenceBaseline>());
        assert!(app.world().contains_resource::<CustodyBaseline>());
    }
}
