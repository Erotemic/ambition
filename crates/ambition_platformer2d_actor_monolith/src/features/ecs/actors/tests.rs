//! Tests for the actor tick helpers: shark charge-crash geometry, nearest-same-kind
//! neighbor lookup, holding-position spread, per-actor crowding, and pose sync.

use super::*;
use ambition_combat::components::CenteredAabb;

#[test]
fn shark_crashes_on_a_fast_charge_blocked_on_either_axis() {
    let chase = 100.0;
    let fast = chase * 2.0; // > chase * 1.5
    let p = ae::Vec2::new(50.0, 50.0);
    let still = ae::Vec2::ZERO;
    // Horizontal charge rammed into a side wall (didn't move, vel zeroed).
    assert!(shark_charge_crashed_geometry(
        ae::Vec2::new(fast, 0.0),
        p,
        p,
        still,
        chase
    ));
    // Vertical charge UP into a ceiling — the case the old X-only check missed.
    assert!(shark_charge_crashed_geometry(
        ae::Vec2::new(0.0, -fast),
        p,
        p,
        still,
        chase
    ));
    // Still travelling (not blocked) → no crash.
    assert!(!shark_charge_crashed_geometry(
        ae::Vec2::new(fast, 0.0),
        ae::Vec2::new(60.0, 50.0),
        p,
        ae::Vec2::new(fast, 0.0),
        chase
    ));
    // A slow drift into the wall is not a hard charge → no crash.
    assert!(!shark_charge_crashed_geometry(
        ae::Vec2::new(chase, 0.0),
        p,
        p,
        still,
        chase
    ));
}

#[test]
fn nearest_neighbor_is_same_kind_and_closest() {
    use ambition_combat::crowd::CrowdKind;
    let reqs = vec![
        ("a".to_string(), ae::Vec2::new(0.0, 0.0), CrowdKind::Ground),
        ("b".to_string(), ae::Vec2::new(10.0, 0.0), CrowdKind::Ground), // closest to a
        (
            "c".to_string(),
            ae::Vec2::new(100.0, 0.0),
            CrowdKind::Ground,
        ),
        (
            "flyer".to_string(),
            ae::Vec2::new(1.0, 0.0),
            CrowdKind::Aerial,
        ), // closer but wrong kind
    ];
    let neighbors = compute_nearest_neighbors(&reqs);
    // a's nearest same-kind neighbor is b (10px), not the aerial flyer
    // (1px, different kind).
    assert_eq!(neighbors.get("a"), Some(&ae::Vec2::new(10.0, 0.0)));
    // The lone aerial actor has no same-kind peer → absent.
    assert!(!neighbors.contains_key("flyer"));
}

/// Same-faction (Enemy) map for the given ids — the common case where anti-clump
/// should fire. Crowding only counts same-faction allies now.
fn same_faction(
    ids: &[&str],
) -> std::collections::HashMap<String, ambition_combat::components::ActorFaction> {
    ids.iter()
        .map(|id| {
            (
                id.to_string(),
                ambition_combat::components::ActorFaction::Enemy,
            )
        })
        .collect()
}

/// No active grudges/targets — the common case for the crowding tests below, which
/// exercise plain same-faction anti-clump (no one is fighting anyone in particular).
fn no_opponents() -> std::collections::HashMap<String, String> {
    std::collections::HashMap::new()
}

#[test]
fn crowding_pushes_clustered_ground_actors_apart() {
    let reqs = vec![
        ("a".to_string(), ae::Vec2::new(0.0, 0.0), CrowdKind::Ground),
        ("b".to_string(), ae::Vec2::new(20.0, 0.0), CrowdKind::Ground), // within 80px
    ];
    let crowding = compute_crowding_by_id(&reqs, &same_faction(&["a", "b"]), &no_opponents());
    let a = crowding.get("a").expect("a is crowded by b");
    let b = crowding.get("b").expect("b is crowded by a");
    assert_eq!(a.same_faction_count, 1);
    // a is left of b → a pushes left (-x), b pushes right (+x).
    assert!(
        a.away_dir.x < 0.0,
        "a should be pushed leftward away from b, got {:?}",
        a.away_dir
    );
    assert!(
        b.away_dir.x > 0.0,
        "b should be pushed rightward away from a, got {:?}",
        b.away_dir
    );
}

#[test]
fn crowding_ignores_actors_outside_the_radius() {
    let reqs = vec![
        ("a".to_string(), ae::Vec2::new(0.0, 0.0), CrowdKind::Ground),
        (
            "b".to_string(),
            ae::Vec2::new(500.0, 0.0),
            CrowdKind::Ground,
        ), // > 80px
    ];
    assert!(
        compute_crowding_by_id(&reqs, &same_faction(&["a", "b"]), &no_opponents()).is_empty(),
        "actors farther apart than the crowding radius get no signal"
    );
}

