//! Persist the player's inventory + wallet across save/load.
//!
//! `OwnedItems` (the 24-item OoT catalog) and the player's `BodyWallet` are
//! live state, not part of `AmbitionGameSave` — so a session's earned items + money
//! evaporated on restart. This module mirrors them into the save (which the
//! existing autosave writes to disk) and restores them on load, keyed by stable
//! `dialog_id` so the save survives catalog reordering. Equipped state is a
//! handoff (re-equip from the grid on load).
//!
//! ⛔⛔ **it mirrors QUANTITIES, and a held object is not one.** `to_persisted`
//! reads the stored counts, never `OwnedItems::count`, which projects the body's
//! hand. Writing the projection would put the object into the save as a row, and
//! the next load would restore the row while the room that authors the object
//! re-authors the object — one weapon saved, two loaded.
//!
//! ⭐ **and the held object is kept by the OTHER leg now** (2026-08-16). D132
//! recorded that a held weapon reached disk as nothing at all and was therefore
//! LOST across save/load, and named that as not-a-resting-state.
//! `crate::session::durable_horizon` closes it: an object is persisted as an
//! OCCURRENCE — identity, whereabouts, and the hand holding it — never as a
//! quantity. The two populations stay disjoint, which is what stops the "one
//! weapon saved, two loaded" failure from arriving by the new road instead.
//!
//! ⚠ **what still does NOT coordinate is a granted QUANTITY that has been turned
//! into an instance.** The mint does not spend the row, so the same quantity can
//! manifest a second object; spending it needs `OwnedItems` inside the checkpoint
//! horizon first. That is D132's surviving half, unchanged by the durable slice.

use bevy::prelude::*;

use crate::items::OwnedItems;
use crate::session::durable_horizon::SaveRestored;
use ambition_characters::actor::BodyWallet;
use ambition_persistence::save::AmbitionGameSave;

/// Apply the saved inventory + wallet to the live state **once**, after the save
/// is loaded and the player exists. A fresh save (never persisted —
/// `inventory_saved == false`) keeps the live starter set.
///
/// ⚠ **it OWNS the [`SaveRestored`] latch and its siblings only read it.** The
/// latch means "the file has been applied to this world", and this is the leg
/// whose precondition is strictest — it waits for a body carrying a
/// `BodyWallet` — so setting it here is what makes the occurrence leg chained
/// in front of it land on the same frame rather than one frame apart.
pub fn restore_inventory_from_save(
    mut restored: ResMut<SaveRestored>,
    save: Res<AmbitionGameSave>,
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
/// `AmbitionGameSave` on an actual change, so autosave's change-detection throttle
/// stays honest. Gated on the restore so it can't run first.
pub fn persist_inventory_to_save(
    restored: Res<SaveRestored>,
    owned: Res<OwnedItems>,
    // SLOT-0 BY DESIGN: see `restore_inventory_from_save` — the save file is the
    // local player's, so only slot 0's wallet is persisted.
    wallet_q: Query<&BodyWallet, crate::actor::PrimaryPlayerOnly>,
    mut save: ResMut<AmbitionGameSave>,
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
