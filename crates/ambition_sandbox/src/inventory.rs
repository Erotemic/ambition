//! Inventory/adventure menu model + overlay panel.
//!
//! The runtime owns a flat `PlayerInventory` resource (item kind → count).
//! The presentation side renders a phone-friendly adventure menu with
//! left/right tabs for Items, Map, and Quests. The menu consumes the semantic
//! `MenuControlFrame` instead of raw keyboard/gamepad/touch input so desktop,
//! gamepad, Android touch, and future controller schemes can all drive the same
//! UI contract.
//!
//! Items are currently minimal: pressing confirm uses the selected item. The
//! only effect today is `HealthPotion`, which heals the player by a fixed amount
//! and decrements the stack.

use bevy::prelude::*;

#[cfg(feature = "input")]
use crate::game_mode::GameMode;
#[cfg(feature = "input")]
use crate::input::MenuControlFrame;

mod effects;
mod input;
mod model;
mod pointer;
mod ui;

#[cfg(test)]
mod tests;

#[cfg(feature = "input")]
pub use self::input::inventory_input;
pub use self::model::{
    InventoryBackButton, InventoryDescriptionText, InventoryItemRow, InventoryRoot,
    InventoryStatusText, InventoryTab, InventoryTabButton, InventoryTabContentText,
    InventoryTitleText, InventoryUiState, ItemKind, PlayerInventory,
};
#[cfg(feature = "input")]
pub use self::pointer::inventory_pointer_input;
pub use self::ui::{spawn_inventory_panel, sync_inventory_panel};
