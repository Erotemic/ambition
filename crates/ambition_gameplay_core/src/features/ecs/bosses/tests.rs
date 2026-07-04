//! Tests for boss sprite-metrics derivation (the pure, App-free path) and the
//! boss tick/sync systems.

use super::*;
use ambition_engine_core as ae;

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
/// Boss-fold pin (fable review §A1): a live boss strike now spawns its Boss-faction
/// `Hitbox` through the SHARED moveset runtime — `trigger_boss_attack_moves` starts the
/// active profile's move and `advance_move_playback` spawns the strike hitbox from its
/// Active-window hit volume — retiring the bespoke `sync_boss_strike_hitboxes` poll. A
/// subtly-broken wiring (no move, wrong faction, no geometry) would deal no strike
/// damage and escape the contact-only `boss_contact_iframes` test; this guards it.
#[test]
fn boss_strike_spawns_a_boss_hitbox_through_the_moveset() {
    use ambition_characters::brain::{BossAttackIntent, BossAttackProfile, BossCapability};
    use bevy::prelude::*;

    let combat_size = ae::Vec2::new(80.0, 80.0);
    let behavior = crate::features::bosses::BossBehaviorProfile::clockwork_warden();
    // The boss's attack moveset: a FloorSlam geometry strike → an Active-window move.
    let cap = BossCapability {
        specials: vec![(BossAttackProfile::Strike("floor_slam".to_string()), 0.5)],
    };
    let moveset = crate::features::boss_attack_moveset(&cap, &behavior, combat_size, &[])
        .expect("a boss with a strike → a moveset");

    let mut app = App::new();
    app.init_resource::<ambition_time::WorldTime>();
    app.world_mut()
        .resource_mut::<ambition_time::WorldTime>()
        .scaled_dt = 0.016;
    app.world_mut()
        .resource_mut::<ambition_time::WorldTime>()
        .raw_dt = 0.016;
    app.add_message::<crate::combat::moveset::MoveEventMessage>();
    app.add_systems(
        Update,
        (
            crate::features::trigger_boss_attack_moves,
            crate::combat::moveset::advance_move_playback,
        )
            .chain(),
    );
    // §A1 split: the driver's fire INTENT names FloorSlam → the trigger starts the move.
    let intent = BossAttackIntent {
        active_profile: Some(BossAttackProfile::Strike("floor_slam".to_string())),
        ..Default::default()
    };
    app.world_mut().spawn((
        crate::combat::components::ActorFaction::Boss,
        crate::actor::BodyKinematics {
            pos: ae::Vec2::new(300.0, 300.0),
            vel: ae::Vec2::ZERO,
            size: combat_size,
            facing: 1.0,
        },
        intent,
        moveset,
        super::super::FeatureSimEntity,
    ));
    // Frame 1 triggers the move; frame 2 advances it into its Active window.
    app.update();
    app.update();

    let mut q = app.world_mut().query::<&crate::combat::hitbox::Hitbox>();
    let hits: Vec<_> = q.iter(app.world()).collect();
    assert!(
        !hits.is_empty(),
        "a live boss strike should spawn at least one Boss hitbox via the moveset"
    );
    let hb = hits[0];
    assert_eq!(
        hb.source,
        crate::combat::components::ActorFaction::Boss,
        "boss strike hitbox must carry the Boss faction so apply_hitbox_damage routes it"
    );
    assert!(hb.damage >= 1, "strike hitbox should deal damage");
    // The hit volume (FloorSlam sits below the body) is non-degenerate.
    assert!(
        hb.half_extent.x > 0.0 && hb.half_extent.y > 0.0,
        "strike hitbox should have a real extent from the move's hit volume"
    );
}

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
        ambition_characters::actor::BossBrain::Dormant,
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
        ambition_characters::actor::BossBrain::Dormant,
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

/// End-to-end guard for Jon's "attacks on the mockingbird whiff" bug
/// (2026-06-21). The mockingbird's sprite RON now carries `body_metrics`
/// (a static alpha-bbox hurtbox), and `sprite_target_for_boss` maps the
/// "mockingbird" behavior to its `"mockingbird_boss"` sheet target — so
/// the full chain (behavior id → sheet target → baked registry →
/// body_metrics → derived combat geometry) resolves the body box instead
/// of falling back to the bare, frame-unaligned `combat_size` box. Both
/// pieces are required: drop the RON body_metrics OR the target mapping
/// and the lookup misses again.
#[test]
fn mockingbird_resolves_a_body_hurtbox_from_the_baked_registry() {
    use crate::features::bosses::BossBehaviorProfile;

    // The behavior id must map to the sheet target the RON declares,
    // otherwise the registry lookup misses (the masked half of the bug).
    assert_eq!(
        sprite_target_for_boss("mockingbird"),
        "mockingbird_boss",
        "mockingbird behavior must map to its 'mockingbird_boss' sheet target",
    );

    let registry = crate::character_sprites::baked_sheet_registry();
    let behavior = BossBehaviorProfile::mockingbird();
    let combat_size = behavior.combat_size.unwrap_or(ae::Vec2::new(500.0, 185.0));
    let pos = ae::Vec2::new(500.0, 400.0);
    let mut boss = super::super::boss_clusters::BossClusterScratch::new(
        "boss_mockingbird",
        "Mockingbird",
        ae::Aabb::new(pos, combat_size * 0.5),
        ambition_characters::actor::BossBrain::Dormant,
    );
    boss.config.behavior = behavior;

    let (metrics, derived_size) = boss_sprite_metrics_from_registry(boss.as_ref(), &registry)
        .expect("mockingbird sheet target should have body metrics in the baked registry");
    // Unlike GNU-ton (per-animation hurtboxes), the mockingbird's body
    // comes from a single static alpha bbox.
    assert!(
        metrics.body_pixel_bbox.is_some(),
        "mockingbird should carry a static body_pixel_bbox hurtbox",
    );
    assert!(
        derived_size.is_some(),
        "a static body bbox should derive a combat_size from the visible body",
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
