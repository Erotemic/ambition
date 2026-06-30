//! Smirking Behemoth eye-beam boss-special Technique.
//!
//! Split out of the former 1.8k-line `specials.rs` (2026-06-15) — see
//! [`super`] (`specials/mod.rs`) for the shared module overview.

use super::*;

/// approximate target point and the strike spawns a single line of fast
/// projectile boxes toward it. Content-owned; attached via
/// `register_required_components` (see module docs).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct EyeBeamState {
    pub locked_target: Option<ae::Vec2>,
    pub fired_this_strike: bool,
}

const EYE_BEAM_OWNER_PREFIX: &str = "smirking_behemoth_eye_beam";

/// The eye-beam special's content key (matches the boss-schedule
/// `Special("eye_beam")` beats in `boss_profiles.ron`).
pub const EYE_BEAM_KEY: &str = "eye_beam";

// Eye-beam tuning — content-owned (moved off the engine with the special).
// Kept high so the attack reads as a short bubble-laser line, not a barrage.
const SHOT_SPEED: f32 = 780.0;
const DAMAGE: i32 = 1;
const BOX_COUNT: u8 = 5;
const BOX_SPACING: f32 = 26.0;
const HALF_EXTENT_X: f32 = 15.0;
const HALF_EXTENT_Y: f32 = 8.0;
const LIFETIME_S: f32 = 0.58;

/// Technique: Smirking Behemoth eye beam.
///
/// During the `Special("eye_beam")` telegraph the boss locks the currently
/// tracked player position. On the first strike tick it emits a short line of
/// fast bubble-laser projectile boxes from the eye toward that locked point.
/// This deliberately does **not** reuse overfit-volley's sample barrage,
/// because the cut-rope boss needs one readable beam rather than a cloud of
/// slow memorized shots. Params are content-owned consts (above).
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
    // Which actors emitted an eye-beam Special this tick (the strike edge).
    let mut firing: std::collections::HashSet<Entity> = std::collections::HashSet::new();
    for msg in messages.read() {
        if let ActionRequest::Special {
            spec: SpecialActionSpec::Special(key),
        } = &msg.request
        {
            if key == EYE_BEAM_KEY {
                firing.insert(msg.actor);
            }
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
            Some(BossAttackProfile::Special(ref k)) if k == EYE_BEAM_KEY
        );
        if in_telegraph {
            if state.locked_target.is_none() {
                state.locked_target = player_pos;
            }
            state.fired_this_strike = false;
            continue;
        }

        if !firing.contains(&entity) {
            state.locked_target = None;
            state.fired_this_strike = false;
            continue;
        }
        let (shot_speed, damage, box_count, box_spacing, half_x, half_y, lifetime_s) = (
            SHOT_SPEED,
            DAMAGE,
            BOX_COUNT,
            BOX_SPACING,
            HALF_EXTENT_X,
            HALF_EXTENT_Y,
            LIFETIME_S,
        );
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
                    shots: vec![EnemyProjectileSpawn {
                        origin: beam_origin,
                        dir,
                        speed: shot_speed.max(1.0),
                        damage,
                        max_lifetime: lifetime_s.max(0.05),
                        half_extent: ae::Vec2::new(half_x.max(1.0), half_y.max(1.0)),
                        owner_id: format!("{}:{}", EYE_BEAM_OWNER_PREFIX, boss.config.id),
                        gravity: 0.0,
                        visual_tag: 0,
                    }],
                },
            });
        }
        state.fired_this_strike = true;
        state.locked_target = None;
    }
}
