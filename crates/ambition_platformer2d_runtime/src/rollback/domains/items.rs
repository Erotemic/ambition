//! **The items domain's rollback schema** (Campaign 2, R3).
//!
//! Ground items and the pickup bookkeeping a rewind has to put back — including who was reaching for what.
//!
//! ⚠ **relocation only.** The registrations were extracted mechanically and the
//! schema baseline verifies the result is byte-identical — a retyped call is
//! exactly the mistake that would slip through review and not through the
//! baseline.
//!
//! ⚠ the owner label stays `ambition_platformer2d_runtime` because this module is in it, and
//! must be: `ambition_items` sits below the runtime in the crate graph. R1's
//! recorded decision is that this is the right shape for every domain below the
//! runtime; crates above it own their schemas directly.

use bevy::prelude::App;

use super::super::AmbitionRollbackApp;

const OWNER: &str = "ambition_platformer2d_runtime";

/// Register everything the items domain needs rewound.
pub(in crate::rollback) fn register(app: &mut App) {
    app.rollback_resource_clone::<ambition_items::OwnedItems>(OWNER, "resource.owned_items");
    // **What a merchant conversation agreed to**, and what one gave away.
    //
    // ⛔ **both used to be a direct mutation of `OwnedItems` from `Update`** —
    // the Yarn command system, on the presentation side of the boundary,
    // writing the rollback resource registered right above. A rewind restored
    // the bag and nothing re-ran the command, because the Yarn runner is not
    // rewound and does not execute between resimulated ticks. Cleared on load
    // like every other released narrative fact: the resimulated tick is handed
    // the request again by the ledger rather than remembering it.
    app.clear_message_on_rollback::<ambition_items::ItemGrantRequested>(
        OWNER,
        "message.item_grant_requested",
    );
    app.clear_message_on_rollback::<ambition_items::shop::ShopTransactionRequested>(
        OWNER,
        "message.shop_transaction_requested",
    );
    // ⛔ **the LATCH rewinds with the state it guards, 2026-08-04.**
    // `SaveRestored` is an "already applied" flag, and all three things it
    // coordinates were rollback-registered while it was not:
    // `OwnedItems` (above), `BodyWallet` (characters domain) and
    // `AmbitionGameSave` (the root schema).
    //
    // ⚠ **it was `InventoryRestored` until 2026-08-16, and it is registered from
    // the ITEM domain for that historical reason rather than a structural one.**
    // The flag now means "the loaded save has been applied to this world", and
    // the durable occurrence horizon (`session::durable_horizon`) reads it too —
    // deliberately ONE latch, because a second one would be a second answer to a
    // single question and free to disagree the day one leg's precondition was met
    // and the other's was not. `OwnedItems` is still the largest thing it gates,
    // so the registration stays here rather than acquiring a new home.
    //
    // So a rewind past the restore undid its EFFECT and kept the record of
    // having applied it. On the next `Update`,
    // `restore_inventory_from_save` returned early on the latch and never
    // reapplied the save, while `persist_inventory_to_save` — chained right
    // after it, and gated on that same latch being TRUE — wrote the live
    // STARTER set back over the loaded save with `inventory_saved = true`, for
    // autosave to put on disk.
    //
    // ⭐ that is word for word the failure the latch's own doc says it exists to
    // stop: *"so the write-back can't fire before the restore and clobber a
    // loaded save with the starter set."* It prevented the ordering it was
    // written for and caused the same outcome by another route.
    //
    // ⚠ **being in literal `Update` is not the exemption it looks like.** The
    // systems are not resimulated, but the STATE they wrote is rolled back, and
    // an un-rewound latch then reports work as done that no longer is. The
    // standing rule is about what the flag GATES, not which schedule sets it.
    //
    // ⚠ **a bare `_clone`, and I tried `_clone_checksum` first and was wrong.**
    // Projecting the bool into the checksum looks strictly better — a one-field
    // latch whose only probe is PRESENCE is probed by nothing, since presence is
    // `true` forever. But these systems run in literal `Update`, so the latch is
    // set OUTSIDE resimulation and does not move in step with the sim ticks the
    // checksum covers: `the_calibration_lab_is_checksum_stable_at_rest` went red
    // immediately, along with most of the rollback oracle.
    //
    // ⭐ so the value must REWIND (which `_clone` does) and must NOT be
    // CHECKSUMMED (which `_clone` also does). It is named in the exit oracle's
    // presence-only list with that reason, which is the alternative that test's
    // own message offers.
    app.rollback_resource_clone::<
        ambition_platformer2d_actor_monolith::session::durable_horizon::SaveRestored,
    >(OWNER, "resource.save_restored");
}
