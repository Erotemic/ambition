//! Plugin that installs the Ambition portal adapters.
//!
//! Registers the input adapter (ControlFrame → portal intents) and the
//! inventory drop adapter into the portal subsystem's schedule sets so the
//! reusable portal core consumes intents the same frame they are produced. The
//! pickup adapter is registered alongside the held-item simulation (in
//! `ambition_platformer2d_actor_monolith::items::pickup`) because it must run last in that set, after the core
//! fire system, so picking up the gun doesn't also fire on the same press.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::schedule::GameplayGated;
use ambition_portal2d::{
    clear_portals_on_reset, portal_fire_system, portal_teleport_ground_items, portal_transit,
    publish_portal_carves, PortalSet,
};

use super::ability_adapter::{
    restore_wall_abilities_after_transit, suppress_ledge_grab_during_transit, warp_portal_input,
};
use super::carve_adapter::{bridge_portal_carves, sync_portal_host_depths};
use super::fire_adapter::resolve_portal_fire_intent;
use super::input_adapter::portal_input_adapter_system;
use super::inventory_adapter::{drop_portal_gun_system, pickup_portal_gun_system};
use super::reset_adapter::bridge_room_reset_to_clear_portals;
use super::sfx_adapter::play_portal_sfx;
use super::shot_adapter::portal_projectile_step;
use super::transit_adapter::{sync_ground_items_to_transitable, sync_transitable_to_ground_items};
use super::transit_body_adapter::{
    apply_portal_carried_momentum, ensure_portal_bodies, ensure_projectile_portal_bodies,
    portal_player_input_adapter, reconcile_kernel_bodies_after_portal_transit,
    rotate_projectile_acceleration_after_portal_transit, sync_portal_reorient_from_settings,
};
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;

pub fn register_rollback_state(
    registrar: &mut impl ambition_platformer2d_core::snapshot::RollbackRegistrar,
) {
    registrar.rollback_component_clone::<crate::portal::host_adapter::PortalHostScanned>(
        "ambition_content",
        "portal.host_scanned",
    );
}

/// Installs the Ambition-specific portal input/inventory adapters.
pub struct AmbitionPortalAdaptersPlugin;

impl Plugin for AmbitionPortalAdaptersPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // Input-shaping warp: apply the portal-owned `PortalInputWarp` /
        // `PortalEmission` guards to the player's movement intent. INPUT is not a
        // crate concern (Stage 19 Phase 5a) — registered here in the same
        // `PortalSet::InputWarp` slot the core used; `app/plugins.rs` still wires
        // that set into `PlayerInput` and the intent brackets around it.
        app.add_systems(sim, warp_portal_input.in_set(PortalSet::InputWarp));

        // Play the portal audio cues from the portal-owned signals (Stage 19
        // Phase 5a — the crate emits `PortalShotFired` / `PortalBodyEntered` /
        // `PortalBodyTransited`, this adapter maps them to sfx). Runs in
        // `PortalSet::Transit` after `portal_transit` so the ENTER/EXIT signals
        // emitted this frame are played the same frame; the FIRE signal from
        // `portal_fire_system` (an earlier set in the frame) is read here too.
        app.add_systems(
            sim,
            play_portal_sfx
                .in_set(PortalSet::Transit)
                .after(portal_transit),
        );

        // Mutates `BodyAbilities`, so it is Ambition ability glue (Stage 19 Phase 5a),
        // registered in the same `PortalSet::TransitGuards` slot the core used.
        app.add_systems(
            sim,
            (
                suppress_ledge_grab_during_transit,
                restore_wall_abilities_after_transit,
            )
                .in_set(PortalSet::TransitGuards),
        );

        // CC6 host attachment: attribute placed portals to the identified
        // face they sit on, then re-derive each hosted aperture's frame from
        // its host every frame (§5-P2 step 2). Runs at the FRONT of
        // `PortalSet::Transit`, before the crate's link/evict/transit chain
        // (eviction must see the post-move apertures + their frame deltas),
        // and `Transit` is ordered after the platform sync via the encounter
        // schedule's chain, so the aperture reads this frame's host pose.
        app.add_systems(
            sim,
            (
                crate::portal::host_adapter::attach_portal_hosts,
                crate::portal::host_adapter::refresh_hosted_portal_frames,
            )
                .chain()
                .in_set(PortalSet::Transit)
                .before(ambition_portal2d::PortalLinkResolution),
        );
        // The attribution latch is rollback state.
        //
        // `attach_portal_hosts` is one-shot: a portal that failed to attach stays a
        // static aperture rather than re-scanning every frame. Losing that latch on a
        // restore is not benign, because attribution reads `RoomGeometry` AND
        // `MovingPlatformSet` — a re-scan on a later frame sees platforms in a
        // different place and can attach a portal the confirmed timeline left static,
        // writing a `host`/`host_lift` into `PlacedPortal` that no peer agreed to.
        //
        // This is the same shape as the unregistered `Collected` latch the rollback
        // oracle caught earlier: a marker whose ABSENCE is a decision.
        {
            let mut registrar =
                ambition_platformer2d_runtime::rollback::SchemaRollbackRegistrar::new(app);
            register_rollback_state(&mut registrar);
        }

        app.add_systems(
            sim,
            bridge_portal_carves
                .in_set(PortalSet::Carves)
                .after(publish_portal_carves),
        );

        app.add_systems(
            sim,
            sync_portal_host_depths
                .in_set(PortalSet::Carves)
                .before(publish_portal_carves),
        );

        // Advance in-flight portal shots against the concrete `RoomGeometry` (the
        // world-seam adapter, Phase 2 Seam 2). Runs in the weapon set after the
        // core fire system, preserving the old `toggle → fire → step` order; the
        // pure decision lives in `ambition_portal2d::step_portal_shot`.
        app.add_systems(
            sim,
            portal_projectile_step
                .in_set(GameplayGated)
                .in_set(PortalSet::WeaponAndProjectiles)
                .after(portal_fire_system),
        );

        // Bridge the Ambition room-reset event → the portal-owned `ClearPortals`
        // signal (Phase 2 Seam 4), before `clear_portals_on_reset` consumes it in
        // the same `PortalSet::RoomReset` frame.
        app.add_systems(
            sim,
            bridge_room_reset_to_clear_portals
                .in_set(PortalSet::RoomReset)
                .before(clear_portals_on_reset),
        );

        // Translate this frame's ControlFrame into portal intents BEFORE the
        // core weapon/projectile consumers (ordered via PortalSet::InputAdapter
        // in the portal plugin).
        app.add_systems(
            sim,
            portal_input_adapter_system
                .in_set(GameplayGated)
                .in_set(PortalSet::InputAdapter)
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::PlayerSimulation),
        );

        // Resolve the `FirePortalGun` gesture → the generic `PortalFireIntent` (origin from the
        // player's body, dir from the aim, channel from the held gun) the core fire system
        // consumes (Phase 2 Seam 3).
        app.add_systems(
            sim,
            resolve_portal_fire_intent
                .in_set(GameplayGated)
                .in_set(PortalSet::InputAdapter)
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::PlayerSimulation)
                .after(portal_input_adapter_system),
        );

        // Portal-gun ground pickups: the Ambition inventory grant on Attack.
        // Ordered `.after(arm_portal_pickups)` (registered by
        // `ambition_platformer2d_actor_monolith::items::pickup` in `ItemPickupSet::CoreHeldItems`) so the
        // arm → grant chain edge is identical to the old inline `.chain()`,
        // and after `portal_fire` (via the set order) so grabbing the gun
        // does not also fire on the same Attack press.
        app.add_systems(
            sim,
            pickup_portal_gun_system
                .in_set(GameplayGated)
                .in_set(ambition_platformer2d_shared_tangle::schedule::ItemPickupSet::CoreHeldItems)
                .after(ambition_portal2d::PortalPickupArming),
        );

        // The drop consumer touches Ambition item state (StashedActionSet), so
        // it lives in the content adapter. It reads DropPortalGun, so order it
        // with the rest of the portal weapon systems.
        app.add_systems(
            sim,
            drop_portal_gun_system
                .in_set(GameplayGated)
                // `PortalSet::WeaponAndProjectiles` is wired
                // `.in_set(PlayerSimulation)` in `wire_portal_schedule`, so the
                // parent placement is already implied — a direct
                // `.in_set(PlayerSimulation)` would be a redundant hierarchy edge.
                .in_set(PortalSet::WeaponAndProjectiles),
        );

        // --- Portal bodies for the input warp --- `warp_portal_input` resolves
        // each warped body's `DrivingParticipant` and edits that seat's frame
        // directly; the bracketing adapters that used to mirror a global
        // `PlayerMovementIntent` to and from `ControlFrame` are gone with it.
        app.add_systems(
            sim,
            ensure_portal_bodies
                .in_set(GameplayGated)
                .in_set(PortalSet::Transit)
                .before(portal_transit),
        );
        // Mirror the `portal_reverses_facing` gameplay setting into the global
        // `PortalTuning::reorient_facing` knob each frame, before the transit core
        // reads it, so the toggle takes effect live.
        app.add_systems(
            sim,
            sync_portal_reorient_from_settings
                .in_set(PortalSet::Transit)
                .before(portal_transit),
        );
        // Stage 19 Phase 4 — opt PROJECTILE entities into the SAME generic
        // `portal_transit` core, with their own free-flying policy
        // (reorient:false, carry_velocity:true). Same `.before(portal_transit)`
        // ordering as `ensure_portal_bodies` so a freshly-spawned projectile is
        // tagged before transit sees it.
        //
        // Ordering of projectile INTEGRATION vs transit: projectile motion integrates in
        // `Platformer2dSimulationPhaseMonolith::Combat` (`step_projectiles`), which is chained
        // AFTER `Platformer2dSimulationPhaseMonolith::PlayerSimulation` (where
        // `PortalSet::Transit` lives). The transit machine is a multi-frame aperture/centroid
        // latch, so the one-frame sampling cadence is correct; what matters is that it is
        // consistent, which the set chain guarantees.
        app.add_systems(
            sim,
            ensure_projectile_portal_bodies
                .in_set(GameplayGated)
                .in_set(PortalSet::Transit)
                .before(portal_transit),
        );
        // `portal_player_input_adapter` reproduces the player's input/trace bits
        // (BodyTeleported + PortalEmission + PortalInputWarp) from the core's
        // `PortalBodyTransited` event, AFTER transit, so they exist the same
        // frame the controller runs — exactly as the old inline insertion did.
        // Kernel-body transit reconciliation (ADR 0024 authority model): the
        // core wrote the pose; this completes the transit for cluster-bearing
        // bodies (contacts/attachment/motion-record), same frame, post-transit.
        app.add_systems(
            sim,
            reconcile_kernel_bodies_after_portal_transit
                .in_set(GameplayGated)
                .in_set(PortalSet::Transit)
                .after(portal_transit),
        );
        app.add_systems(
            sim,
            portal_player_input_adapter
                .in_set(GameplayGated)
                .in_set(PortalSet::Transit)
                .after(portal_transit),
        );
        // The projectile half of the same reconciliation: a carried WORLD
        // acceleration must rotate with the velocity it accompanies.
        app.add_systems(
            sim,
            rotate_projectile_acceleration_after_portal_transit
                .in_set(GameplayGated)
                .in_set(PortalSet::Transit)
                .after(portal_transit),
        );
        // Carried momentum: every transferred body's mapped exit velocity
        // becomes its `carried_run` floor the same frame (conserved fling,
        // tight ordinary control). Actor-generic — after portal_transit so
        // `BodyKinematics::vel` is already the exit velocity.
        app.add_systems(
            sim,
            apply_portal_carried_momentum
                .in_set(GameplayGated)
                .in_set(PortalSet::Transit)
                .after(portal_transit),
        );

        // --- GroundItem <-> PortalTransitable bracketing around item transit ---
        // Portal core teleports the generic `PortalTransitable` body; these
        // adapters attach it to `GroundItem`s and mirror it around
        // `portal_teleport_ground_items`.
        app.add_systems(
            sim,
            sync_ground_items_to_transitable
                .in_set(GameplayGated)
                .in_set(PortalSet::Transit)
                .before(portal_teleport_ground_items),
        );
        app.add_systems(
            sim,
            sync_transitable_to_ground_items
                .in_set(GameplayGated)
                .in_set(PortalSet::Transit)
                .after(portal_teleport_ground_items),
        );
    }
}

