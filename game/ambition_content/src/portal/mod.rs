//! Ambition-specific portal adapters.
//!
//! These modules translate Ambition game concepts (the [`ControlFrame`] input
//! channel and the portal gun, an ordinary held item) into the reusable,
//! content-agnostic portal intent/outcome messages exposed by
//! [`ambition_portal2d`]. The reusable portal mechanic never imports Ambition input
//! or inventory types; this boundary owns that glue.
//!
//! Gated behind the `portal` feature.
//!
//! [`ControlFrame`]: ambition_platformer2d_core::ControlFrame

mod ability_adapter;
mod carve_adapter;
mod fire_adapter;
mod host_adapter;
mod input_adapter;
mod inventory_adapter;
mod plugin;
mod reset_adapter;
mod sfx_adapter;
mod shot_adapter;

pub use ability_adapter::{
    withhold_wall_verbs_during_transit, SuppressWallAbilitiesInPortal, PORTAL_TRANSIT,
};
pub use carve_adapter::bridge_portal_carves;
pub use fire_adapter::resolve_portal_fire_intent;
pub use input_adapter::{pick_aim, portal_input_adapter_system};
pub use inventory_adapter::{opens_portals, reconcile_portal_generators, PORTAL_GENERATOR_ITEMS};
pub use plugin::AmbitionPortalAdaptersPlugin;
pub use reset_adapter::bridge_room_reset_to_clear_portals;
pub use sfx_adapter::play_portal_sfx;
pub use shot_adapter::portal_projectile_step;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod throw_precedence_tests;
