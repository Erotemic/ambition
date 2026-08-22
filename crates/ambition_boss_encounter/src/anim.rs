//! Boss animation-state derivation from boss-owned runtime state.

use ambition_combat::components::FeatureId;
use bevy::prelude::*;

pub fn boss_anim_state_for(
    boss: crate::BossRef<'_>,
    // Liveness + damage-blink from the boss's shared body components (§A1).
    alive: bool,
    hit_flash: f32,
    attack_state: &ambition_characters::brain::BossAttackState,
    brain: &ambition_characters::brain::Brain,
) -> crate::sprites::BossAnimState {
    // attack_active / attack_windup read the move-derived
    // BossAttackState read-model instead of mirror fields on BossRuntime.
    // pattern_timer remains durable brain cursor state; non-BossPattern
    // brains (test fixtures) fall back to 0.0.
    let pattern_timer = brain
        .boss_pattern_state()
        .map(|s| s.pattern_timer)
        .unwrap_or(0.0);
    crate::sprites::BossAnimState {
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
        crate::BossClusterRef,
        &ambition_characters::actor::BodyHealth,
        &ambition_characters::actor::BodyCombat,
        &ambition_characters::brain::BossAttackState,
        &ambition_characters::brain::Brain,
    )>,
) -> Option<(
    bevy::prelude::Entity,
    crate::sprites::BossAnimState,
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
    catalog: &crate::BossCatalog,
    id: &str,
    bosses: &Query<(
        bevy::prelude::Entity,
        &FeatureId,
        crate::BossClusterRef,
        &ambition_characters::actor::BodyHealth,
        &ambition_characters::actor::BodyCombat,
        &ambition_characters::brain::BossAttackState,
        &ambition_characters::brain::Brain,
    )>,
    anim: crate::sprites::BossAnim,
    frame_index: usize,
) -> Option<(
    bevy::prelude::Entity,
    crate::attack_geometry::BossAnimationFrameSample,
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
                        crate::attack_geometry::BossAnimationFrameSample {
                            profile: Some(profile.clone()),
                            frame_index,
                            animation_key: boss_animation_key_for_sample(catalog, profile, anim),
                        },
                    ));
                }
            }
            if result.is_none() {
                if let Some(profile) = attack_state.telegraph_profile.as_ref() {
                    if telegraph_expected == Some(anim) {
                        result = Some((
                            entity,
                            crate::attack_geometry::BossAnimationFrameSample {
                                profile: Some(profile.clone()),
                                frame_index,
                                animation_key: boss_animation_key_for_sample(
                                    catalog, profile, anim,
                                ),
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
            if result.is_none() && anim == crate::sprites::BossAnim::Rest {
                result = Some((
                    entity,
                    crate::attack_geometry::BossAnimationFrameSample {
                        profile: None,
                        frame_index,
                        animation_key: Some("rest".into()),
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
        crate::BossClusterRef,
        &ambition_characters::actor::BodyHealth,
        &ambition_characters::actor::BodyCombat,
        &ambition_characters::brain::BossAttackState,
        &ambition_characters::brain::Brain,
    )>,
) -> Option<crate::sprites::BossAnimState> {
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

fn boss_anim_for_attack_profile(
    profile: &ambition_characters::brain::BossAttackProfile,
) -> Option<crate::sprites::BossAnim> {
    use crate::sprites::BossAnim;
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
    catalog: &crate::BossCatalog,
    profile: &ambition_characters::brain::BossAttackProfile,
    anim: crate::sprites::BossAnim,
) -> Option<String> {
    use crate::sprites::BossAnim;
    match (profile.move_id().as_str(), anim) {
        // GNU-ton has profile-specific dangerous boxes (for example
        // `gnu_shockwave`) but the damageable head/body box should follow
        // the rendered row. Keep the sample keyed to the visual row so
        // authored row frames are the source of truth for hurtboxes.
        ("hand_slam" | "converging_shockwave", BossAnim::FloorSlam) => Some("hand_slam".into()),
        ("hand_sweep", BossAnim::SideSweep) => Some("hand_sweep".into()),
        ("head_descent", BossAnim::SpikeHalo) => Some("head_down".into()),
        // GNU-ton's apple rain reads the head row for its damageable hurtbox.
        ("apple_rain", BossAnim::SpikeHalo) => Some("head_down".into()),
        _ => crate::behavior::boss_animation_keys_for_profile(catalog, profile)
            .first()
            .cloned(),
    }
}

#[cfg(test)]
mod sample_key_agrees_with_profile_keys_tests {
    use super::*;
    use ambition_characters::brain::BossAttackProfile;

    /// Hardcoded sample-key overrides must name a row claimed by the driving
    /// attack profile. `apple_rain` is excluded because its special-profile row
    /// list comes from the runtime `BossCatalog`.
    #[test]
    fn every_hardcoded_sample_key_names_a_row_its_profile_claims() {
        let catalog = crate::test_boss_catalog();
        for move_id in [
            "head_descent",
            "converging_shockwave",
            "hand_slam",
            "hand_sweep",
        ] {
            let profile = BossAttackProfile::Strike(move_id.to_string());
            let anim = boss_anim_for_attack_profile(&profile)
                .unwrap_or_else(|| panic!("{move_id} maps to a boss anim"));
            let key = boss_animation_key_for_sample(catalog, &profile, anim)
                .unwrap_or_else(|| panic!("{move_id} yields a sample key"));
            let claimed =
                crate::behavior::boss_animation_keys_for_profile(catalog, &profile);
            assert!(
                claimed.iter().any(|candidate| *candidate == key),
                "the sample writer emits `{key}` for `{move_id}`, and the profile \
                 claims {claimed:?}. If the key is not among them, then swapping the \
                 four profile-identity checks to a key comparison changes which \
                 hitbox this boss presents — and the animator fold stops being a \
                 rename"
            );
        }
    }
}
