//! Portal plugin assembly: reusable simulation plus an optional portal-gun opener.
//! Render, authoring, and debug stay in downstream adapters.

use bevy::prelude::*;

use super::messages::{ClearPortals, PortalBodyEntered, PortalFireIntent, PortalShotFired};
use super::schedule::PortalSet;
use super::{
    clear_portals_on_reset, portal_fire_system, portal_teleport_ground_items, portal_transit,
    publish_portal_carves, tick_portal_cooldowns, BodyTeleported, PortalBodyTransited,
    PortalCarves, PortalTuning,
};
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;

/// Backward-compatible full portal composition.
///
/// Existing Ambition hosts install this and keep the historical gun-enabled
/// behavior. Portal-only consumers may install [`PortalSimulationPlugin`]
/// directly and avoid the gun control/custody vocabulary entirely.
pub struct PortalPlugin;

impl Plugin for PortalPlugin {
    fn build(&self, app: &mut App) {
        // Backward-compatible full portal experience: the reusable portal
        // simulation plus the optional gun opener. A game that wants static,
        // scripted, or moving portals without gun vocabulary installs only
        // `PortalSimulationPlugin`.
        app.add_plugins((PortalSimulationPlugin, PortalGunPlugin));
    }
}

/// Optional portal-gun opener layered over the reusable portal simulation.
///
/// This plugin owns only gun vocabulary and gun lifetime policy. It translates
/// no host input itself; Ambition's inventory/input adapters still live above
/// this crate. The generic shot path remains in [`PortalSimulationPlugin`] and
/// consumes [`PortalFireIntent`], so scripts or other emitters do not need a gun.
pub struct PortalGunPlugin;