#[test]
fn crowding_ignores_a_different_faction_opponent() {
    // The spectator-duel stall: two hostiles of DIFFERENT factions stand within
    // the crowding radius. Anti-clump is for same-faction allies fanning out, so
    // a different-faction opponent must NOT register as crowding — otherwise the
    // back-actor hold rule freezes both fighters instead of letting them close.
    let reqs = vec![
        (
            "pca".to_string(),
            ae::Vec2::new(0.0, 0.0),
            CrowdKind::Ground,
        ),
        (
            "robot".to_string(),
            ae::Vec2::new(20.0, 0.0),
            CrowdKind::Ground,
        ), // within 80px
    ];
    let mut factions = std::collections::HashMap::new();
    factions.insert(
        "pca".to_string(),
        ambition_combat::components::ActorFaction::Enemy,
    );
    factions.insert(
        "robot".to_string(),
        ambition_combat::components::ActorFaction::Boss,
    );
    assert!(
        compute_crowding_by_id(&reqs, &factions, &no_opponents()).is_empty(),
        "different-faction opponents must not crowd each other"
    );
}

#[test]
fn crowding_ignores_a_same_faction_grudge_opponent() {
    // The grudge-duel stall: two SAME-faction `Npc`s feuding via a mutual grudge
    // stand within the crowding radius. Each is actively TARGETING the other, so —
    // even though they share a faction — neither must register the other as a
    // crowding ally, or the back-actor hold rule freezes the duel (the exact regress
    // the duel reframe hit). The `opponent_id_by_id` map (id → the id it's fighting)
    // overrides the same-faction default.
    let reqs = vec![
        (
            "pca".to_string(),
            ae::Vec2::new(0.0, 0.0),
            CrowdKind::Ground,
        ),
        (
            "robot".to_string(),
            ae::Vec2::new(20.0, 0.0),
            CrowdKind::Ground,
        ), // within 80px
    ];
    let mut opponents = std::collections::HashMap::new();
    opponents.insert("pca".to_string(), "robot".to_string());
    opponents.insert("robot".to_string(), "pca".to_string());
    assert!(
        compute_crowding_by_id(&reqs, &same_faction(&["pca", "robot"]), &opponents).is_empty(),
        "two same-faction bodies fighting EACH OTHER must not anti-clump apart"
    );
}

#[test]
fn aerial_actors_crowd_at_a_wider_radius_than_ground() {
    // 150px apart: outside the 80px ground radius but inside the 220px
    // aerial radius. Two flyers crowd; two ground actors at the same
    // spacing do not.
    let aerial = vec![
        ("f1".to_string(), ae::Vec2::new(0.0, 0.0), CrowdKind::Aerial),
        (
            "f2".to_string(),
            ae::Vec2::new(150.0, 0.0),
            CrowdKind::Aerial,
        ),
    ];
    assert!(
        !compute_crowding_by_id(&aerial, &same_faction(&["f1", "f2"]), &no_opponents()).is_empty(),
        "aerial actors crowd at 150px (aerial radius 220)"
    );
    let ground = vec![
        ("g1".to_string(), ae::Vec2::new(0.0, 0.0), CrowdKind::Ground),
        (
            "g2".to_string(),
            ae::Vec2::new(150.0, 0.0),
            CrowdKind::Ground,
        ),
    ];
    assert!(
        compute_crowding_by_id(&ground, &same_faction(&["g1", "g2"]), &no_opponents()).is_empty(),
        "ground actors don't crowd at 150px (>80px ground radius)"
    );
}

fn burning_shark_enemy() -> super::actor_clusters::ActorClusterSeed {
    let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(126.0, 52.0));
    let mut seed = super::actor_clusters::ActorClusterSeed::new(
        "burning_shark".to_string(),
        "Burning Shark".to_string(),
        aabb,
        ambition_entity_catalog::placements::CharacterBrain::Custom("fixture_mount".into()),
        &[],
    );
    // the capability is STATED, because it is the input under test. It
    // arrived from the `fixture_mount` archetype row's `charge_crash_explodes`
    // until AC6 deleted the rows; a death trait is a character's fact now, and
    // the function below takes the resolved capability rather than a body.
    seed.caps.charge_crash_explodes = true;
    seed
}

#[test]
fn sync_actor_pose_uses_feature_aabb_and_actor_facing() {
    use bevy::prelude::{App, Update};

    let mut app = App::new();
    app.add_systems(Update, sync_actor_poses_from_feature_aabbs);

    let mut enemy = burning_shark_enemy();
    enemy.kin.facing = -1.0;
    let entity = app
        .world_mut()
        .spawn((
            FeatureSimEntity,
            CenteredAabb::from_center_size(ae::Vec2::new(40.0, 80.0), ae::Vec2::new(20.0, 30.0)),
            ambition_combat::components::ActorPose::default(),
            enemy.into_components(),
        ))
        .id();

    app.update();

    let entity_ref = app.world().entity(entity);
    let pose = entity_ref
        .get::<ambition_combat::components::ActorPose>()
        .unwrap();
    assert_eq!(pose.center, ae::Vec2::new(40.0, 80.0));
    assert_eq!(pose.feet, ae::Vec2::new(40.0, 95.0));
    assert_eq!(pose.facing, -1.0);
    assert!(
        entity_ref
            .get::<bevy::transform::components::Transform>()
            .is_none(),
        "ActorPose sync should not require a gameplay Transform shim"
    );
}

