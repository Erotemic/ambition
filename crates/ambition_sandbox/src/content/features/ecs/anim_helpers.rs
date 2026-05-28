//! ECS read-only lookup helpers for sprite/animation systems.
//!
//! Presentation code calls these by id to drive enemy/npc/boss sprite
//! swaps, hit-flash, and animation rows without taking on a query for
//! every feature family itself.

use super::*;

pub fn ecs_npc_name<'a>(
    id: &str,
    actors: &'a Query<(&FeatureId, &ActorRuntime)>,
) -> Option<&'a str> {
    actors.iter().find_map(|(feature_id, actor)| {
        if feature_id.as_str() != id {
            return None;
        }
        match actor {
            ActorRuntime::Peaceful(npc) => Some(npc.name.as_str()),
            ActorRuntime::Hostile(enemy) => enemy.sprite_override_npc_name.as_deref(),
        }
    })
}

pub fn ecs_enemy_sprite_override<'a>(
    id: &str,
    actors: &'a Query<(&FeatureId, &ActorRuntime)>,
) -> Option<&'a str> {
    actors.iter().find_map(|(feature_id, actor)| {
        if feature_id.as_str() != id {
            return None;
        }
        match actor {
            ActorRuntime::Hostile(enemy) => enemy.sprite_override_npc_name.as_deref(),
            _ => None,
        }
    })
}

/// Per-enemy display name, used by `upgrade_enemy_sprites` as a
/// fallback sprite-lookup key when no explicit `sprite_override` is
/// set. Lets direct `EnemySpawn` entities (no NPC migration history)
/// pick up a content sheet by name — e.g. the intro raiders resolve
/// to `fascist_enforcer_spritesheet`
/// via the intro NPC sprite registry without authors having to
/// double-register them as an `enemy_sprite_registry`.
pub fn ecs_enemy_name<'a>(
    id: &str,
    actors: &'a Query<(&FeatureId, &ActorRuntime)>,
) -> Option<&'a str> {
    actors.iter().find_map(|(feature_id, actor)| {
        if feature_id.as_str() != id {
            return None;
        }
        match actor {
            ActorRuntime::Hostile(enemy) => Some(enemy.name.as_str()),
            _ => None,
        }
    })
}

pub fn ecs_enemy_anim_state(
    id: &str,
    actors: &Query<(&FeatureId, &ActorRuntime)>,
) -> Option<crate::presentation::character_sprites::EnemyAnimState> {
    actors.iter().find_map(|(feature_id, actor)| {
        if feature_id.as_str() != id {
            return None;
        }
        match actor {
            ActorRuntime::Hostile(enemy) => {
                Some(crate::presentation::character_sprites::EnemyAnimState {
                    vel: enemy.vel,
                    facing: enemy.facing,
                    alive: enemy.alive,
                    attack_active: enemy.attack_timer > 0.0,
                    attack_windup: enemy.attack_windup_timer > 0.0,
                    hit_flash: enemy.hit_flash > 0.0,
                })
            }
            _ => None,
        }
    })
}

pub fn ecs_npc_anim_state(
    id: &str,
    actors: &Query<(&FeatureId, &ActorRuntime)>,
) -> Option<crate::presentation::character_sprites::NpcAnimState> {
    actors.iter().find_map(|(feature_id, actor)| {
        if feature_id.as_str() != id {
            return None;
        }
        match actor {
            ActorRuntime::Peaceful(npc) => {
                Some(crate::presentation::character_sprites::NpcAnimState {
                    vel: npc.vel,
                    facing: npc.facing,
                    hit_flash: npc.hit_flash > 0.0,
                })
            }
            _ => None,
        }
    })
}

/// ECS chest-opened lookup for sprite swapping.
pub fn ecs_chest_opened(
    id: &str,
    chests: &Query<(&FeatureId, Option<&Opened>), With<ChestFeature>>,
) -> Option<bool> {
    chests
        .iter()
        .find(|(feature_id, _)| feature_id.as_str() == id)
        .map(|(_, opened)| opened.is_some())
}

/// ECS breakable-state lookup for sprite swapping.
pub fn ecs_breakable_state(
    id: &str,
    breakables: &Query<(&FeatureId, &BreakableFeature)>,
) -> Option<crate::interaction::BreakableState> {
    breakables
        .iter()
        .find(|(feature_id, _)| feature_id.as_str() == id)
        .map(|(_, breakable)| breakable.breakable.state)
}

pub fn ecs_boss_name<'a>(
    id: &str,
    bosses: &'a Query<(&FeatureId, &BossFeature, &crate::brain::BossAttackState)>,
) -> Option<&'a str> {
    bosses.iter().find_map(|(feature_id, boss, _)| {
        (feature_id.as_str() == id).then_some(boss.boss.name.as_str())
    })
}

pub fn ecs_boss_anim_state(
    id: &str,
    bosses: &Query<(
        &FeatureId,
        &BossFeature,
        &crate::brain::BossAttackState,
        &crate::brain::Brain,
    )>,
) -> Option<crate::boss_encounter::sprites::BossAnimState> {
    bosses
        .iter()
        .find_map(|(feature_id, boss, attack_state, brain)| {
            if feature_id.as_str() != id {
                return None;
            }
            let boss = &boss.boss;
            // attack_active / attack_windup read the brain's
            // BossAttackState (single source of truth) instead of
            // mirror fields on BossRuntime. pattern_timer comes from
            // the brain's BossPatternState; non-BossPattern brains
            // (test fixtures) fall back to 0.0.
            let pattern_timer = brain
                .boss_pattern_state()
                .map(|s| s.pattern_timer)
                .unwrap_or(0.0);
            Some(crate::boss_encounter::sprites::BossAnimState {
                alive: boss.alive,
                attack_active: attack_state.active_profile.is_some(),
                attack_windup: attack_state.telegraph_profile.is_some(),
                hit_flash: boss.hit_flash > 0.0,
                pattern_timer,
            })
        })
}
