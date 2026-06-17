//! Plugins wiring the LDtk runtime spine into the Bevy app.
//!
//! `AmbitionLdtkRegistrationPlugin` registers the entity bundle/markers so
//! bevy_ecs_ldtk spawns Ambition entities; `LdtkRuntimeSpinePlugin` adds the
//! index-rebuild systems. `sync_plugin_spawned_ambition_entities` attaches
//! gameplay semantics + names to freshly spawned plugin entities. Components
//! live in sibling `components`, rebuild systems in `systems`.

use bevy::prelude::{
    Added, App, Commands, Entity, IntoScheduleConfigs, Name, Plugin, Query, ResMut, Update,
};
use bevy_ecs_ldtk::prelude::{EntityInstance as PluginEntityInstance, LdtkEntityAppExt};

use super::components::{
    AmbitionLdtkEntity, AmbitionLdtkMarkerBundle, LdtkDamageVolume, LdtkOneWayPlatform, LdtkSolid,
};
use super::indices::LdtkRuntimeSpineStats;
use crate::app::SandboxSet;

pub struct AmbitionLdtkRegistrationPlugin;

impl Plugin for AmbitionLdtkRegistrationPlugin {
    fn build(&self, app: &mut App) {
        for identifier in AMBITION_LDTK_ENTITY_IDENTIFIERS {
            app.register_ldtk_entity::<AmbitionLdtkMarkerBundle>(identifier);
        }
    }
}

/// Module-local Bevy plugin for the LDtk runtime-spine indexes.
///
/// Owns the chain that walks plugin-spawned Ambition entities
/// (`sync_plugin_spawned_ambition_entities`), rebuilds the per-active-
/// area solid / one-way / hazard runtime indexes, and pins parity with
/// the JSON adapter via the spine parity check.
///
/// Runs in [`SandboxSet::LdtkRuntimeSpine`] (configured by
/// `app/schedule.rs`). Carved out of `app/plugins.rs::register_ldtk_runtime_spine_systems`
/// per OVERNIGHT-TODO #6 — every system in this chain lives under
/// `ldtk_world::bevy_runtime`, so it's the right domain to own the
/// schedule registration.
pub struct LdtkRuntimeSpinePlugin;

impl Plugin for LdtkRuntimeSpinePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                sync_plugin_spawned_ambition_entities,
                super::systems::rebuild_ldtk_runtime_spine_index,
                super::systems::rebuild_ldtk_runtime_solid_index,
                super::systems::rebuild_ldtk_runtime_one_way_index,
                super::systems::rebuild_ldtk_runtime_damage_index,
                super::parity::check_ldtk_runtime_spine_parity,
            )
                .chain()
                .in_set(SandboxSet::LdtkRuntimeSpine),
        );
    }
}

pub fn sync_plugin_spawned_ambition_entities(
    mut commands: Commands,
    mut stats: ResMut<LdtkRuntimeSpineStats>,
    query: Query<(Entity, &PluginEntityInstance), Added<PluginEntityInstance>>,
) {
    for (entity, instance) in &query {
        stats.spawned_entities = stats.spawned_entities.saturating_add(1);
        stats.revision = stats.revision.saturating_add(1);
        let ambition_entity = AmbitionLdtkEntity {
            iid: instance.iid.clone(),
            identifier: instance.identifier.clone(),
            px: [instance.px.x, instance.px.y],
            size: [instance.width, instance.height],
            world: instance.world_x.zip(instance.world_y).map(|(x, y)| [x, y]),
        };
        stats.last_entity = format!("{} {}", ambition_entity.identifier, ambition_entity.iid);
        stats.sample_entity = ambition_entity.summary();

        // Attach typed Ambition components for promoted collision-heavy LDtk
        // categories. The generic `AmbitionLdtkEntity` always lands; typed
        // sibling components let downstream systems query specifically without
        // identifier-string matching.
        let mut entity_commands = commands.entity(entity);
        entity_commands.insert((
            Name::new(format!(
                "LDtk {} {}",
                ambition_entity.identifier, ambition_entity.iid
            )),
            ambition_entity.clone(),
        ));
        // Plugin-spawned `Solid` LDtk entities get the typed `LdtkSolid`
        // component so the `LdtkRuntimeSolidIndex` collision authority can
        // pick them up without reparsing identifiers.
        match ambition_entity.identifier.as_str() {
            "Solid" => {
                entity_commands.insert(LdtkSolid {
                    level_px: ambition_entity.px,
                    size: ambition_entity.size,
                });
            }
            "OneWayPlatform" => {
                entity_commands.insert(LdtkOneWayPlatform {
                    level_px: ambition_entity.px,
                    size: ambition_entity.size,
                });
            }
            "DamageVolume" | "HazardBlock" => {
                entity_commands.insert(LdtkDamageVolume {
                    level_px: ambition_entity.px,
                    size: ambition_entity.size,
                    // `damage` is not yet part of the LDtk schema; default
                    // to the JSON adapter's hazard amount (1).
                    damage: 1,
                });
            }
            _ => {}
        }
    }
}

pub const AMBITION_LDTK_ENTITY_IDENTIFIERS: &[&str] = &[
    "PlayerStart",
    "Solid",
    "OneWayPlatform",
    "BlinkWall",
    "HazardBlock",
    "PogoOrb",
    "ReboundPad",
    "LoadingZone",
    "DamageVolume",
    "KinematicPath",
    "NpcSpawn",
    "PickupSpawn",
    "GroundItem",
    "PortalGunSpawn",
    "ShrineSpawn",
    "GravityZone",
    "ChestSpawn",
    "BreakablePlatform",
    "BreakablePogoOrb",
    "EnemySpawn",
    "BossSpawn",
    "DebugLabel",
    "CameraZone",
    "StitchedBoundary",
    "EncounterTrigger",
    "Switch",
    "LockWall",
    "WaterVolume",
    "MovingPlatform",
    "Prop",
    "Portal",
];
