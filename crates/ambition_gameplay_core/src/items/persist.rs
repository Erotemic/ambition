//! Persist the player's inventory + wallet across save/load.
//!
//! `OwnedItems` (the 24-item OoT catalog) and the player's `BodyWallet` are
//! live state, not part of `SandboxSave` — so a session's earned items + money
//! evaporated on restart. This module mirrors them into the save (which the
//! existing autosave writes to disk) and restores them on load, keyed by stable
//! `dialog_id` so the save survives catalog reordering. Equipped state is a
//! handoff (re-equip from the grid on load).

use bevy::prelude::*;

use crate::actor::BodyWallet;
use crate::actor::{PlayerEntity, PrimaryPlayer};
use crate::items::OwnedItems;
use crate::persistence::save::SandboxSave;

/// Set once the saved inventory has been applied to the live state (or skipped
/// for a fresh save), so the write-back can't fire before the restore and
/// clobber a loaded save with the starter set.
#[derive(Resource, Default)]
pub struct InventoryRestored(pub bool);

/// Apply the saved inventory + wallet to the live state **once**, after the save
/// is loaded and the player exists. A fresh save (never persisted —
/// `inventory_saved == false`) keeps the live starter set.
pub fn restore_inventory_from_save(
    mut restored: ResMut<InventoryRestored>,
    save: Res<SandboxSave>,
    mut owned: ResMut<OwnedItems>,
    mut wallet_q: Query<&mut BodyWallet, (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    if restored.0 {
        return;
    }
    let Ok(mut wallet) = wallet_q.single_mut() else {
        return; // wait until the player exists
    };
    let data = save.data();
    if data.inventory_saved {
        owned.apply_persisted(&data.items);
        wallet.balance = data.wallet;
    }
    restored.0 = true;
}

/// Mirror the live inventory + wallet into the save whenever they differ from
/// the saved form (autosave then writes the dirtied save to disk). Only touches
/// `SandboxSave` on an actual change, so autosave's change-detection throttle
/// stays honest. Gated on the restore so it can't run first.
pub fn persist_inventory_to_save(
    restored: Res<InventoryRestored>,
    owned: Res<OwnedItems>,
    wallet_q: Query<&BodyWallet, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    mut save: ResMut<SandboxSave>,
) {
    if !restored.0 {
        return;
    }
    let Ok(wallet) = wallet_q.single() else {
        return;
    };
    let items = owned.to_persisted();
    let data = save.data();
    if data.inventory_saved && data.wallet == wallet.balance && data.items == items {
        return; // unchanged → leave the save clean (no redundant autosave)
    }
    let data = save.data_mut();
    data.items = items;
    data.wallet = wallet.balance;
    data.inventory_saved = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::Item;

    fn app_with(save: SandboxSave, owned: OwnedItems, wallet: i32) -> (App, Entity) {
        let mut app = App::new();
        app.insert_resource(save);
        app.insert_resource(owned);
        app.init_resource::<InventoryRestored>();
        app.add_systems(
            Update,
            (restore_inventory_from_save, persist_inventory_to_save).chain(),
        );
        let player = app
            .world_mut()
            .spawn((PlayerEntity, PrimaryPlayer, BodyWallet { balance: wallet }))
            .id();
        (app, player)
    }

    #[test]
    fn a_loaded_save_restores_items_and_wallet_over_the_starter() {
        let mut save = SandboxSave::default();
        save.data_mut().inventory_saved = true;
        save.data_mut().wallet = 137;
        // HealthCell is a stacking consumable; Bomb is a unique weapon (cap 1).
        save.data_mut().items = vec![
            crate::persistence::save_data::PersistedItem::new(Item::HealthCell.dialog_id(), 4),
            crate::persistence::save_data::PersistedItem::new(Item::Bomb.dialog_id(), 1),
        ];
        // Live state is the starter (Fireball etc.), wallet 0.
        let (mut app, player) = app_with(save, OwnedItems::starter(), 0);
        app.update();
        let owned = app.world().resource::<OwnedItems>();
        assert_eq!(
            owned.count(Item::HealthCell),
            4,
            "restored the saved consumable count"
        );
        assert_eq!(owned.count(Item::Bomb), 1, "restored the unique weapon");
        assert_eq!(
            owned.count(Item::Fireball),
            0,
            "the saved set REPLACES the starter"
        );
        assert_eq!(
            app.world().get::<BodyWallet>(player).unwrap().balance,
            137,
            "restored the saved wallet"
        );
    }

    #[test]
    fn a_fresh_save_keeps_the_starter_and_then_persists_it() {
        // inventory_saved == false → fresh; keep the live starter + wallet.
        let (mut app, _player) = app_with(SandboxSave::default(), OwnedItems::starter(), 25);
        app.update();
        let owned = app.world().resource::<OwnedItems>();
        assert!(
            owned.count(Item::Fireball) > 0,
            "fresh save keeps the starter"
        );
        // …and persist wrote the starter + wallet back into the save.
        let data = app.world().resource::<SandboxSave>().data().clone();
        assert!(
            data.inventory_saved,
            "the fresh inventory is now marked saved"
        );
        assert_eq!(data.wallet, 25, "wallet persisted");
        assert!(
            data.items
                .iter()
                .any(|i| i.id == Item::Fireball.dialog_id()),
            "the starter items were written to the save"
        );
    }

    #[test]
    fn round_trips_the_owned_counts_by_id() {
        // to_persisted / apply_persisted survive a round-trip (the storage half).
        // Consumables stack; unique items (Bomb) cap at 1 via grant.
        let mut owned = OwnedItems::default();
        owned.grant(Item::Bomb, 1);
        owned.grant(Item::HealthCell, 5);
        owned.grant(Item::ManaCell, 2);
        let persisted = owned.to_persisted();
        let mut restored = OwnedItems::starter();
        restored.apply_persisted(&persisted);
        assert_eq!(restored.count(Item::Bomb), 1);
        assert_eq!(restored.count(Item::HealthCell), 5);
        assert_eq!(restored.count(Item::ManaCell), 2);
        assert_eq!(
            restored.count(Item::Fireball),
            0,
            "apply replaces, not merges"
        );
    }
}
