//! Portal simulation assembly (E5 step 5, behind the `portal` feature):
//! [`ambition_portal2d::PortalPlugin`] plus the schedule
//! placement for portal's internal sets — each mapped to its sandbox phase,
//! cross-set ordering edge, and gameplay run condition.
//!
//! ⚠ ORDERING LANDMINES (the feel/correctness contract; moved verbatim from
//! `ambition_app::app::plugins::wire_portal_schedule`). The portal-continuity,
//! gravity-room, and projectile-transit app suites are the parity harness —
//! any break here goes RED there, not silently wrong:
//! - **Carves** publish after gravity-zone collection, before core simulation.
//! - **InputWarp** rewrites input after `interaction_input_system` and before
//!   the primary `ControlFrame` is committed to `SlotControls` (the Move-axis-fix window).
//! - **Transit** teleports after body + ground-item integration so THIS
//!   frame's integrated positions are what cross the portal.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::schedule::gameplay_allowed;
use ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use ambition_portal2d::PortalSet;

/// Adds `PortalPlugin` and places its sets in the sandbox schedule. Part of
/// [`crate::PlatformerEnginePlugins`] when the `portal` feature is on.
pub struct PortalSchedulePlugin;

impl Plugin for PortalSchedulePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.add_plugins(ambition_portal2d::PortalPlugin);

        // Carves publish after gravity-zone collection and before core
        // simulation.
        app.configure_sets(
            sim,
            PortalSet::Carves
                .in_set(ambition_platformer2d_shared_tangle::schedule::GameplaySimulationRoot)
                .after(ambition_platformer2d_shared_tangle::gravity::GravityZonesCollected)
                .before(Platformer2dSimulationPhaseMonolith::CoreSimulation),
        );

        // InputWarp: input rewrite in the player-input phase, after
        // interaction input and before the finalized primary frame is published
        // to SlotControls (the Move-axis-fix window), gated to gameplay.
        app.configure_sets(
            sim,
            PortalSet::InputWarp
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerInput)
                .after(ambition_platformer2d_actor_monolith::control::InteractionInputBuffered)
                .before(ambition_platformer2d_actor_monolith::control::PrimarySlotInputCommit)
                .run_if(gameplay_allowed),
        );

        // Weapon maintenance stays ungated for orphan cleanup / roll
        // readiness.
        app.configure_sets(
            sim,
            PortalSet::WeaponAndProjectiles
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerSimulation)
                .run_if(gameplay_allowed),
        );
        app.configure_sets(
            sim,
            PortalSet::WeaponMaintenance
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerSimulation),
        );

        // RoomReset: reset-time portal cleanup in the room-transition phase,
        // after the content layer's room-reset work (e.g. a boss-arena reset).
        app.configure_sets(
            sim,
            PortalSet::RoomReset
                .in_set(Platformer2dSimulationPhaseMonolith::RoomTransition)
                .after(ambition_platformer2d_actor_monolith::session::reset::ContentRoomResetSet),
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

        // Transit: teleports run after body + ground-item integration so this
        // frame's integrated body positions are what cross the portal. Body
        // integration completes in `WorldPrep`; `PlayerSimulation` runs after
        // it, so membership + the CoreHeldItems edge are enough. Gated to
        // gameplay.
        app.configure_sets(
            sim,
            PortalSet::Transit
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerSimulation)
                .after(ambition_platformer2d_actor_monolith::items::pickup::ItemPickupSet::CoreHeldItems)
                .run_if(gameplay_allowed),
        );
    }
}
