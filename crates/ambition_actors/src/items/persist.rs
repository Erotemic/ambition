//! Persist the player's inventory + wallet across save/load.
//!
//! `OwnedItems` (the 24-item OoT catalog) and the player's `BodyWallet` are
//! live state, not part of `SandboxSave` — so a session's earned items + money
//! evaporated on restart. This module mirrors them into the save (which the
//! existing autosave writes to disk) and restores them on load, keyed by stable
//! `dialog_id` so the save survives catalog reordering. Equipped state is a
//! handoff (re-equip from the grid on load).

use bevy::prelude::*;

use crate::items::OwnedItems;
use ambition_characters::actor::BodyWallet;
use ambition_persistence::save::SandboxSave;

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
    // SLOT-0 BY DESIGN: the SAVE FILE belongs to the local player. `BodyWallet` is
    // body vocabulary (a currency-dropping NPC carries one), but only slot 0's
    // balance round-trips through the save.
    mut wallet_q: Query<&mut BodyWallet, crate::actor::PrimaryPlayerOnly>,
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
    // SLOT-0 BY DESIGN: see `restore_inventory_from_save` — the save file is the
    // local player's, so only slot 0's wallet is persisted.
    wallet_q: Query<&BodyWallet, crate::actor::PrimaryPlayerOnly>,
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
mod tests;
