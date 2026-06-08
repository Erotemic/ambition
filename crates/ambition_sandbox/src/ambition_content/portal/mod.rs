//! Ambition-specific portal adapters.
//!
//! These modules translate Ambition game concepts (the [`ControlFrame`] input
//! channel and the [`OwnedItems`] inventory roster) into the reusable,
//! content-agnostic portal intent/outcome messages exposed by
//! [`crate::portal`]. The reusable portal mechanic never imports Ambition input
//! or inventory types; this boundary owns that glue.
//!
//! [`ControlFrame`]: crate::input::ControlFrame
//! [`OwnedItems`]: crate::items::OwnedItems
//!
//! This is the first slice of the `ambition_content` boundary (Stage 9 / Task H);
//! Stage 11 / Task J expands it to the rest of the named Ambition content.

mod carve_adapter;
mod input_adapter;
mod inventory_adapter;
mod plugin;
mod transit_adapter;
mod transit_body_adapter;

pub use carve_adapter::bridge_portal_carves;
pub use input_adapter::{pick_aim, portal_input_adapter_system};
pub use inventory_adapter::{
    drop_portal_gun_system, equip_portal_gun, pickup_portal_gun_system, unequip_portal_gun,
};
pub use plugin::AmbitionPortalAdaptersPlugin;
pub use transit_adapter::{
    apply_movement_intent_to_control, sync_ground_items_to_transitable,
    sync_movement_intent_from_control, sync_transitable_to_ground_items,
};
pub use transit_body_adapter::{ensure_portal_bodies, portal_player_input_adapter};
