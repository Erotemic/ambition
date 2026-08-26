use super::*;
use crate::enemy_projectile::test_support::live_projectile_bodies;
use crate::features::ecs::actor_clusters::ActorClusterSeed;
use crate::projectile::ProjectileSeqCounter;
use ambition_characters::brain::{ActionSet, RangedActionSpec, RangedCommitment};

/// Build a rider-shaped hostile actor: standalone PirateRaider
/// archetype on the runtime side, but the caller is expected to
/// attach a [`crate::features::RidingOn`] component to the
/// spawned entity so the ranged-projectile handler routes the
/// fire through the lasersword path.
use super::super::actor_clusters::ActorClusterBundle;

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
    app.add_message::<crate::projectile::ProjectileSpawnRequest>();
    app.init_resource::<ProjectileSeqCounter>();
    // The consumer emits ProjectileSpawnRequest; chain the immediate materializer
    // so the projectile entity spawns within the update.
    app.add_systems(
        Update,
        (
            spawn_projectiles_from_brain_actions,
            ambition_projectiles::materialize_projectiles_for_this_tick,
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
    // and presentation defaults.
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
                spec: RangedActionSpec::rock(300.0, 1),
                origin: actor_pos,
                dir: ae::Vec2::new(1.0, 0.0),
                dir_policy: ae::GameplayFramePolicy::WorldSpace,
                commitment: RangedCommitment::Attempt,
            },
        });
    app.update();
    let projectiles = live_projectile_bodies(&mut app);
    assert_eq!(projectiles.len(), 1);
    assert_eq!(
        projectiles[0].body.kin.pos,
        actor_pos + ae::Vec2::new(0.0, -8.0),
        "an ordinary body fires from its authored body origin, not the gun-sword hand",
    );
    let mut owners = app
        .world_mut()
        .query::<&crate::projectile::ProjectileOwner>();
    assert_eq!(
        owners.single(app.world()).expect("one projectile owner").0,
        actor,
        "the firing body entity is the sole owner identity carried by the shot",
    );
}

/// The ranged-fire consumer stamps the firing actor's authored ranged
/// visual id onto the spawned projectile independently of ownership. A
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
                spec: RangedActionSpec::rock(300.0, 1),
                origin: actor_pos,
                dir: ae::Vec2::new(1.0, 0.0),
                dir_policy: ae::GameplayFramePolicy::WorldSpace,
                commitment: RangedCommitment::Attempt,
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
    actor_bundle.1 .6.surface_normal = ae::Vec2::new(-1.0, 0.0);
    let actor = app.world_mut().spawn(actor_bundle).id();
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
        .write(ActorActionMessage {
            actor,
            request: ActionRequest::Ranged {
                spec: RangedActionSpec::rock(300.0, 1),
                origin: actor_pos,
                dir: ae::Vec2::new(1.0, 0.0),
                dir_policy: ae::GameplayFramePolicy::ControlledBodyLocal,
                commitment: RangedCommitment::Attempt,
            },
        });
    app.update();
    let projectiles = live_projectile_bodies(&mut app);
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
                spec: RangedActionSpec::bolt(500.0, 1),
                origin: actor_pos,
                dir: ae::Vec2::new(1.0, 0.0),
                dir_policy: ae::GameplayFramePolicy::WorldSpace,
                commitment: RangedCommitment::Attempt,
            },
        });
    app.update();
    assert!(
        live_projectile_bodies(&mut app).is_empty(),
        "dead actor must not spawn a projectile",
    );
}

/// Suppress unused-import noise from the test-only `ActionSet`
/// reference — kept for callers that grow this module's tests.
fn _silence_action_set_import(_: ActionSet) {}

// Melee-start is a moveset `"attack"` move for every body now
// (`combat::moveset::trigger_moveset_moves`); it is pinned through the REAL schedule by
// `ambition_app/tests/enemy_attacks_player.rs` (actor melee lands on the player),
// `possession_end_to_end.rs` (possessed actor melee), and the body-generic `unified_melee.rs` tests
// (player + peaceful-NPC-with-kit + hostile actor all enter the SAME lifecycle from
// `ActorActionMessage::Melee`).

/// Silence the test-only helper.
#[test]
fn default_combat_tuning_helper_exists() {
    let _ = default_combat_tuning();
}

/// ⭐⭐ AN ACCEPTED MOVE'S SHOT IS OWED; A CONTROLLER'S POLL IS NOT.
///
/// Two roads reach this one consumer. A brain emits `fire` on every in-band
/// tick and rate-limits itself nowhere, so its request is an ATTEMPT and this
/// weapon recharge is the only thing between it and a stream of projectiles. A
/// moveset fire event is the other road: the body accepted the move a quarter
/// of a second ago and PAID for it then (`moveset::start_move`), and the
/// windup the player watched was the promise.
///
/// ⛔ ASKING THE SAME QUESTION FOR BOTH is what made an accepted Charge Shot
/// play its charge, flash its muzzle and fire nothing.
///
/// ⛔ THE ARMS STRADDLE THE ONE THING UNDER TEST — same hot weapon, same
/// request, different commitment — so a consumer that had simply stopped
/// enforcing the floor would fail the second arm.
#[test]
fn a_committed_shot_fires_through_a_hot_weapon_and_an_attempt_does_not() {
    fn shots_fired(commitment: RangedCommitment) -> usize {
        let mut app = build_app();
        let actor_pos = ae::Vec2::new(300.0, 300.0);
        let aabb = ae::Aabb::new(actor_pos, ae::Vec2::new(14.0, 23.0));
        let enemy = ActorClusterSeed::new(
            "hot_weapon",
            "Skitter",
            aabb,
            ambition_entity_catalog::placements::CharacterBrain::Custom("small_skitter".into()),
            &[],
        );
        let mut bundle = enemy_actor(enemy);
        // The weapon is MID-RECHARGE. `.1 .7` is `BodyMelee` in the cluster
        // bundle — the body-side authority on the ranged fire rate.
        bundle.1 .7.ranged_cooldown = 0.9;
        let actor = app.world_mut().spawn(bundle).id();
        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
            .write(ActorActionMessage {
                actor,
                request: ActionRequest::Ranged {
                    spec: RangedActionSpec::rock(300.0, 1),
                    origin: actor_pos,
                    dir: ae::Vec2::new(1.0, 0.0),
                    dir_policy: ae::GameplayFramePolicy::WorldSpace,
                    commitment,
                },
            });
        app.update();
        live_projectile_bodies(&mut app).len()
    }

    assert_eq!(
        shots_fired(RangedCommitment::CommittedMove),
        1,
        "the move was accepted and its recharge already spent — refusing here \
         drops a shot the fighter committed to and the player was shown"
    );
    assert_eq!(
        shots_fired(RangedCommitment::Attempt),
        0,
        "a controller poll has promised nothing, so the weapon's recharge is \
         still the rate limit — the floor MOVED upstream for committed moves, \
         it did not go away"
    );
}
