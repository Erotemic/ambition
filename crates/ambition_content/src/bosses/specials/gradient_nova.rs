//! Exploding Gradient runaway nova boss-special Technique.
//!
//! Split out of the former 1.8k-line `specials.rs` (2026-06-15) — see
//! [`super`] (`specials/mod.rs`) for the shared module overview.

use super::*;

// ---- Exploding Gradient's runaway nova (content-only, open-seam special) ----

/// Content key for Exploding Gradient's nova — matches the
/// `Special("gradient_nova")` beats in `boss_profiles.ron`.
pub const GRADIENT_NOVA_KEY: &str = "gradient_nova";

const NOVA_COUNT: u32 = 16;
const NOVA_BASE_SPEED: f32 = 260.0;
const NOVA_DAMAGE: i32 = 1;
const NOVA_HALF_EXTENT: ae::Vec2 = ae::Vec2::new(9.0, 9.0);
const NOVA_LIFETIME: f32 = 1.6;
const NOVA_SPAWN_RADIUS: f32 = 28.0;
const NOVA_OWNER_PREFIX: &str = "gradient_nova";

/// Per-boss gate for the Exploding Gradient nova. One omnidirectional burst per
/// strike — no target lock; the runaway gradients explode outward from the boss.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ExplodingGradientState {
    pub fired_this_strike: bool,
}

/// Pure: the `count` `(dir, speed)` pairs of a radial nova. Directions spread
/// evenly around the full circle; speeds come in three tiers (the "runaway
/// gradient magnitudes" blowing up unevenly), so the expanding front is ragged
/// rather than a clean ring. Deterministic — the testable core of the Technique.
fn gradient_nova(count: u32, base_speed: f32) -> Vec<(ae::Vec2, f32)> {
    let n = count.max(1);
    (0..n)
        .map(|i| {
            let theta = std::f32::consts::TAU * (i as f32) / (n as f32);
            let dir = ae::Vec2::new(theta.cos(), theta.sin());
            // 3 speed tiers: ×1.0, ×1.5, ×2.0 — runaway magnitudes.
            let speed = base_speed * (1.0 + 0.5 * (i % 3) as f32);
            (dir, speed)
        })
        .collect()
}

