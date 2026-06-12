//! Inventory data model.
//!
//! The runtime owns a flat `PlayerInventory` resource (item kind → count) plus
//! the `InventoryUiState` resource driven by the unified menu.

mod model;

pub use self::model::{InventoryUiState, ItemKind, PlayerInventory};
