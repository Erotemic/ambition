use super::*;
use crate::engine_core as ae;

/// The extracted pure metrics derivation (`boss_sprite_metrics_from_registry`)
/// reproduces GNU-ton's metrics without the ECS system, via
/// `crate::character_sprites::baked_sheet_registry()` (no Bevy `App`). It pins a
/// non-obvious structural fact discovered while extracting it:
/// GNU-ton's combat geometry comes entirely from its **per-animation
/// hurtboxes** (the `animations` map), not from static
/// `body_pixel_parts`/`bbox` — so the derivation finds no body bbox,
/// leaves `combat_offset` at zero, and derives no combat size. A
/// regression that started emitting static body parts for GNU-ton
/// (changing its combat envelope + pogo zone) would trip this.
#[test]
fn boss_spawn_hurtboxes_resolves_without_panicking() {
    // The headless renderer helper builds a transient boss + baked
    // registry and returns its rest-pose hurtboxes. Smoke-guard that
    // it resolves a non-empty volume (real metrics or the combat-size
    // fallback) and never panics.
    let aabb = ae::Aabb::new(ae::Vec2::new(500.0, 400.0), ae::Vec2::new(110.0, 110.0));
    let hbs = boss_spawn_hurtboxes(
        "boss_gnu_ton",
        "GNU-ton",
        aabb,
        crate::actor::BossBrain::Dormant,
    );
    assert!(!hbs.is_empty(), "a boss should expose at least one hurtbox");
}

#[test]
fn gnu_ton_metrics_come_from_per_animation_hurtboxes() {
    use crate::features::bosses::BossBehaviorProfile;

    let registry = crate::character_sprites::baked_sheet_registry();
    let pos = ae::Vec2::new(500.0, 400.0);
    let behavior = BossBehaviorProfile::gnu_ton();
    let combat_size = behavior.combat_size.unwrap_or(ae::Vec2::new(220.0, 220.0));
    let mut boss = super::super::boss_clusters::BossClusterScratch::new(
        "boss_gnu_ton",
        "GNU-ton",
        ae::Aabb::new(pos, combat_size * 0.5),
        crate::actor::BossBrain::Dormant,
    );
    boss.config.behavior = behavior;

    let (metrics, derived_size) = boss_sprite_metrics_from_registry(boss.as_ref(), &registry)
        .expect("gnu_ton sprite target should have body metrics in the baked registry");
    // The head/hand hurtboxes (what damageable_volumes consumes) live
    // in the per-animation map.
    assert!(
        !metrics.animations.is_empty(),
        "gnu_ton should carry per-animation hurtboxes"
    );
    assert!(
        metrics.animations.contains_key("rest"),
        "gnu_ton should have a 'rest' animation hurtbox"
    );
    // No static body bbox → no offset / derived size.
    assert_eq!(
        metrics.combat_offset,
        ae::Vec2::ZERO,
        "gnu_ton has no static body_pixel_parts, so combat_offset stays zero"
    );
    assert!(
        metrics.body_pixel_parts.is_empty() && metrics.body_pixel_bbox.is_none(),
        "gnu_ton's body geometry is per-animation, not static parts"
    );
    assert!(
        derived_size.is_none(),
        "with no static body bbox, no combat_size is derived"
    );
}

#[test]
fn front_wall_clearance_ignores_floor_below_body_lane() {
    let body = ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(40.0, 80.0));
    let world = ae::World::new(
        "test",
        ae::Vec2::new(400.0, 300.0),
        ae::Vec2::ZERO,
        vec![ae::Block::solid(
            "floor",
            // Floor tile whose top just touches the boss feet.  This is
            // support geometry, not a side wall the boss would run into.
            ae::Vec2::new(100.0, 204.0),
            ae::Vec2::new(260.0, 24.0),
        )],
    );
    assert_eq!(
        horizontal_front_wall_clearance(&world, body, 1.0, 200.0),
        None
    );
}

#[test]
fn front_wall_clearance_ignores_small_floor_skin_overlap() {
    let body = ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(40.0, 80.0));
    let world = ae::World::new(
        "test",
        ae::Vec2::new(400.0, 300.0),
        ae::Vec2::ZERO,
        vec![ae::Block::solid(
            "floor_skin",
            // Top is 2 px above the body bottom.  Integration/contact
            // tolerance can create this tiny overlap, but it should not
            // block horizontal approach.
            ae::Vec2::new(100.0, 202.0),
            ae::Vec2::new(260.0, 24.0),
        )],
    );
    assert_eq!(
        horizontal_front_wall_clearance(&world, body, 1.0, 200.0),
        None
    );
}

#[test]
fn front_wall_clearance_reports_side_wall_in_direction_of_player() {
    let body = ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(40.0, 80.0));
    let world = ae::World::new(
        "test",
        ae::Vec2::new(400.0, 300.0),
        ae::Vec2::ZERO,
        // Block::solid is (name, min, size), so this wall spans
        // x:[180, 200]. The body right edge sits at 140, so the
        // edge-to-edge clearance toward the player is 180 - 140 = 40.
        vec![ae::Block::solid(
            "wall",
            ae::Vec2::new(180.0, 100.0),
            ae::Vec2::new(20.0, 160.0),
        )],
    );
    let clearance = horizontal_front_wall_clearance(&world, body, 1.0, 200.0).unwrap();
    assert!((clearance - 40.0).abs() < 0.01, "clearance = {clearance}");
    assert_eq!(
        horizontal_front_wall_clearance(&world, body, -1.0, 200.0),
        None
    );
}
