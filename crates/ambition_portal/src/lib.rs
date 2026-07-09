//! Reusable, content-free portal mechanic.
//!
//! Owns portal topology, placement, transit math, carves, lifecycle, events,
//! and schedule labels. Hosts provide input/inventory bindings, collision-world
//! application, room-reset policy, rendering/audio/VFX, and content-specific
//! behavior through adapters around [`PortalPlugin`].
//!
//! The current Ambition portal-gun workflow is kept in clearly named
//! `gun_*` compatibility modules. It is not the conceptual core of this crate:
//! games should be able to use static portals, scripted emitters, arbitrary
//! portal openers, and moving portals without adopting a gun.
//!
//! Any entity with [`BodyKinematics`](ambition_platformer_primitives::body::BodyKinematics),
//! [`PortalBody`], and a [`PortalPolicy`] can use the generic
//! [`portal_transit`] path. The crate depends only on `bevy`,
//! `ambition_engine_core`, and `ambition_platformer_primitives`, so it stays
//! deterministic and host-free.

mod color;
mod eviction;
mod gun;
mod gun_lifecycle;
mod gun_pickup;
mod gun_projectile;
mod lifecycle;
mod link;
mod messages;
/// Pure portal-piece geometry — the Core invariant. Public because a host's
/// world-overlay carve and debug overlay read `pieces` directly.
pub mod pieces;
mod placement;
mod plugin;
mod schedule;
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
pub use ambition_platformer_primitives::orientation::{
    ensure_actor_roll, update_actor_roll, ActorRoll,
};
pub use ambition_platformer_primitives::transit::rotate_velocity_between_normals as portal_transform_velocity;

pub use color::{PortalChannel, PortalChannelColor, PortalGunColor};
pub use eviction::{evict_straddlers_on_portal_change, PortalFrameHistory};
pub use gun::{portal_toggle_system, PortalGun};
pub use gun_lifecycle::despawn_orphaned_portals;
pub use gun_pickup::{arm_portal_pickups, PortalGunPickup};
pub use gun_projectile::{
    is_portal_placeable, portal_fire_system, step_portal_shot, PortalShot, PortalShotStep,
    PortalShotWorld,
};
pub use lifecycle::clear_portals_on_reset;
pub use link::{equalize_pair_apertures, link_hash, resolve_portal_links, PortalLink};
pub use messages::{
    ClearPortals, DropPortalGun, FirePortalGun, PickUpPortalGun, PortalBodyEntered,
    PortalFireIntent, PortalGunEquipped, PortalShotFired, TogglePortalGun,
};
pub use pieces::{portal_map_rotation, set_portal_map_rotation};
pub use placement::{
    measure_host_depth, portal_facing_flips, portal_facing_flips_for_convention, portal_fits,
    portal_input_warp_flips_horizontal, portal_input_warp_flips_horizontal_for_convention,
    portal_transit_roll, raycast_through_portals, raycast_through_portals_tuned, somersault_roll,
    somersault_roll_for_convention, transit_step, transit_step_with_tuning, SweptSample,
    TransitStep,
};
pub use transit::{
    portal_teleport_ground_items, portal_transit, publish_portal_carves, tick_portal_cooldowns,
    BodyTeleported, PlayerMovementIntent, PortalBody, PortalBodyTransited, PortalCarves,
    PortalEmission, PortalInputWarp, PortalPolicy, PortalTransit,
    PortalTransitable,
};
pub use tuning::{sync_portal_tuning_convention, PortalConvention, PortalTuning};
pub use types::{
    find_portal, portal_half_extent, portal_half_extent_with_length, portal_opening_half,
    PlacedPortal, PortalHostDepths, PortalTransitCooldown, MIN_EXIT_SPEED, PORTAL_VISUAL_THICKNESS,
};
pub use view::{
    aperture_wedge, aperture_wedge_multi, blend_cones, copy_roll, copy_transform,
    copy_transform_for_convention, map_viewpoint_frame, view_cone, view_point, visible_cone,
    window_eye, PortalCopyTransform, PortalViewMap, PortalViewpointFrame, ViewCone,
};

pub use plugin::{PortalPlugin, PortalSimulationPlugin};
pub use schedule::PortalSet;
