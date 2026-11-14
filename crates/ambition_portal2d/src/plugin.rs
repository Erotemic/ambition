//! Portal plugin assembly: reusable simulation plus optional portal generators.
//! Render, authoring, and debug stay in downstream adapters.

use bevy::prelude::*;

use super::messages::{ClearPortals, PortalBodyEntered, PortalFireIntent, PortalShotFired};
use super::schedule::PortalSet;
use super::{
    clear_portals_on_reset, portal_fire_system, portal_transit,
    publish_portal_carves, tick_portal_cooldowns, BodyTeleported, PortalBodyTransited,
    PortalCarves, PortalTuning,
};
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;

/// Backward-compatible full portal composition.
///
/// Portal simulation plus generators. Consumers that want portals with no
/// generator install [`PortalSimulationPlugin`] only.
pub struct PortalPlugin;

impl Plugin for PortalPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((PortalSimulationPlugin, PortalGeneratorPlugin));
    }
}

/// Portal generators layered over the reusable portal simulation: the
/// generator's own gestures (fire, toggle) and its portals' lifetime.
///
/// Reads no host input and knows nothing of what a generator is; a host
/// gives one to whatever opens portals. The shot path is in
/// [`PortalSimulationPlugin`] and consumes [`PortalFireIntent`], so other
/// emitters do not need a generator.
pub struct PortalGeneratorPlugin;

