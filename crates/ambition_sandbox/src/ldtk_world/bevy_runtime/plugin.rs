use bevy::prelude::{Added, App, Commands, Entity, Name, Plugin, Query, ResMut};
use bevy_ecs_ldtk::prelude::{EntityInstance as PluginEntityInstance, LdtkEntityAppExt};

use super::components::{
    AmbitionLdtkEntity, AmbitionLdtkMarkerBundle, LdtkDamageVolume, LdtkOneWayPlatform, LdtkSolid,
};
use super::indices::LdtkRuntimeSpineStats;

pub struct AmbitionLdtkRegistrationPlugin;

impl Plugin for AmbitionLdtkRegistrationPlugin {
    fn build(&self, app: &mut App) {
        for identifier in AMBITION_LDTK_ENTITY_IDENTIFIERS {
            app.register_ldtk_entity::<AmbitionLdtkMarkerBundle>(identifier);
        }
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
];
