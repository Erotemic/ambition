//! ECS-feature spawn paths.
//!
//! Both static room features (authored objects from `RoomSpec`) and
//! dynamic encounter mobs land here. The two entry points stay near
//! each other so the shape of the spawned components (bundles +
//! markers + actor snapshot) is easy to diff when a new family is
//! added.

use super::*;
use crate::content::features::util::room_spec_paths;
use bevy::prelude::Name;

/// Spawn ECS-native feature entities for every static feature object in a room.
pub fn spawn_room_feature_entities(commands: &mut Commands, room: &crate::rooms::RoomSpec) {
    let paths = room_spec_paths(room);
    for object in &room.world.objects {
        spawn_room_feature_entity(commands, object, &paths);
    }
}

fn spawn_room_feature_entity(
    commands: &mut Commands,
    object: &ae::RoomObject,
    paths: &[(String, ae::KinematicPath)],
) {
    let feature_aabb = FeatureAabb::from_aabb(object.aabb);
    match &object.kind {
        ae::RoomObjectKind::DamageVolume(volume) => {
            let hazard = HazardRuntime::new_with_paths(
                object.id.clone(),
                object.name.clone(),
                object.aabb,
                volume.clone(),
                paths,
            );
            commands.spawn((
                Name::new(format!("Feature hazard: {}", object.name)),
                FeatureSimEntity,
                RoomVisual,
                FeatureId::new(object.id.clone()),
                FeatureName::new(object.name.clone()),
                FeatureAabb::from_center_size(hazard.pos, hazard.size),
                HazardFeature::new(hazard),
            ));
        }
        ae::RoomObjectKind::BossSpawn(brain) => {
            let boss = BossRuntime::new(
                object.id.clone(),
                object.name.clone(),
                object.aabb,
                brain.clone(),
            );
            let initial_phase = BossPhase::from_alive(boss.alive);
            commands.spawn((
                Name::new(format!("Feature boss: {}", object.name)),
                FeatureSimEntity,
                RoomVisual,
                FeatureId::new(object.id.clone()),
                FeatureName::new(object.name.clone()),
                FeatureAabb::from_center_size(boss.pos, boss.render_size()),
                BossPatternTimer(boss.pattern_timer),
                initial_phase,
                BossFeature::new(boss),
            ));
        }
        ae::RoomObjectKind::Pickup(pickup) => {
            commands.spawn((
                Name::new(format!("Feature pickup: {}", object.name)),
                PickupBundle::new(&object.id, &object.name, feature_aabb, pickup.clone()),
            ));
        }
        ae::RoomObjectKind::Chest(chest) => {
            commands.spawn((
                Name::new(format!("Feature chest: {}", object.name)),
                ChestBundle::new(&object.id, &object.name, feature_aabb, chest.clone()),
            ));
        }
        ae::RoomObjectKind::Breakable(breakable) => {
            let mut entity = commands.spawn((
                Name::new(format!("Feature breakable: {}", object.name)),
                FeatureSimEntity,
                RoomVisual,
                FeatureId::new(object.id.clone()),
                FeatureName::new(object.name.clone()),
                feature_aabb,
                BreakableFeature::new(breakable.clone()),
                StandTimer(0.0),
            ));
            if breakable.collision.blocks_movement() {
                entity.insert(SandboxSolidContributor);
            }
            if breakable.pogo_refresh
                || (breakable.collision.blocks_movement() && breakable.trigger.allows_stand())
            {
                entity.insert(PogoTargetContributor);
            }
        }
        ae::RoomObjectKind::EnemySpawn(brain) => {
            let actor = ActorRuntime::Hostile(EnemyRuntime::new(
                object.id.clone(),
                object.name.clone(),
                object.aabb,
                brain.clone(),
                paths,
            ));
            let (identity, disposition, health, combat, intent, cooldowns) =
                actor_component_snapshot(&actor);
            commands.spawn((
                Name::new(format!("Feature actor enemy: {}", object.name)),
                EnemyActorBundle {
                    base: FeatureBaseBundle::new(&object.id, &object.name, feature_aabb),
                    identity,
                    disposition,
                    health,
                    combat,
                    intent,
                    cooldowns,
                },
                actor,
            ));
        }
        ae::RoomObjectKind::Interactable(interactable) => {
            if matches!(interactable.kind, ae::InteractionKind::Npc { .. }) {
                let actor = ActorRuntime::Peaceful(NpcRuntime::new_with_paths(
                    object.id.clone(),
                    object.name.clone(),
                    object.aabb,
                    interactable.clone(),
                    paths,
                ));
                let (identity, disposition, health, combat, intent, cooldowns) =
                    actor_component_snapshot(&actor);
                commands.spawn((
                    Name::new(format!("Feature actor npc: {}", object.name)),
                    EnemyActorBundle {
                        base: FeatureBaseBundle::new(&object.id, &object.name, feature_aabb),
                        identity,
                        disposition,
                        health,
                        combat,
                        intent,
                        cooldowns,
                    },
                    actor,
                ));
            } else if let ae::InteractionKind::Custom(payload) = &interactable.kind {
                if let Some(activation) = crate::encounter::SwitchActivation::parse_custom(payload)
                {
                    commands.spawn((
                        Name::new(format!("Feature switch: {}", object.name)),
                        FeatureSimEntity,
                        RoomVisual,
                        FeatureId::new(object.id.clone()),
                        FeatureName::new(object.name.clone()),
                        feature_aabb,
                        SwitchFeature::new(activation),
                        SwitchOn(false),
                    ));
                }
            }
        }
        _ => {}
    }
}

/// Spawn one hostile actor for an encounter wave.
///
/// The encounter system still owns wave timing, but the mob itself is a normal
/// feature entity queried by actor, projectile, rendering, and health systems.
pub fn spawn_encounter_mob(
    commands: &mut Commands,
    encounter_id: impl Into<String>,
    id: String,
    brain: ae::EnemyBrain,
    pos: ae::Vec2,
    size: ae::Vec2,
) {
    let encounter_id = encounter_id.into();
    let archetype = EnemyArchetype::from_brain(&brain);
    let aabb = ae::Aabb::new(pos, size * 0.5);
    let mut enemy = EnemyRuntime::new(id.clone(), id.clone(), aabb, brain, &[]);
    enemy.archetype = archetype;
    enemy.health = ae::Health::new(archetype.max_health());
    // Encounter mobs should not auto-respawn like training sandbags.
    enemy.respawn_timer = 999_999.0;
    let actor = ActorRuntime::Hostile(enemy);
    let (identity, disposition, health, combat, intent, cooldowns) =
        actor_component_snapshot(&actor);
    commands.spawn((
        Name::new(format!("Encounter mob: {id}")),
        FeatureSimEntity,
        RoomVisual,
        FeatureId::new(id.clone()),
        FeatureName::new(id),
        FeatureAabb::from_center_size(pos, size),
        identity,
        disposition,
        health,
        combat,
        intent,
        cooldowns,
        actor,
        EncounterMob::new(encounter_id),
    ));
}

/// Despawn all ECS mobs owned by an encounter attempt.
pub fn despawn_encounter_mobs(
    commands: &mut Commands,
    mobs: &Query<(Entity, &EncounterMob, &FeatureId, &ActorCombatState)>,
    encounter_id: &str,
) {
    for (entity, mob, _, _) in mobs.iter() {
        if mob.encounter_id == encounter_id {
            commands.entity(entity).despawn();
        }
    }
}
