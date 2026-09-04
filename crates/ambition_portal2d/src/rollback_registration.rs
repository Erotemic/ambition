//! Rollback declaration owned by `ambition_portal2d`.
//!
//! Portal simulation and the optional portal-gun opener publish separate
//! registration functions for the same reason their Bevy plugins are separate:
//! static/scripted portal users should not inherit gun state merely by opting
//! into portal topology and transit. [`register_rollback_state`] remains the
//! compatibility composition and registers both surfaces.
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
    registrar.rollback_component_clone::<crate::PortalBody>(OWNER, "portal.body");
    registrar.rollback_component_clone::<crate::PortalPolicy>(OWNER, "portal.policy");
    registrar.rollback_component_clone::<crate::PortalTransit>(OWNER, "portal.transit");
    registrar.rollback_component_clone::<crate::PlacedPortal>(OWNER, "portal.placed");
    registrar.rollback_component_clone_probed::<crate::PortalTransitCooldown>(
        OWNER,
        "portal.transit_cooldown",
        |cooldown| cooldown.remaining.to_bits() as u64,
    );
    registrar.rollback_component_clone::<crate::PortalEmission>(OWNER, "portal.emission");
    // A shot is a generic portal opener: scripts, AI, moving emitters, or a gun
    // can all produce the same PortalFireIntent.
    //
    // ⛔⛔ THE ANCHOR IS A SEPARATE FACT FROM THE CODEC, and this shipped with
    // only the codec. `rollback_component_clone` says what to save IF the entity
    // is in the rollback envelope; `require_rollback` is what PUTS it there
    // (`register_required_components::<PortalShot, Rollback>`). A shot is spawned
    // MID-MATCH by `portal_fire_system`, carries authoritative `pos`/`vel`/
    // `traveled`, and decides where a portal opens — so a shot on an abandoned
    // prediction branch could keep flying to a placement the authoritative
    // timeline never fired. Every registry-shaped check read this as covered.
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
    // Historical alias retained so the full compatibility registration keeps the existing
    // rollback schema byte-for-byte.
    registrar.clear_message_on_rollback::<crate::ClearPortals>(OWNER, "message.portal_clear");
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
    // The pair a body owns outlives the gun in its hand, so it is state the
    // same way the gun is: a rollback that restored the hand but not the
    // ownership would re-equip the wrong gun after the resimulation.
    registrar
        .rollback_component_clone::<crate::OwnedPortalGunPair>(OWNER, "portal.owned_gun_pair");
    registrar.clear_message_on_rollback::<crate::DropPortalGun>(OWNER, "message.drop_portal_gun");
    registrar.clear_message_on_rollback::<crate::DropPortalGun>(OWNER, "message.portal_gun_drop");
    registrar.clear_message_on_rollback::<crate::FirePortalGun>(OWNER, "message.fire_portal_gun");
    registrar.clear_message_on_rollback::<crate::FirePortalGun>(OWNER, "message.portal_gun_fire");
    registrar
        .clear_message_on_rollback::<crate::PickUpPortalGun>(OWNER, "message.pick_up_portal_gun");
    registrar
        .clear_message_on_rollback::<crate::PickUpPortalGun>(OWNER, "message.portal_gun_pick_up");
    registrar.clear_message_on_rollback::<crate::PortalGunEquipped>(
        OWNER,
        "message.portal_gun_equipped",
    );
    registrar
        .clear_message_on_rollback::<crate::TogglePortalGun>(OWNER, "message.toggle_portal_gun");
    registrar
        .clear_message_on_rollback::<crate::TogglePortalGun>(OWNER, "message.portal_gun_toggle");
}

/// Backward-compatible full portal registration used by the existing runtime.
/// New portal-only compositions may call [`register_portal_rollback_state`]
/// without adopting the gun.
pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    register_portal_rollback_state(registrar);
    register_portal_gun_rollback_state(registrar);
}
