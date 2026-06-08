//! Plugin that installs the Ambition portal adapters.
//!
//! Registers the input adapter (ControlFrame → portal intents) and the
//! inventory drop adapter into the portal subsystem's schedule sets so the
//! reusable portal core consumes intents the same frame they are produced. The
//! pickup adapter is registered alongside the held-item simulation (in
//! `crate::items::pickup`) because it must run last in that set, after the core
//! fire system, so picking up the gun doesn't also fire on the same press.

use bevy::prelude::*;

use crate::portal::{
    clear_portals_on_reset, portal_fire_system, portal_teleport_ground_items, portal_transit,
    publish_portal_carves, warp_portal_input, PortalSet,
};

use super::carve_adapter::bridge_portal_carves;
use super::fire_adapter::resolve_portal_fire_intent;
use super::input_adapter::portal_input_adapter_system;
use super::inventory_adapter::drop_portal_gun_system;
use super::reset_adapter::bridge_room_reset_to_clear_portals;
use super::shot_adapter::portal_projectile_step;
use super::transit_adapter::{
    apply_movement_intent_to_control, sync_ground_items_to_transitable,
    sync_movement_intent_from_control, sync_transitable_to_ground_items,
};
use super::transit_body_adapter::{ensure_portal_bodies, portal_player_input_adapter};

/// Installs the Ambition-specific portal input/inventory adapters.
pub struct AmbitionPortalAdaptersPlugin;

