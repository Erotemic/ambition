//! **Wiring the DURABLE save horizon into every composition, not just the
//! visible one.**
//!
//! ⛔⛔ **THE FINDING THIS PLUGIN EXISTS TO CLOSE.** The durable-save leg — the
//! `SaveRestored` latch plus `restore_inventory_from_save` /
//! `persist_inventory_to_save` — was installed by
//! `app::plugins::install_menu_setup_and_hotkeys`, inside
//! `add_presentation_plugins`, which is documented "visible binary only". **No
//! headless composition scheduled it at all.** So the authority that decides what
//! a player's file says never ran in any test, which is most of why the durable
//! horizon went two days without one — the first fixture to reach it
//! (`two_persistence_authorities_for_one_item`) had to call the shipped systems
//! by hand and said so at the call site.
//!
//! ⭐ **a save is not presentation.** It is the state of the world across process
//! lifetimes, and every composition that simulates a world has one. Registering it
//! beside a pause menu made "does this build render?" decide "does this build
//! remember?", which are not the same question and were never meant to be.
//!
//! # Where it sits, and why here rather than in the sim schedule
//!
//! In literal `Update`, exactly where the inventory leg already was. That is not
//! inertia: the systems are not resimulated by the rollback host, and moving them
//! into the sim schedule would put a file-writing mirror inside the window that
//! gets rewound and replayed. The state they touch IS rollback state and IS
//! rewound — which is what the `SaveRestored` registration is for — but the
//! systems themselves run once per frame at the top level, and the autosave that
//! consumes their output already refuses to commit a predicted world.
//!
//! # The chain, and every edge in it is load-bearing
//!
//! ```text
//! restore_durable_horizon      installs the ledger + three baselines, asks for the resume
//! restore_inventory_from_save  applies the catalog + wallet, and SETS the latch
//! persist_inventory_to_save    mirrors the catalog back (gated on the latch)
//! persist_durable_horizon_to_save  mirrors the ledger back (gated on the latch)
//! ```
//!
//! ⛔ **the two restores are in front of the two mirrors and the latch is set
//! between them**, so no mirror can write a world that has not been told what the
//! file said — the exact failure the latch was invented for, reached by ordering
//! instead of by rollback.

use bevy::prelude::*;

use ambition_platformer2d_actor_monolith::items::persist::{
    persist_inventory_to_save, restore_inventory_from_save,
};
use ambition_platformer2d_actor_monolith::session::durable_horizon::{
    persist_durable_horizon_to_save, restore_durable_horizon, SaveRestored,
};

/// Installs the durable save horizon: the one "already applied" latch and the
/// four systems that read and write the save file's view of the world.
///
/// ⚠ **it does NOT install `AmbitionGameSave` or the autosave.** The resource is
/// core sim state and the disk writer is `PersistenceSchedulePlugin`; a headless
/// composition therefore mirrors into the save and simply never commits it to a
/// file, which is the correct behaviour for a harness and is what makes a
/// save/load fixture possible without touching a disk at all.
pub struct DurableSaveHorizonPlugin;

impl Plugin for DurableSaveHorizonPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SaveRestored>().add_systems(
            Update,
            (
                restore_durable_horizon,
                restore_inventory_from_save,
                persist_inventory_to_save,
                persist_durable_horizon_to_save,
            )
                .chain(),
        );
    }
}
