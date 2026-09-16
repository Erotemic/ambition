//! Persist the player's inventory + wallet across save/load.
//!
//! `OwnedItems` (the 24-item OoT catalog) and the player's `BodyWallet` are
//! live state, not part of `AmbitionGameSave` — so a session's earned items + money
//! evaporated on restart. This module mirrors them into the save (which the
//! existing autosave writes to disk) and restores them on load, keyed by stable
//! `dialog_id` so the save survives catalog reordering. Equipped state is a
//! handoff (re-equip from the grid on load).
//!
//! it mirrors QUANTITIES, and a held object is not one. `to_persisted`
//! reads the stored counts, never `OwnedItems::count`, which projects the body's
//! hand. Writing the projection would put the object into the save as a row, and
//! the next load would restore the row while the room that authors the object
//! re-authors the object — one weapon saved, two loaded.
//!
//! and the held object is kept by the OTHER leg now. `crate:session:durable_horizon` closes
//! it: an object is persisted as an OCCURRENCE — identity, whereabouts, and the hand holding it
//! — never as a quantity. The two populations stay disjoint, which is what stops the "one
//! weapon saved, two loaded" failure from arriving by the new road instead.
//!
//! the quantity/instance boundary now coordinates at both horizons. The
//! mint spends a granted quantity, `OwnedItemsBaseline` restores it on death,
//! and this module adopts that baseline only after the saved bag itself has been
//! applied. A held object remains an occurrence; a stored row remains a quantity.

use bevy::prelude::*;

use ambition_items::OwnedItems;
use crate::session::durable_horizon::SaveRestored;
use ambition_characters::actor::BodyWallet;
use ambition_persistence::save::AmbitionGameSave;

/// ⛔⛤ **RESET NEW GAME WIPED THE SAVE AND THE NEXT PERSISTENCE PASS PUT THE OLD
/// RUN BACK — MEASURED BY A 2026-09-13 REVIEW.**
///
/// `process_new_game_reset_request` writes `AmbitionGameSaveData::default()`,
/// resets encounters/bosses/quests, clears occurrences and rebuilds the room. It
/// does not touch `OwnedItems`, the primary player's `BodyWallet`, the item
/// baselines, or [`SaveRestored`] — and `SaveRestored` staying TRUE is what
/// closes the loop:
///
/// ```text
/// Reset New Game:   save = default, live bag = the old run's, SaveRestored = true
/// next pass:        persist_inventory_to_save sees them differ
///                   → writes the OLD inventory and wallet into the NEW save
/// ```
///
/// ⇒ The wipe did not stay a wipe, and stale checkpoint baselines could restore
/// old-run durable state on a later death.
///
/// ⭐ **THE DOMAIN OWNS ITS OWN FRESH-RUN STATE, which is why this is here and
/// not another field the reset monolith knows about.** `NewGameResetCommitted` is
/// the lifecycle FACT; each durable domain reduces it. Teaching the central reset
/// the internals of every durable subsystem is how the next subsystem gets
/// forgotten.
///
/// ⚠ **FRESH-RUN, NOT EMPTY.** The bag is `OwnedItems::starter()` — what
/// `ambition_content`'s plugin installs at App build, so a new game begins with
/// what a new PROCESS begins with. The wallet is `BodyWallet::default()`: no
/// character definition authors a starting balance (checked), so zero is that
/// same value. ⇒ If one ever does, this owes the definition rather than a
/// constant.
pub fn reset_inventory_on_new_game(
    mut committed: MessageReader<crate::session::reset::NewGameResetCommitted>,
    mut owned: ResMut<OwnedItems>,
    mut restored: ResMut<SaveRestored>,
    mut minted_baseline: Option<ResMut<crate::items::pickup::minted_horizon::MintedItemBaseline>>,
    mut owned_baseline: Option<ResMut<crate::items::pickup::minted_horizon::OwnedItemsBaseline>>,
    // ⭐ THE TWO OCCURRENCE BASELINES, RESET HERE rather than re-adopted from
    // the wiped file. See the note at the end of this function.
    mut occurrence_baseline: Option<
        ResMut<ambition_platformer2d_shared_tangle::lifecycle::OccurrenceBaseline>,
    >,
    mut custody_baseline: Option<
        ResMut<ambition_platformer2d_shared_tangle::lifecycle::CustodyBaseline>,
    >,
    mut wallet_q: Query<
        &mut BodyWallet,
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
) {
    if committed.read().next().is_none() {
        return;
    }
    *owned = OwnedItems::starter();
    if let Ok(mut wallet) = wallet_q.single_mut() {
        *wallet = BodyWallet::default();
    }
    if let Some(baseline) = minted_baseline.as_deref_mut() {
        *baseline = Default::default();
    }
    if let Some(baseline) = owned_baseline.as_deref_mut() {
        *baseline = Default::default();
    }
    // ⛔⛤ **AND THE LAST TWO DURABLE DOMAINS, DIRECTLY — WHICH IS WHAT LETS THE
    // LATCH STAY TRUE.** This function used to end `restored.0 = false`, sending
    // the fresh run back through the generic load road so that
    // `adopt_occurrence_checkpoint_from_save` would re-adopt these two from the
    // wiped file. Every other fresh-run durable fact above is already reset
    // HERE; these two were the only reason to re-enter that road at all.
    //
    // ⇒ Resetting them here makes New Game a self-contained simulation
    // transaction, and it removes the ONLY mid-session `false -> true`
    // transition of `SaveRestored` in the codebase — measured by census: this
    // line was its only lowering. That transition re-ran a chain of `Update`
    // systems over rollback-owned state while a GGRS session was already live,
    // which is the defect Q135 is about
    // (`docs/planning/awaiting-maintainer-decision.md`). An engine feature that
    // replaces the save during a live match is not something this line should
    // have been creating by accident.
    //
    // ⚠ `AuthoredOccurrences` is NOT reset here, and that is not an omission:
    // `process_new_game_reset_request` clears it with `forget_everything()` in
    // the same committed transaction, one system earlier.
    //
    // ⚠ AND THE WINDOW IT CLOSED WAS TWO FRAMES WIDE AND INVISIBLE TO A
    // BETWEEN-STEP READ — the chain raised the latch again in the same frame it
    // was lowered, so `sim.step()`-boundary sampling reported it true throughout.
    // Held by
    // `a_new_game_clears_the_occurrence_baselines_without_lowering_the_latch`,
    // which samples at the HEAD of that chain for exactly that reason.
    if let Some(baseline) = occurrence_baseline.as_deref_mut() {
        *baseline = Default::default();
    }
    if let Some(baseline) = custody_baseline.as_deref_mut() {
        *baseline = Default::default();
    }
    debug_assert!(
        restored.0,
        "a New Game committed while the durable restore had never completed, so \
         the fresh-run values above are being written over a load still in flight"
    );
}