impl Plugin for AmbitionPortalAdaptersPlugin {
    fn build(&self, app: &mut App) {
        // Bridge portal-owned carves → the host collision overlay. Runs in
        // `PortalSet::Carves` (which is `.before(CoreSimulation)`), after
        // `publish_portal_carves` fills `PortalCarves`, so the overlay sees the
        // same carves the same frame the in-core write used to produce.
        app.add_systems(
            Update,
            bridge_portal_carves
                .in_set(PortalSet::Carves)
                .after(publish_portal_carves),
        );

        // Advance in-flight portal shots against the concrete `GameWorld` (the
        // world-seam adapter, Phase 2 Seam 2). Runs in the weapon set after the
        // core fire system, preserving the old `toggle → fire → step` order; the
        // pure decision lives in `crate::portal::step_portal_shot`.
        app.add_systems(
            Update,
            portal_projectile_step
                .run_if(crate::gameplay_allowed)
                .in_set(PortalSet::WeaponAndProjectiles)
                .after(portal_fire_system),
        );

        // Bridge the Ambition room-reset event → the portal-owned `ClearPortals`
        // signal (Phase 2 Seam 4), before `clear_portals_on_reset` consumes it in
        // the same `PortalSet::RoomReset` frame.
        app.add_systems(
            Update,
            bridge_room_reset_to_clear_portals
                .in_set(PortalSet::RoomReset)
                .before(clear_portals_on_reset),
        );

        // Translate this frame's ControlFrame into portal intents BEFORE the
        // core weapon/projectile consumers (ordered via PortalSet::InputAdapter
        // in the portal plugin).
        app.add_systems(
            Update,
            portal_input_adapter_system
                .run_if(crate::gameplay_allowed)
                .in_set(PortalSet::InputAdapter)
                .in_set(crate::app::SandboxSet::PlayerSimulation),
        );

        // Resolve the `FirePortalGun` gesture → the generic `PortalFireIntent`
        // (origin from the player's body, dir from the aim, channel from the held
        // gun) the core fire system consumes (Phase 2 Seam 3). Runs after the
        // input adapter (which emits `FirePortalGun`) and, being in
        // `PortalSet::InputAdapter`, before `PortalSet::WeaponAndProjectiles`
        // (where `portal_fire_system` reads the intent) — same frame, identical
        // behavior to the old in-core player+gun read.
        app.add_systems(
            Update,
            resolve_portal_fire_intent
                .run_if(crate::gameplay_allowed)
                .in_set(PortalSet::InputAdapter)
                .in_set(crate::app::SandboxSet::PlayerSimulation)
                .after(portal_input_adapter_system),
        );

        // The drop consumer touches Ambition item state (StashedActionSet), so
        // it lives in the content adapter. It reads DropPortalGun, so order it
        // with the rest of the portal weapon systems.
        app.add_systems(
            Update,
            drop_portal_gun_system
                .run_if(crate::gameplay_allowed)
                // `PortalSet::WeaponAndProjectiles` is wired
                // `.in_set(PlayerSimulation)` in `wire_portal_schedule`, so the
                // parent placement is already implied — a direct
                // `.in_set(PlayerSimulation)` would be a redundant hierarchy edge.
                .in_set(PortalSet::WeaponAndProjectiles),
        );

        // --- Movement-intent bracketing around portal core's input warp ---
        // Portal core's `warp_portal_input` reads + mutates the content-agnostic
        // `PlayerMovementIntent`; these adapters mirror it to/from `ControlFrame`
        // so the result is byte-identical to the old direct-`ControlFrame` mutate.
        // Sync ControlFrame -> intent BEFORE the warp, apply intent -> ControlFrame
        // AFTER it.
        //
        // BOTH brackets must sit in the same `[populate ControlFrame ->
        // sync_local_player_input_frame consumes it]` window that
        // `warp_portal_input` itself occupies, exactly as the old in-`warp`
        // direct mutate did. `warp_portal_input` is `.in_set(PlayerInput)` and
        // `.before(sync_local_player_input_frame)`; pin the brackets the same way
        // so:
        //   * `sync_movement_intent_from_control` reads the FRESH per-frame axis
        //     (PlayerInput runs after the `.before(CoreSimulation)` populate), and
        //   * `apply_movement_intent_to_control` writes the (round-tripped or
        //     warped) axis back BEFORE the player consumes it.
        // Without these anchors the brackets float (their only constraint is
        // `.before/.after(warp_portal_input)`), so the scheduler can run the
        // write-back AFTER the consume and the read BEFORE the populate — the
        // write-back then stamps a STALE intent over the fresh axis, which reads
        // as a dead / sticky Move axis.
        app.add_systems(
            Update,
            sync_movement_intent_from_control
                .run_if(crate::gameplay_allowed)
                // `PortalSet::InputWarp` is wired `.in_set(PlayerInput)` (and
                // `.before(sync_local_player_input_frame)`) in
                // `wire_portal_schedule`, so the parent placement + consume window
                // are already implied — a direct `.in_set(PlayerInput)` would be a
                // redundant hierarchy edge.
                .in_set(PortalSet::InputWarp)
                .before(warp_portal_input),
        );
        app.add_systems(
            Update,
            apply_movement_intent_to_control
                .run_if(crate::gameplay_allowed)
                // `PortalSet::InputWarp` already places this in `PlayerInput`
                // (see `wire_portal_schedule`), so a direct
                // `.in_set(PlayerInput)` would be a redundant hierarchy edge.
                // `InputSet::Populate` is a SEPARATE set (NOT nested under
                // `PlayerInput`), so it stays — it pins this write-back inside the
                // `Populate.before(sync_local_player_input_frame)` consume window.
                .in_set(PortalSet::InputWarp)
                .in_set(crate::input::InputSet::Populate)
                .after(warp_portal_input)
                .before(crate::player::sync_local_player_input_frame),
        );
        // The player-input adapter reads `PlayerMovementIntent` as the warp
        // anchor; re-sync from `ControlFrame` immediately before the generic
        // transit core (and thus before the input adapter) so the anchor matches
        // the live held direction (as it did when transit read `ControlFrame`).
        app.add_systems(
            Update,
            sync_movement_intent_from_control
                .run_if(crate::gameplay_allowed)
                .in_set(PortalSet::Transit)
                .before(portal_transit),
        );

        // --- Identity → policy tagging + player-input reproduction ---
        // `ensure_portal_bodies` adds the `PortalBody` marker + the right
        // `PortalPolicy` to the player and every actor BEFORE the generic
        // `portal_transit` core runs, so the SET of bodies that transit is
        // identical to the old player + actor split.
        app.add_systems(
            Update,
            ensure_portal_bodies
                .run_if(crate::gameplay_allowed)
                .in_set(PortalSet::Transit)
                .before(portal_transit),
        );
        // `portal_player_input_adapter` reproduces the player's input/trace bits
        // (BodyTeleported + PortalEmission + PortalInputWarp) from the core's
        // `PortalBodyTransited` event, AFTER transit, so they exist the same
        // frame the controller runs — exactly as the old inline insertion did.
        app.add_systems(
            Update,
            portal_player_input_adapter
                .run_if(crate::gameplay_allowed)
                .in_set(PortalSet::Transit)
                .after(portal_transit),
        );

        // --- GroundItem <-> PortalTransitable bracketing around item transit ---
        // Portal core teleports the generic `PortalTransitable` body; these
        // adapters attach it to `GroundItem`s and mirror it around
        // `portal_teleport_ground_items`.
        app.add_systems(
            Update,
            sync_ground_items_to_transitable
                .run_if(crate::gameplay_allowed)
                .in_set(PortalSet::Transit)
                .before(portal_teleport_ground_items),
        );
        app.add_systems(
            Update,
            sync_transitable_to_ground_items
                .run_if(crate::gameplay_allowed)
                .in_set(PortalSet::Transit)
                .after(portal_teleport_ground_items),
        );
    }
}

