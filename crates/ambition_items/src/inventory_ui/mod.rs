//! Inventory menu-navigation state.
//!
//! The item store itself is the `OwnedItems` catalog (`crate`); this
//! module owns only the `InventoryUiState` resource (selection / tab / scroll)
//! driven by the unified menu.

mod model;

pub use self::model::InventoryUiState;
