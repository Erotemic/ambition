//! Portal mechanic facade.
//!
//! The portal gun is the flagship player ability (vertical slice): fire a
//! blue/orange portal pair onto solid surfaces, then travel between them
//! carrying your momentum (Portal-style). The implementation is split into
//! responsibility submodules behind this facade so external paths
//! (`crate::portal::X`) keep working while routine portal changes touch one
//! small file instead of a multi-thousand-line module:
//!
//! - [`color`] — [`PortalColor`] and channel parse/display.
//! - [`types`] — shared [`PlacedPortal`] body, geometry constants, and small helpers.
//! - [`gun`] — the held [`PortalGun`] and its equip / toggle state.
//! - [`pickup`] — the world [`PortalGunPickup`] and pickup/drop systems.
//! - [`shot`] — the in-flight [`PortalShot`] and firing.
//! - [`placement`] — portal-aware raycast, fit check, and the `transit_step`
//!   decision machine.
//! - [`transit`] — player/actor/item transit systems plus the carve / input
//!   guards.
//! - [`lifecycle`] — orphan cleanup, room-reset, and the gravity-flip switch.
//! - [`presentation`] — visible-build visual sync (registered by the
//!   presentation plugin).
//!
//! It stays deterministic (no RNG, no per-frame allocation in the hot path) so
//! it runs identically in the headless sim.

mod color;
mod gun;
mod lifecycle;
mod pickup;
mod placement;
mod plugin;
mod presentation;
mod schedule;
mod shot;
mod transit;
mod types;

#[cfg(test)]
mod tests;

pub use crate::platformer_runtime::collision::raycast_solids;
pub use crate::platformer_runtime::orientation::{ensure_actor_roll, update_actor_roll, ActorRoll};
pub use crate::platformer_runtime::transit::rotate_velocity_between_normals as portal_transform_velocity;

pub use color::PortalColor;
pub use gun::{
    equip_portal_gun, portal_dev_toggle_system, portal_toggle_system, unequip_portal_gun, PortalGun,
};
pub use lifecycle::{
    clear_portals_on_reset, despawn_orphaned_portals, gravity_flip_switch_system,
    reset_gravity_on_room_reset, GravityFlipSwitch,
};
pub use pickup::{
    arm_portal_pickups, drop_portal_gun_system, pickup_portal_gun_system, PortalGunPickup,
};
pub use placement::{
    portal_facing_flips, portal_fits, portal_transit_roll, raycast_through_portals,
    somersault_roll, transit_step, TransitStep,
};
pub use presentation::{
    load_portal_gun_art, sync_gravity_switch_visual, sync_gravity_zone_visual,
    sync_portal_body_pieces, sync_portal_disorientation_indicator, sync_portal_mode_indicator,
    sync_portal_visuals, GravitySwitchVisual, GravityZoneVisual, PortalBodyPiece,
    PortalDisorientIndicator, PortalGunArt, PortalModeIndicator, PortalVisual,
};
pub use shot::{portal_fire_system, portal_projectile_step, PortalShot};
pub use transit::{
    portal_teleport_ground_items, portal_transit_actors, portal_transit_system,
    publish_portal_carves, suppress_ledge_grab_during_transit, tick_portal_cooldowns,
    warp_portal_input, BodyTeleported, PortalEmission, PortalInputWarp, PortalTransit,
    SuppressWallAbilitiesInPortal,
};
pub use types::{portal_half_extent, PlacedPortal, PortalTransitCooldown};

pub use plugin::{PortalPlugin, PortalSimulationPlugin};
pub use schedule::PortalSet;
