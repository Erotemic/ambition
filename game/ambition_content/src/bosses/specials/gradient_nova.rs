//! Exploding Gradient runaway nova boss-special Technique.

use bevy::prelude::*;

use ambition_characters::brain::{
    action_set::ActionRequest, ActorActionMessage, SpecialActionSpec,
};
use ambition_boss_encounter::BossClusterRef;
use ambition_platformer2d_actor_monolith::features::FeatureSimEntity;
use ambition_platformer2d_core as ae;
use ambition_projectiles::{ProjectileSpawn, ProjectileSpawnRequest, ProjectileStart};

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
    mut projectiles: MessageWriter<ProjectileSpawnRequest>,
    mut messages: MessageReader<ActorActionMessage>,
    mut bosses: Query<
        (
            Entity,
            BossClusterRef,
            &ambition_characters::actor::BodyHealth,
            &mut ExplodingGradientState,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let mut firing: std::collections::HashSet<Entity> = std::collections::HashSet::new();
    for msg in messages.read() {
        if let ActionRequest::Special {
            spec: SpecialActionSpec::Special(key),
            ..
        } = &msg.request
        {
            if key == GRADIENT_NOVA_KEY {
                firing.insert(msg.actor);
            }
        }
    }
    for (entity, boss_feature, health, mut state) in &mut bosses {
        let boss = boss_feature.as_boss_ref();
        if !firing.contains(&entity) {
            state.fired_this_strike = false;
            continue;
        }
        if !health.alive() || state.fired_this_strike {
            continue;
        }
        let origin = boss.kin.pos + boss.config.behavior.projectile_origin_offset;
        for (dir, speed) in gradient_nova(NOVA_COUNT, NOVA_BASE_SPEED) {
            projectiles.write(ProjectileSpawnRequest::open(
                entity,
                ProjectileSpawn {
                    origin: origin + dir * NOVA_SPAWN_RADIUS,
                    dir,
                    speed,
                    damage: NOVA_DAMAGE,
                    max_lifetime: NOVA_LIFETIME,
                    half_extent: NOVA_HALF_EXTENT,
                    gravity: 0.0,
                    visual_id: String::new(),
                    // Straight shot: this ability authors no bounce.
                    bounces: 0,
                    bounce_on_world_contact: false,
                    boomerang_return_s: None,
                },
                ProjectileStart::StepThisTick,
            ));
        }
        state.fired_this_strike = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use ambition_time::WorldTime;

    /// End-to-end wiring check (public-API only): drive a boss to fire the
    /// gradient nova and confirm the full burst of projectile entities
    /// materializes through `ProjectileSpawnRequest` → the projectile-domain
    /// materializer. Validates the consumer → request → spawn pipeline that the
    /// projectile specials share — catching a wiring/registration mistake the
    /// pure-core tests can't. Builds the boss via `BossClusterScratch` (public),
    /// so no engine `test-support` plumbing is needed.
    #[test]
    fn gradient_nova_consumer_materializes_a_full_burst_of_projectiles() {
        use ambition_entity_catalog::placements::BossBrain;
        use ambition_boss_encounter::BossClusterScratch;
        use ambition_projectiles::{
            materialize_projectiles_for_this_tick, ProjectileSeqCounter,
            ProjectileSpawnRequest,
        };

        // Use the same App-local provider catalog production composition builds.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ActorActionMessage>();
        app.add_message::<ProjectileSpawnRequest>();
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
                materialize_projectiles_for_this_tick,
            )
                .chain(),
        );

        let aabb = ae::Aabb::new(ae::Vec2::new(640.0, 400.0), ae::Vec2::new(64.0, 64.0));
        let boss_catalog = crate::bosses::authored_boss_catalog();
        let boss = BossClusterScratch::new(
            &boss_catalog,
            "test_boss",
            "Test Boss",
            aabb,
            BossBrain::Dormant,
        )
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
                    params: Default::default(),
                },
            });
        app.update();

        let count = app
            .world_mut()
            .query_filtered::<(), With<ambition_projectiles::LiveProjectile>>()
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
