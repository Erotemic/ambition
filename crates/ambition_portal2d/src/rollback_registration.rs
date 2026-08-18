//! Rollback declaration owned by `ambition_portal2d`.
//!
//! This module names this domain's concrete rewindable state while the host
//! supplies the backend through [`RollbackRegistrar`]. It deliberately contains
//! no `bevy_ggrs` dependency and no host/composition logic.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

const OWNER: &str = env!("CARGO_PKG_NAME");

/// Register everything the portal domain needs rewound.
pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    registrar.require_rollback::<crate::PlacedPortal>(OWNER, "entity:placed_portal");
    // ⚠ **the PICKUP, not just the placed portal** (2026-08-06, K2b edit 2).
    // The gun pickup carries `SimId`, `SpawnOrigin` and `TransactionId` — all
    // rollback-registered state — and had no anchor, so every one of those
    // registrations was INERT on it: the registry listed them, the coverage
    // sweep counted them as accounted, and nothing restored them.
    //
    // ⭐ **it was invisible because the entity was outside the swept
    // population.** Direct entry built its world UNSCOPED, so the pickup carried
    // no `SessionScopedEntity` and the sweep never looked at it. Deleting the
    // build-time root put the whole authored room inside a session scope, which
    // is what made this visible — the same class as `WorldItem`, found the same
    // way, one composition later.
    registrar.require_rollback::<crate::PortalGunPickup>(
        OWNER,
        "entity:portal_gun_pickup",
    );
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
    registrar.rollback_component_clone::<crate::PortalGunPickup>(OWNER, "portal.gun_pickup");
    registrar.rollback_component_clone::<crate::PortalEmission>(OWNER, "portal.emission");
    registrar.rollback_component_clone::<crate::PortalShot>(OWNER, "portal.shot");
    registrar.rollback_component_clone::<crate::PortalGun>(OWNER, "portal.gun");
    registrar.declare_rollback_derived_component::<crate::PortalTransitable>(
        OWNER,
        "derived.portal_transitable",
        "mirrored from the item's authoritative body every frame, before transit reads it",
    );
    registrar.declare_rollback_derived_resource::<crate::PlayerMovementIntent>(
        OWNER,
        "derived.portal_player_movement_intent",
        "republished from the current controller frame before portal transit",
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
    registrar.clear_message_on_rollback::<crate::DropPortalGun>(
        OWNER,
        "message.drop_portal_gun",
    );
    registrar.clear_message_on_rollback::<crate::FirePortalGun>(
        OWNER,
        "message.fire_portal_gun",
    );
    registrar.clear_message_on_rollback::<crate::PickUpPortalGun>(
        OWNER,
        "message.pick_up_portal_gun",
    );
    registrar.clear_message_on_rollback::<crate::PortalBodyEntered>(
        OWNER,
        "message.portal_body_entered",
    );
    registrar.clear_message_on_rollback::<crate::PortalFireIntent>(
        OWNER,
        "message.portal_fire_intent",
    );
    registrar.clear_message_on_rollback::<crate::PortalGunEquipped>(
        OWNER,
        "message.portal_gun_equipped",
    );
    registrar.clear_message_on_rollback::<crate::PortalShotFired>(
        OWNER,
        "message.portal_shot_fired",
    );
    registrar.clear_message_on_rollback::<crate::TogglePortalGun>(
        OWNER,
        "message.toggle_portal_gun",
    );
    registrar.clear_message_on_rollback::<crate::BodyTeleported>(
        OWNER,
        "message.body_teleported",
    );
    registrar.clear_message_on_rollback::<crate::PortalBodyTransited>(
        OWNER,
        "message.portal_body_transited",
    );
    registrar.clear_message_on_rollback::<crate::ClearPortals>(OWNER, "message.portal_clear");
    registrar.clear_message_on_rollback::<crate::DropPortalGun>(
        OWNER,
        "message.portal_gun_drop",
    );
    registrar.clear_message_on_rollback::<crate::FirePortalGun>(
        OWNER,
        "message.portal_gun_fire",
    );
    registrar.clear_message_on_rollback::<crate::PickUpPortalGun>(
        OWNER,
        "message.portal_gun_pick_up",
    );
    registrar.clear_message_on_rollback::<crate::PortalBodyEntered>(
        OWNER,
        "message.portal_body_entered",
    );
    registrar.clear_message_on_rollback::<crate::PortalFireIntent>(
        OWNER,
        "message.portal_fire_intent",
    );
    registrar.clear_message_on_rollback::<crate::PortalGunEquipped>(
        OWNER,
        "message.portal_gun_equipped",
    );
    registrar.clear_message_on_rollback::<crate::PortalShotFired>(
        OWNER,
        "message.portal_shot_fired",
    );
    registrar.clear_message_on_rollback::<crate::TogglePortalGun>(
        OWNER,
        "message.portal_gun_toggle",
    );
    registrar.clear_message_on_rollback::<crate::BodyTeleported>(
        OWNER,
        "message.body_teleported",
    );
    registrar.clear_message_on_rollback::<crate::PortalBodyTransited>(
        OWNER,
        "message.portal_body_transited",
    );
}