#[cfg(test)]
mod schedule_tests {
    //! The input-ordering contract the portal warp depends on: a system tagged
    //! `InputSet::Route` runs before the canonical slot publication, so a writer
    //! that carries no manual ordering still lands before the consume.

    use bevy::prelude::*;

    use ambition_platformer2d_actor_monolith::schedule::configure_platformer2d_simulation_phases;
    use ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith;

    // A seat-frame writer whose ONLY scheduling constraint is set
    // membership: `InputSet::Route`. It carries no manual ordering against
    // the consumer, so it can only land before the consume if the structural
    // `InputSet::Route.before(PrimarySlotInputCommit)` contract holds.
    fn populate_only_via_set(mut raw: ResMut<ambition_characters::control::SeatRawFrames>) {
        let slot = ambition_characters::control::PlayerSlot::PRIMARY;
        let mut frame = raw.get(slot);
        frame.axis_x = 0.75;
        raw.set(slot, frame);
    }

    /// The general input contract: any system tagged `InputSet::Route` is pinned BEFORE the
    /// canonical primary-slot publication in
    /// `Platformer2dSimulationPhaseMonolith::PlayerInput`.
    #[test]
    fn input_set_populate_runs_before_primary_slot_publication() {
        use ambition_characters::control::{PlayerSlot, SeatRawFrames, SlotControls};
        use ambition_platformer2d_actor_monolith::control::PrimarySlotInputCommit;
        use ambition_platformer2d_actor_monolith::schedule::publish_seat_controls_when_nobody_else_does;

        let mut app = App::new();
        configure_platformer2d_simulation_phases(&mut app);
        app.world_mut()
            .spawn(ambition_platformer2d_shared_tangle::lifecycle::SessionRoot(
                ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId(0),
            ));
        app.init_resource::<SeatRawFrames>();
        app.init_resource::<SlotControls>();

        app.add_systems(
            Update,
            populate_only_via_set.in_set(ambition_input::InputSet::Route),
        );
        app.add_systems(
            Update,
            publish_seat_controls_when_nobody_else_does
                .in_set(PrimarySlotInputCommit)
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerInput),
        );

        app.update();

        let observed = app
            .world()
            .resource::<SlotControls>()
            .get(PlayerSlot::PRIMARY)
            .axis_x;
        assert_eq!(
            observed, 0.75,
            "a Route-tagged seat-frame writer must run before primary-slot \
             publication; SlotControls[PRIMARY] captured axis_x = {observed} \
             instead of the shaped 0.75"
        );
    }
}