#[cfg(test)]
mod schedule_tests {
    //! Regression guard for the movement-axis "dead/sticky" input bug.
    //!
    //! The portal movement-intent brackets mirror `ControlFrame` axes into
    //! `PlayerMovementIntent` before `warp_portal_input` and back to
    //! `ControlFrame` after. They MUST live inside the same
    //! `[populate ControlFrame -> player consumes ControlFrame]` window the old
    //! in-`warp` direct mutate occupied. When the brackets only carried
    //! `.before/.after(warp_portal_input)` (and no SandboxSet / consume anchor)
    //! they floated: the read could run before the per-frame populate and the
    //! write-back after the player had already consumed the frame, stamping a
    //! STALE intent over the fresh axis — the live Move axis read as dead/sticky.
    //!
    //! This test reproduces that window with a stand-in populate (before
    //! `CoreSimulation`) and a stand-in consume (the tail of `PlayerInput`) plus
    //! the REAL bracket systems + REAL `warp_portal_input`, and asserts the axis
    //! the consumer sees is the one populate wrote this frame (no active warp =
    //! pure round-trip).
    use bevy::prelude::*;

    use crate::app::{configure_sandbox_sets, SandboxSet};
    use crate::input::ControlFrame;
    use crate::player::{PlayerEntity, PrimaryPlayer};
    use crate::portal::{warp_portal_input, PlayerMovementIntent};

    use super::super::transit_adapter::{
        apply_movement_intent_to_control, sync_movement_intent_from_control,
    };

    #[derive(Resource, Default)]
    struct ConsumedAxis(f32);

    // Stand-in for the device populate (`populate_control_frame_from_actions`),
    // which runs `.before(SandboxSet::CoreSimulation)`.
    fn populate_fresh_axis(mut frame: ResMut<ControlFrame>) {
        frame.axis_x = -1.0;
    }

    // Stand-in for the player consume (`sync_local_player_input_frame`), the tail
    // of `SandboxSet::PlayerInput`. Records the axis it observes.
    fn consume_axis(frame: Res<ControlFrame>, mut consumed: ResMut<ConsumedAxis>) {
        consumed.0 = frame.axis_x;
    }

