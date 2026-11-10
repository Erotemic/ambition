//! Portal simulation assembly and schedule placement.
//!
//! Required ordering:
//! - Pose (hosted apertures, links) after the moving platforms advance, then Carves, both
//!   before bodies integrate.
//! - Input warp runs after interaction input and before the primary frame is committed to
//!   `SlotControls`.
//! - Frame (link resolution, straddler eviction) is a pose-dependent carry: it
//!   runs after the other carries and before hazards read the travelled path.
//! - Transit runs after body and ground-item integration so current-frame positions cross,
//!   and after the path's contacts, because a crossing may collapse the path.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::schedule::gameplay_allowed;
use ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use ambition_portal2d::PortalSet;

/// Adds portal simulation and places its sets in the simulation schedule. Part of
/// [`crate::PlatformerEnginePlugins`] when the `portal` feature is on.
pub struct PortalSchedulePlugin;

impl Plugin for PortalSchedulePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.add_plugins(ambition_portal2d::PortalSimulationPlugin);
        // The consequences of a transit for the body a seat drives.
        app.add_message::<ambition_platformer2d_actor_monolith::avatar::trail::TrailContinuityBreak>();
        app.add_systems(
            sim,
            (
                crate::portal_seat::guard_the_driven_body_after_portal_transit
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated),
            )
                .in_set(PortalSet::Transited),
        );
        app.add_systems(sim, crate::portal_seat::warp_portal_input.in_set(PortalSet::InputWarp));
        // Pose: the hosts have moved this tick (the platforms advance first),
        // and no body has integrated against them yet. Each hosted aperture
        // takes its host's pose here, and the carves are cut from that pose,
        // so the hole a body integrates into, the plane it crosses and the
        // host are one pose.
        app.configure_sets(
            sim,
            PortalSet::Pose
                .in_set(ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate)
                .after(ambition_platformer2d_shared_tangle::schedule::MovingPlatformsAdvanced)
                .run_if(gameplay_allowed),
        );
        app.configure_sets(
            sim,
            PortalSet::Carves
                .in_set(ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate)
                .after(PortalSet::Pose)
                .after(ambition_platformer2d_shared_tangle::gravity::GravityZonesCollected),
        );

        // InputWarp: input rewrite in the player-input phase, after
        // interaction input and before the finalized primary frame is published
        // to SlotControls (the Move-axis-fix window), gated to gameplay.
        app.configure_sets(
            sim,
            PortalSet::InputWarp
                .in_set(Platformer2dSimulationPhase::PlayerInput)
                .after(ambition_platformer2d_actor_monolith::control::InteractionInputBuffered)
                .before(ambition_characters::control::PrimarySlotInputCommit)
                .run_if(gameplay_allowed),
        );

        // Weapon maintenance stays ungated for orphan cleanup / roll
        // readiness.
        app.configure_sets(
            sim,
            PortalSet::WeaponAndProjectiles
                .in_set(Platformer2dSimulationPhase::PlayerSimulation)
                .run_if(gameplay_allowed),
        );
        app.configure_sets(
            sim,
            PortalSet::WeaponMaintenance
                .in_set(Platformer2dSimulationPhase::PlayerSimulation),
        );

        // RoomReset: reset-time portal cleanup in the room-transition phase,
        // after the content layer's room-reset work (e.g. a boss-arena reset).
        app.configure_sets(
            sim,
            PortalSet::RoomReset
                .in_set(Platformer2dSimulationPhase::RoomTransition)
                .after(ambition_platformer2d_shared_tangle::lifecycle::ContentRoomResetSet),
        );

        // TransitGuards: suppress ledge-grab while transiting, BEFORE the
        // unified body integration reads it. Movement lives in `WorldPrep`
        // (`integrate_sim_bodies`), so the guard runs there too, ahead of it.
        // Gated to gameplay.
        app.configure_sets(
            sim,
            PortalSet::TransitGuards
                // The PHASE, not the movement system's name.
                .in_set(
                    ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate,
                )
                .run_if(gameplay_allowed),
        );

        // Frame: eviction shoves a straddler off a moved or closed plane, which
        // is travel — so it settles before hazard contacts read the path. It
        // TESTS the pose (is the box straddling?), so it follows every carry
        // rather than racing them. Same host edges as Transit, which follows it.
        app.configure_sets(
            sim,
            PortalSet::Frame
                .in_set(Platformer2dSimulationPhase::PlayerSimulation)
                .after(ambition_platformer2d_shared_tangle::schedule::ItemPickupSet::CoreHeldItems)
                .in_set(ambition_platformer2d_shared_tangle::schedule::BodyPathSet::Constrain)
                .run_if(gameplay_allowed),
        );

        // Passages: where each pair and each body in transit is this tick, for
        // combat to cut its volumes by. After the transit, before combat.
        app.configure_sets(
            sim,
            PortalSet::Passages.in_set(Platformer2dSimulationPhase::PlayerSimulation),
        );

        // Transit: teleports run after body + ground-item integration so this
        // frame's integrated body positions are what cross the portal. Body
        // integration completes in `WorldPrep`; `PlayerSimulation` runs after
        // it, so membership + the CoreHeldItems edge are enough. Gated to
        // gameplay.
        app.configure_sets(
            sim,
            PortalSet::Transit
                .in_set(Platformer2dSimulationPhase::PlayerSimulation)
                .after(ambition_platformer2d_shared_tangle::schedule::ItemPickupSet::CoreHeldItems)
                // Portal CCD reads the settled path, after every carry.
                .in_set(ambition_platformer2d_shared_tangle::schedule::BodyPathSet::Crossing)
                .run_if(gameplay_allowed),
        );

        // The sets that act on portals sleep in rooms whose game uses none
        // (`Capability::Portals`). The publishers (`Carves`, `Passages`), the
        // orphan cleanup and the reset are not in it: they write an empty
        // answer there, not the last one of a room with portals.
        {
            use ambition_platformer2d_shared_tangle::capability::{Capability, CapabilitySet};
            app.configure_sets(
                sim,
                (
                    PortalSet::Pose,
                    PortalSet::InputAdapter,
                    PortalSet::InputWarp,
                    PortalSet::WeaponAndProjectiles,
                    PortalSet::TransitGuards,
                    PortalSet::Frame,
                    PortalSet::Transit,
                )
                    .in_set(CapabilitySet(Capability::Portals)),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PortalSchedulePlugin;
    use ambition_platformer2d_shared_tangle::construction::ConstructionSchemaCatalog;

    #[test]
    fn simulation_only_has_no_gun_schema_and_the_gun_can_be_selected() {
        let mut app = bevy::prelude::App::new();
        app.add_plugins(PortalSchedulePlugin);
        assert!(app.world().get_resource::<ConstructionSchemaCatalog>().is_none_or(|catalog|
            !catalog.contains_domain(ambition_portal2d::PORTAL_GUN_CONSTRUCTION_DOMAIN)));
        app.add_plugins(ambition_portal2d::PortalGunPlugin);
        let catalog = app.world().resource::<ConstructionSchemaCatalog>();
        assert!(catalog.contains_domain(ambition_portal2d::PORTAL_GUN_CONSTRUCTION_DOMAIN));
        let dump = catalog.deterministic_dump();
        let mut other = bevy::prelude::App::new();
        other.add_plugins((PortalSchedulePlugin, ambition_portal2d::PortalGunPlugin));
        assert_eq!(dump, other.world().resource::<ConstructionSchemaCatalog>().deterministic_dump());
    }
}
