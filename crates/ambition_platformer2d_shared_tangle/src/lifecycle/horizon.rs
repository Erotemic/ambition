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
//! So this module owns **vocabulary and ordering only**: two messages and two
//! sets. Each domain declares its own baseline value, captures it from its own
//! live authority, and restores it into that same authority.
//!
//! ⚠ **and the cost of that choice is stated rather than defended against: a
//! domain that acquires reset-relevant state and never subscribes here loses it
//! silently.** There is no registry that would notice. A registry was considered
//! and refused — it would have to be type-erased to hold unrelated domain values,
//! and a type-erased registry that nothing can enumerate meaningfully is a
//! hand-kept list wearing a checker's clothes. What defends this instead is that
//! the behavioural fixture drives real domains: a domain that silently drops its
//! state fails the scenario, not a registration assertion.
//!
//! # The two messages
//!
//! [`CheckpointCommitted`] and [`ResetToCheckpoint`] are deliberately NOT the
//! existing `RoomReplayRequested`. That channel means *rebuild the active room*
//! and content emits it on a level **completion** as well as on a death — a flag
//! touched, an act cleared. Restoring a reset baseline when the player just WON
//! would take the reward back off them.

use bevy::prelude::{Message, SystemSet};

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
