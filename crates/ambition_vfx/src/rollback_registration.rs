//! Rollback declaration owned by `ambition_vfx`.
//!
//! These are transient in-tick effect channels. A load discards messages from
//! the abandoned branch; the resimulated producer emits the authoritative ones.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

const OWNER: &str = env!("CARGO_PKG_NAME");

pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    registrar.clear_message_on_rollback::<crate::EffectRequest>(OWNER, "message.effect_request");
    registrar.clear_message_on_rollback::<crate::vfx::DebrisBurstMessage>(
        OWNER,
        "message.debris_burst",
    );
    // Stable wire name intentionally retains the historical spelling.
    registrar.clear_message_on_rollback::<crate::FxRequest>(OWNER, "message.explosion_request");
    registrar.clear_message_on_rollback::<crate::FireworksRequest>(
        OWNER,
        "message.fireworks_request",
    );
    registrar.clear_message_on_rollback::<crate::VfxMessage>(OWNER, "message.vfx");
    // The knockout beat, on the same footing as every other presentation intent
    // beside it: a reader's cursor is `Local` state GGRS never rewinds, so the
    // channel is cleared rather than restored.
    registrar.clear_message_on_rollback::<crate::vfx::KnockoutBeatRequested>(
        OWNER,
        "message.knockout_beat",
    );
}
