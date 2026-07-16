//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::enemy_projectile::test_support::enemy_projectile_bodies;
use crate::enemy_projectile::EnemyProjectileState;
use crate::features::ecs::actor_clusters::ActorClusterSeed;
use crate::projectile::ProjectileSeqCounter;
use ambition_characters::brain::{ActionSet, RangedActionSpec};

/// Build a rider-shaped hostile actor: standalone PirateRaider
/// archetype on the runtime side, but the caller is expected to
/// attach a [`crate::features::RidingOn`] component to the
/// spawned entity so the ranged-projectile handler routes the
/// fire through the lasersword path.
type ActorClusterBundle = (
    super::super::actor_clusters::BodyKinematics,
    super::super::actor_clusters::ActorStatus,
    ambition_characters::actor::BodyHealth,
    super::super::actor_clusters::ActorConfig,
    super::super::actor_clusters::ActorMotionPath,
    crate::features::ActorSurfaceState,
    crate::features::BodyMelee,
    crate::actor::AncillaryMovementBundle,
    crate::combat::CombatCapabilities,
    crate::combat::CombatTuning,
);

/// Spawnable (disposition + clusters) bundle for an enemy test fixture.
fn enemy_actor(enemy: ActorClusterSeed) -> (crate::features::ActorDisposition, ActorClusterBundle) {
    (
        crate::features::ActorDisposition::Hostile,
        enemy.into_components(),
    )
}

fn pirate_rider_actor(pos: ae::Vec2) -> (crate::features::ActorDisposition, ActorClusterBundle) {
    let aabb = ae::Aabb::new(pos, ae::Vec2::new(14.0, 23.0));
    let enemy = ActorClusterSeed::new(
        "rider_a",
        "Pirate Raider",
        aabb,
        ambition_entity_catalog::placements::CharacterBrain::Custom("pirate_raider".into()),
        &[],
    );
    enemy_actor(enemy)
}

fn build_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<ActorActionMessage>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<ambition_vfx::EffectRequest>();
    app.init_resource::<EnemyProjectileState>();
    app.init_resource::<ProjectileSeqCounter>();
    // Phase 3b: the consumer emits SpawnProjectile; chain the enemy-pool
    // applier so the projectile entity spawns within the update.
    app.add_systems(
        Update,
        (
            spawn_enemy_projectiles_from_brain_actions,
            crate::enemy_projectile::apply_projectile_effects,
        )
            .chain(),
    );
    app
}

#[test]
fn ranged_message_for_non_pirate_uses_body_origin_not_hand() {
    let mut app = build_app();
    let actor_pos = ae::Vec2::new(300.0, 300.0);
    // Use Combatant (a melee archetype) — its spec is irrelevant
    // here; the consumer only branches on archetype for origin
    // and owner_id formatting.
    let aabb = ae::Aabb::new(actor_pos, ae::Vec2::new(14.0, 23.0));
    let enemy = ActorClusterSeed::new(
        "skitter_a",
        "Skitter",
        aabb,
        ambition_entity_catalog::placements::CharacterBrain::Custom("small_skitter".into()),
        &[],
    );
    let actor = app.world_mut().spawn(enemy_actor(enemy)).id();
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
        .write(ActorActionMessage {
            actor,
            request: ActionRequest::Ranged {
                spec: RangedActionSpec::Rock {
                    speed: 300.0,
                    damage: 1,
                },
                origin: actor_pos,
                dir: ae::Vec2::new(1.0, 0.0),
                dir_policy: ae::GameplayFramePolicy::WorldSpace,
            },
        });
    app.update();
    let projectiles = enemy_projectile_bodies(&mut app);
    assert_eq!(projectiles.len(), 1);
    let owner = &projectiles[0].owner_id;
    assert!(
        !owner.starts_with("lasersword:"),
        "non-pirate archetype must not get lasersword owner_id; got {owner:?}",
    );
}

