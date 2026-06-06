use bevy::prelude::*;

use super::schedule::PortalSet;
use super::{
    clear_portals_on_reset, despawn_orphaned_portals, drop_portal_gun_system, portal_fire_system,
    portal_projectile_step, portal_teleport_ground_items, portal_toggle_system,
    portal_transit_actors, portal_transit_system, publish_portal_carves,
    reset_gravity_on_room_reset, suppress_ledge_grab_during_transit, tick_portal_cooldowns,
    warp_portal_input, BodyTeleported, SuppressWallAbilitiesInPortal,
};
use crate::platformer_runtime::orientation::{ensure_actor_roll, update_actor_roll};

/// Top-level portal mechanic plugin.
///
/// This is the public plugin app assembly should install. It currently delegates
/// to the simulation plugin, leaving room for future render, authoring, and
/// debug adapters to become independent subplugins.
pub struct PortalPlugin;

impl Plugin for PortalPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PortalSimulationPlugin);
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
        app.add_message::<BodyTeleported>();
        app.init_resource::<SuppressWallAbilitiesInPortal>();
        app.init_resource::<crate::physics::GravityField>();
        app.init_resource::<crate::physics::BaseGravity>();
        app.init_resource::<crate::physics::GravityZones>();

        // Snapshot all gravity zones once per frame BEFORE actor integrators read
        // them, so every body can resolve local gravity by position. PlacedPortal carves
        // are published with the same early-world snapshot cadence.
        app.add_systems(
            Update,
            (
                crate::physics::oscillate_gravity_zones,
                crate::physics::collect_gravity_zones,
                publish_portal_carves,
            )
                .chain()
                .in_set(PortalSet::GravityAndCarves)
                .before(crate::app::SandboxSet::CoreSimulation),
        );

        app.add_systems(
            Update,
            warp_portal_input
                .in_set(PortalSet::InputWarp)
                .in_set(crate::app::SandboxSet::PlayerInput)
                .after(crate::app::interaction_input_system)
                .before(crate::player::sync_local_player_input_frame)
                .run_if(crate::gameplay_allowed),
        );

        app.add_systems(
            Update,
            (
                drop_portal_gun_system.run_if(crate::gameplay_allowed),
                portal_toggle_system.run_if(crate::gameplay_allowed),
                portal_fire_system.run_if(crate::gameplay_allowed),
                portal_projectile_step.run_if(crate::gameplay_allowed),
                // Portals must not outlive their gun (the "destroyed" case).
                despawn_orphaned_portals,
                // Make sure the player can carry an aerial roll through portals.
                ensure_actor_roll,
            )
                .chain()
                .in_set(PortalSet::WeaponAndProjectiles)
                .in_set(crate::app::SandboxSet::PlayerSimulation),
        );

        app.add_systems(
            Update,
            (clear_portals_on_reset, reset_gravity_on_room_reset)
                .chain()
                .in_set(PortalSet::RoomReset)
                .in_set(crate::app::SandboxSet::RoomTransition)
                .after(crate::boss_encounter::reset_cut_rope_boss_arena_on_room_reset),
        );

        // Suppress ledge-grab while transiting so the carved aperture edges are
        // not grabbed before movement integration probes for a ledge.
        app.add_systems(
            Update,
            suppress_ledge_grab_during_transit
                .in_set(PortalSet::TransitGuards)
                .in_set(crate::app::SandboxSet::PlayerSimulation)
                .before(crate::app::player_simulation_system)
                .run_if(crate::gameplay_allowed),
        );

        // Teleports run after player and ground-item integration so this frame's
        // integrated body positions are what cross the portal. Order against
        // the item subsystem's public set instead of its concrete physics
        // function.
        app.add_systems(
            Update,
            (
                tick_portal_cooldowns,
                portal_transit_system,
                portal_teleport_ground_items,
                portal_transit_actors,
                update_actor_roll,
            )
                .chain()
                .in_set(PortalSet::Transit)
                .in_set(crate::app::SandboxSet::PlayerSimulation)
                .after(crate::app::player_simulation_system)
                .after(crate::item_pickup::ItemPickupSet::CoreHeldItems)
                .run_if(crate::gameplay_allowed),
        );
    }
}