/// Technique: Exploding Gradient nova (content-only; open-seam special).
pub fn spawn_gradient_nova_from_special_messages(
    mut effects: MessageWriter<EffectRequest>,
    mut messages: MessageReader<ActorActionMessage>,
    mut bosses: Query<
        (Entity, BossClusterRef, &mut ExplodingGradientState),
        With<FeatureSimEntity>,
    >,
) {
    let mut firing: std::collections::HashSet<Entity> = std::collections::HashSet::new();
    for msg in messages.read() {
        if let ActionRequest::Special {
            spec: SpecialActionSpec::Special(key),
        } = &msg.request
        {
            if key == GRADIENT_NOVA_KEY {
                firing.insert(msg.actor);
            }
        }
    }
    for (entity, boss_feature, mut state) in &mut bosses {
        let boss = boss_feature.as_boss_ref();
        if !firing.contains(&entity) {
            state.fired_this_strike = false;
            continue;
        }
        if !boss.status.alive || state.fired_this_strike {
            continue;
        }
        let origin = boss.kin.pos + boss.config.behavior.projectile_origin_offset;
        for (dir, speed) in gradient_nova(NOVA_COUNT, NOVA_BASE_SPEED) {
            effects.write(EffectRequest {
                owner: entity,
                effect: Effect::Projectiles {
                    faction: ProjectileFaction::Enemy,
                    shots: vec![EnemyProjectileSpawn {
                        origin: origin + dir * NOVA_SPAWN_RADIUS,
                        dir,
                        speed,
                        damage: NOVA_DAMAGE,
                        max_lifetime: NOVA_LIFETIME,
                        half_extent: NOVA_HALF_EXTENT,
                        owner_id: format!("{}:{}", NOVA_OWNER_PREFIX, boss.config.id),
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
    use super::super::*;
    use super::*;

    /// End-to-end wiring check (public-API only): drive a boss to fire the
    /// gradient nova and confirm the full burst of projectile entities
    /// materializes through `Effect::Projectiles` → the engine's projectile
    /// executor. Validates the consumer → effect → spawn pipeline that all five
    /// new content specials share — catching a wiring/registration mistake the
    /// pure-core tests can't. Builds the boss via `BossClusterScratch` (public),
    /// so no engine `test-support` plumbing is needed.
    #[test]
    fn gradient_nova_consumer_materializes_a_full_burst_of_projectiles() {
        use ambition_gameplay_core::actor::BossBrain;
        use ambition_gameplay_core::enemy_projectile::{
            apply_projectile_effects, EnemyProjectile, EnemyProjectileState,
        };
        use ambition_gameplay_core::features::BossClusterScratch;
        use ambition_gameplay_core::projectile::ProjectileSeqCounter;

        // The boss-profile registry must be installed before `BossClusterScratch`
        // resolves a behavior (the lib panics otherwise in non-test builds, which
        // is how the lib compiles for a content test). Idempotent.
        crate::bosses::install_boss_roster();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ActorActionMessage>();
        app.add_message::<EffectRequest>();
        app.init_resource::<EnemyProjectileState>();
        app.init_resource::<ProjectileSeqCounter>();
        app.init_resource::<WorldTime>();
        {
            let mut wt = app.world_mut().resource_mut::<WorldTime>();
            wt.scaled_dt = 1.0 / 60.0;
            wt.raw_dt = 1.0 / 60.0;
        }
        app.add_systems(
            Update,
            (
                spawn_gradient_nova_from_special_messages,
                apply_projectile_effects,
            )
                .chain(),
        );

        let aabb = ae::Aabb::new(ae::Vec2::new(640.0, 400.0), ae::Vec2::new(64.0, 64.0));
        let boss = BossClusterScratch::new("test_boss", "Test Boss", aabb, BossBrain::Dormant)
            .into_components();
        let actor = app
            .world_mut()
            .spawn((FeatureSimEntity, ExplodingGradientState::default(), boss))
            .id();

        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
            .write(ActorActionMessage {
                actor,
                request: ActionRequest::Special {
                    spec: SpecialActionSpec::Special(GRADIENT_NOVA_KEY.to_string()),
                },
            });
        app.update();

        let count = app
            .world_mut()
            .query_filtered::<(), With<EnemyProjectile>>()
            .iter(app.world())
            .count();
        assert_eq!(
            count, NOVA_COUNT as usize,
            "the full nova burst should materialize as projectile entities",
        );
    }

    #[test]
    fn gradient_nova_spreads_full_circle_with_runaway_speed_tiers() {
        let nova = gradient_nova(16, 260.0);
        assert_eq!(nova.len(), 16);
        for (dir, speed) in &nova {
            assert!((dir.length() - 1.0).abs() < 1e-3, "dir is a unit vector");
            assert!(*speed >= 260.0, "speed never below base");
            assert!(*speed <= 260.0 * 2.0 + 1e-3, "speed capped at the top tier");
        }
        // Three distinct speed tiers are present (runaway magnitudes).
        let tiers: std::collections::BTreeSet<i32> = nova
            .iter()
            .map(|(_, s)| (s / 130.0).round() as i32)
            .collect();
        assert_eq!(tiers.len(), 3, "three runaway speed tiers");
        // Directions cover all four quadrants (a full nova, not a fan).
        assert!(nova.iter().any(|(d, _)| d.x > 0.5 && d.y.abs() < 0.5));
        assert!(nova.iter().any(|(d, _)| d.x < -0.5 && d.y.abs() < 0.5));
        assert!(nova.iter().any(|(d, _)| d.y > 0.5));
        assert!(nova.iter().any(|(d, _)| d.y < -0.5));
        assert_eq!(gradient_nova(0, 260.0).len(), 1, "degenerate count is safe");
    }
}
