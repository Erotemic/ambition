//! **The RESET horizon: what a death puts back, and what it does not.**
//!
//! # Three horizons, and they are three because they disagree
//!
//! ```text
//! current world truth   what is true right now, after everything that has happened
//! checkpoint truth      what a death/retry restores
//! durable save truth    what survives closing the program
//! ```
//!
//! ⛔ **these are not three views of one value and must never be collapsed into
//! one.** Ordinary room unload/reload preserves *current* truth — walking out of
//! a room and back in changes nothing about what happened in it. A debug
//! "restore authored room" deliberately reconstructs *authored source* state,
//! which is a fourth thing again. Save/load is a serialization horizon with its
//! own compatibility rules. All four involve reconstruction, and that shared
//! mechanism is exactly why they get conflated.
//!
//! # The maintainer's rule (2026-08-15), and why it is not an item rule
//!
//! > Death/retry restores the latest committed checkpoint.
//!
//! ```text
//! C0: key on pedestal
//!   pick up key, die before committing        → reset to C0: key back on the pedestal
//!   pick up key again, commit C1, die         → reset to C1: key still held, pedestal empty
//!   after C1 pick up a temporary item, die    → key still held, temporary item back at its C1 place
//! ```
//!
//! ⛔⛔ **do not encode this as `KeyItem => survives death`.** The third line is
//! the one that kills the item-kind reading: an ordinary item survives if its
//! new disposition was committed, and a key item reverts if acquiring it
//! happened after the current checkpoint. The checkpoint decides, and the kind of
//! thing never enters the question. A kind rule is a second authority that starts
//! disagreeing with the checkpoint the first time content changes.
//!
//! # ⭐ The baseline is a PROJECTION OF DOMAINS, not a resource
//!
//! ```text
//! checkpoint baseline = snapshot of each authoritative domain, taken by that domain
//! ```
//!
//! ⛔ **not** one giant resource into which every reset-relevant fact is stuffed.
//! That shape reads as economical and costs the thing this module exists to
//! keep: the occurrence ledger answers *what happened to an authored
//! occurrence*, the custody state answers *what a body carries*, and they are
//! different questions with different owners, different lifetimes and different
//! producers. A combined resource makes every future domain that wants a reset
//! fact edit one struct, and makes every reader of that struct able to reach
//! facts it has no business knowing.
//!
//! The shared layer owns the vocabulary plus its own typed domain contribution:
//! [`LifecycleCheckpointHorizonPlugin`] installs the occurrence/custody baselines
//! and their systems, while the host owns only cross-domain phase placement. Other
//! domains contribute their own plugins rather than extending a central type list.
//!
//! ⛔ **there is still no erased registry.** A registry of unrelated baseline
//! values would need `Any` / `TypeId` or boxed callbacks and would recreate the
//! hand-kept census behind a dynamic facade. Typed Bevy plugins and the existing
//! rollback registrar give each domain a compile-time offer without creating a
//! universal mutable service locator.
//!
//! # The two messages
//!
//! [`CheckpointCommitted`] and [`ResetToCheckpoint`] are deliberately NOT the
//! existing `RoomReplayRequested`. That channel means *rebuild the active room*
//! and content emits it on a level **completion** as well as on a death — a flag
//! touched, an act cleared. Restoring a reset baseline when the player just WON
//! would take the reward back off them.

use bevy::prelude::{App, IntoScheduleConfigs, Message, Plugin, SystemSet};

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

use crate::schedule::SimScheduleExt;

use super::{
    capture_custody_baseline, capture_occurrence_baseline, restore_occurrence_baseline,
    CustodyBaseline, OccurrenceBaseline,
};

/// **A checkpoint was committed: every contributing domain records its baseline
/// now.**
///
/// ⭐ **a world EVENT, not a body position.** The save shrine already writes a
/// `PersistedCheckpoint { room, x, y }`, and that value answers *where the body
/// comes back*, which is the smallest part of the question. What was missing is
/// an INSTANT at which the rest of the world can be recorded, and this is it.
///
/// ⚠ **emitted by whatever a game decides a checkpoint is.** The engine does not
/// decide: a shrine, a flag, a room entry and an autosave are all legitimate,
/// and a game with no checkpoints at all simply never writes this and gets a
/// death that restores the empty baseline — which is the sandbox reset, and is
/// the degenerate case rather than a special one.
#[derive(Message, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CheckpointCommitted;

/// **Put the world back to the last committed checkpoint.**
///
/// ⛔ **this is the DEATH/RETRY horizon and nothing else.** Not a room unload,
/// not a room transition, not a save load. Each of those preserves or replaces
/// current truth by its own rule; this one and only this one rewinds the world
/// to a baseline.
///
/// ⚠ **it is a request, not a report.** Writing it asks the horizon to be
/// restored; the restoring happens in [`CheckpointRestore`], and a host that
/// registers no domain systems there gets a no-op rather than a half-restore.
#[derive(Message, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResetToCheckpoint;

/// **Where a domain records its baseline**, reading [`CheckpointCommitted`].
///
/// Every member runs in the same frame and none may read another domain's
/// baseline: a capture reads LIVE state and writes its own snapshot, so the
/// order within this set never matters. That independence is the property that
/// makes the domains genuinely separable rather than nominally so — the moment
/// one capture wants another's output, they are one domain wearing two names.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CheckpointCapture;

/// **Where a domain writes its baseline back**, reading [`ResetToCheckpoint`].
///
/// ⭐ **ordered BEFORE the room rebuild, and that edge is the whole
/// transaction.** Reconstruction asks the occurrence ledger what became of each
/// authored record; if the ledger were restored after the rebuild, the room
/// would be rebuilt against the world the player just died in and the baseline
/// would apply from the next room load onward — an off-by-one-room bug that
/// looks like nothing until somebody dies twice in different rooms.
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