impl Plugin for PortalGeneratorPlugin {
    fn build(&self, app: &mut App) {
        use super::messages::{FirePortalGenerator, TogglePortalGenerator, TogglePortalGeneratorsActive};

        let sim = app.sim_schedule();
        app.add_message::<FirePortalGenerator>();
        app.add_message::<TogglePortalGenerator>();
        app.add_message::<TogglePortalGeneratorsActive>();

        // The toggles run before the fire, so a press that toggles and a press
        // that fires on one tick fire the new end.
        app.add_systems(
            sim,
            (
                super::portal_generators_active_toggle_system
                    .run_if(ambition_platformer2d_core::a_message_waits::<TogglePortalGeneratorsActive>),
                super::portal_toggle_system.run_if(ambition_platformer2d_core::a_message_waits::<TogglePortalGenerator>),
            )
                .chain()
                .before(super::portal_fire_system)
                .in_set(PortalSet::WeaponAndProjectiles),
        );
        app.add_systems(
            sim,
            super::despawn_orphaned_portals
                .run_if(super::generator_lifecycle::a_portal_or_shot_exists)
                .in_set(PortalSet::WeaponMaintenance),
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
        app.init_resource::<crate::PortalHostDepthsByRoom>();
        app.init_resource::<PortalTuning>();
        crate::observation::install(app);
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
        // Only portal-internal ordering is declared here. The host places each
        // [`PortalSet`] in its phases, adds edges against host systems, and adds
        // run conditions (e.g. "gameplay allowed"), right after
        // `add_plugins(PortalPlugin)`.

        // Published at the same point as the gravity-zone snapshot (before
        // `CoreSimulation`); the host declares that placement.
        app.add_systems(
            sim,
            publish_portal_carves
                .run_if(crate::transit::portals_or_their_carves)
                .in_set(PortalSet::Carves),
        );

        // The input warp (`warp_portal_input`) is the host runtime's
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
            portal_fire_system
                .run_if(ambition_platformer2d_core::a_message_waits::<crate::PortalFireIntent>)
                .in_set(PortalSet::WeaponAndProjectiles),
        );

        app.add_systems(
            sim,
            clear_portals_on_reset
                .run_if(ambition_platformer2d_core::a_message_waits::<crate::ClearPortals>)
                .in_set(PortalSet::RoomReset),
        );

        // Ledge-grab suppression during transit is a host adapter in
        // `PortalSet::TransitGuards`; it reads the `PortalTransit` latch.

        // Teleports run after actor and ground-item integration so this frame's
        // integrated body positions are what cross the portal.
        app.init_resource::<crate::PortalFrameHistory>();
        // A composition with no session lifecycle has no activation. The
        // reader then reads an empty channel.
        app.add_message::<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeActivated>();
        app.add_systems(
            Update,
            crate::forget_portal_frames_on_activation
                .run_if(ambition_platformer2d_core::a_message_waits::<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeActivated>)
                .in_set(ambition_platformer2d_shared_tangle::lifecycle::SessionScopeSet::Activate),
        );
        app.configure_sets(sim, PortalSet::Pose.before(PortalSet::Carves));
        app.configure_sets(sim, PortalSet::Frame.before(PortalSet::Transit));
        app.init_resource::<ambition_platformer2d_shared_tangle::body::geometry::BodyGeometry>();
        app.add_systems(sim, (
            ambition_platformer2d_shared_tangle::body::geometry::publish_body_geometry_before_motion,
            crate::publish_portal_apertures,
        ).chain().before(crate::publish_portal_carves).in_set(PortalSet::Carves));
        app.configure_sets(sim, PortalSet::Passages.after(PortalSet::Transit));
        app.add_systems(sim, (
            ambition_platformer2d_shared_tangle::body::geometry::publish_body_geometry_after_motion,
            crate::passages::publish_portal_passages,
        ).chain().in_set(PortalSet::Passages));
        app.configure_sets(
            sim,
            PortalSet::Transited
                .in_set(PortalSet::Transit)
                .after(portal_transit),
        );
        app.add_systems(
            sim,
            (
                // Resolve link ids before the pair geometry is read.
                // with the tick's pose: the carve, eviction and transit see the
                // results.
                crate::resolve_portal_links.in_set(crate::PortalLinkResolution),
            )
                .chain()
                // Both visit placed portals only.
                .run_if(any_with_component::<crate::PlacedPortal>)
                .in_set(PortalSet::Pose),
        );
        // A portal that moved or closed under a straddling body pushes it
        // clear, before transit runs.
        app.add_systems(
            sim,
            crate::evict_straddlers_on_portal_change
                .run_if(crate::portals_or_their_frames)
                .in_set(crate::PortalHistoryAdvance)
                .in_set(PortalSet::Frame),
        );
        // A body that straddles an opening with part of it past an end is
        // held by the host there: it goes through whole or not at all.
        app.add_systems(
            sim,
            crate::snag_straddlers
                .run_if(any_with_component::<crate::PlacedPortal>)
                .after(crate::PortalHistoryAdvance)
                .in_set(PortalSet::Frame),
        );
        app.add_systems(
            sim,
            (
                tick_portal_cooldowns.run_if(any_with_component::<crate::PortalTransitCooldown>),
                portal_transit.run_if(crate::transit::portals_or_transits),
                crate::reconcile_transited_bodies.run_if(ambition_platformer2d_core::a_message_waits::<crate::PortalBodyTransited>),
            )
                .chain()
                .in_set(PortalSet::Transit),
        );
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::{App, Messages};

    use super::{PortalGeneratorPlugin, PortalPlugin, PortalSimulationPlugin};
    use crate::{PortalFireIntent, TogglePortalGenerator};

    #[test]
    fn simulation_only_portals_install_no_generator_control_channel() {
        let mut app = App::new();
        app.add_plugins(PortalSimulationPlugin);

        assert!(app
            .world()
            .contains_resource::<Messages<PortalFireIntent>>());
        assert!(
            !app.world().contains_resource::<Messages<TogglePortalGenerator>>(),
            "static/scripted portal users must not inherit generator control vocabulary",
        );
    }

    #[test]
    fn the_generator_layer_adds_its_gestures() {
        let mut app = App::new();
        app.add_plugins((PortalSimulationPlugin, PortalGeneratorPlugin));
        assert!(app.world().contains_resource::<Messages<TogglePortalGenerator>>());
    }

    #[test]
    fn compatibility_plugin_still_composes_the_full_portal_experience() {
        let mut app = App::new();
        app.add_plugins(PortalPlugin);
        assert!(app
            .world()
            .contains_resource::<Messages<PortalFireIntent>>());
        assert!(app.world().contains_resource::<Messages<TogglePortalGenerator>>());
    }
}
