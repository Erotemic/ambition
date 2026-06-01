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
            ActorRuntime::Npc(npc) => Some(npc.name.as_str()),
            ActorRuntime::Enemy(enemy) => enemy.sprite_override_npc_name.as_deref(),
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
            ActorRuntime::Enemy(enemy) => enemy.sprite_override_npc_name.as_deref(),
            _ => None,
        }
    })
}

/// Per-enemy display name, used by `upgrade_enemy_sprites` as a
/// fallback sprite-lookup key when no explicit `sprite_override` is
/// set. Lets direct `EnemySpawn` entities (no NPC migration history)
/// pick up a content sheet by name — e.g. the intro raiders resolve
/// to `raid_enforcer_spritesheet`
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
            ActorRuntime::Enemy(enemy) => Some(enemy.name.as_str()),
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
            ActorRuntime::Enemy(enemy) => {
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
            ActorRuntime::Npc(npc) => Some(crate::presentation::character_sprites::NpcAnimState {
                vel: npc.vel,
                facing: npc.facing,
                hit_flash: npc.hit_flash > 0.0,
            }),
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

fn boss_anim_for_attack_profile(
    profile: &crate::brain::BossAttackProfile,
) -> Option<crate::boss_encounter::sprites::BossAnim> {
    use crate::boss_encounter::sprites::BossAnim;
    use crate::brain::BossAttackProfile;
    match profile {
        BossAttackProfile::FloorSlam
        | BossAttackProfile::GnuHandSlam
        | BossAttackProfile::GnuShockwave => Some(BossAnim::FloorSlam),
        BossAttackProfile::SideSweep
        | BossAttackProfile::GnuHandSweep
        | BossAttackProfile::Broadside => Some(BossAnim::SideSweep),
        BossAttackProfile::FullBodyPulse
        | BossAttackProfile::GnuHeadDescent
        | BossAttackProfile::GnuAppleRain
        | BossAttackProfile::OverfitVolley
        | BossAttackProfile::EyeBeam
        | BossAttackProfile::MinimaTrap
        | BossAttackProfile::SaddlePoint
        | BossAttackProfile::GradientCascade => Some(BossAnim::SpikeHalo),
        BossAttackProfile::GradientLane | BossAttackProfile::DiveLane => Some(BossAnim::DashEcho),
        BossAttackProfile::WingSweep => None,
    }
}

fn boss_animation_key_for_sample(
    profile: &crate::brain::BossAttackProfile,
    anim: crate::boss_encounter::sprites::BossAnim,
) -> Option<&'static str> {
    use crate::boss_encounter::sprites::BossAnim;
    use crate::brain::BossAttackProfile;
    match (profile, anim) {
        // GNU-ton has profile-specific dangerous boxes (for example
        // `gnu_shockwave`) but the damageable head/body box should follow
        // the rendered row. Keep the sample keyed to the visual row so
        // authored row frames are the source of truth for hurtboxes.
        (BossAttackProfile::GnuHandSlam | BossAttackProfile::GnuShockwave, BossAnim::FloorSlam) => {
            Some("hand_slam")
        }
        (BossAttackProfile::GnuHandSweep, BossAnim::SideSweep) => Some("hand_sweep"),
        (
            BossAttackProfile::GnuHeadDescent | BossAttackProfile::GnuAppleRain,
            BossAnim::SpikeHalo,
        ) => Some("head_down"),
        _ => super::super::bosses::boss_animation_keys_for_profile(profile)
            .first()
            .copied(),
    }
}

fn boss_anim_state_for(
    boss: &BossFeature,
    attack_state: &crate::brain::BossAttackState,
    brain: &crate::brain::Brain,
) -> crate::boss_encounter::sprites::BossAnimState {
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
    crate::boss_encounter::sprites::BossAnimState {
        alive: boss.alive,
        attack_active: attack_state.active_profile.is_some(),
        attack_windup: attack_state.telegraph_profile.is_some(),
        hit_flash: boss.hit_flash > 0.0,
        windup_anim: attack_state
            .telegraph_profile
            .as_ref()
            .and_then(boss_anim_for_attack_profile),
        active_anim: attack_state
            .active_profile
            .as_ref()
            .and_then(boss_anim_for_attack_profile),
        pattern_timer,
        facing: boss.facing,
    }
}

pub fn ecs_boss_anim_state_and_entity(
    id: &str,
    bosses: &Query<(
        bevy::prelude::Entity,
        &FeatureId,
        &BossFeature,
        &crate::brain::BossAttackState,
        &crate::brain::Brain,
    )>,
) -> Option<(
    bevy::prelude::Entity,
    crate::boss_encounter::sprites::BossAnimState,
)> {
    bosses
        .iter()
        .find_map(|(entity, feature_id, boss, attack_state, brain)| {
            if feature_id.as_str() != id {
                return None;
            }
            Some((entity, boss_anim_state_for(boss, attack_state, brain)))
        })
}

/// Return the currently rendered attack-frame sample for a boss,
/// but only when the chosen visual row is directly driven by the
/// boss attack profile.
///
/// Hit/death/rest overrides deliberately return `None`; geometry
/// callers then fall back to elapsed-time sampling instead of using a
/// frame from the wrong visual row.
pub fn ecs_boss_animation_frame_sample(
    id: &str,
    bosses: &Query<(
        bevy::prelude::Entity,
        &FeatureId,
        &BossFeature,
        &crate::brain::BossAttackState,
        &crate::brain::Brain,
    )>,
    anim: crate::boss_encounter::sprites::BossAnim,
    frame_index: usize,
) -> Option<(
    bevy::prelude::Entity,
    crate::features::BossAnimationFrameSample,
)> {
    bosses
        .iter()
        .find_map(|(entity, feature_id, _boss, attack_state, _brain)| {
            if feature_id.as_str() != id {
                return None;
            }
            let active_expected = attack_state
                .active_profile
                .as_ref()
                .and_then(boss_anim_for_attack_profile);
            let telegraph_expected = attack_state
                .telegraph_profile
                .as_ref()
                .and_then(boss_anim_for_attack_profile);
            let mut result = None;
            if let Some(profile) = attack_state.active_profile.as_ref() {
                if active_expected == Some(anim) {
                    result = Some((
                        entity,
                        crate::features::BossAnimationFrameSample {
                            profile: profile.clone(),
                            frame_index,
                            animation_key: boss_animation_key_for_sample(profile, anim),
                        },
                    ));
                }
            }
            if result.is_none() {
                if let Some(profile) = attack_state.telegraph_profile.as_ref() {
                    if telegraph_expected == Some(anim) {
                        result = Some((
                            entity,
                            crate::features::BossAnimationFrameSample {
                                profile: profile.clone(),
                                frame_index,
                                animation_key: boss_animation_key_for_sample(profile, anim),
                            },
                        ));
                    }
                }
            }
            result
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
            Some(boss_anim_state_for(boss, attack_state, brain))
        })
}
