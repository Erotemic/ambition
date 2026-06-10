//! Ambition-specific portal adapters.
//!
//! These modules translate Ambition game concepts (the [`ControlFrame`] input
//! channel and the [`OwnedItems`] inventory roster) into the reusable,
//! content-agnostic portal intent/outcome messages exposed by
//! [`ambition_sandbox::portal`]. The reusable portal mechanic never imports Ambition input
//! or inventory types; this boundary owns that glue.
//!
//! [`ControlFrame`]: ambition_sandbox::input::ControlFrame
//! [`OwnedItems`]: ambition_sandbox::items::OwnedItems
//!
//! This is the first slice of the `ambition_content` boundary (Stage 9 / Task H);
//! Stage 11 / Task J expands it to the rest of the named Ambition content.

mod ability_adapter;
mod carve_adapter;
mod fire_adapter;
mod input_adapter;
mod inventory_adapter;
mod plugin;
mod reset_adapter;
mod sfx_adapter;
mod shot_adapter;
mod transit_adapter;
mod transit_body_adapter;

pub use ability_adapter::{
    suppress_ledge_grab_during_transit, warp_portal_input, SuppressWallAbilitiesInPortal,
};
pub use carve_adapter::bridge_portal_carves;
pub use fire_adapter::resolve_portal_fire_intent;
pub use input_adapter::{pick_aim, portal_input_adapter_system};
pub use inventory_adapter::{
    drop_portal_gun_system, equip_portal_gun, pickup_portal_gun_system, unequip_portal_gun,
};
pub use plugin::AmbitionPortalAdaptersPlugin;
pub use reset_adapter::bridge_room_reset_to_clear_portals;
pub use sfx_adapter::play_portal_sfx;
pub use shot_adapter::portal_projectile_step;
pub use transit_adapter::{
    apply_movement_intent_to_control, sync_ground_items_to_transitable,
    sync_movement_intent_from_control, sync_transitable_to_ground_items,
};
pub use transit_body_adapter::{
    ensure_portal_bodies, ensure_projectile_portal_bodies, portal_player_input_adapter,
};

#[cfg(test)]
mod tests;
