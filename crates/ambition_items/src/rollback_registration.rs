//! Rollback declaration owned by `ambition_items`.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

const OWNER: &str = env!("CARGO_PKG_NAME");

pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    registrar.rollback_resource_clone::<crate::OwnedItems>(OWNER, "resource.owned_items");
    registrar.clear_message_on_rollback::<crate::ItemGrantRequested>(
        OWNER,
        "message.item_grant_requested",
    );
    registrar.clear_message_on_rollback::<crate::shop::ShopTransactionRequested>(
        OWNER,
        "message.shop_transaction_requested",
    );
}
