//! Plugin that installs the Ambition portal adapters.
//!
//! Registers the input adapter (ControlFrame → portal intents) and the
//! inventory drop adapter into the portal subsystem's schedule sets so the
//! reusable portal core consumes intents the same frame they are produced. The
//! pickup adapter is registered alongside the held-item simulation (in
//! `crate::item_pickup`) because it must run last in that set, after the core
//! fire system, so picking up the gun doesn't also fire on the same press.

use bevy::prelude::*;

use crate::portal::PortalSet;

use super::input_adapter::portal_input_adapter_system;
use super::inventory_adapter::drop_portal_gun_system;

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
    }
}
