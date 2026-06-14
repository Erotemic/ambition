//! Boss special-attack **Techniques** — the content-owned systems that drive
//! each named boss special. A Technique reads the boss's brain signal
//! (`ActorActionMessage::Special`) + its per-boss temporal state, and emits
//! generic `ambition_sandbox::effects::Effect`s for the engine to execute. The
//! engine owns no boss-special behavior; it lives here.
//!
//! Each Technique's per-boss state component is content-owned too, attached to
//! every boss via `register_required_components::<BossConfig, _>()` in
//! [`super::AmbitionBossContentPlugin`] — so the machinery lib names no boss
//! technique.
//!
//! Migrated from `ambition_sandbox::features::ecs::brain_effects` one Technique
//! at a time. First: the Smirking Behemoth eye beam.

use bevy::prelude::*;

use ambition_sandbox::brain::{
    action_set::ActionRequest, ActorActionMessage, BossAttackProfile, BossAttackState,
    SpecialActionSpec,
};
use ambition_sandbox::effects::{Effect, EffectRequest};
use ambition_sandbox::engine_core::{self as ae, AabbExt};
use ambition_sandbox::enemy_projectile::EnemyProjectileSpawn;
use ambition_sandbox::features::{ActorTarget, BossClusterRef, FeatureSimEntity};
use ambition_sandbox::player::{BodyKinematics, PlayerEntity};
use ambition_sandbox::projectile::ProjectileFaction;

/// Per-boss state for the Smirking Behemoth eye beam. The telegraph locks an
/// approximate target point and the strike spawns a single line of fast
/// projectile boxes toward it. Content-owned; attached via
/// `register_required_components` (see module docs).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct EyeBeamState {
    pub locked_target: Option<ae::Vec2>,
    pub fired_this_strike: bool,
}

const EYE_BEAM_OWNER_PREFIX: &str = "smirking_behemoth_eye_beam";

/// Technique: Smirking Behemoth eye beam.
///
/// During the `LockOnBeam` telegraph the boss locks the currently tracked
/// player position. On the first strike tick it emits a short line of fast
/// bubble-laser projectile boxes from the eye toward that locked point. This
/// deliberately does **not** reuse MemorizedVolley's sample barrage, because the
/// cut-rope boss needs one readable beam rather than a cloud of slow memorized
/// shots.
pub fn spawn_eye_beam_from_special_messages(
    mut effects: MessageWriter<EffectRequest>,
    mut messages: MessageReader<ActorActionMessage>,
    player_query: Query<&BodyKinematics, With<PlayerEntity>>,
    mut bosses: Query<
        (
            Entity,
            BossClusterRef,
            &BossAttackState,
            &mut EyeBeamState,
            Option<&ActorTarget>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let mut active_strike_params: std::collections::HashMap<
        Entity,
        (f32, i32, u8, f32, f32, f32, f32),
    > = std::collections::HashMap::new();
    for msg in messages.read() {
        if let ActionRequest::Special {
            spec:
                SpecialActionSpec::LockOnBeam {
                    shot_speed,
                    damage,
                    box_count,
                    box_spacing,
                    half_extent_x,
                    half_extent_y,
                    lifetime_s,
                },
        } = msg.request
        {
            active_strike_params.insert(
                msg.actor,
                (
                    shot_speed,
                    damage,
                    box_count,
                    box_spacing,
                    half_extent_x,
                    half_extent_y,
                    lifetime_s,
                ),
            );
        }
    }

    for (entity, boss_feature, attack_state, mut state, actor_target) in &mut bosses {
        let boss = boss_feature.as_boss_ref();
        let player_pos = actor_target.and_then(|t| {
            t.entity
                .and_then(|e| player_query.get(e).ok())
                .map(|kin| kin.aabb().center())
                .or(Some(t.pos))
        });
        if !boss.status.alive {
            state.locked_target = None;
            state.fired_this_strike = false;
            continue;
        }

        let in_telegraph = matches!(
            attack_state.telegraph_profile,
            Some(BossAttackProfile::LockOnBeam)
        );
        let strike_params = active_strike_params.get(&entity).copied();
        if in_telegraph {
            if state.locked_target.is_none() {
                state.locked_target = player_pos;
            }
            state.fired_this_strike = false;
            continue;
        }

        let Some((shot_speed, damage, box_count, box_spacing, half_x, half_y, lifetime_s)) =
            strike_params
        else {
            state.locked_target = None;
            state.fired_this_strike = false;
            continue;
        };
        if state.fired_this_strike {
            continue;
        }
        let target = state.locked_target.or(player_pos).unwrap_or(boss.kin.pos);
        let offset = ae::Vec2::new(
            boss.config.behavior.projectile_origin_offset.x * boss.kin.facing.signum(),
            boss.config.behavior.projectile_origin_offset.y,
        );
        let origin = boss.kin.pos + offset;
        let delta = target - origin;
        let dir = if delta.length_squared() < 1e-4 {
            ae::Vec2::new(boss.kin.facing.signum(), 0.0)
        } else {
            delta.normalize()
        };
        let count = box_count.max(1);
        let spacing = box_spacing.max(1.0);
        for i in 0..count {
            let beam_origin = origin + dir * spacing * f32::from(i);
            effects.write(EffectRequest {
                owner: entity,
                effect: Effect::Projectiles {
                    faction: ProjectileFaction::Enemy,
                    shots: vec![EnemyProjectileSpawn {
                        origin: beam_origin,
                        dir,
                        speed: shot_speed.max(1.0),
                        damage,
                        max_lifetime: lifetime_s.max(0.05),
                        half_extent: ae::Vec2::new(half_x.max(1.0), half_y.max(1.0)),
                        owner_id: format!("{}:{}", EYE_BEAM_OWNER_PREFIX, boss.config.id),
                        gravity: 0.0,
                    }],
                },
            });
        }
        state.fired_this_strike = true;
        state.locked_target = None;
    }
}
