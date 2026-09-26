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
/// Portal simulation plus the portal gun. Consumers that want portals without
/// a gun install [`PortalSimulationPlugin`] only.
pub struct PortalPlugin;

impl Plugin for PortalPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((PortalSimulationPlugin, PortalGunPlugin));
    }
}

/// Optional portal-gun opener layered over the reusable portal simulation.
///
/// Owns only gun vocabulary and gun lifetime. It reads no host input; the
/// Ambition inventory and input adapters are above this crate. The shot path
/// is in [`PortalSimulationPlugin`] and consumes [`PortalFireIntent`], so other
/// emitters do not need a gun.
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

        // Construction metadata is data only. The constructor is the closed
        // `PortalGunConstruction` dispatch, never selected from this catalog.
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

        // Toggle is gun policy; fire is in the simulation plugin. The explicit
        // edge keeps the order without portal core knowing about the gun.
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
/// Keeps portal scheduling with the portal mechanic. App assembly decides
/// whether to install [`PortalPlugin`].
pub struct PortalSimulationPlugin;

impl Plugin for PortalSimulationPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.add_message::<BodyTeleported>();
        // Emitted by `portal_transit` on every Transfer. Host input adapters
        // read it for BodyTeleported, PortalEmission, and PortalInputWarp.
        app.add_message::<PortalBodyTransited>();
        // Fire intent (origin, dir, channel). A host maps a gun, script, AI,
        // or moving emitter into it.
        app.add_message::<PortalFireIntent>();
        // Reset signal; the host room-reset adapter emits it.
        app.add_message::<ClearPortals>();
        // Audio signals (not sfx) for fire and aperture entry; a host audio
        // adapter maps them to sfx. The exit cue uses `PortalBodyTransited`.
        app.add_message::<PortalShotFired>();
        app.add_message::<PortalBodyEntered>();
        // `publish_portal_carves` writes here; the host bridge copies it into
        // the host collision overlay each frame.
        app.init_resource::<PortalCarves>();
        app.init_resource::<crate::PortalHostDepths>();
        app.init_resource::<PortalTuning>();
        // The inspector panel edits `EditablePortalTuning`; the sim reads
        // `PortalTuning` (in `GgrsSchedule` under rollback). Edits go through
        // the shared mechanical-edit protocol: propose, admit, publish, all in
        // `PreUpdate` before `RunGgrsSystems`, so a replay does not see live
        // panel values (`Q120`).
        app.init_resource::<crate::tuning::EditablePortalTuning>();
        app.init_resource::<ambition_platformer2d_core::PendingMechanicalEdits>();
        app.init_resource::<ambition_platformer2d_core::MechanicalEditAdmission>();
        ambition_platformer2d_shared_tangle::schedule::configure_mechanical_edit_sets(app);
        app.add_systems(
            bevy::app::PreUpdate,
            (
                crate::tuning::propose_editable_portal_tuning
                    .in_set(ambition_platformer2d_core::MechanicalEditSet::Propose),
                crate::tuning::publish_editable_portal_tuning
                    .in_set(ambition_platformer2d_core::MechanicalEditSet::Publish),
            ),
        );
        // The aim hint (`PortalAimHint`) is render-only and initialised by the
        // host presentation layer.

        // Only portal-internal ordering is declared here. The host places each
        // [`PortalSet`] in its phases, adds edges against host systems, and adds
        // run conditions (e.g. "gameplay allowed"), right after
        // `add_plugins(PortalPlugin)`.

        // Published at the same point as the gravity-zone snapshot (before
        // `CoreSimulation`); the host declares that placement.
        app.add_systems(sim, publish_portal_carves.in_set(PortalSet::Carves));

        // The input warp (`warp_portal_input`) is in the host portal adapter
        // (`PortalSet::InputWarp`). Portal core owns only the marker components
        // (`PortalInputWarp`, `PortalEmission`).

        // The drop consumer lives in the inventory adapter while it touches host item state.
        app.configure_sets(
            sim,
            PortalSet::InputAdapter.before(PortalSet::WeaponAndProjectiles),
        );
        // The host gates this weapon set with `gameplay_allowed`. The
        // maintenance set below is not gated and runs after it.
        app.configure_sets(
            sim,
            PortalSet::WeaponMaintenance.after(PortalSet::WeaponAndProjectiles),
        );
        // Host adapters run their shot stepper after `portal_fire_system`,
        // using the pure `step_portal_shot`.
        app.add_systems(
            sim,
            portal_fire_system.in_set(PortalSet::WeaponAndProjectiles),
        );

        app.add_systems(sim, clear_portals_on_reset.in_set(PortalSet::RoomReset));

        // Ledge-grab suppression during transit is a host adapter in
        // `PortalSet::TransitGuards`; it reads the `PortalTransit` latch.

        // Teleports run after actor and ground-item integration so this frame's
        // integrated body positions are what cross the portal.
        app.init_resource::<crate::PortalFrameHistory>();
        app.configure_sets(sim, PortalSet::Frame.before(PortalSet::Transit));
        app.add_systems(
            sim,
            (
                // Link ids to channel pairs, then equalize each pair's opening.
                // First, so transit, carve, and eviction see the results.
                crate::resolve_portal_links.in_set(crate::PortalLinkResolution),
                crate::equalize_pair_apertures,
                // A portal that moved or closed under a straddling body pushes
                // it clear, before transit runs.
                crate::evict_straddlers_on_portal_change,
            )
                .chain()
                .in_set(PortalSet::Frame),
        );
        app.add_systems(
            sim,
            (
                tick_portal_cooldowns,
                portal_transit,
                crate::reconcile_transited_bodies,
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
