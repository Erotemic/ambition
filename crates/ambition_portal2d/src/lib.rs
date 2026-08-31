//! Reusable, content-free portal mechanic.
//!
//! Owns portal topology, placement, transit math, carves, lifecycle, events,
//! and schedule labels. Hosts provide input/inventory bindings, collision-world
//! application, room-reset policy, rendering/audio/VFX, and content-specific
//! behavior through adapters around [`PortalPlugin`].
//!
//! TODO(compat-remove): move the Ambition portal-gun workflow out of this generic mechanic
//! crate and delete the `gun_*` compatibility modules.
//!
//! Any entity with [`BodyKinematics`](ambition_platformer2d_shared_tangle::body::BodyKinematics),
//! [`PortalBody`], and a [`PortalPolicy`] can use the generic
//! [`portal_transit`] path. The crate depends only on `bevy`,
//! `ambition_platformer2d_core`, and `ambition_platformer2d_shared_tangle`, so it stays
//! deterministic and host-free.

mod color;
mod eviction;
mod gun;
mod gun_construction;
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
/// `ambition_portal2d_presentation` default renderer or a host's own) build
/// capture cameras + cone UVs from it.
pub mod view;

// TODO(compat-remove): migrate host callers to the owning crates, then remove these lower-crate
// re-exports from the portal API.
pub use ambition_platformer2d_shared_tangle::orientation::{
    ensure_actor_roll, update_actor_roll, ActorRoll,
};
pub use ambition_platformer2d_shared_tangle::transit::rotate_velocity_between_normals as portal_transform_velocity;

pub use color::{PortalChannel, PortalChannelColor, PortalGunColor};
pub use eviction::{evict_straddlers_on_portal_change, PortalFrameHistory};
pub use gun::{portal_toggle_system, PortalGun};
pub use gun_construction::{
    install_portal_gun_construction_recipes, portal_gun_construction_registry,
    recipe_authored_portal_gun, PortalGunConstruction, PortalGunConstructionParams,
    PortalGunConstructionPlan, PortalGunConstructionRegistry, PortalGunConstructionRequest,
    PORTAL_GUN_CONSTRUCTION_DOMAIN, RECIPE_AUTHORED_PORTAL_GUN,
};
pub use gun_lifecycle::despawn_orphaned_portals;
pub use gun_pickup::{arm_portal_pickups, PortalGunPickup, PortalPickupArming};
pub use gun_projectile::{
    is_portal_placeable, portal_fire_system, step_portal_shot, PortalShot, PortalShotStep,
    PortalShotWorld,
};
pub use lifecycle::clear_portals_on_reset;
pub use link::{
    equalize_pair_apertures, link_hash, resolve_portal_links, PortalLink, PortalLinkResolution,
};
pub use messages::{
    ClearPortals, DropPortalGun, FirePortalGun, PickUpPortalGun, PortalBodyEntered,
    PortalFireIntent, PortalGunEquipped, PortalShotFired, TogglePortalGun,
};
pub use placement::{
    measure_host_depth, portal_facing_flips_for_convention, portal_fits,
    portal_input_warp_flips_horizontal_for_convention, portal_transit_roll,
    raycast_through_portals, raycast_through_portals_tuned, somersault_roll_for_convention,
    transit_step, transit_step_with_tuning, SweptSample, TransitStep,
};
pub use transit::{
    portal_teleport_ground_items, portal_transit, publish_portal_carves, tick_portal_cooldowns,
    BodyTeleported, PortalBody, PortalBodyTransited, PortalCarves, PortalEmission, PortalInputWarp,
    PortalPolicy, PortalTransit, PortalTransitable,
};
pub use tuning::{PortalConvention, PortalTuning};
pub use types::{
    find_portal, portal_half_extent, portal_half_extent_with_length, portal_opening_half,
    PlacedPortal, PortalHostDepths, PortalTransitCooldown, MIN_EXIT_SPEED, PORTAL_VISUAL_THICKNESS,
};
pub use view::{
    aperture_wedge, aperture_wedge_multi, blend_cones, copy_roll, copy_transform,
    copy_transform_for_convention, map_viewpoint_frame, view_cone, view_point, visible_cone,
    window_eye, PortalCopyTransform, PortalViewMap, PortalViewpointFrame, ViewCone,
};

pub use plugin::{PortalGunPlugin, PortalPlugin, PortalSimulationPlugin};
pub use schedule::PortalSet;

// Domain-owned rollback declaration; the host supplies the backend registrar.
mod rollback_registration;
pub use rollback_registration::{
    register_portal_gun_rollback_state, register_portal_rollback_state, register_rollback_state,
};
