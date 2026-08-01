//! **The portal domain's rollback schema** (Campaign 2, R3).
//!
//! 37 registrations: the placed apertures, the gun and its pickup, the transit
//! bookkeeping, and the message buffers a rewound tick must not replay.
//!
//! ⚠ **relocation only.** R3: *"preserve registration order and projections;
//! verify the resulting schema fingerprint is unchanged. Do not strengthen probes
//! or alter snapshot behavior in the same commit as the relocation."*
//! `rollback_schema_baseline` is what verifies it, and it names the exact line if
//! a call was mistranscribed — which is why these were extracted mechanically
//! rather than retyped.
//!
//! ⚠ the owner label stays `ambition_platformer2d_runtime` because this module is IN
//! `ambition_platformer2d_runtime`, and it must be: `ambition_portal2d` sits below the runtime
//! in the crate graph and cannot depend on the registration vocabulary. See R1's
//! recorded decision — that is the shape for every domain below the runtime, and
//! crates ABOVE it (`ambition_content`) own their schemas directly.

use bevy::prelude::App;

use super::super::AmbitionRollbackApp;

const OWNER: &str = "ambition_platformer2d_runtime";

/// Register everything the portal domain needs rewound.
pub(in crate::rollback) fn register(app: &mut App) {
    app.require_rollback::<ambition_portal2d::PlacedPortal>(OWNER, "entity:placed_portal");
    app.rollback_resource_clone::<ambition_portal2d::PortalFrameHistory>(
        OWNER,
        "resource.portal_frame_history",
    );
    app.rollback_component_clone::<ambition_portal2d::PortalBody>(OWNER, "portal.body");
    app.rollback_component_clone::<ambition_portal2d::PortalPolicy>(OWNER, "portal.policy");
    app.rollback_component_clone::<ambition_portal2d::PortalTransit>(OWNER, "portal.transit");
    app.rollback_component_clone::<ambition_portal2d::PlacedPortal>(OWNER, "portal.placed");
    app.rollback_component_clone_probed::<ambition_portal2d::PortalTransitCooldown>(
        OWNER,
        "portal.transit_cooldown",
        |cooldown| cooldown.remaining.to_bits() as u64,
    );
    app.rollback_component_clone::<ambition_portal2d::PortalGunPickup>(OWNER, "portal.gun_pickup");
    app.rollback_component_clone::<ambition_portal2d::PortalEmission>(OWNER, "portal.emission");
    app.rollback_component_clone::<ambition_portal2d::PortalShot>(OWNER, "portal.shot");
    app.rollback_component_clone::<ambition_portal2d::PortalGun>(OWNER, "portal.gun");
    app.declare_rollback_derived_component::<ambition_portal2d::PortalTransitable>(
        OWNER,
        "derived.portal_transitable",
        "mirrored from the item's authoritative body every frame, before transit reads it",
    );
    app.declare_rollback_derived_resource::<ambition_portal2d::PlayerMovementIntent>(
        OWNER,
        "derived.portal_player_movement_intent",
        "republished from the current controller frame before portal transit",
    );
    app.declare_rollback_derived_resource::<ambition_portal2d::PortalCarves>(
        OWNER,
        "derived.portal_carves",
        "rebuilt from placed portals and transit occupancy each frame",
    );
    app.declare_rollback_derived_resource::<ambition_portal2d::PortalHostDepths>(
        OWNER,
        "derived.portal_host_depths",
        "republished from the authoritative collision world each frame",
    );
    app.clear_message_on_rollback::<ambition_portal2d::ClearPortals>(OWNER, "message.clear_portals");
    app.clear_message_on_rollback::<ambition_portal2d::DropPortalGun>(
        OWNER,
        "message.drop_portal_gun",
    );
    app.clear_message_on_rollback::<ambition_portal2d::FirePortalGun>(
        OWNER,
        "message.fire_portal_gun",
    );
    app.clear_message_on_rollback::<ambition_portal2d::PickUpPortalGun>(
        OWNER,
        "message.pick_up_portal_gun",
    );
    app.clear_message_on_rollback::<ambition_portal2d::PortalBodyEntered>(
        OWNER,
        "message.portal_body_entered",
    );
    app.clear_message_on_rollback::<ambition_portal2d::PortalFireIntent>(
        OWNER,
        "message.portal_fire_intent",
    );
    app.clear_message_on_rollback::<ambition_portal2d::PortalGunEquipped>(
        OWNER,
        "message.portal_gun_equipped",
    );
    app.clear_message_on_rollback::<ambition_portal2d::PortalShotFired>(
        OWNER,
        "message.portal_shot_fired",
    );
    app.clear_message_on_rollback::<ambition_portal2d::TogglePortalGun>(
        OWNER,
        "message.toggle_portal_gun",
    );
    app.clear_message_on_rollback::<ambition_portal2d::BodyTeleported>(
        OWNER,
        "message.body_teleported",
    );
    app.clear_message_on_rollback::<ambition_portal2d::PortalBodyTransited>(
        OWNER,
        "message.portal_body_transited",
    );
    app.clear_message_on_rollback::<ambition_portal2d::ClearPortals>(OWNER, "message.portal_clear");
    app.clear_message_on_rollback::<ambition_portal2d::DropPortalGun>(
        OWNER,
        "message.portal_gun_drop",
    );
    app.clear_message_on_rollback::<ambition_portal2d::FirePortalGun>(
        OWNER,
        "message.portal_gun_fire",
    );
    app.clear_message_on_rollback::<ambition_portal2d::PickUpPortalGun>(
        OWNER,
        "message.portal_gun_pick_up",
    );
    app.clear_message_on_rollback::<ambition_portal2d::PortalBodyEntered>(
        OWNER,
        "message.portal_body_entered",
    );
    app.clear_message_on_rollback::<ambition_portal2d::PortalFireIntent>(
        OWNER,
        "message.portal_fire_intent",
    );
    app.clear_message_on_rollback::<ambition_portal2d::PortalGunEquipped>(
        OWNER,
        "message.portal_gun_equipped",
    );
    app.clear_message_on_rollback::<ambition_portal2d::PortalShotFired>(
        OWNER,
        "message.portal_shot_fired",
    );
    app.clear_message_on_rollback::<ambition_portal2d::TogglePortalGun>(
        OWNER,
        "message.portal_gun_toggle",
    );
    app.clear_message_on_rollback::<ambition_portal2d::BodyTeleported>(
        OWNER,
        "message.body_teleported",
    );
    app.clear_message_on_rollback::<ambition_portal2d::PortalBodyTransited>(
        OWNER,
        "message.portal_body_transited",
    );
}
