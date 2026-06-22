//! Spawn-time NPC<->enemy conversion: building the enemy/NPC component
//! snapshots an authored actor needs, and the hostile-NPC conversion plan.

use super::super::*;
use super::*;

/// Build the enemy component seed an NPC uses when its aggression policy
/// flips it hostile. The NPC keeps the same entity identity; only the
/// NPC-only cluster is replaced with the enemy cluster.
fn enemy_cluster_for_hostile_npc(
    config: &super::super::npc_clusters::NpcConfig,
    kin: &super::super::enemy_clusters::BodyKinematics,
    surface: &ActorSurfaceState,
) -> super::super::enemy_clusters::EnemyClusterSeed {
    let brain_id = hostile_enemy_brain_for_npc(config);
    let mut enemy = super::super::enemy_clusters::EnemyClusterSeed::new(
        config.id.clone(),
        config.name.clone(),
        ae::Aabb::new(kin.pos, kin.size * 0.5),
        crate::actor::EnemyBrain::Custom(brain_id.into()),
        &[],
    );
    enemy.kin.pos = kin.pos;
    enemy.config.spawn.pos = config.spawn;
    enemy.kin.size = ae::Vec2::new(kin.size.x.max(22.0), kin.size.y.max(38.0));
    enemy.config.spawn.size = enemy.kin.size;
    enemy.kin.vel = kin.vel;
    enemy.kin.facing = kin.facing;
    enemy.surface.on_ground = surface.on_ground;
    if config.name != "Kernel Guide NPC" {
        enemy.config.sprite_override_npc_name = Some(config.name.clone());
    }
    enemy
}

pub(crate) fn hostile_enemy_spec_for_npc(
    config: &super::super::npc_clusters::NpcConfig,
) -> super::super::super::enemies::EnemyArchetypeSpec {
    let brain = crate::actor::EnemyBrain::Custom(hostile_enemy_brain_for_npc(config).into());
    super::super::super::enemies::spec_for_brain(&brain)
}

fn hostile_enemy_brain_for_npc(config: &super::super::npc_clusters::NpcConfig) -> &'static str {
    let dialogue_id = match &config.interactable.kind {
        crate::interaction::InteractionKind::Npc { dialogue_id, .. } => dialogue_id.as_deref(),
        _ => None,
    };
    let id = config.id.to_ascii_lowercase();
    let name = config.name.to_ascii_lowercase();
    let dialogue = dialogue_id.unwrap_or("").to_ascii_lowercase();
    let looks_like_pirate_heavy = id.contains("pirate_heavy")
        || name.contains("broadside bess")
        || name.contains("iron mary")
        || name.contains("salt annet")
        || dialogue.contains("pirate_heavy");
    if looks_like_pirate_heavy {
        return "pirate_heavy";
    }
    let looks_like_pirate = id.contains("pirate")
        || name.contains("pirate")
        || name.contains("quartermaster")
        || name.contains("lookout")
        || name.contains("navigator")
        || dialogue.contains("pirate");
    if looks_like_pirate {
        return "pirate_raider";
    }
    "medium_striker"
}

/// Build the read-model mirror components from an enemy cluster seed.
pub fn enemy_component_snapshot(
    enemy: &super::super::enemy_clusters::EnemyClusterSeed,
) -> (
    ActorIdentity,
    ActorDisposition,
    ActorHealth,
    ActorCombatState,
    ActorIntent,
    ActorCooldowns,
) {
    (
        ActorIdentity::new(enemy.config.id.clone(), enemy.config.name.clone())
            .with_sprite_override(enemy.config.sprite_override_npc_name.clone()),
        ActorDisposition::Hostile,
        ActorHealth::new(enemy.status.health),
        ActorCombatState::hostile(
            enemy.status.alive,
            enemy.status.hit_flash,
            enemy.attack.windup_timer,
            enemy.attack.active_timer,
            enemy.config.tuning.is_sandbag,
        ),
        ActorIntent::new(enemy.status.ai_mode),
        ActorCooldowns {
            attack_cooldown: enemy.attack.cooldown,
            respawn_timer: enemy.status.respawn_timer,
        },
    )
}

