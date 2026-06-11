//! The reusable, content-free **portal mechanic** crate.
//!
//! This is a small physics/mechanic plugin, not "Ambition's portal gun ripped
//! into a crate": it owns portals, the portal gun's place/replace/channel
//! mechanic, placement + aperture transit math, portal carves (the geometry —
//! not how a host applies them to its collision), portal **body movement**
//! through a pair, lifecycle, and the portal events. It does NOT own input,
//! inventory, room-reset policy, the collision-world implementation,
//! rendering/audio/VFX, fireball semantics, player abilities, achievements, or
//! how carves alter collision — those are the host's adapters around
//! [`PortalPlugin`].
//!
//! ## How a host uses it
//! 1. `app.add_plugins(PortalPlugin)` — registers the mechanic systems +
//!    portal-owned messages/resources in the [`PortalSet`] schedule labels.
//! 2. Tag the bodies that should transit: any entity carrying
//!    [`BodyKinematics`](ambition_platformer_runtime::body::BodyKinematics) +
//!    [`PortalBody`] + a behavioral [`PortalPolicy`] uses the ONE generic
//!    [`portal_transit`] algorithm (players, enemies, bosses, projectiles —
//!    identity → policy is the host's job, never the crate's).
//! 3. Bridge the seams with adapters: produce [`PortalFireIntent`] from input,
//!    copy [`PortalCarves`] into the host collision representation, emit
//!    [`ClearPortals`] on room reset, play sfx from the portal-owned audio
//!    signals ([`PortalShotFired`] / [`PortalBodyEntered`] /
//!    [`PortalBodyTransited`]), and shape input / player abilities off the
//!    portal-owned crossing components ([`PortalInputWarp`] / [`PortalEmission`]
//!    / [`PortalTransit`]). The crate emits everything an adapter needs; it never
//!    names the host.
//!
//! Depends ONLY on `bevy` + `ambition_engine_core` + `ambition_platformer_runtime`
//! — never on a host crate. It stays deterministic (no RNG, no per-frame
//! allocation in the hot path) so it runs identically in a headless sim.
//!
//! The implementation is split into responsibility submodules:
//! - [`color`] — [`PortalGunColor`], [`PortalChannelColor`], and the unifying
//!   [`PortalChannel`] (parse/display/pairing).
//! - [`types`] — shared [`PlacedPortal`] body, geometry constants, small helpers.
//! - [`gun`] — the held [`PortalGun`] and its toggle state.
//! - [`pickup`] — the world [`PortalGunPickup`] and the arm-timer tick.
//! - [`shot`] — the in-flight [`PortalShot`] + the pure [`step_portal_shot`]
//!   helper over [`SolidWorldQuery`](ambition_platformer_runtime::world_query::SolidWorldQuery).
//! - [`placement`] — portal-aware raycast, fit check, and the [`transit_step`]
//!   decision machine.
//! - [`transit`] — the one generic [`portal_transit`] algorithm + the carve
//!   publish + cooldown tick, plus the portal-owned crossing components.
//! - [`lifecycle`] — portal orphan cleanup and reset-time portal clearing.
//! - [`pieces`] — the pure portal-piece geometry (the Core invariant).

mod color;
mod eviction;
mod gun;
mod lifecycle;
mod link;
mod messages;
mod pickup;
/// Pure portal-piece geometry — the Core invariant. Public because a host's
/// world-overlay carve and debug overlay read `pieces` directly.
pub mod pieces;
mod placement;
mod plugin;
mod schedule;
mod shot;
mod transit;
mod tuning;
mod types;
/// Pure through-portal VIEW geometry (the view map — always a proper rotation
/// — and the view cone). Public because renderers (the
/// `ambition_portal_presentation` default renderer or a host's own) build
/// capture cameras + cone UVs from it.
pub mod view;

// Re-export the lower-crate surfaces the portal mechanic builds on, so a host's
// `crate::portal::…` facade and the portal adapters keep resolving these paths.
pub use ambition_platformer_runtime::orientation::{
    ensure_actor_roll, update_actor_roll, ActorRoll,
};
pub use ambition_platformer_runtime::transit::rotate_velocity_between_normals as portal_transform_velocity;
pub use ambition_platformer_runtime::world_query::raycast_solids;

pub use color::{PortalChannel, PortalChannelColor, PortalGunColor};
pub use eviction::{evict_straddlers_on_portal_change, PortalFrameHistory};
pub use gun::{portal_toggle_system, PortalGun};
pub use lifecycle::{clear_portals_on_reset, despawn_orphaned_portals};
pub use link::{equalize_pair_apertures, link_hash, resolve_portal_links, PortalLink};
pub use messages::{
    ClearPortals, DropPortalGun, FirePortalGun, PickUpPortalGun, PortalBodyEntered,
    PortalFireIntent, PortalGunEquipped, PortalShotFired, TogglePortalGun,
};
pub use pickup::{arm_portal_pickups, PortalGunPickup};
pub use pieces::{portal_map_rotation, set_portal_map_rotation};
pub use placement::{
    portal_facing_flips, portal_facing_flips_for_convention, portal_fits,
    portal_input_warp_flips_horizontal, portal_input_warp_flips_horizontal_for_convention,
    portal_transit_roll, raycast_through_portals, raycast_through_portals_tuned, somersault_roll,
    somersault_roll_for_convention, transit_step, transit_step_with_tuning, TransitStep,
};
pub use shot::{
    is_portal_placeable, portal_fire_system, step_portal_shot, PortalShot, PortalShotStep,
    PortalShotWorld,
};
pub use transit::{
    portal_teleport_ground_items, portal_transit, publish_portal_carves, tick_portal_cooldowns,
    BodyTeleported, PlayerMovementIntent, PortalBody, PortalBodyTransited, PortalCarves,
    PortalEmission, PortalInputWarp, PortalPolicy, PortalTransit, PortalTransitable,
};
pub use tuning::{sync_portal_tuning_convention, PortalConvention, PortalTuning};
pub use types::{
    find_portal, portal_half_extent, portal_half_extent_with_length, portal_opening_half,
    PlacedPortal, PortalTransitCooldown, MIN_EXIT_SPEED, PORTAL_VISUAL_THICKNESS,
};
pub use view::{
    aperture_wedge, aperture_wedge_multi, blend_cones, copy_roll, copy_transform,
    copy_transform_for_convention, view_cone, view_point, visible_cone, window_eye,
    PortalCopyTransform, PortalViewMap, ViewCone,
};

pub use plugin::{PortalPlugin, PortalSimulationPlugin};
pub use schedule::PortalSet;