/// Apply the saved inventory + wallet to the live state once, after the save
/// is loaded and the player exists. A fresh save (never persisted —
/// `inventory_saved == false`) keeps the live starter set.
///
/// The final durable-horizon completion system owns [`SaveRestored`].
pub fn restore_inventory_from_save(
    restored: Res<SaveRestored>,
    save: Res<AmbitionGameSave>,
    mut owned: ResMut<OwnedItems>,
    mut minted_baseline: Option<ResMut<crate::items::pickup::minted_horizon::MintedItemBaseline>>,
    mut owned_baseline: Option<ResMut<crate::items::pickup::minted_horizon::OwnedItemsBaseline>>,
    // SLOT-0 BY DESIGN: the SAVE FILE belongs to the local player. `BodyWallet` is
    // body vocabulary (a currency-dropping NPC carries one), but only slot 0's
    // balance round-trips through the save.
    mut wallet_q: Query<
        &mut BodyWallet,
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
) {
    if restored.0 {
        return;
    }
    let Ok(mut wallet) = wallet_q.single_mut() else {
        return; // wait until the player exists
    };
    let data = save.data();
    if data.inventory_saved() {
        owned.apply_persisted(data.items());
        wallet.balance = data.wallet();
    }
    // Domain-owned durable adoption.
    crate::items::pickup::minted_horizon::adopt_checkpoint_baselines_from_save(
        data,
        &owned,
        minted_baseline.as_deref_mut(),
        owned_baseline.as_deref_mut(),
    );
    // Do not raise `SaveRestored` here. The durable-horizon completion system
    // runs after every domain adopter and owns the one global completion fact.
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
    wallet_q: Query<&BodyWallet, ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly>,
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
    if data.inventory_saved() && data.wallet() == wallet.balance && data.items() == items {
        return; // unchanged → leave the save clean (no redundant autosave)
    }
    // ⛔ `data_mut()` ONLY PAST THE GUARD ABOVE. Reaching it derefs the
    // `ResMut`, which marks the resource changed whether or not the value
    // differs -- that is what the early return protects.
    save.data_mut().set_inventory(items, wallet.balance);
}

#[cfg(test)]
mod tests;
