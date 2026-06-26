//! Mode Collapse converging ring boss-special Technique.
//!
//! Split out of the former 1.8k-line `specials.rs` (2026-06-15) — see
//! [`super`] (`specials/mod.rs`) for the shared module overview.

use super::*;

// ===================================================================
// NEW boss special authored entirely on the open `Special(key)` seam —
// Mode Collapse's converging ring. Proves the engine-content boundary:
// behavior + params + state + telegraph + schedule all live here in
// `ambition_content`; NOTHING in the engine lib (`ambition_gameplay_core`) was
// edited to add it. (The app's combat schedule wires the consumer in, the
// same as every other special consumer — composition is the app's job.)
// ===================================================================

/// Content key for Mode Collapse's converging-ring special — matches the
/// `Special("mode_collapse_converge")` beats authored in `boss_profiles.ron`.
pub const MODE_COLLAPSE_KEY: &str = "mode_collapse_converge";

// Tuning — content-owned.
const MC_RING_COUNT: u32 = 12;
const MC_RING_RADIUS: f32 = 190.0;
const MC_RING_SPEED: f32 = 320.0;
const MC_RING_DAMAGE: i32 = 1;
const MC_RING_HALF_EXTENT: ae::Vec2 = ae::Vec2::new(10.0, 10.0);
const MC_RING_LIFETIME: f32 = 1.5;
const MC_OWNER_PREFIX: &str = "mode_collapse_converge";

/// Per-boss state for the Mode Collapse converging ring. The telegraph locks the
/// player's position; the strike spawns a ring of inward-aimed projectiles around
/// that locked point — the "diverse population" collapsing onto a single mode.
/// Read: step off the spot you were standing on during the wind-up.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ModeCollapseState {
    pub locked_target: Option<ae::Vec2>,
    pub fired_this_strike: bool,
}

/// Pure: the `count` `(origin, dir)` pairs of a converging ring of `radius`
/// around `center`. Each projectile spawns evenly around the ring and aims
/// inward at the center. Deterministic (angle 0 first), so it's unit-testable
/// without the ECS — the testable core of the Technique.
fn converge_ring(center: ae::Vec2, count: u32, radius: f32) -> Vec<(ae::Vec2, ae::Vec2)> {
    let n = count.max(1);
    (0..n)
        .map(|i| {
            let theta = std::f32::consts::TAU * (i as f32) / (n as f32);
            let offset = ae::Vec2::new(theta.cos(), theta.sin()) * radius;
            (center + offset, (-offset).normalize_or_zero())
        })
        .collect()
}

/// Technique: Mode Collapse converging ring (content-only; open-seam special).
pub fn spawn_mode_collapse_converge_from_special_messages(
    mut effects: MessageWriter<EffectRequest>,
    mut messages: MessageReader<ActorActionMessage>,
    player_query: Query<&BodyKinematics, With<PlayerEntity>>,
    mut bosses: Query<
        (
            Entity,
            BossClusterRef,
            &BossAttackState,
            &mut ModeCollapseState,
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
            if key == MODE_COLLAPSE_KEY {
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
            Some(BossAttackProfile::Special(ref k)) if k == MODE_COLLAPSE_KEY
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
        if state.fired_this_strike {
            continue;
        }
        let center = state.locked_target.or(player_pos).unwrap_or(boss.kin.pos);
        for (origin, dir) in converge_ring(center, MC_RING_COUNT, MC_RING_RADIUS) {
            if dir.length_squared() < 1e-4 {
                continue;
            }
            effects.write(EffectRequest {
                owner: entity,
                effect: Effect::Projectiles {
                    faction: ProjectileFaction::Enemy,
                    shots: vec![EnemyProjectileSpawn {
                        origin,
                        dir,
                        speed: MC_RING_SPEED,
                        damage: MC_RING_DAMAGE,
                        max_lifetime: MC_RING_LIFETIME,
                        half_extent: MC_RING_HALF_EXTENT,
                        owner_id: format!("{}:{}", MC_OWNER_PREFIX, boss.config.id),
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

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;

    #[test]
    fn mode_collapse_ring_spawns_around_center_and_aims_inward() {
        let center = ae::Vec2::new(640.0, 400.0);
        let radius = 190.0;
        let ring = converge_ring(center, 12, radius);
        assert_eq!(ring.len(), 12, "one projectile per ring slot");
        for (origin, dir) in &ring {
            // Each projectile spawns on the ring (radius away from center)...
            let to_origin = *origin - center;
            assert!(
                (to_origin.length() - radius).abs() < 1e-2,
                "origin should sit on the ring at radius {radius}",
            );
            // ...and aims inward (back toward the collapsing mode).
            assert!(
                dir.dot(to_origin) < 0.0,
                "dir {dir:?} must point inward toward center",
            );
            assert!((dir.length() - 1.0).abs() < 1e-3, "dir is normalized");
        }
        // Degenerate guard: a zero count still yields one safe slot, never panics.
        assert_eq!(converge_ring(center, 0, radius).len(), 1);
    }
}
