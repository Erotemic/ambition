//! Exploding Gradient runaway nova boss-special Technique.

use bevy::prelude::*;

use ambition_characters::brain::{
    ActorActionMessage,
};
use ambition_boss_encounter::BossClusterRef;
use ambition_platformer2d::actor::FeatureSimEntity;
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
    let firing = super::actors_firing(&mut messages, GRADIENT_NOVA_KEY);
    for (entity, boss_feature, health, mut state) in &mut bosses {
        let boss = boss_feature.as_boss_ref();
        if !firing.contains_key(&entity) {
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
                    splash_half_extent: 0.0,
                    boomerang_return_s: None,
                },
                ProjectileStart::StepThisTick,
            )
            .fired_by_move_if_any(firing.get(&entity).copied().flatten()),
            );
        }
        state.fired_this_strike = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::brain::action_set::{ActionRequest, SpecialActionSpec};

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
                move_instance: None,
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

    /// Drive the same road with a move occurrence on the request and read it
    /// back off the SPAWNED PROJECTILE ENTITIES.
    ///
    /// Returns `(materialized, stamped_with)` so one body serves both arms.
    fn nova_projectile_stamps(asked_by: Option<u32>) -> (usize, Vec<Option<u32>>) {
        use ambition_boss_encounter::BossClusterScratch;
        use ambition_entity_catalog::placements::BossBrain;
        use ambition_projectiles::{
            materialize_projectiles_for_this_tick, ProjectileSeqCounter, ProjectileSpawnRequest,
        };

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
                move_instance: asked_by,
            });
        app.update();

        let mut q = app.world_mut().query_filtered::<(
            Option<&ambition_projectiles::FiredByMoveInstance>,
            (),
        ), With<ambition_projectiles::LiveProjectile>>();
        let stamps: Vec<Option<u32>> = q
            .iter(app.world())
            .map(|(stamp, ())| stamp.map(|s| s.0))
            .collect();
        (stamps.len(), stamps)
    }

    /// ⛔⛤ WITNESS — A12 BLOCKER 2, THE CONTENT HALF. A TECHNIQUE'S PROJECTILE
    /// CARRIES THE MOVE USE THAT ASKED FOR IT.
    ///
    /// ⛔⛔ THE DEFECT THIS FAILS ON. A boss `Special(key)` profile compiles to a
    /// move whose Active window carries a `sustain_effect`; that bridges to
    /// `ActorActionMessage::Special`, and THIS system turns it into projectiles.
    /// The bolts outlive the move — the sentinel's live 2.4s. Every one of them
    /// spawned with `move_instance: None`, and `moveset::verdict_belongs_to`
    /// admits `None` against ANY playback, so a bolt fired by move A and landing
    /// during move B credited B with a hit it never earned.
    ///
    /// ⭐⭐ IT READS THE SPAWNED ENTITY, NOT THE REQUEST. The request is the
    /// system's own output; the entity is what survives into the tick where the
    /// damage is resolved, which is the only place the number matters. A test
    /// that stops at the request cannot see the materializer drop it.
    ///
    /// ⚠ AND THE NUMBER TRAVELS ONE WAY ONLY. Nothing downstream may recover it
    /// by reading the owner's `MovePlayback`, because by the time a bolt lands
    /// the authoring move is over — that re-read IS the defect.
    #[test]
    fn a_nova_bolt_carries_the_move_use_that_fired_it() {
        let (count, stamps) = nova_projectile_stamps(Some(7));
        assert_eq!(
            count, NOVA_COUNT as usize,
            "the premise: the full burst materialized, so the stamps below are \
             a claim about projectiles that exist"
        );
        assert!(
            stamps.iter().all(|s| *s == Some(7)),
            "every bolt names the move use that asked for it; got {stamps:?}. An \
             unstamped bolt is credited to whatever move plays when it lands."
        );
    }

    /// ⭐ AND `None` IS AN ANSWER, NOT A HOLE. A boss brain that presses its
    /// special directly — no move behind it — has no use to name, and the
    /// technique must pass that through rather than invent a number. A stamp
    /// here would be a FALSE provenance, which is worse than none: it would
    /// deny a real move its own hit.
    #[test]
    fn a_brain_pressed_nova_stamps_no_move_use() {
        let (count, stamps) = nova_projectile_stamps(None);
        assert_eq!(count, NOVA_COUNT as usize, "the premise: the burst materialized");
        assert!(
            stamps.iter().all(|s| s.is_none()),
            "a special with no move behind it stamps nothing; got {stamps:?}"
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
