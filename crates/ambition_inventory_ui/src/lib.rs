//! Inventory menu-navigation state.
//!
//! The item store itself is the `OwnedItems` catalog in `ambition_items`;
//! this crate owns only the `InventoryUiState` resource (selection / tab /
//! scroll / focus) driven by the unified menu. Keeping this state here lets the
//! reusable item catalog stay below menu-navigation and presentation tiers.

mod model;

pub use self::model::{InventoryTab, InventoryUiState};
