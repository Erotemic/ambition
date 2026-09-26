//! Rollback declaration owned by `ambition_portal2d`.
//!
//! Portal simulation and the portal gun register separately, like their
//! plugins, so portal-only users do not get gun state.
//! [`register_rollback_state`] registers both.
//!
//! The host supplies the backend through [`RollbackRegistrar`]. This module has
//! no `bevy_ggrs` dependency and no host/composition logic.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

const OWNER: &str = env!("CARGO_PKG_NAME");

/// Register reusable portal topology/transit/shot state, with no portal-gun
/// custody or control vocabulary.
pub fn register_portal_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    registrar.require_rollback::<crate::PlacedPortal>(OWNER, "entity:placed_portal");
    registrar.rollback_resource_clone::<crate::PortalFrameHistory>(
        OWNER,
        "resource.portal_frame_history",
    );
    registrar.rollback_component_clone::<crate::PortalTransit>(OWNER, "portal.transit");
    registrar.rollback_component_clone::<crate::PlacedPortal>(OWNER, "portal.placed");
    registrar.rollback_component_clone_probed::<crate::PortalTransitCooldown>(
        OWNER,
        "portal.transit_cooldown",
        |cooldown| cooldown.remaining.to_bits() as u64,
    );
    registrar.rollback_component_clone::<crate::PortalEmission>(OWNER, "portal.emission");
    // A shot is a generic portal opener (any emitter of `PortalFireIntent`).
    //
    // The codec and the anchor are separate. `rollback_component_clone` says
    // what to save if the entity is in the rollback set; `require_rollback`
    // puts it there. A shot is spawned mid-match and decides where a portal
    // opens, so it must be an anchor, or a mispredicted shot could survive a
    // rollback.
    registrar.require_rollback::<crate::PortalShot>(OWNER, "entity:portal_shot");
    registrar.rollback_component_clone::<crate::PortalShot>(OWNER, "portal.shot");
    registrar.declare_rollback_derived_component::<crate::PortalTransitable>(
        OWNER,
        "derived.portal_transitable",
        "mirrored from the item's authoritative body every frame, before transit reads it",
    );
    registrar.declare_rollback_derived_resource::<crate::PortalCarves>(
        OWNER,
        "derived.portal_carves",
        "rebuilt from placed portals and transit occupancy each frame",
    );
    registrar.declare_rollback_derived_resource::<crate::PortalHostDepths>(
        OWNER,
        "derived.portal_host_depths",
        "republished from the authoritative collision world each frame",
    );
    registrar.clear_message_on_rollback::<crate::ClearPortals>(OWNER, "message.clear_portals");
    registrar.clear_message_on_rollback::<crate::PortalBodyEntered>(
        OWNER,
        "message.portal_body_entered",
    );
    registrar
        .clear_message_on_rollback::<crate::PortalFireIntent>(OWNER, "message.portal_fire_intent");
    registrar
        .clear_message_on_rollback::<crate::PortalShotFired>(OWNER, "message.portal_shot_fired");
    registrar.clear_message_on_rollback::<crate::BodyTeleported>(OWNER, "message.body_teleported");
    registrar.clear_message_on_rollback::<crate::PortalBodyTransited>(
        OWNER,
        "message.portal_body_transited",
    );
}

/// Register state owned specifically by the optional held portal-gun workflow.
pub fn register_portal_gun_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    // The pickup carries SimId, SpawnOrigin and TransactionId, so it must be an
    // entity anchor whenever the gun capability is installed.
    registrar.require_rollback::<crate::PortalGunPickup>(OWNER, "entity:portal_gun_pickup");
    registrar.rollback_component_clone::<crate::PortalGunPickup>(OWNER, "portal.gun_pickup");
    registrar.rollback_component_clone::<crate::PortalGun>(OWNER, "portal.gun");
    // The owned pair outlives the gun in hand, so it is rollback state too.
    // Probed, so the checksum sees the pair value and not only presence. The
    // projection is the `u8` itself.
    registrar.rollback_component_clone_probed::<crate::OwnedPortalGunPair>(
        OWNER,
        "portal.owned_gun_pair",
        |pair| u64::from(pair.0),
    );
    registrar.clear_message_on_rollback::<crate::DropPortalGun>(OWNER, "message.drop_portal_gun");
    registrar.clear_message_on_rollback::<crate::FirePortalGun>(OWNER, "message.fire_portal_gun");
    registrar
        .clear_message_on_rollback::<crate::PickUpPortalGun>(OWNER, "message.pick_up_portal_gun");
    registrar.clear_message_on_rollback::<crate::PortalGunEquipped>(
        OWNER,
        "message.portal_gun_equipped",
    );
    registrar
        .clear_message_on_rollback::<crate::TogglePortalGun>(OWNER, "message.toggle_portal_gun");
}

/// Full portal registration, including the gun. Portal-only compositions call
/// [`register_portal_rollback_state`].
pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    register_portal_rollback_state(registrar);
    register_portal_gun_rollback_state(registrar);
}
