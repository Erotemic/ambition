//! **The authored-logic domain: the one channel authored content's verbs travel
//! on.**
//!
//! `RunAuthoredCommand` is written by the narrative-input ledger (a `.yarn`
//! `<<command …>>` line) or by any simulation system that wants an authored verb
//! performed, and it is drained by `run_requested_authored_commands` in
//! `AuthoredCommandSet`.
//!
//! ⭐ **the buffer is normally EMPTY at a snapshot boundary**, because the
//! dispatcher drains it in the same frame the ledger released it and holds no
//! `MessageReader` cursor at all — which is the trap this registration usually
//! exists to close. What it covers is the one window that is not: a request
//! released onto a frame the host then rewinds past before `AuthoredCommandSet`
//! ran. ⛔ the ledger re-releases that request on the resimulated tick, so a
//! survivor in the buffer would perform the authored verb twice — and *"grant
//! the item"* is not idempotent.
//!
//! ⚠ **the CATALOG is not here and must not be.** `CommandCatalog` is waived in
//! `rollback_coverage` because `publish` is private and a tick holds a `World`,
//! never an `App`; registering it would restore a byte-identical value every
//! frame and would quietly make the privacy argument look optional.

use bevy::prelude::App;

use super::super::AmbitionRollbackApp;

const OWNER: &str = "ambition_platformer2d_shared_tangle";

/// Register the authored-command request channel.
pub(in crate::rollback) fn register(app: &mut App) {
    app.clear_message_on_rollback::<
        ambition_platformer2d_shared_tangle::authored_logic::RunAuthoredCommand,
    >(OWNER, "message.run_authored_command");
}