impl Plugin for PortalGunPlugin {
    fn build(&self, app: &mut App) {
        use super::messages::{
            DropPortalGun, FirePortalGun, PickUpPortalGun, PortalGunEquipped, TogglePortalGun,
        };

        let sim = app.sim_schedule();
        app.add_message::<FirePortalGun>();
        app.add_message::<TogglePortalGun>();
        app.add_message::<DropPortalGun>();
        app.add_message::<PickUpPortalGun>();
        app.add_message::<PortalGunEquipped>();

        // Construction metadata is federated as DATA. The actual constructor
        // remains the closed `PortalGunConstruction` dispatch and is never
        // selected from this catalog.
        app.init_resource::<
            ambition_platformer2d_shared_tangle::construction::ConstructionSchemaCatalog,
        >();
        let registry = super::portal_gun_construction_registry();
        app.world_mut()
            .resource_mut::<
                ambition_platformer2d_shared_tangle::construction::ConstructionSchemaCatalog,
            >()
            .try_contribute(
                super::PORTAL_GUN_CONSTRUCTION_DOMAIN,
                registry.deterministic_dump(),
            )
            .expect("the portal-gun construction schema cannot conflict with itself");

        // Toggle is gun-local policy; generic fire consumes a PortalFireIntent
        // and remains in the simulation plugin. Keeping the explicit edge
        // preserves the old same-set ordering without making portal core know a
        // gun exists.
        app.add_systems(
            sim,
            super::portal_toggle_system
                .before(super::portal_fire_system)
                .in_set(PortalSet::WeaponAndProjectiles),
        );
        app.add_systems(
            sim,
            super::despawn_orphaned_portals.in_set(PortalSet::WeaponMaintenance),
        );
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
        let sim = app.sim_schedule();
        app.add_message::<BodyTeleported>();
        // Emitted by the generic `portal_transit` core on every Transfer; the
        // Host input adapters read it to reproduce the transiting body's
        // input/trace bits (BodyTeleported, PortalEmission, PortalInputWarp).
        app.add_message::<PortalBodyTransited>();
        // Reusable portal intent / outcome messages. Host input/inventory
        // adapters write these; core consumes them, staying content-agnostic.
        // Generic fire intent the core fire system consumes (origin/dir/channel);
        // a host may map a gun gesture, script, AI, or moving emitter into this.
        app.add_message::<PortalFireIntent>();
        // Portal-owned reset signal; the host room-reset adapter emits it so
        // core never names the host reset event.
        app.add_message::<ClearPortals>();
        // Portal-owned audio SIGNALS (not sfx): the crate emits these on a fire /
        // aperture entry; a host audio adapter maps them to the sfx vocabulary. The EXIT cue rides `PortalBodyTransited` (`exit_pos`).
        app.add_message::<PortalShotFired>();
        app.add_message::<PortalBodyEntered>();
        // Portal-owned carve output. `publish_portal_carves` writes the aperture
        // geometry here; the host bridge copies it into the host collision
        // overlay each frame (portal core never names the concrete overlay).
        app.init_resource::<PortalCarves>();
        app.init_resource::<crate::PortalHostDepths>();
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
        app.add_systems(sim, publish_portal_carves.in_set(PortalSet::Carves));

        // The host input warp (`warp_portal_input`) is an INPUT-shaping adapter
        // and lives in the host portal adapter
        // (registered in `PortalSet::InputWarp` there). Portal core owns only the
        // marker components it sets on a crossing (`PortalInputWarp` /
        // `PortalEmission`).

        // The drop consumer lives in the inventory adapter while it touches host item state.
        app.configure_sets(
            sim,
            PortalSet::InputAdapter.before(PortalSet::WeaponAndProjectiles),
        );
        // The gameplay-gated weapon systems. The host gates this set with
        // `gameplay_allowed`; the maintenance set below stays ungated (matching
        // the pre-extraction per-system gating) and chains after it.
        app.configure_sets(
            sim,
            PortalSet::WeaponMaintenance.after(PortalSet::WeaponAndProjectiles),
        );
        // Host adapters run their world-reading shot stepper after
        // `portal_fire_system`; core keeps only the pure `step_portal_shot`
        // helper over `SolidWorldQuery`.
        app.add_systems(
            sim,
            portal_fire_system.in_set(PortalSet::WeaponAndProjectiles),
        );

        app.add_systems(sim, clear_portals_on_reset.in_set(PortalSet::RoomReset));

        // Ledge-grab suppression while transiting mutates host ability state, so
        // it remains a host ability adapter registered in `PortalSet::TransitGuards`.
        // Portal core owns only the `PortalTransit` latch it reads off.

        // Teleports run after actor and ground-item integration so this frame's
        // integrated body positions are what cross the portal.
        app.init_resource::<crate::PortalFrameHistory>();
        app.add_systems(
            sim,
            (
                // Explicit link-id authoring → channel pairs, then shrink each
                // pair's opening to the MIN (centered, no scaling). First, so
                // transit/carve/eviction see resolved channels + equalized
                // apertures this frame.
                crate::resolve_portal_links.in_set(crate::PortalLinkResolution),
                crate::equalize_pair_apertures,
                // moved/closed under a straddler shoves it clear (vs ripping it
                // in half). Runs first so transit never acts on a body the
                // closing plane already evicted.
                crate::evict_straddlers_on_portal_change,
                tick_portal_cooldowns,
                portal_transit,
                portal_teleport_ground_items,
            )
                .chain()
                .in_set(PortalSet::Transit),
        );
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::{App, Messages};

    use ambition_platformer2d_shared_tangle::construction::ConstructionSchemaCatalog;

    use super::{PortalGunPlugin, PortalPlugin, PortalSimulationPlugin};
    use crate::{PortalFireIntent, TogglePortalGun, PORTAL_GUN_CONSTRUCTION_DOMAIN};

    #[test]
    fn simulation_only_portals_install_no_gun_control_channel() {
        let mut app = App::new();
        app.add_plugins(PortalSimulationPlugin);

        assert!(app
            .world()
            .contains_resource::<Messages<PortalFireIntent>>());
        assert!(
            !app.world().contains_resource::<Messages<TogglePortalGun>>(),
            "static/scripted portal users must not inherit portal-gun control vocabulary",
        );
    }

    #[test]
    fn gun_layer_publishes_its_construction_schema() {
        let mut app = App::new();
        app.add_plugins((PortalSimulationPlugin, PortalGunPlugin));

        assert!(app.world().contains_resource::<Messages<TogglePortalGun>>());
        assert!(app
            .world()
            .resource::<ConstructionSchemaCatalog>()
            .contains_domain(PORTAL_GUN_CONSTRUCTION_DOMAIN),);
    }

    #[test]
    fn compatibility_plugin_still_composes_the_full_portal_experience() {
        let mut app = App::new();
        app.add_plugins(PortalPlugin);
        assert!(app
            .world()
            .contains_resource::<Messages<PortalFireIntent>>());
        assert!(app.world().contains_resource::<Messages<TogglePortalGun>>());
    }
}
