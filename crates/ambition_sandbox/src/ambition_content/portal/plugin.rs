//! Plugin that installs the Ambition portal adapters.
//!
//! Registers the input adapter (ControlFrame → portal intents) and the
//! inventory drop adapter into the portal subsystem's schedule sets so the
//! reusable portal core consumes intents the same frame they are produced. The
//! pickup adapter is registered alongside the held-item simulation (in
//! `crate::item_pickup`) because it must run last in that set, after the core
//! fire system, so picking up the gun doesn't also fire on the same press.

use bevy::prelude::*;

use crate::portal::{
    portal_teleport_ground_items, portal_transit_system, warp_portal_input, PortalSet,
};

use super::input_adapter::portal_input_adapter_system;
use super::inventory_adapter::drop_portal_gun_system;
use super::transit_adapter::{
    apply_movement_intent_to_control, sync_ground_items_to_transitable,
    sync_movement_intent_from_control, sync_transitable_to_ground_items,
};

/// Installs the Ambition-specific portal input/inventory adapters.
pub struct AmbitionPortalAdaptersPlugin;

impl Plugin for AmbitionPortalAdaptersPlugin {
    fn build(&self, app: &mut App) {
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

        // The drop consumer touches Ambition item state (StashedActionSet), so
        // it lives in the content adapter. It reads DropPortalGun, so order it
        // with the rest of the portal weapon systems.
        app.add_systems(
            Update,
            drop_portal_gun_system
                .run_if(crate::gameplay_allowed)
                .in_set(PortalSet::WeaponAndProjectiles)
                .in_set(crate::app::SandboxSet::PlayerSimulation),
        );

        // --- Movement-intent bracketing around portal core's input warp ---
        // Portal core's `warp_portal_input` reads + mutates the content-agnostic
        // `PlayerMovementIntent`; these adapters mirror it to/from `ControlFrame`
        // so the result is byte-identical to the old direct-`ControlFrame` mutate.
        // Sync ControlFrame -> intent BEFORE the warp, apply intent -> ControlFrame
        // AFTER it.
        app.add_systems(
            Update,
            sync_movement_intent_from_control
                .run_if(crate::gameplay_allowed)
                .in_set(PortalSet::InputWarp)
                .before(warp_portal_input),
        );
        app.add_systems(
            Update,
            apply_movement_intent_to_control
                .run_if(crate::gameplay_allowed)
                .in_set(PortalSet::InputWarp)
                .after(warp_portal_input),
        );
        // `portal_transit_system` reads `PlayerMovementIntent` as the warp anchor;
        // re-sync from `ControlFrame` immediately before it so the anchor matches
        // the live held direction (as it did when transit read `ControlFrame`).
        app.add_systems(
            Update,
            sync_movement_intent_from_control
                .run_if(crate::gameplay_allowed)
                .in_set(PortalSet::Transit)
                .before(portal_transit_system),
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
