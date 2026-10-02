//! Boss animation-state derivation from boss-owned runtime state.

use ambition_combat::components::FeatureId;
use bevy::prelude::*;

pub fn boss_anim_state_for(
    // The row each attack plays is authored data in the boss catalog.
    catalog: &crate::BossCatalog,
    boss: crate::BossRef<'_>,
    // Liveness from the boss's shared body components. The damage flash is
    // not an input: this state drives the sim cursor that feeds boss
    // geometry, and a presentation duration must not move it.
    alive: bool,
    attack_state: &ambition_characters::brain::BossAttackState,
    brain: &ambition_characters::brain::Brain,
) -> ambition_sprite_sheet::boss::BossAnimState {
    // attack_active / attack_windup read the move-derived `BossAttackState`
    // read-model. pattern_timer is durable brain cursor state; non-BossPattern
    // brains (test fixtures) fall back to 0.0.
    let pattern_timer = brain
        .boss_pattern_state()
        .map(|s| s.pattern_timer)
        .unwrap_or(0.0);
    ambition_sprite_sheet::boss::BossAnimState {
        alive,
        attack_active: attack_state.active_profile.is_some(),
        attack_windup: attack_state.telegraph_profile.is_some(),
        windup_anim: attack_state
            .telegraph_profile
            .as_ref()
            .and_then(|profile| catalog.attack_animation(profile)),
        active_anim: attack_state
            .active_profile
            .as_ref()
            .and_then(|profile| catalog.attack_animation(profile)),
        pattern_timer,
        // Drawn side, not facing: an `Unmirrored` boss is drawn toward +x.
        facing: boss.drawn_side(),
        pos: boss.kin.pos,
    }
}

pub fn ecs_boss_anim_state_and_entity(
    catalog: &crate::BossCatalog,
    id: &str,
    bosses: &Query<(
        bevy::prelude::Entity,
        &FeatureId,
        crate::BossClusterRef,
        &ambition_characters::actor::BodyHealth,
        &ambition_characters::brain::BossAttackState,
        &ambition_characters::brain::Brain,
    )>,
) -> Option<(
    bevy::prelude::Entity,
    ambition_sprite_sheet::boss::BossAnimState,
)> {
    bosses.iter().find_map(
        |(entity, feature_id, boss, health, attack_state, brain)| {
            if feature_id.as_str() != id {
                return None;
            }
            Some((
                entity,
                boss_anim_state_for(
                    catalog,
                    boss.as_boss_ref(),
                    health.alive(),
                    attack_state,
                    brain,
                ),
            ))
        },
    )
}

/// Return the currently rendered attack-frame sample for a boss, but only
/// when the chosen visual row is driven by the boss attack profile.
///
/// The death override returns `None`, so geometry callers fall back to
/// elapsed-time sampling and do not use a frame from the wrong row. A hit
/// reaction never reaches here: the sim cursor does not enter the `Hit` row
/// (see `boss_anim_state_for`).
pub fn ecs_boss_animation_frame_sample(
    catalog: &crate::BossCatalog,
    id: &str,
    bosses: &Query<(
        bevy::prelude::Entity,
        &FeatureId,
        crate::BossClusterRef,
        &ambition_characters::actor::BodyHealth,
        &ambition_characters::brain::BossAttackState,
        &ambition_characters::brain::Brain,
    )>,
    anim: ambition_sprite_sheet::boss::BossAnim,
    frame_index: usize,
) -> Option<(
    bevy::prelude::Entity,
    crate::attack_geometry::BossAnimationFrameSample,
)> {
    bosses.iter().find_map(
        |(entity, feature_id, _boss, _health, attack_state, _brain)| {
            if feature_id.as_str() != id {
                return None;
            }
            let active_expected = attack_state
                .active_profile
                .as_ref()
                .and_then(|profile| catalog.attack_animation(profile));
            let telegraph_expected = attack_state
                .telegraph_profile
                .as_ref()
                .and_then(|profile| catalog.attack_animation(profile));
            let mut result = None;
            if let Some(profile) = attack_state.active_profile.as_ref() {
                if active_expected == Some(anim) {
                    result = Some((
                        entity,
                        crate::attack_geometry::BossAnimationFrameSample {
                            profile: Some(profile.clone()),
                            frame_index,
                            animation_key: catalog.hurtbox_sample_row(profile),
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
                                animation_key: catalog.hurtbox_sample_row(profile),
                            },
                        ));
                    }
                }
            }
            // Idle/rest: not driven by an attack profile, but still emit a
            // sample, so the rest-pose hurtbox bobs with the breathing
            // animation. The Death row stays `None`: geometry keeps the
            // rest-pose shape and does not follow a recoil/death frame.
            if result.is_none() && anim == ambition_sprite_sheet::boss::BossAnim::Rest {
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
    catalog: &crate::BossCatalog,
    id: &str,
    bosses: &Query<(
        &FeatureId,
        crate::BossClusterRef,
        &ambition_characters::actor::BodyHealth,
        &ambition_characters::brain::BossAttackState,
        &ambition_characters::brain::Brain,
    )>,
) -> Option<ambition_sprite_sheet::boss::BossAnimState> {
    bosses
        .iter()
        .find_map(|(feature_id, boss, health, attack_state, brain)| {
            if feature_id.as_str() != id {
                return None;
            }
            Some(boss_anim_state_for(
                catalog,
                boss.as_boss_ref(),
                health.alive(),
                attack_state,
                brain,
            ))
        })
}

#[cfg(test)]
mod sample_key_agrees_with_profile_keys_tests {
    use ambition_characters::brain::BossAttackProfile;

    /// The authored hurtbox sample row of a strike names a row the strike
    /// claims. `apple_rain` is not in the list: it claims no row, and its
    /// sample row is the head row.
    #[test]
    fn every_authored_sample_row_names_a_row_its_strike_claims() {
        let catalog = crate::test_boss_catalog();
        for move_id in [
            "head_descent",
            "converging_shockwave",
            "hand_slam",
            "hand_sweep",
        ] {
            let profile = BossAttackProfile::Strike(move_id.to_string());
            let key = catalog
                .hurtbox_sample_row(&profile)
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
