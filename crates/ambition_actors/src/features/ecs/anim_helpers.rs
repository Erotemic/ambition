//! ECS read-only lookup helpers for sprite/animation systems.
//!
//! Presentation code calls these by id to drive enemy/npc/boss sprite
//! swaps, hit-flash, and animation rows without taking on a query for
//! every feature family itself.

use super::*;

/// Advance every non-player actor's movement-driven anim overlays (landing /
/// dash-startup) one frame, via the SAME [`crate::features::advance_body_anim_overlays`]
/// the player tick runs — so [`crate::character_sprites::pick_actor_anim`] can show
/// those poses (fable review §A9). The home player ([`crate::actor::PlayerEntity`])
/// is excluded (it advances its own overlays in the player tick), so no body is
/// advanced twice; a possessed non-player body IS advanced here. Uses `sim_dt`
/// (world-anchored animation), so the poses pause and slow with the sim. Scheduled
/// right before [`rebuild_actor_anim_index`] (its reader) and skipped headless
/// with it — these overlays are presentation-only.
pub fn advance_actor_anim_overlays(
    world_time: Res<ambition_time::WorldTime>,
    mut actors: Query<
        (
            &crate::actor::BodyGroundState,
            &crate::actor::BodyKinematics,
            &ambition_engine_core::BodyMotionFacts,
            &mut crate::actor::BodyAnimFacts,
        ),
        Without<crate::actor::PlayerEntity>,
    >,
) {
    let dt = world_time.sim_dt();
    for (ground, kin, facts, mut anim) in &mut actors {
        crate::features::advance_body_anim_overlays(
            ground.on_ground,
            kin.vel.y,
            facts.dashing,
            &mut anim,
            dt,
        );
    }
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
) -> Option<ambition_interaction::BreakableState> {
    breakables
        .iter()
        .find(|(feature_id, _)| feature_id.as_str() == id)
        .map(|(_, breakable)| breakable.breakable.state)
}

// `ecs_boss_name` is GONE: the boss's static identity (name + behavior id) is
// materialized into `BossRenderIndex` (see `rebuild_boss_render_index`), which
// `upgrade_boss_sprites` reads by id — so binding a boss sheet no longer
// live-queries the boss clusters.

fn boss_anim_for_attack_profile(
    profile: &ambition_characters::brain::BossAttackProfile,
) -> Option<crate::boss_encounter::sprites::BossAnim> {
    use crate::boss_encounter::sprites::BossAnim;
    match profile.move_id().as_str() {
        "floor_slam" | "hand_slam" | "converging_shockwave" => Some(BossAnim::FloorSlam),
        "side_sweep" | "hand_sweep" | "broadside" => Some(BossAnim::SideSweep),
        "hazard_column" | "dive_lane" => Some(BossAnim::DashEcho),
        "wing_sweep" => None,
        // `full_body_pulse`, `head_descent`, and every content special fall back
        // to the spike-halo telegraph anim (a ring of damage around the boss) —
        // the closest generic visual cue. Covers the former DebrisRain /
        // MemorizedVolley / LockOnBeam / PitTrap / RotatingCross / MinionCascade.
        _ => Some(BossAnim::SpikeHalo),
    }
}

fn boss_animation_key_for_sample(
    profile: &ambition_characters::brain::BossAttackProfile,
    anim: crate::boss_encounter::sprites::BossAnim,
) -> Option<&'static str> {
    use crate::boss_encounter::sprites::BossAnim;
    match (profile.move_id().as_str(), anim) {
        // GNU-ton has profile-specific dangerous boxes (for example
        // `gnu_shockwave`) but the damageable head/body box should follow
        // the rendered row. Keep the sample keyed to the visual row so
        // authored row frames are the source of truth for hurtboxes.
        ("hand_slam" | "converging_shockwave", BossAnim::FloorSlam) => Some("hand_slam"),
        ("hand_sweep", BossAnim::SideSweep) => Some("hand_sweep"),
        ("head_descent", BossAnim::SpikeHalo) => Some("head_down"),
        // GNU-ton's apple rain reads the head row for its damageable hurtbox.
        ("apple_rain", BossAnim::SpikeHalo) => Some("head_down"),
        _ => super::super::bosses::boss_animation_keys_for_profile(profile)
            .first()
            .copied(),
    }
}

pub fn boss_anim_state_for(
    boss: super::boss_clusters::BossRef<'_>,
    // Liveness + damage-blink from the boss's shared body components (§A1).
    alive: bool,
    hit_flash: f32,
    attack_state: &ambition_characters::brain::BossAttackState,
    brain: &ambition_characters::brain::Brain,
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
        alive,
        attack_active: attack_state.active_profile.is_some(),
        attack_windup: attack_state.telegraph_profile.is_some(),
        hit_flash: hit_flash > 0.0,
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
        pos: boss.kin.pos,
    }
}

pub fn ecs_boss_anim_state_and_entity(
    id: &str,
    bosses: &Query<(
        bevy::prelude::Entity,
        &FeatureId,
        super::boss_clusters::BossClusterRef,
        &ambition_characters::actor::BodyHealth,
        &ambition_characters::actor::BodyCombat,
        &ambition_characters::brain::BossAttackState,
        &ambition_characters::brain::Brain,
    )>,
) -> Option<(
    bevy::prelude::Entity,
    crate::boss_encounter::sprites::BossAnimState,
)> {
    bosses.iter().find_map(
        |(entity, feature_id, boss, health, combat, attack_state, brain)| {
            if feature_id.as_str() != id {
                return None;
            }
            Some((
                entity,
                boss_anim_state_for(
                    boss.as_boss_ref(),
                    health.alive(),
                    combat.hit_flash,
                    attack_state,
                    brain,
                ),
            ))
        },
    )
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
        &ambition_characters::actor::BodyHealth,
        &ambition_characters::actor::BodyCombat,
        &ambition_characters::brain::BossAttackState,
        &ambition_characters::brain::Brain,
    )>,
    anim: crate::boss_encounter::sprites::BossAnim,
    frame_index: usize,
) -> Option<(
    bevy::prelude::Entity,
    crate::features::BossAnimationFrameSample,
)> {
    bosses.iter().find_map(
        |(entity, feature_id, _boss, _health, _combat, attack_state, _brain)| {
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
        },
    )
}

pub fn ecs_boss_anim_state(
    id: &str,
    bosses: &Query<(
        &FeatureId,
        super::boss_clusters::BossClusterRef,
        &ambition_characters::actor::BodyHealth,
        &ambition_characters::actor::BodyCombat,
        &ambition_characters::brain::BossAttackState,
        &ambition_characters::brain::Brain,
    )>,
) -> Option<crate::boss_encounter::sprites::BossAnimState> {
    bosses
        .iter()
        .find_map(|(feature_id, boss, health, combat, attack_state, brain)| {
            if feature_id.as_str() != id {
                return None;
            }
            Some(boss_anim_state_for(
                boss.as_boss_ref(),
                health.alive(),
                combat.hit_flash,
                attack_state,
                brain,
            ))
        })
}
