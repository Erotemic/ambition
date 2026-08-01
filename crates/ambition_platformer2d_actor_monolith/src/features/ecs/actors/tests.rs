//! Tests for the actor tick helpers: shark charge-crash geometry, nearest-same-kind
//! neighbor lookup, holding-position spread, per-actor crowding, and pose sync.

use super::*;

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
    use crate::combat::slots::SlotKind;
    let reqs = vec![
        ("a".to_string(), ae::Vec2::new(0.0, 0.0), SlotKind::Melee),
        ("b".to_string(), ae::Vec2::new(10.0, 0.0), SlotKind::Melee), // closest to a
        ("c".to_string(), ae::Vec2::new(100.0, 0.0), SlotKind::Melee),
        (
            "flyer".to_string(),
            ae::Vec2::new(1.0, 0.0),
            SlotKind::Aerial,
        ), // closer but wrong kind
    ];
    let neighbors = compute_nearest_neighbors(&reqs);
    // a's nearest same-kind neighbor is b (10px), not the aerial flyer
    // (1px, different kind).
    assert_eq!(neighbors.get("a"), Some(&ae::Vec2::new(10.0, 0.0)));
    // The lone aerial actor has no same-kind peer → absent.
    assert!(!neighbors.contains_key("flyer"));
}

#[test]
fn unassigned_actors_spread_across_distinct_holding_positions() {
    use crate::combat::slots::{assign_slots, CombatSlotBoard, SlotKind, SlotRequest};
    // 2 melee slots, 4 melee actors → 2 win slots, 2 are leftover.
    let mut board = CombatSlotBoard::new(2, 80.0, 0, 0.0, 0.0);
    let target = ae::Vec2::ZERO;
    let requests: Vec<(String, ae::Vec2, SlotKind)> = (0..4)
        .map(|i| {
            (
                format!("e{i}"),
                ae::Vec2::new(i as f32 * 30.0, 0.0),
                SlotKind::Melee,
            )
        })
        .collect();
    let slot_reqs: Vec<SlotRequest> = requests
        .iter()
        .map(|(id, pos, kind)| SlotRequest {
            actor_id: id,
            actor_pos: *pos,
            kind: *kind,
        })
        .collect();
    assign_slots(&mut board, target, &slot_reqs);

    let holding = compute_holding_positions(&board, &requests, target);
    let assigned = requests
        .iter()
        .filter(|(id, _, _)| board.slot_for(id).is_some())
        .count();
    assert_eq!(assigned, 2, "two actors should win the two slots");
    assert_eq!(
        holding.len(),
        2,
        "the two leftover actors get holding positions"
    );
    // The leftover actors are spread round-robin across the two slots'
    // holding points — they must not share a single clump point.
    let positions: Vec<ae::Vec2> = holding.values().copied().collect();
    assert_ne!(
        positions[0], positions[1],
        "leftover actors must occupy distinct holding positions, not clump"
    );
    // Deterministic: recomputing yields the same map.
    assert_eq!(
        holding,
        compute_holding_positions(&board, &requests, target)
    );
}

/// Same-faction (Enemy) map for the given ids — the common case where anti-clump
/// should fire. Crowding only counts same-faction allies now.
fn same_faction(
    ids: &[&str],
) -> std::collections::HashMap<String, super::super::super::components::ActorFaction> {
    ids.iter()
        .map(|id| {
            (
                id.to_string(),
                super::super::super::components::ActorFaction::Enemy,
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
    use crate::combat::slots::SlotKind;
    let reqs = vec![
        ("a".to_string(), ae::Vec2::new(0.0, 0.0), SlotKind::Melee),
        ("b".to_string(), ae::Vec2::new(20.0, 0.0), SlotKind::Melee), // within 80px
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
    use crate::combat::slots::SlotKind;
    let reqs = vec![
        ("a".to_string(), ae::Vec2::new(0.0, 0.0), SlotKind::Melee),
        ("b".to_string(), ae::Vec2::new(500.0, 0.0), SlotKind::Melee), // > 80px
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
    use crate::combat::slots::SlotKind;
    let reqs = vec![
        ("pca".to_string(), ae::Vec2::new(0.0, 0.0), SlotKind::Melee),
        (
            "robot".to_string(),
            ae::Vec2::new(20.0, 0.0),
            SlotKind::Melee,
        ), // within 80px
    ];
    let mut factions = std::collections::HashMap::new();
    factions.insert(
        "pca".to_string(),
        super::super::super::components::ActorFaction::Enemy,
    );
    factions.insert(
        "robot".to_string(),
        super::super::super::components::ActorFaction::Boss,
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
    use crate::combat::slots::SlotKind;
    let reqs = vec![
        ("pca".to_string(), ae::Vec2::new(0.0, 0.0), SlotKind::Melee),
        (
            "robot".to_string(),
            ae::Vec2::new(20.0, 0.0),
            SlotKind::Melee,
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
    use crate::combat::slots::SlotKind;
    // 150px apart: outside the 80px ground radius but inside the 220px
    // aerial radius. Two flyers crowd; two ground actors at the same
    // spacing do not.
    let aerial = vec![
        ("f1".to_string(), ae::Vec2::new(0.0, 0.0), SlotKind::Aerial),
        (
            "f2".to_string(),
            ae::Vec2::new(150.0, 0.0),
            SlotKind::Aerial,
        ),
    ];
    assert!(
        !compute_crowding_by_id(&aerial, &same_faction(&["f1", "f2"]), &no_opponents()).is_empty(),
        "aerial actors crowd at 150px (aerial radius 220)"
    );
    let ground = vec![
        ("g1".to_string(), ae::Vec2::new(0.0, 0.0), SlotKind::Melee),
        ("g2".to_string(), ae::Vec2::new(150.0, 0.0), SlotKind::Melee),
    ];
    assert!(
        compute_crowding_by_id(&ground, &same_faction(&["g1", "g2"]), &no_opponents()).is_empty(),
        "ground actors don't crowd at 150px (>80px ground radius)"
    );
}

fn burning_shark_enemy() -> super::actor_clusters::ActorClusterSeed {
    let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(126.0, 52.0));
    super::actor_clusters::ActorClusterSeed::new(
        "burning_shark".to_string(),
        "Burning Shark".to_string(),
        aabb,
        ambition_entity_catalog::placements::CharacterBrain::Custom("burning_flying_shark".into()),
        &[],
    )
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
            crate::features::ActorPose::default(),
            enemy.into_components(),
        ))
        .id();

    app.update();

    let entity_ref = app.world().entity(entity);
    let pose = entity_ref.get::<crate::features::ActorPose>().unwrap();
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
