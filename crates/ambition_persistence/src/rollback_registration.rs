//! Rollback declaration owned by `ambition_persistence`.
//!
//! This module names this domain's concrete rewindable state while the host
//! supplies the backend through [`RollbackRegistrar`]. It deliberately contains
//! no `bevy_ggrs` dependency and no host/composition logic.
//!
//! ⭐ MOVED OUT OF THE RUNTIME 2026-08-26, for the reason the clock moved before
//! it: a new save field or quest resource becomes an obligation the compiler
//! shows to whoever adds it HERE, instead of a thing to remember in a crate that
//! does not otherwise know this domain exists. ⛔ the STABLE NAMES did not
//! change, so the wire did not move — only the OWNER string, from the
//! composition to the crate that defines the types.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

const OWNER: &str = env!("CARGO_PKG_NAME");

/// Register the saved-game and quest state a rewind has to reproduce.
pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    registrar
        .rollback_resource_clone::<crate::save::AmbitionGameSave>(OWNER, "resource.sandbox_save")
        .rollback_resource_clone::<crate::quest::registry::QuestRegistry>(
            OWNER,
            "resource.quest_registry",
        )
        // ⛔ A SAME-TICK HANDSHAKE. The quest advance is announced and consumed
        // inside one tick, so a cursor GGRS did not rewind would let the
        // consumer fire for an advance the resimulation never committed to.
        //
        // ⚠ DECLARED ONCE. The runtime declared this message TWICE under the
        // same stable name — the registry treats an identical re-registration as
        // idempotent, so nothing failed and nothing said so. That is what a
        // hand-kept list in a distant crate accumulates.
        .clear_message_on_rollback::<crate::quest::QuestAdvanceRequested>(
            OWNER,
            "message.quest_advance_requested",
        );
}
