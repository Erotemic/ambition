//! ECS-feature spawn paths.
//!
//! Both static room features (authored entities from `RoomSpec`) and
//! dynamic encounter mobs land here. The static path is per-family —
//! one loop per `RoomSpec.{pickups,chests,…}` — so adding a new
//! authored entity type is "add a new Vec on RoomSpec + a new loop
//! here" rather than "edit a match arm somewhere."

use super::*;
use crate::content::features::util::room_spec_paths;
use bevy::prelude::Name;

/// Spawn ECS-native feature entities for every authored static
/// feature in a room. One loop per family.
pub fn spawn_room_feature_entities(commands: &mut Commands, room: &crate::rooms::RoomSpec) {
    let paths = room_spec_paths(room);
    for hazard in &room.hazards {
        spawn_hazard(commands, hazard, &paths);
    }
    for boss in &room.boss_spawns {
        spawn_boss(commands, boss);
    }
    for pickup in &room.pickups {
        spawn_pickup(commands, pickup);
    }
    for chest in &room.chests {
        spawn_chest(commands, chest);
    }
    for breakable in &room.breakables {
        spawn_breakable(commands, breakable);
    }
    for enemy in &room.enemy_spawns {
        spawn_enemy(commands, enemy, &paths);
    }
    for interactable in &room.interactables {
        spawn_interactable(commands, interactable, &paths);
    }
    // DebugLabel and DestinationLabel are presentation-only and don't
    // spawn ECS feature entities today. The presentation layer reads
    // them off `RoomSpec` directly.
}

fn spawn_hazard(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<ae::DamageVolume>,
    paths: &[(String, ae::KinematicPath)],
) {
    let hazard = HazardRuntime::new_with_paths(
        authored.id.clone(),
        authored.name.clone(),
        authored.aabb,
        authored.payload.clone(),
        paths,
    );
    commands.spawn((
        Name::new(format!("Feature hazard: {}", authored.name)),
        FeatureSimEntity,
        RoomVisual,
        FeatureId::new(authored.id.clone()),
        FeatureName::new(authored.name.clone()),
        FeatureAabb::from_center_size(hazard.pos, hazard.size),
        HazardFeature::new(hazard),
    ));
}

fn spawn_boss(commands: &mut Commands, authored: &crate::rooms::Authored<ae::BossBrain>) {
    let boss = BossRuntime::new(
        authored.id.clone(),
        authored.name.clone(),
        authored.aabb,
        authored.payload.clone(),
    );
    let initial_phase = BossPhase::from_alive(boss.alive);
    commands.spawn((
        Name::new(format!("Feature boss: {}", authored.name)),
        FeatureSimEntity,
        RoomVisual,
        FeatureId::new(authored.id.clone()),
        FeatureName::new(authored.name.clone()),
        FeatureAabb::from_center_size(boss.pos, boss.render_size()),
        BossPatternTimer(boss.pattern_timer),
        initial_phase,
        super::ActorFaction::Boss,
        super::ActorTarget::default(),
        BossFeature::new(boss),
    ));
}

fn spawn_pickup(commands: &mut Commands, authored: &crate::rooms::Authored<ae::Pickup>) {
    let feature_aabb = FeatureAabb::from_aabb(authored.aabb);
    commands.spawn((
        Name::new(format!("Feature pickup: {}", authored.name)),
        PickupBundle::new(
            &authored.id,
            &authored.name,
            feature_aabb,
            authored.payload.clone(),
        ),
    ));
}

fn spawn_chest(commands: &mut Commands, authored: &crate::rooms::Authored<ae::Chest>) {
    let feature_aabb = FeatureAabb::from_aabb(authored.aabb);
    commands.spawn((
        Name::new(format!("Feature chest: {}", authored.name)),
        ChestBundle::new(
            &authored.id,
            &authored.name,
            feature_aabb,
            authored.payload.clone(),
        ),
    ));
}

fn spawn_breakable(commands: &mut Commands, authored: &crate::rooms::Authored<ae::Breakable>) {
    let feature_aabb = FeatureAabb::from_aabb(authored.aabb);
    let breakable = &authored.payload;
    let mut entity = commands.spawn((
        Name::new(format!("Feature breakable: {}", authored.name)),
        FeatureSimEntity,
        RoomVisual,
        FeatureId::new(authored.id.clone()),
        FeatureName::new(authored.name.clone()),
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

fn spawn_enemy(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<ae::EnemyBrain>,
    paths: &[(String, ae::KinematicPath)],
) {
    let feature_aabb = FeatureAabb::from_aabb(authored.aabb);
    let actor = ActorRuntime::Hostile(EnemyRuntime::new(
        authored.id.clone(),
        authored.name.clone(),
        authored.aabb,
        authored.payload.clone(),
        paths,
    ));
    let (identity, disposition, health, combat, intent, cooldowns) =
        actor_component_snapshot(&actor);
    commands.spawn((
        Name::new(format!("Feature actor enemy: {}", authored.name)),
        EnemyActorBundle {
            base: FeatureBaseBundle::new(&authored.id, &authored.name, feature_aabb),
            identity,
            disposition,
            faction: super::ActorFaction::Enemy,
            target: super::ActorTarget::default(),
            health,
            combat,
            intent,
            cooldowns,
        },
        actor,
    ));
}

fn spawn_interactable(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<ae::Interactable>,
    paths: &[(String, ae::KinematicPath)],
) {
    let feature_aabb = FeatureAabb::from_aabb(authored.aabb);
    let interactable = &authored.payload;
    if matches!(interactable.kind, ae::InteractionKind::Npc { .. }) {
        let npc = NpcRuntime::new_with_paths(
            authored.id.clone(),
            authored.name.clone(),
            authored.aabb,
            interactable.clone(),
            paths,
        );
        // Build the brain from the authored NPC fields before
        // wrapping into the ActorRuntime variant. Patrol-radius > 0
        // or an authored motion path → Patrol brain; otherwise
        // StandStill. ActionSet stays peaceful by default.
        let brain = npc.build_brain();
        let actor = ActorRuntime::Peaceful(npc);
        let (identity, disposition, health, combat, intent, cooldowns) =
            actor_component_snapshot(&actor);
        commands.spawn((
            Name::new(format!("Feature actor npc: {}", authored.name)),
            EnemyActorBundle {
                base: FeatureBaseBundle::new(&authored.id, &authored.name, feature_aabb),
                identity,
                disposition,
                faction: super::ActorFaction::Npc,
                target: super::ActorTarget::default(),
                health,
                combat,
                intent,
                cooldowns,
            },
            actor,
            brain,
            crate::brain::ActionSet::peaceful(),
            crate::brain::ActorControl::default(),
        ));
    } else if let ae::InteractionKind::Custom(payload) = &interactable.kind {
        if let Some(activation) = crate::encounter::SwitchActivation::parse_custom(payload) {
            commands.spawn((
                Name::new(format!("Feature switch: {}", authored.name)),
                FeatureSimEntity,
                RoomVisual,
                FeatureId::new(authored.id.clone()),
                FeatureName::new(authored.name.clone()),
                feature_aabb,
                SwitchFeature::new(activation),
                SwitchOn(false),
            ));
        }
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
    let feature_aabb = FeatureAabb::from_center_size(pos, size);
    commands.spawn((
        Name::new(format!("Encounter mob: {id}")),
        EnemyActorBundle {
            base: FeatureBaseBundle::new(&id, &id, feature_aabb),
            identity,
            disposition,
            faction: super::ActorFaction::Enemy,
            target: super::ActorTarget::default(),
            health,
            combat,
            intent,
            cooldowns,
        },
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
