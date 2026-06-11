use bevy::prelude::*;

use super::messages::{
    ClearPortals, DropPortalGun, FirePortalGun, PickUpPortalGun, PortalBodyEntered,
    PortalFireIntent, PortalGunEquipped, PortalShotFired, TogglePortalGun,
};
use super::schedule::PortalSet;
use super::{
    clear_portals_on_reset, despawn_orphaned_portals, portal_fire_system,
    portal_teleport_ground_items, portal_toggle_system, portal_transit, publish_portal_carves,
    sync_portal_tuning_convention, tick_portal_cooldowns, BodyTeleported, PlayerMovementIntent,
    PortalBodyTransited, PortalCarves, PortalTuning,
};
use ambition_platformer_runtime::orientation::{ensure_actor_roll, update_actor_roll};

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
        // Portal-owned audio SIGNALS (not sfx): the crate emits these on a fire /
        // aperture entry; an Ambition audio adapter
        // (the host portal adapter) maps them to the
        // sfx vocabulary. The EXIT cue rides `PortalBodyTransited` (`exit_pos`).
        app.add_message::<PortalShotFired>();
        app.add_message::<PortalBodyEntered>();
        // Portal-owned carve output. `publish_portal_carves` writes the aperture
        // geometry here; the Ambition bridge copies it into the host collision
        // overlay each frame (portal core never names `FeatureEcsWorldOverlay`).
        app.init_resource::<PortalCarves>();
        // Content-agnostic movement intent: portal core's transit + input warp
        // read/mutate this instead of the Ambition `ControlFrame`; the content
        // input adapter (the host portal adapter) mirrors it to/from
        // `ControlFrame` each frame.
        app.init_resource::<PlayerMovementIntent>();
        app.init_resource::<PortalTuning>();
        // NOTE: the held-gun aim hint (`PortalAimHint`) is a render-only resource
        // owned by the HOST presentation layer (it is not part of the headless
        // mechanic), so it is initialised host-side behind the render feature, not
        // here. The portal *simulation* carries no render-only resource.

        // Portal systems are registered `.in_set(PortalSet::X)` with only
        // PORTAL-INTERNAL ordering here. The placement of each [`PortalSet`] into
        // the host's app phases, the cross-set `.after`/`.before` edges against
        // host systems, and any run condition (e.g. "gameplay allowed") are all
        // declared HOST-SIDE (the host wires the portal schedule right after
        // `add_plugins(PortalPlugin)`). This keeps the crate free of host schedule
        // labels / systems / run conditions so it stays standalone; the execution
        // order is identical — the same edges are simply declared from the other
        // side of the seam.

        // PlacedPortal carves are published with the same early-world snapshot
        // cadence as the gravity-zone snapshot (`collect_gravity_zones` before
        // `CoreSimulation`); that cross-set placement is declared sandbox-side.
        app.add_systems(Update, publish_portal_carves.in_set(PortalSet::Carves));

        // The Ambition input warp (`warp_portal_input`) is an INPUT-shaping
        // adapter and lives in the host portal adapter
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
        app.add_systems(
            Update,
            sync_portal_tuning_convention.in_set(PortalSet::InputAdapter),
        );

        // The gameplay-gated weapon systems. The host gates this set with
        // `gameplay_allowed`; the maintenance set below stays ungated (matching
        // the pre-extraction per-system gating) and chains after it.
        app.configure_sets(
            Update,
            PortalSet::WeaponMaintenance.after(PortalSet::WeaponAndProjectiles),
        );
        // `portal_projectile_step` (the GameWorld-reading shot stepper) moved to
        // the Ambition adapter the host portal adapter
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
        // the host portal adapter (registered in
        // `PortalSet::TransitGuards` there). Portal core owns only the
        // `PortalTransit` latch it reads off.

        // Teleports run after player and ground-item integration so this frame's
        // integrated body positions are what cross the portal.
        app.init_resource::<crate::PortalFrameHistory>();
        app.add_systems(
            Update,
            (
                // Explicit link-id authoring → channel pairs, then shrink each
                // pair's opening to the MIN (centered, no scaling). First, so
                // transit/carve/eviction see resolved channels + equalized
                // apertures this frame.
                crate::resolve_portal_links,
                crate::equalize_pair_apertures,
                // JON'S RULE: AVOID PUSHOUT — the ONE exception: a portal that
                // moved/closed under a straddler shoves it clear (vs ripping it
                // in half). Runs first so transit never acts on a body the
                // closing plane already evicted.
                crate::evict_straddlers_on_portal_change,
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