/// Flip an entity from peaceful NPC to hostile enemy in place: attach
/// the enemy cluster components, set the `Enemy` marker, and mirror the
/// read-model components. Shared by runtime stimulus and save-load
/// provoke paths.
fn make_entity_enemy(
    commands: &mut Commands,
    entity: Entity,
    actor: &mut ActorRuntime,
    hostile: &super::super::enemy_clusters::EnemyClusterSeed,
    identity: &mut ActorIdentity,
    disposition: &mut ActorDisposition,
    health: &mut ActorHealth,
    combat: &mut ActorCombatState,
    intent: &mut ActorIntent,
    cooldowns: &mut ActorCooldowns,
) {
    *actor = ActorRuntime::Enemy;
    commands
        .entity(entity)
        // Drop the NPC-only cluster components so the entity stops
        // matching `NpcClusterQueryData` (and thus `update_ecs_npcs`);
        // the shared kin/surface/motion components are overwritten by
        // the enemy bundle below.
        .remove::<(
            super::super::npc_clusters::NpcConfig,
            super::super::npc_clusters::NpcStatus,
        )>()
        .insert(hostile.clone().into_components());
    let (next_id, next_disp, next_health, next_combat, next_intent, next_cd) =
        enemy_component_snapshot(hostile);
    *identity = next_id;
    *disposition = next_disp;
    *health = next_health;
    *combat = next_combat;
    *intent = next_intent;
    *cooldowns = next_cd;
}

/// Complete conversion recipe for an NPC that becomes hostile.
///
/// The plan keeps the enemy cluster seed together with the brain/action pair
/// derived from the current combat kit and held item. Callers can tweak the
/// seed for the trigger source or save flags, then apply it to the entity in
/// one place.
pub(crate) struct HostileNpcConversionPlan {
    hostile: super::super::enemy_clusters::EnemyClusterSeed,
    brain: crate::brain::Brain,
    action_set: crate::brain::ActionSet,
}

impl HostileNpcConversionPlan {
    pub(crate) fn from_npc(
        config: &super::super::npc_clusters::NpcConfig,
        kin: &super::super::enemy_clusters::BodyKinematics,
        surface: &ActorSurfaceState,
        combat_kit: &CombatKit,
        held_item: Option<&HeldItem>,
    ) -> Self {
        let hostile = enemy_cluster_for_hostile_npc(config, kin, surface);
        Self::from_hostile_cluster(hostile, combat_kit, held_item)
    }

    pub(crate) fn from_hostile_cluster(
        hostile: super::super::enemy_clusters::EnemyClusterSeed,
        combat_kit: &CombatKit,
        held_item: Option<&HeldItem>,
    ) -> Self {
        let (brain, action_set) =
            super::super::brain_builders::aggressive_brain_and_action_set_for_enemy(
                &hostile.config,
                combat_kit,
                held_item,
            );
        Self {
            hostile,
            brain,
            action_set,
        }
    }

    pub(crate) fn with_chase(mut self) -> Self {
        self.hostile.status.ai_mode = crate::actor::ai::CharacterAiMode::Chase;
        self
    }

    pub(crate) fn with_dead_state(mut self) -> Self {
        self.hostile.status.alive = false;
        self.hostile.status.health.current = 0;
        self
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply(
        self,
        commands: &mut Commands,
        entity: Entity,
        actor: &mut ActorRuntime,
        identity: &mut ActorIdentity,
        disposition: &mut ActorDisposition,
        health: &mut ActorHealth,
        combat: &mut ActorCombatState,
        intent: &mut ActorIntent,
        cooldowns: &mut ActorCooldowns,
    ) {
        make_entity_enemy(
            commands,
            entity,
            actor,
            &self.hostile,
            identity,
            disposition,
            health,
            combat,
            intent,
            cooldowns,
        );
        commands
            .entity(entity)
            .insert((self.brain, self.action_set));
    }
}

type ActorSnapshot = (
    ActorIdentity,
    ActorDisposition,
    ActorHealth,
    ActorCombatState,
    ActorIntent,
    ActorCooldowns,
);

/// Build the read-model mirror components for an NPC from its clusters.
pub fn npc_component_snapshot(
    config: &super::super::npc_clusters::NpcConfig,
    status: &super::super::npc_clusters::NpcStatus,
) -> ActorSnapshot {
    (
        ActorIdentity::new(config.id.clone(), config.name.clone()),
        ActorDisposition::Peaceful,
        ActorHealth::new(crate::actor::Health::new(1)),
        // `strike_count` is a write-only read-model field (no behavioral
        // reader); the provoke accumulator now lives on `ActorAggression`.
        ActorCombatState::peaceful(0, status.hit_flash),
        ActorIntent::new(crate::actor::ai::CharacterAiMode::Idle),
        ActorCooldowns::default(),
    )
}

/// Mirror an NPC's clusters onto the read-model components.
pub(crate) fn sync_actor_components_from_npc(
    npc: &super::super::npc_clusters::NpcMut<'_>,
    identity: &mut ActorIdentity,
    disposition: &mut ActorDisposition,
    health: &mut ActorHealth,
    combat: &mut ActorCombatState,
    intent: &mut ActorIntent,
    cooldowns: &mut ActorCooldowns,
) {
    let (i, d, h, c, it, cd) = npc_component_snapshot(npc.config, npc.status);
    *identity = i;
    *disposition = d;
    *health = h;
    *combat = c;
    *intent = it;
    *cooldowns = cd;
}