    #[test]
    fn portal_intent_brackets_do_not_clobber_the_fresh_move_axis() {
        let mut app = App::new();
        configure_sandbox_sets(&mut app);
        app.init_resource::<ControlFrame>();
        app.init_resource::<PlayerMovementIntent>();
        app.init_resource::<ConsumedAxis>();
        // A primary player so `warp_portal_input` runs its body.
        app.world_mut().spawn((PlayerEntity, PrimaryPlayer));

        app.add_systems(
            Update,
            populate_fresh_axis.before(SandboxSet::CoreSimulation),
        );
        // Real brackets + real warp, anchored exactly as the plugin wires them.
        // Wire the brackets exactly as the plugin does: both inside
        // `SandboxSet::PlayerInput` (so the read runs after the
        // `.before(CoreSimulation)` populate) and the write-back
        // `.before` the consumer (so the fresh/round-tripped axis reaches the
        // player this frame).
        app.add_systems(
            Update,
            sync_movement_intent_from_control
                .in_set(SandboxSet::PlayerInput)
                .before(warp_portal_input),
        );
        app.add_systems(
            Update,
            warp_portal_input
                .in_set(SandboxSet::PlayerInput)
                .before(consume_axis),
        );
        app.add_systems(
            Update,
            apply_movement_intent_to_control
                .in_set(SandboxSet::PlayerInput)
                .after(warp_portal_input)
                .before(consume_axis),
        );
        app.add_systems(Update, consume_axis.in_set(SandboxSet::PlayerInput));

        app.update();

        let consumed = app.world().resource::<ConsumedAxis>().0;
        assert_eq!(
            consumed, -1.0,
            "the player must consume this frame's fresh Move axis (-1.0); got \
             {consumed}. A 0.0 here means the portal intent write-back stamped a \
             stale/empty PlayerMovementIntent over the fresh ControlFrame axis \
             (the dead/sticky Move-axis regression)."
        );
    }

    // A `ControlFrame` writer whose ONLY scheduling constraint is set
    // membership: `InputSet::Populate`. It carries no manual ordering against
    // the consumer, so it can only land before the consume if the structural
    // `Populate.before(sync_local_player_input_frame)` contract holds.
    fn populate_only_via_set(mut frame: ResMut<ControlFrame>) {
        frame.axis_x = 0.75;
    }

    /// The general input contract: any system tagged `InputSet::Populate` is
    /// pinned BEFORE the real gameplay consumer (`sync_local_player_input_frame`),
    /// purely by set membership + the `configure_sandbox_sets` constraint — no
    /// per-system `.before` anchor required. The consumer snapshots
    /// `ControlFrame` into the player's `PlayerInputFrame`, so observing the
    /// written axis there proves the populate ran first.
    #[test]
    fn input_set_populate_runs_before_the_real_consumer() {
        use crate::player::{
            sync_local_player_input_frame, LocalPlayer, PlayerEntity, PlayerInputFrame,
        };

        let mut app = App::new();
        configure_sandbox_sets(&mut app);
        app.init_resource::<ControlFrame>();
        let player = app
            .world_mut()
            .spawn((PlayerEntity, LocalPlayer, PlayerInputFrame::default()))
            .id();

        app.add_systems(
            Update,
            populate_only_via_set.in_set(crate::input::InputSet::Populate),
        );
        app.add_systems(
            Update,
            sync_local_player_input_frame.in_set(SandboxSet::PlayerInput),
        );

        app.update();

        let observed = app
            .world()
            .entity(player)
            .get::<PlayerInputFrame>()
            .expect("player has an input frame")
            .frame
            .axis_x;
        assert_eq!(
            observed, 0.75,
            "a Populate-tagged ControlFrame writer must run before the consumer \
             (sync_local_player_input_frame); the consumer snapshotted axis_x = \
             {observed} instead of the populated 0.75 — InputSet::Populate is not \
             pinned before the consume boundary."
        );
    }
}
