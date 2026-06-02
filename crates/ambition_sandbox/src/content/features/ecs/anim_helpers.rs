//! ECS read-only lookup helpers for sprite/animation systems.
//!
//! Presentation code calls these by id to drive enemy/npc/boss sprite
//! swaps, hit-flash, and animation rows without taking on a query for
//! every feature family itself.

use super::*;

/// QueryData tuple for actor-sprite lookups: the runtime variant tag
/// plus the enemy cluster components (`None` on NPCs). Systems declare
/// `Query<ActorSpriteData>`; the helpers take `&Query<ActorSpriteData>`.
pub type ActorSpriteData = (
    &'static FeatureId,
    &'static ActorRuntime,
    Option<&'static super::enemy_clusters::ActorKinematics>,
    Option<&'static super::enemy_clusters::EnemyStatus>,
    Option<&'static ActorAttackState>,
    Option<&'static super::enemy_clusters::EnemyConfig>,
    Option<&'static super::npc_clusters::NpcConfig>,
    Option<&'static super::npc_clusters::NpcStatus>,
);

pub fn ecs_npc_name(id: &str, actors: &Query<ActorSpriteData>) -> Option<String> {
    actors
        .iter()
        .find_map(|(feature_id, actor, _, _, _, config, npc_config, _)| {
            if feature_id.as_str() != id {
                return None;
            }
            match actor {
                ActorRuntime::Npc => npc_config.map(|c| c.name.clone()),
                ActorRuntime::Enemy => config.and_then(|c| c.sprite_override_npc_name.clone()),
            }
        })
}

pub fn ecs_enemy_sprite_override(id: &str, actors: &Query<ActorSpriteData>) -> Option<String> {
    actors
        .iter()
        .find_map(|(feature_id, actor, _, _, _, config, _, _)| {
            if feature_id.as_str() != id {
                return None;
            }
            match actor {
                ActorRuntime::Enemy => config.and_then(|c| c.sprite_override_npc_name.clone()),
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
pub fn ecs_enemy_name(id: &str, actors: &Query<ActorSpriteData>) -> Option<String> {
    actors
        .iter()
        .find_map(|(feature_id, actor, _, _, _, config, _, _)| {
            if feature_id.as_str() != id {
                return None;
            }
            match actor {
                ActorRuntime::Enemy => config.map(|c| c.name.clone()),
                _ => None,
            }
        })
}

pub fn ecs_enemy_anim_state(
    id: &str,
    actors: &Query<ActorSpriteData>,
) -> Option<crate::presentation::character_sprites::EnemyAnimState> {
    actors
        .iter()
        .find_map(|(feature_id, actor, kin, status, attack, _, _, _)| {
            if feature_id.as_str() != id {
                return None;
            }
            match actor {
                ActorRuntime::Enemy => {
                    let kin = kin?;
                    let status = status?;
                    let attack = attack?;
                    Some(crate::presentation::character_sprites::EnemyAnimState {
                        vel: kin.vel,
                        facing: kin.facing,
                        alive: status.alive,
                        attack_active: attack.is_active(),
                        attack_windup: attack.is_winding_up(),
                        hit_flash: status.hit_flash > 0.0,
                    })
                }
                _ => None,
            }
        })
}

pub fn ecs_npc_anim_state(
    id: &str,
    actors: &Query<ActorSpriteData>,
) -> Option<crate::presentation::character_sprites::NpcAnimState> {
    actors
        .iter()
        .find_map(|(feature_id, actor, kin, _, _, _, _, npc_status)| {
            if feature_id.as_str() != id {
                return None;
            }
            match actor {
                ActorRuntime::Npc => {
                    let kin = kin?;
                    let status = npc_status?;
                    Some(crate::presentation::character_sprites::NpcAnimState {
                        vel: kin.vel,
                        facing: kin.facing,
                        hit_flash: status.hit_flash > 0.0,
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
    bosses: &'a Query<(&FeatureId, super::boss_clusters::BossClusterRef, &crate::brain::BossAttackState)>,
) -> Option<&'a str> {
    bosses.iter().find_map(|(feature_id, boss, _)| {
        (feature_id.as_str() == id).then_some(boss.config.name.as_str())
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
    boss: super::boss_clusters::BossRef<'_>,
    attack_state: &crate::brain::BossAttackState,
    brain: &crate::brain::Brain,
) -> crate::boss_encounter::sprites::BossAnimState {
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
        alive: boss.status.alive,
        attack_active: attack_state.active_profile.is_some(),
        attack_windup: attack_state.telegraph_profile.is_some(),
        hit_flash: boss.status.hit_flash > 0.0,
        windup_anim: attack_state
            .telegraph_profile
            .as_ref()
            .and_then(boss_anim_for_attack_profile),
        active_anim: attack_state
            .active_profile
            .as_ref()
            .and_then(boss_anim_for_attack_profile),
        pattern_timer,
        facing: boss.kin.facing,
    }
}

pub fn ecs_boss_anim_state_and_entity(
    id: &str,
    bosses: &Query<(
        bevy::prelude::Entity,
        &FeatureId,
        super::boss_clusters::BossClusterRef,
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
            Some((entity, boss_anim_state_for(boss.as_boss_ref(), attack_state, brain)))
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
        super::boss_clusters::BossClusterRef,
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
                            profile: Some(profile.clone()),
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
                                profile: Some(profile.clone()),
                                frame_index,
                                animation_key: boss_animation_key_for_sample(profile, anim),
                            },
                        ));
                    }
                }
            }
            // Idle/rest: not driven by any attack profile, but still emit
            // a sample so the rest-pose hurtbox bobs with the breathing
            // animation instead of locking to frame 0. Hit/Death rows are
            // deliberately left as `None` — geometry should stay on the
            // rest-pose shape rather than chase a recoil/death frame.
            if result.is_none() && anim == crate::boss_encounter::sprites::BossAnim::Rest {
                result = Some((
                    entity,
                    crate::features::BossAnimationFrameSample {
                        profile: None,
                        frame_index,
                        animation_key: Some("rest"),
                    },
                ));
            }
            result
        })
}

pub fn ecs_boss_anim_state(
    id: &str,
    bosses: &Query<(
        &FeatureId,
        super::boss_clusters::BossClusterRef,
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
            Some(boss_anim_state_for(boss.as_boss_ref(), attack_state, brain))
        })
}
