//! OoT-style 6×4 item-grid inventory menu (feature `oot_inventory`).
//!
//! This is the "Select Item" subscreen Jon asked to integrate: a fixed 6×4 grid
//! showing **all 24 possible pickup items** ([`crate::items`]), with owned items
//! bright and un-acquired ones dimmed, OoT-style. Confirming a slot equips a
//! weapon (via the shared [`crate::item_pickup`] held-item seam) or uses a
//! consumable.
//!
//! ## Easy-to-cut seam
//!
//! Everything player-facing here lives behind the `oot_inventory` Cargo feature.
//! Drop the feature and the legacy 3-tab adventure menu ([`crate::inventory`])
//! takes over the Inventory button again — the always-on [`crate::items`] catalog
//! + [`crate::items::OwnedItems`] resource stay, so pickups/dialogue keep working
//! either way. The registration in `app/plugins.rs` swaps which *input* system
//! handles the Inventory action based on this feature.
//!
//! ## Renderer
//!
//! The current renderer is native Bevy UI (text labels stand in for item icons
//! until art exists). The data model (`OwnedItems` + `GridMenuState`) is renderer-
//! independent on purpose: the heavier 3D OoT "cube" renderer in the
//! `ambition_menu` submodule (vendored `bevy_lunex`) can later consume
//! the same state without touching the catalog or input/effects. See that
//! submodule's `DESIGN-OOT-DEMO.md`.

pub mod effects;
pub mod input;
pub mod state;
pub mod ui;

#[cfg(test)]
mod tests;

pub use input::{grid_menu_input, grid_menu_pointer_input};
pub use state::GridMenuState;
pub use ui::{spawn_grid_menu, sync_grid_menu, GridMenuRoot, GridSlot};

use bevy::prelude::*;

/// Register the OoT menu's resource + spawn/sync visuals (NOT the input systems,
/// which the caller chains into the existing input pipeline so ordering matches
/// the legacy menu it replaces).
pub fn install_grid_menu_visuals(app: &mut App) {
    app.init_resource::<GridMenuState>().add_systems(
        Update,
        sync_grid_menu.after(crate::app::SandboxSet::CoreSimulation),
    );
    // `spawn_grid_menu` is registered by the caller at `Startup` alongside the
    // other menu panels so it shares their `.after(setup_simulation_system)`
    // ordering.
}
