//! Portal mechanic facade.
//!
//! The portal gun is the flagship player ability (vertical slice): fire a
//! blue/orange portal pair onto solid surfaces, then travel between them
//! carrying your momentum (Portal-style). The implementation is split into
//! responsibility submodules behind this facade so external paths
//! (`crate::portal::X`) keep working while routine portal changes touch one
//! small file instead of a multi-thousand-line module:
//!
//! - [`color`] — [`PortalGunColor`], [`PortalChannelColor`], and the unifying
//!   [`PortalChannel`] (parse/display/pairing).
//! - [`types`] — shared [`PlacedPortal`] body, geometry constants, and small helpers.
//! - [`gun`] — the held [`PortalGun`] and its equip / toggle state.
//! - [`pickup`] — the world [`PortalGunPickup`] and pickup/drop systems.
//! - [`shot`] — the in-flight [`PortalShot`] and firing.
//! - [`placement`] — portal-aware raycast, fit check, and the `transit_step`
//!   decision machine.
//! - [`transit`] — player/actor/item transit systems plus the carve / input
//!   guards.
//! - [`lifecycle`] — portal orphan cleanup and room-reset portal clearing.
//! - `presentation` — visible-build visual sync (sprites, rings, body pieces,
//!   disorientation FX). Render-only: compiled and re-exported **exclusively**
//!   behind the `portal_render` feature, and registered by the presentation
//!   plugin. The portal *simulation* (every module above) builds without it, so
//!   portal core is render-independent.
//!
//! It stays deterministic (no RNG, no per-frame allocation in the hot path) so
//! it runs identically in the headless sim.

mod color;
mod gun;
mod lifecycle;
mod messages;
mod pickup;
/// Pure portal-piece geometry — the Core invariant (was the root
/// `crate::portal_pieces`). Public because the host world-overlay carve and the
/// debug overlay read `crate::portal::pieces` directly.
pub mod pieces;
mod placement;
mod plugin;
/// Portal presentation (sprites, rings, body pieces, disorientation FX). Render
/// only — compiled and registered exclusively behind the `portal_render` feature
/// so portal *simulation* (transit, placement, lifecycle, carve, pieces
/// geometry) builds without any render-facing systems or components.
#[cfg(feature = "portal_render")]
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

pub use color::{PortalChannel, PortalChannelColor, PortalGunColor};
/// The `F7` dev off-switch is only wired by the render layer (visible build), so
/// its re-export rides the same feature even though the system itself is
/// render-free (generic Bevy keyboard input).
#[cfg(feature = "portal_render")]
pub use gun::portal_dev_toggle_system;
pub use gun::{portal_toggle_system, PortalGun};
pub use lifecycle::{clear_portals_on_reset, despawn_orphaned_portals};
pub use messages::{
    DropPortalGun, FirePortalGun, PickUpPortalGun, PortalGunEquipped, TogglePortalGun,
};
pub use pickup::{arm_portal_pickups, PortalGunPickup};
pub use placement::{
    portal_facing_flips, portal_fits, portal_transit_roll, raycast_through_portals,
    somersault_roll, transit_step, TransitStep,
};
#[cfg(feature = "portal_render")]
pub use presentation::{
    load_portal_gun_art, sync_portal_body_pieces, sync_portal_disorientation_indicator,
    sync_portal_mode_indicator, sync_portal_visuals, PortalAimHint, PortalBodyPiece,
    PortalDisorientIndicator, PortalGunArt, PortalModeIndicator, PortalVisual,
};
pub use shot::{portal_fire_system, portal_projectile_step, PortalShot};
pub use transit::{
    portal_teleport_ground_items, portal_transit, publish_portal_carves,
    suppress_ledge_grab_during_transit, tick_portal_cooldowns, warp_portal_input, BodyTeleported,
    PlayerMovementIntent, PortalBody, PortalBodyTransited, PortalCarves, PortalEmission,
    PortalInputWarp, PortalPolicy, PortalTransit, PortalTransitable, SuppressWallAbilitiesInPortal,
};
pub use types::{portal_half_extent, PlacedPortal, PortalTransitCooldown};

pub use plugin::{PortalPlugin, PortalSimulationPlugin};
pub use schedule::PortalSet;