/// The ranged-fire consumer stamps the firing actor's authored ranged
/// visual id onto the spawned projectile (by open id, not owner_id). A
/// `cellular_automaton_fighter` authored `ranged_visual: "glider"` fires a
/// `"glider"`-id shot.
#[test]
fn ranged_shot_carries_archetype_authored_visual_id() {
    let mut app = build_app();
    let actor_pos = ae::Vec2::new(300.0, 300.0);
    let aabb = ae::Aabb::new(actor_pos, ae::Vec2::new(14.0, 23.0));
    let enemy = ActorClusterSeed::new(
        "pca_test",
        "Perfect Cell-ular Automaton",
        aabb,
        ambition_entity_catalog::placements::CharacterBrain::Custom(
            "cellular_automaton_fighter".into(),
        ),
        &[],
    );
    let mut bundle = enemy_actor(enemy);
    // Author the ranged visual as the runtime archetype projection would.
    bundle.1 .3.tuning.ranged_visual = "glider".to_string();
    let actor = app.world_mut().spawn(bundle).id();
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
        .write(ActorActionMessage {
            actor,
            request: ActionRequest::Ranged {
                spec: RangedActionSpec::Rock {
                    speed: 300.0,
                    damage: 1,
                },
                origin: actor_pos,
                dir: ae::Vec2::new(1.0, 0.0),
                dir_policy: ae::GameplayFramePolicy::WorldSpace,
            },
        });
    app.update();
    let mut q = app
        .world_mut()
        .query::<&crate::projectile::ProjectileVisualId>();
    let ids: Vec<_> = q.iter(app.world()).map(|v| v.0.clone()).collect();
    assert_eq!(
        ids,
        vec!["glider".to_string()],
        "the PCA's authored ranged_visual must ride onto the spawned shot"
    );
}

#[test]
fn ranged_message_converts_local_direction_at_consumer_frame() {
    let mut app = build_app();
    let actor_pos = ae::Vec2::new(300.0, 300.0);
    let aabb = ae::Aabb::new(actor_pos, ae::Vec2::new(14.0, 23.0));
    let enemy = ActorClusterSeed::new(
        "side_gravity_shooter",
        "Skitter",
        aabb,
        ambition_entity_catalog::placements::CharacterBrain::Custom("small_skitter".into()),
        &[],
    );
    let mut actor_bundle = enemy_actor(enemy);
    // surface_normal points away from the support; gravity_dir is its
    // negative. Here local down is world +X, so local side/right maps to
    // world -Y under the arbitrary AccelerationFrame transform.
    actor_bundle.1 .5.surface_normal = ae::Vec2::new(-1.0, 0.0);
    let actor = app.world_mut().spawn(actor_bundle).id();
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
        .write(ActorActionMessage {
            actor,
            request: ActionRequest::Ranged {
                spec: RangedActionSpec::Rock {
                    speed: 300.0,
                    damage: 1,
                },
                origin: actor_pos,
                dir: ae::Vec2::new(1.0, 0.0),
                dir_policy: ae::GameplayFramePolicy::ControlledBodyLocal,
            },
        });
    app.update();
    let projectiles = enemy_projectile_bodies(&mut app);
    assert_eq!(projectiles.len(), 1);
    let dir = projectiles[0].body.kin.vel.normalize_or_zero();
    assert!(
        dir.y < -0.99 && dir.x.abs() < 0.01,
        "local side/right under +X down should fire world -Y, got {dir:?}"
    );
}

#[test]
fn ranged_message_for_dead_actor_is_dropped() {
    let mut app = build_app();
    let actor_pos = ae::Vec2::new(300.0, 300.0);
    let mut actor_runtime = pirate_rider_actor(actor_pos);
    // .1 = cluster bundle; BodyHealth (liveness authority) is at .1.2.
    actor_runtime.1 .2.health.current = 0;
    let actor = app.world_mut().spawn(actor_runtime).id();
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
        .write(ActorActionMessage {
            actor,
            request: ActionRequest::Ranged {
                spec: RangedActionSpec::Bolt {
                    speed: 500.0,
                    damage: 1,
                },
                origin: actor_pos,
                dir: ae::Vec2::new(1.0, 0.0),
                dir_policy: ae::GameplayFramePolicy::WorldSpace,
            },
        });
    app.update();
    assert!(
        enemy_projectile_bodies(&mut app).is_empty(),
        "dead actor must not spawn a projectile",
    );
}

/// Suppress unused-import noise from the test-only `ActionSet`
/// reference — kept for callers that grow this module's tests.
fn _silence_action_set_import(_: ActionSet) {}

// The melee-START unit pins that used to live here
// (`melee_message_starts_enemy_windup_and_cooldown`,
// `melee_message_can_start_windup_for_dismounted_pirate_heavy`,
// `melee_message_during_cooldown_is_dropped`) exercised the deleted
// actor-only `start_enemy_melee_from_brain_actions`. Melee-start is a moveset
// `"attack"` move for every body now (`combat::moveset::trigger_moveset_moves`);
// it is pinned through the REAL schedule by
// `ambition_app/tests/enemy_attacks_player.rs` (actor melee lands on the player),
// `possession_end_to_end.rs` (possessed actor melee), and the body-generic
// `unified_melee.rs` tests (player + peaceful-NPC-with-kit + hostile actor all
// enter the SAME lifecycle from `ActorActionMessage::Melee`).

/// Silence the test-only helper.
#[test]
fn default_combat_tuning_helper_exists() {
    let _ = default_combat_tuning();
}
