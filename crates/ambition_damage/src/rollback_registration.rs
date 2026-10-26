//! Rollback declarations for health effects.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

pub fn register_rollback_state<R: RollbackRegistrar>(registrar: &mut R) {
    #[cfg(feature = "causal")]
        registrar.clear_instrument_message_on_rollback::<crate::BodyHitResolved>(
            OWNER,
            "message.causal_body_hit_resolved",
        );

    #[cfg(feature = "causal")]
        registrar.clear_instrument_message_on_rollback::<crate::BodyReactionApplied>(
            OWNER,
            "message.causal_body_reaction_applied",
        );

    const OWNER: &str = env!("CARGO_PKG_NAME");
    registrar.clear_message_on_rollback::<crate::WalletShieldSpent>(
        OWNER,
        "message.wallet_shield_spent",
    );
    registrar.clear_message_on_rollback::<crate::healing::PlayerHealRequested>(
        OWNER,
        "message.player_heal_requested",
    );
}
