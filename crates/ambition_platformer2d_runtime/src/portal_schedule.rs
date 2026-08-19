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

/// **Why the portal-gun CONSTRUCTION LANE and the portal-gun SCHEMA cannot
/// disagree about whether the capability is installed** (2026-08-19 review).
///
/// The two facts have different owners, and that is the thing worth writing
/// down rather than assuming:
///
/// ```text
/// the executable lane   `#[cfg(feature = "portal")]` in the actor monolith's
///                       room planner — a COMPILE-TIME decision. The lane's
///                       registry is built inline and its domain's
///                       `Services = ()`, so nothing it constructs needs a
///                       plugin to have run.
/// the schema entry      `PortalGunPlugin::build` contributes the gun's
///                       registry dump to `ConstructionSchemaCatalog` — a
///                       RUNTIME composition decision, and the only thing
///                       prepared-content fingerprinting sees.
/// ```
///
/// ⚠ **so a composition that compiled `portal` and installed only
/// [`PortalSimulationPlugin`](ambition_portal2d::PortalSimulationPlugin) would
/// fingerprint content as gun-less while its rooms still built authored gun
/// pickups.** `PortalSimulationPlugin` is public and its own doc invites
/// exactly that ("Portal-only consumers may install `PortalSimulationPlugin`
/// directly"), and it cannot be reached without the `portal` feature, because
/// the whole `ambition_portal2d` dependency is optional and that feature is what
/// turns it on. The invitation and the lane are therefore inseparable.
///
/// ⭐ **what actually prevents the divergence is one line in this file**: this
/// plugin installs `PortalPlugin`, which is simulation PLUS gun, and it is the
/// only place in the workspace that installs portal simulation at all
/// (`PlatformerEnginePlugins` adds this plugin, unconditionally, under the same
/// `portal` feature that compiles the lane). No engine composition can express
/// "portal simulation, no portal gun" today.
///
/// ⛔ **that is a coincidence of composition, not a type**, which is why the
/// test below exists instead of an abstraction. Threading a runtime capability
/// token into room planning would mean a seventh authority on
/// `ActorConstructionContext::for_room_construction` and a new parameter on the
/// six systems that call it — one of which already sits at Bevy's 16-parameter
/// ceiling — to defend a state no composition can currently reach. The test is
/// the cheap half: the day somebody swaps this line for `PortalSimulationPlugin`
/// and makes that state reachable, it goes red and says what else must move.
#[cfg(test)]
mod tests {
    use super::PortalSchedulePlugin;
    use ambition_platformer2d_shared_tangle::construction::ConstructionSchemaCatalog;

    /// **One decision, two consequences.** If this app can plan the portal-gun
    /// construction lane — and it can, because this module only compiles under
    /// the same feature — then the schema catalog must already know the domain.
    #[test]
    fn installing_portal_simulation_also_installs_the_gun_capability_it_can_construct() {
        let mut app = bevy::prelude::App::new();
        app.add_plugins(PortalSchedulePlugin);

        let catalog = app
            .world()
            .get_resource::<ConstructionSchemaCatalog>()
            .expect(
                "the portal composition contributed no construction schema at all, so the \
                 assertion below could not have failed",
            );
        assert!(
            catalog.contains_domain(ambition_portal2d::PORTAL_GUN_CONSTRUCTION_DOMAIN),
            "this composition compiles the portal-gun construction lane into room planning \
             (the lane is `#[cfg(feature = \"portal\")]`, and so is this test) but installs \
             no portal-gun capability, so prepared-content fingerprinting reports a world \
             without guns while rooms still build authored gun pickups. If installing \
             `PortalSimulationPlugin` alone is now a composition the engine wants, the lane \
             needs a runtime capability token threaded into \
             `ActorConstructionContext::for_room_construction` — see this module's header."
        );
    }
}
