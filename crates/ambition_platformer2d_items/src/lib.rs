//! Item adapters that compose actor simulation with item capabilities.
//! Item catalog and inventory definitions belong to `ambition_items`;
//! physical collectible state belongs to `ambition_world_items`, and
//! pressed pickup belongs to `ambition_held_items`.

pub mod conditions;
pub mod match_spawn;
pub mod narrative;
pub mod persist;
pub mod pickup;
pub mod starting_bag;
/// `wallet.can_afford` — the purse's own published question.
pub mod wallet_conditions;

/// Register the item horizon state for rollback.
pub fn register_rollback_state<R: ambition_platformer2d_core::snapshot::RollbackRegistrar>(registrar: &mut R) {
    pickup::minted_horizon::register_checkpoint_rollback_state(registrar);
}
