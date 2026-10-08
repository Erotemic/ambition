//! Rollback declaration owned by `ambition_items`.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

const OWNER: &str = env!("CARGO_PKG_NAME");

pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    registrar.rollback_resource_clone_checksum::<crate::OwnedItems>(
        OWNER,
        "resource.owned_items",
        "each item's stored count, in catalog order (Q129: the bag is compared)",
        crate::OwnedItems::checksum,
    );
    registrar.clear_message_on_rollback::<crate::ItemGrantRequested>(
        OWNER,
        "message.item_grant_requested",
    );
    registrar.clear_message_on_rollback::<crate::shop::ShopTransactionRequested>(
        OWNER,
        "message.shop_transaction_requested",
    );
    registrar.clear_message_on_rollback::<crate::ItemUseRequested>(
        OWNER,
        "message.item_use_requested",
    );
}
