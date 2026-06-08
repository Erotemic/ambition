//! Inventory data model.
//!
//! The runtime owns a flat `PlayerInventory` resource (item kind → count) plus
//! the `InventoryUiState` resource that the unified menu ([`crate::menu`]) drives.
//! The legacy 3-tab adventure-menu UI was deleted in Phase D2; the unified tabbed
//! menu renders the always-on [`crate::items`] catalog instead. Only the data
//! model survives here (`ItemKind`, `PlayerInventory`, `InventoryUiState`, plus
//! the `InventoryTab` enum it carries).

mod model;

pub use self::model::{InventoryUiState, ItemKind, PlayerInventory};
