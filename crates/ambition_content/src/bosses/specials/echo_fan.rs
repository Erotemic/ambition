//! Mockingbird echo fan boss-special Technique.
//!
//! Split out of the former 1.8k-line `specials.rs` (2026-06-15) — see
//! [`super`] (`specials/mod.rs`) for the shared module overview.

use super::*;

// ---- Mockingbird's echo fan (content-only, open-seam; mimic spread) ----

/// Content key for the Mockingbird echo fan — matches the `Special("echo_fan")`
/// beats in `boss_profiles.ron`.
pub const ECHO_FAN_KEY: &str = "echo_fan";

const ECHO_FAN_COUNT: u32 = 7;
const ECHO_FAN_SPREAD_RAD: f32 = 0.9; // total cone width (~52°)
const ECHO_FAN_SPEED: f32 = 300.0;
const ECHO_FAN_DAMAGE: i32 = 1;
const ECHO_FAN_HALF_EXTENT: ae::Vec2 = ae::Vec2::new(9.0, 9.0);
const ECHO_FAN_LIFETIME: f32 = 2.0;
const ECHO_FAN_OWNER_PREFIX: &str = "echo_fan";

/// Per-boss gate for the echo fan. One spread per strike.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct EchoFanState {
    pub fired_this_strike: bool,
}

/// Pure: `count` unit directions evenly fanned across a `spread` cone centered on
/// `aim` — the same shot mimicked across the fan. A single shot when `count == 1`
/// flies straight along `aim`. Deterministic — the testable core of the Technique.
fn echo_fan(aim: ae::Vec2, count: u32, spread: f32) -> Vec<ae::Vec2> {
    let n = count.max(1);
    let base = if aim.length_squared() < 1e-6 {
        0.0
    } else {
        aim.y.atan2(aim.x)
    };
    (0..n)
        .map(|i| {
            // Even spread across [-spread/2, +spread/2]; single shot → straight.
            let t = if n == 1 {
                0.0
            } else {
                (i as f32) / ((n - 1) as f32) - 0.5
            };
            let theta = base + t * spread;
            ae::Vec2::new(theta.cos(), theta.sin())
        })
        .collect()
}

/// Technique: Mockingbird echo fan — copies one shot across a cone aimed at the
/// player (content-only; open-seam special).
pub fn spawn_echo_fan_from_special_messages(
    mut effects: MessageWriter<EffectRequest>,
    mut messages: MessageReader<ActorActionMessage>,
    player_query: Query<&BodyKinematics, With<PlayerEntity>>,
    mut bosses: Query<
        (
            Entity,
            BossClusterRef,
            &mut EchoFanState,
            Option<&ActorTarget>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let mut firing: std::collections::HashSet<Entity> = std::collections::HashSet::new();
    for msg in messages.read() {
        if let ActionRequest::Special {
            spec: SpecialActionSpec::Special(key),
        } = &msg.request
        {
            if key == ECHO_FAN_KEY {
                firing.insert(msg.actor);
            }
        }
    }
    for (entity, boss_feature, mut state, actor_target) in &mut bosses {
        let boss = boss_feature.as_boss_ref();
        if !firing.contains(&entity) {
            state.fired_this_strike = false;
            continue;
        }
        if !boss.status.alive || state.fired_this_strike {
            continue;
        }
        let origin = boss.kin.pos + boss.config.behavior.projectile_origin_offset;
        let player_pos = actor_target.and_then(|t| {
            t.entity
                .and_then(|e| player_query.get(e).ok())
                .map(|kin| kin.aabb().center())
                .or(Some(t.pos))
        });
        // Aim at the player; fall back to straight-ahead by facing if untracked.
        let aim = player_pos
            .map(|p| p - origin)
            .filter(|d| d.length_squared() > 1e-4)
            .unwrap_or_else(|| ae::Vec2::new(boss.kin.facing.signum(), 0.0));
        for dir in echo_fan(aim, ECHO_FAN_COUNT, ECHO_FAN_SPREAD_RAD) {
            effects.write(EffectRequest {
                owner: entity,
                effect: Effect::Projectiles {
                    faction: ProjectileFaction::Enemy,
                    shots: vec![EnemyProjectileSpawn {
                        origin,
                        dir,
                        speed: ECHO_FAN_SPEED,
                        damage: ECHO_FAN_DAMAGE,
                        max_lifetime: ECHO_FAN_LIFETIME,
                        half_extent: ECHO_FAN_HALF_EXTENT,
                        owner_id: format!("{}:{}", ECHO_FAN_OWNER_PREFIX, boss.config.id),
                        gravity: 0.0,
                    }],
                },
            });
        }
        state.fired_this_strike = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::*;

    #[test]
    fn echo_fan_spreads_evenly_around_the_aim() {
        let aim = ae::Vec2::new(1.0, 0.0); // straight right
        let fan = echo_fan(aim, 7, 0.9);
        assert_eq!(fan.len(), 7);
        for d in &fan {
            assert!((d.length() - 1.0).abs() < 1e-3, "unit dirs");
        }
        // Middle shot flies straight along the aim; ends are symmetric about it.
        let mid = fan[3];
        assert!(mid.y.abs() < 1e-3 && mid.x > 0.0, "center shot is the aim");
        assert!(
            (fan[0].y + fan[6].y).abs() < 1e-3,
            "fan symmetric about aim"
        );
        assert!(fan[0].y * fan[6].y < 0.0, "ends straddle the aim");
        // A single shot flies straight along the aim (no spread).
        let one = echo_fan(aim, 1, 0.9);
        assert_eq!(one.len(), 1);
        assert!(one[0].y.abs() < 1e-3 && one[0].x > 0.0);
    }
}
