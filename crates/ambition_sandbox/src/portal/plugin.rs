use bevy::prelude::*;

use super::messages::{
    ClearPortals, DropPortalGun, FirePortalGun, PickUpPortalGun, PortalFireIntent,
    PortalGunEquipped, TogglePortalGun,
};
use super::schedule::PortalSet;
use super::{
    clear_portals_on_reset, despawn_orphaned_portals, portal_fire_system,
    portal_teleport_ground_items, portal_toggle_system, portal_transit, publish_portal_carves,
    tick_portal_cooldowns, BodyTeleported, PlayerMovementIntent, PortalBodyTransited, PortalCarves,
};
use crate::platformer_runtime::orientation::{ensure_actor_roll, update_actor_roll};

/// Top-level portal mechanic plugin.
///
/// This is the public plugin app assembly should install. It currently delegates
/// to the simulation plugin, leaving room for future render, authoring, and
/// debug adapters to become independent subplugins.
pub struct PortalPlugin;

impl Plugin for PortalPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PortalSimulationPlugin);
    }
}

/// Module-local plugin for portal simulation systems and resources.
///
/// This keeps portal-owned scheduling with the portal mechanic instead of
/// growing `app/plugins.rs` as a central registry. App assembly still decides
/// whether to install the top-level [`PortalPlugin`].
pub struct PortalSimulationPlugin;

impl Plugin for PortalSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<BodyTeleported>();
        // Emitted by the generic `portal_transit` core on every Transfer; the
        // Ambition player-input adapter reads it to reproduce the player's
        // input/trace bits (BodyTeleported, PortalEmission, PortalInputWarp).
        app.add_message::<PortalBodyTransited>();
        // Reusable portal intent / outcome messages — the Ambition input and
        // inventory adapters (crate::ambition_content::portal) write these; core
        // consumes them, staying content-agnostic.
        app.add_message::<FirePortalGun>();
        // Generic fire intent the core fire system consumes (origin/dir/channel);
        // the Ambition resolver maps `FirePortalGun` → this from the player + gun.
        app.add_message::<PortalFireIntent>();
        app.add_message::<TogglePortalGun>();
        app.add_message::<DropPortalGun>();
        app.add_message::<PickUpPortalGun>();
        // Portal-owned reset signal; the Ambition room-reset adapter emits it
        // from `ResetRoomFeaturesEvent` so core never names that event.
        app.add_message::<ClearPortals>();
        app.add_message::<PortalGunEquipped>();
        // Portal-owned carve output. `publish_portal_carves` writes the aperture
        // geometry here; the Ambition bridge copies it into the host collision
        // overlay each frame (portal core never names `FeatureEcsWorldOverlay`).
        app.init_resource::<PortalCarves>();
        // Content-agnostic movement intent: portal core's transit + input warp
        // read/mutate this instead of the Ambition `ControlFrame`; the content
        // input adapter (`crate::ambition_content::portal`) mirrors it to/from
        // `ControlFrame` each frame.
        app.init_resource::<PlayerMovementIntent>();
        // Held-gun aim hint for the visible-build presentation, populated by the
        // content input adapter; init only with the render feature so portal
        // *simulation* carries no render-only resource. The content input adapter
        // writes it via `Option<ResMut<PortalAimHint>>`, so it no-ops cleanly when
        // the render layer (and thus this resource) is absent.
        #[cfg(feature = "portal_render")]
        app.init_resource::<super::PortalAimHint>();

        // Portal systems are registered `.in_set(PortalSet::X)` with only
        // PORTAL-INTERNAL ordering here. The placement of each `PortalSet` into
        // its `SandboxSet` phase, the cross-set `.after`/`.before` edges against
        // sandbox app-schedule systems, and the `gameplay_allowed` run condition
        // are all declared sandbox-side in `crate::app::wire_portal_schedule`
        // (called right after `add_plugins(PortalPlugin)` in
        // `add_simulation_plugins`). This lets `crate::portal` avoid naming
        // `SandboxSet`, `crate::app::*` systems, `crate::gameplay_allowed`,
        // `crate::items::pickup::ItemPickupSet`, or `crate::ambition_content::*`
        // so it can become a standalone crate. The execution order is identical:
        // the same edges are simply declared from the other side of the seam.

        // PlacedPortal carves are published with the same early-world snapshot
        // cadence as the gravity-zone snapshot (`collect_gravity_zones` before
        // `CoreSimulation`); that cross-set placement is declared sandbox-side.
        app.add_systems(Update, publish_portal_carves.in_set(PortalSet::Carves));

        // The Ambition input warp (`warp_portal_input`) is an INPUT-shaping
        // adapter and lives in `crate::ambition_content::portal::ability_adapter`
        // (registered in `PortalSet::InputWarp` there). Portal core owns only the
        // marker components it sets on a crossing (`PortalInputWarp` /
        // `PortalEmission`).

        // The Ambition input adapter (which translates ControlFrame into portal
        // intents) runs in PortalSet::InputAdapter, ordered before this set, so
        // these consumers see the intents the same frame. The drop consumer
        // lives in the inventory adapter (it touches Ambition item state).
        app.configure_sets(
            Update,
            PortalSet::InputAdapter.before(PortalSet::WeaponAndProjectiles),
        );

        // The gameplay-gated weapon systems. The host gates this set with
        // `gameplay_allowed`; the maintenance set below stays ungated (matching
        // the pre-extraction per-system gating) and chains after it.
        app.configure_sets(
            Update,
            PortalSet::WeaponMaintenance.after(PortalSet::WeaponAndProjectiles),
        );
        // `portal_projectile_step` (the GameWorld-reading shot stepper) moved to
        // the Ambition adapter `crate::ambition_content::portal::portal_projectile_step`
        // (Phase 2 Seam 2): portal core keeps only the pure `step_portal_shot`
        // helper over `SolidWorldQuery`. The adapter is registered
        // `.after(portal_fire_system)` in this same set, preserving the
        // `toggle → fire → step` order.
        app.add_systems(
            Update,
            (portal_toggle_system, portal_fire_system)
                .chain()
                .in_set(PortalSet::WeaponAndProjectiles),
        );
        app.add_systems(
            Update,
            (
                // Portals must not outlive their gun (the "destroyed" case).
                despawn_orphaned_portals,
                // Make sure the player can carry an aerial roll through portals.
                ensure_actor_roll,
            )
                .chain()
                .in_set(PortalSet::WeaponMaintenance),
        );

        app.add_systems(Update, clear_portals_on_reset.in_set(PortalSet::RoomReset));

        // Ledge-grab suppression while transiting (it mutates the PLAYER's
        // `PlayerAbilities`) is an Ambition ability adapter and lives in
        // `crate::ambition_content::portal::ability_adapter` (registered in
        // `PortalSet::TransitGuards` there). Portal core owns only the
        // `PortalTransit` latch it reads off.

        // Teleports run after player and ground-item integration so this frame's
        // integrated body positions are what cross the portal.
        app.add_systems(
            Update,
            (
                tick_portal_cooldowns,
                portal_transit,
                portal_teleport_ground_items,
                update_actor_roll,
            )
                .chain()
                .in_set(PortalSet::Transit),
        );
    }
}