#[test]
fn shark_charge_crash_detects_solo_charge_wall_hit() {
    let mut enemy = burning_shark_enemy();
    let previous_pos = ae::Vec2::new(120.0, 80.0);
    enemy.kin.pos = previous_pos;
    enemy.kin.vel = ae::Vec2::ZERO;
    enemy.health.reset();
    let charge_vec = ae::Vec2::new(enemy.config.tuning.chase_speed * 2.0, 0.0);
    assert!(shark_charge_crashed_parts(
        &enemy.caps,
        enemy.health.alive(),
        enemy.kin.pos,
        enemy.kin.vel,
        enemy.config.tuning.chase_speed,
        false,
        charge_vec,
        previous_pos,
    ));
}

/// ⭐⭐ THE QUESTION "IS SOMEBODY RIDING ME" IS ANSWERED BY THE SADDLE.
///
/// ⛔⛔ THE TWO TESTS BELOW PASS `is_being_ridden` AS A LITERAL, so they pin the
/// PREDICATE and say nothing about which component the caller reads to fill it.
/// That gap shipped: `integrate_sim_bodies` asked the shark for
/// `Option<&Mounted>`, and `Mounted` is stamped on the RIDER — see
/// `mount::board`, which puts `RidingOn`/`Mounted` on the rider and `MountSlot`
/// on the mount. So a shark with somebody in its saddle answered "nobody is
/// riding me" forever, and the guard on its charge-crash suicide could never
/// become false. Every one of these tests stayed green through all of it.
///
/// ⭐ SO THIS ONE PINS THE WIRING. It asserts the two relationship ends answer
/// DIFFERENT questions, which is the substitution that caused the bug: a body
/// wearing the rider's marker is not thereby being ridden, and a saddle with
/// nobody in it is not either.
///
/// ⚠ IT IS NOT THE GEOMETRY POISON. Proving that an OCCUPIED shark survives a
/// real wall impact while a riderless one detonates needs both bodies driven
/// into stage geometry through the production integrator; that is still owed.
#[test]
fn a_saddle_answers_who_is_riding_and_the_riders_marker_does_not() {
    use ambition_mount::{MountSlot, Mounted};
    use bevy::prelude::*;

    let mut app = App::new();
    let rider = app.world_mut().spawn_empty().id();
    // The mount: a saddle with somebody in it.
    let ridden = app.world_mut().spawn(MountSlot { rider: Some(rider) }).id();
    // The same mount after its rider left: the saddle outlives the ride.
    let empty = app.world_mut().spawn(MountSlot { rider: None }).id();
    // A body wearing the RIDER's marker. `Mounted` on a mount is a category
    // error, and the point is that it reads as a perfectly ordinary `false`.
    let wearing_riders_marker = app.world_mut().spawn(Mounted).id();

    let being_ridden = |app: &App, entity: Entity| -> bool {
        app.world()
            .get::<MountSlot>(entity)
            .is_some_and(|slot| slot.rider.is_some())
    };

    assert!(
        being_ridden(&app, ridden),
        "an occupied saddle did not report a rider, so every rule that protects \
         a ridden mount is disarmed"
    );
    assert!(
        !being_ridden(&app, empty),
        "an EMPTY saddle reported a rider — `MountSlot` outlives a dismount, so \
         presence of the component cannot be the test"
    );
    assert!(
        !being_ridden(&app, wearing_riders_marker),
        "the rider's own marker answered the mount's question"
    );
    // ⛔ AND THE OLD READ IS SHOWN WRONG, not merely absent: this is exactly what
    // `integrate_sim_bodies` used to compute for the shark.
    assert!(
        app.world().get::<Mounted>(ridden).is_none(),
        "a mount carrying a rider has no `Mounted` of its own — which is why \
         reading it always answered false"
    );
}

#[test]
fn shark_charge_crash_ignores_mounted_or_noncharge_cases() {
    let mut enemy = burning_shark_enemy();
    let previous_pos = ae::Vec2::new(120.0, 80.0);
    enemy.kin.pos = previous_pos;
    enemy.kin.vel = ae::Vec2::ZERO;
    enemy.health.reset();
    let chase_speed = enemy.config.tuning.chase_speed;
    let charge_vec = ae::Vec2::new(chase_speed * 2.0, 0.0);
    assert!(!shark_charge_crashed_parts(
        &enemy.caps,
        enemy.health.alive(),
        enemy.kin.pos,
        enemy.kin.vel,
        chase_speed,
        true,
        charge_vec,
        previous_pos,
    ));
    assert!(!shark_charge_crashed_parts(
        &enemy.caps,
        enemy.health.alive(),
        enemy.kin.pos,
        enemy.kin.vel,
        chase_speed,
        false,
        ae::Vec2::new(chase_speed, 0.0),
        previous_pos,
    ));
}
