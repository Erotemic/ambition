//! Tests for boss sprite-metrics derivation (the pure, App-free path) and the
//! boss tick/sync systems.

use super::*;
use crate::behavior::BossBehaviorProfileExt;
use ambition_platformer2d_core as ae;

/// GNU-ton's combat geometry comes entirely from its per-animation hurtboxes
/// (the `animations` map), not from static `body_pixel_parts`/`bbox`, so the
/// derivation finds no body bbox, leaves `combat_offset` at zero, and derives
/// no combat size. Broken wiring (no move, wrong faction, no geometry) would
/// deal no strike damage and pass the contact-only `boss_contact_iframes`
/// test; this guards it.
#[test]
fn boss_strike_spawns_a_boss_hitbox_through_the_moveset() {
    use ambition_characters::brain::{BossAttackIntent, BossAttackProfile, BossCapability};
    use bevy::prelude::*;

    let combat_size = ae::Vec2::new(80.0, 80.0);
    let behavior = crate::pattern::profile::BossBehaviorProfile::clockwork_warden();
    // The boss's attack moveset: a FloorSlam geometry strike → an Active-window move.
    let cap = BossCapability {
        specials: vec![(BossAttackProfile::Strike("floor_slam".to_string()), 0.5)],
    };
    let moveset = crate::attack_moveset::boss_attack_moveset(
        &ambition_characters::brain::boss_pattern::BossBeat::of_capability(&cap, &[]),
        &behavior,
        combat_size,
    )
        .expect("a boss with a strike → a moveset");

    let mut app = App::new();
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    app.init_resource::<ambition_combat::authored_volumes::AuthoredAttackVolumeResolver>();
    app.init_resource::<ambition_time::WorldTime>();
    app.world_mut()
        .resource_mut::<ambition_time::WorldTime>()
        .set_sim_dt(0.016);
    app.world_mut()
        .resource_mut::<ambition_time::WorldTime>()
        .set_wall_dt(0.016);
    app.add_message::<ambition_combat::moveset::MoveEventMessage>();
    app.add_message::<ambition_vfx::vfx::VfxInRoom>();
    app.add_systems(
        Update,
        (
            crate::ecs::trigger_boss_attack_moves,
            ambition_combat::moveset::advance_move_playback,
        )
            .chain(),
    );
    // §A1 split: the driver's fire intent names FloorSlam → the trigger starts the move.
    let intent = BossAttackIntent {
        active_profile: Some(BossAttackProfile::Strike("floor_slam".to_string())),
        ..Default::default()
    };
    app.world_mut().spawn((
        ambition_characters::actor::ActorFaction::Boss,
        ambition_platformer2d_core::BodyKinematics {
            pos: ae::Vec2::new(300.0, 300.0),
            vel: ae::Vec2::ZERO,
            size: combat_size,
            facing: 1.0,
        },
        intent,
        moveset,
        ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
    ));
    // Frame 1 triggers the move; frame 2 advances it into its Active window.
    app.update();
    app.update();

    let mut q = app.world_mut().query::<&ambition_combat::hitbox::Hitbox>();
    let hits: Vec<_> = q.iter(app.world()).collect();
    assert!(
        !hits.is_empty(),
        "a live boss strike should spawn at least one Boss hitbox via the moveset"
    );
    let hb = hits[0];
    assert_eq!(
        hb.source,
        ambition_vfx::HitSide::Boss,
        "boss strike hitbox must carry the Boss side so apply_hitbox_damage routes it"
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
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    // The headless renderer helper builds a transient boss and baked registry
    // and returns its rest-pose hurtboxes. Check that it returns a non-empty
    // volume (real metrics or the combat-size fallback) and never panics.
    let aabb = ae::Aabb::new(ae::Vec2::new(500.0, 400.0), ae::Vec2::new(110.0, 110.0));
    let hbs = boss_spawn_hurtboxes(&sprite_content,
        crate::test_boss_catalog(),
        "boss_gnu_ton",
        "GNU-ton",
        aabb,
        ambition_entity_catalog::placements::BossBrain::Dormant,
    );
    assert!(!hbs.is_empty(), "a boss should expose at least one hurtbox");
}

/// A sheet whose body geometry is authored per animation (a head hurtbox that
/// bobs with the idle and dives with the head-descent) resolves through
/// `boss_sprite_metrics_from_registry` as an animation map with no static
/// parts, the opposite of the mockingbird's single alpha-bbox below. The fused
/// `gnu_ton_boss` sheet (the Hall's GNU-ton exhibit) is that shape.
///
/// The second half is the opposite shape: the scholar's own `gnu_ton_rider`
/// sheet authors ONE static body, and it is his collision — the fight's
/// complaint was a scholar drawn at a sixth of his size.
#[test]
fn a_per_animation_hurtbox_sheet_yields_animation_metrics_not_static_parts() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    use crate::pattern::profile::BossBehaviorProfile;

    let registry = ambition_sprite_sheet::baked_sheet_registry(&sprite_content);
    let pos = ae::Vec2::new(500.0, 400.0);
    let mut behavior = BossBehaviorProfile::gnu_ton_rider();
    // Aim the lookup at the fused exhibit sheet, which carries the head.
    behavior.sprite_target = Some("gnu_ton_boss".to_string());
    let combat_size = ae::Vec2::new(220.0, 220.0);
    let mut boss = crate::BossClusterScratch::new(&sprite_content,
        crate::test_boss_catalog(),
        "boss_giant_gnu",
        "Giant GNU",
        ae::Aabb::new(pos, combat_size * 0.5),
        ambition_entity_catalog::placements::BossBrain::Dormant,
    );
    boss.config.behavior = behavior;

    let (metrics, derived_size) =
        boss_sprite_metrics_from_registry(crate::test_boss_catalog(), boss.as_ref(), &registry)
            .expect("the gnu_ton_boss sheet has body metrics in the baked registry");
    // The head/hand hurtboxes (what damageable_volumes consumes) live
    // in the per-animation map.
    assert!(
        !metrics.animations.is_empty(),
        "the giant should carry per-animation hurtboxes"
    );
    assert!(
        metrics.animations.contains_key("rest"),
        "the giant should have a 'rest' animation hurtbox"
    );
    // No static body bbox → no offset / derived size.
    assert_eq!(
        metrics.combat_offset,
        ae::Vec2::ZERO,
        "the giant has no static body_pixel_parts, so combat_offset stays zero"
    );
    assert!(
        metrics.body_pixel_parts.is_empty() && metrics.body_pixel_bbox.is_none(),
        "the giant's body geometry is per-animation, not static parts"
    );
    assert!(
        derived_size.is_none(),
        "with no static body bbox, no combat_size is derived"
    );

    // The rider, on his own 192x192 sheet, authors one static body; drawn at
    // 0.875 world units per pixel (a 96-unit basis x 1.75 over 192 px), it IS
    // his collision.
    let mut rider = crate::BossClusterScratch::new(&sprite_content,
        crate::test_boss_catalog(),
        "boss_gnu_ton_rider",
        "GNU-ton",
        ae::Aabb::new(pos, ae::Vec2::new(27.0, 48.0)),
        ambition_entity_catalog::placements::BossBrain::Dormant,
    );
    rider.config.behavior = BossBehaviorProfile::gnu_ton_rider();
    let (rider_metrics, rider_size) =
        boss_sprite_metrics_from_registry(crate::test_boss_catalog(), rider.as_ref(), &registry)
            .expect("the scholar's sheet authors his body");
    assert!(rider_metrics.animations.is_empty());
    let body = rider_metrics.body_pixel_bbox.expect("his sheet publishes one static body");
    let size = rider_size.expect("a static body derives his combat size");
    // Against the body and the frame his published sheet states, not a copy
    // of their pixels: the art is redrawn (a 126 px body in a 192 px frame,
    // 0.875 units a pixel, before the rows of a fight, 2026-10-12).
    let per_pixel = ae::Vec2::new(
        rider_metrics.sprite_render_size.x / rider_metrics.frame_width as f32,
        rider_metrics.sprite_render_size.y / rider_metrics.frame_height as f32,
    );
    let drawn = ae::Vec2::new(body.w as f32 * per_pixel.x, body.h as f32 * per_pixel.y);
    assert!(
        (size.x - drawn.x).abs() < 1.0 && (size.y - drawn.y).abs() < 1.0,
        "his collision is his drawn body ({}x{} px at {per_pixel:?} = {drawn:?}): {size:?}",
        body.w,
        body.h,
    );
    // Not a sixth of his size: the defect this guards.
    assert!(size.y > 90.0, "the scholar stands {} units, a fraction of his drawn body", size.y);
}

#[test]
fn mockingbird_resolves_a_body_hurtbox_from_the_baked_registry() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    use crate::pattern::profile::BossBehaviorProfile;

    let registry = ambition_sprite_sheet::baked_sheet_registry(&sprite_content);
    let behavior = BossBehaviorProfile::mockingbird();
    // The behavior must map to the sheet target the RON declares (its
    // authored `sprite_target`), or the registry lookup misses.
    assert_eq!(
        sprite_target_for_boss(crate::test_boss_catalog(), &behavior),
        "mockingbird_boss_v2",
        "mockingbird behavior must map to its 'mockingbird_boss_v2' sheet target",
    );
    let combat_size = behavior.combat_size.unwrap_or(ae::Vec2::new(500.0, 185.0));
    let pos = ae::Vec2::new(500.0, 400.0);
    let mut boss = crate::BossClusterScratch::new(&sprite_content,
        crate::test_boss_catalog(),
        "boss_mockingbird",
        "Mockingbird",
        ae::Aabb::new(pos, combat_size * 0.5),
        ambition_entity_catalog::placements::BossBrain::Dormant,
    );
    boss.config.behavior = behavior;

    let (metrics, derived_size) =
        boss_sprite_metrics_from_registry(crate::test_boss_catalog(), boss.as_ref(), &registry)
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
            // Top is 2 px above the body bottom. Integration/contact tolerance
            // can create this small overlap, but it must not block horizontal
            // approach.
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

/// A boss with no left/right variant is drawn, and so shaped, toward `+x`
/// whichever way it faces: its off-centre envelope stays on the side its art is.
#[test]
fn an_unmirrored_boss_is_shaped_the_way_it_is_drawn() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let mut boss = crate::BossClusterScratch::new(&sprite_content,
        crate::test_boss_catalog(),
        "boss_backdrop",
        "Backdrop",
        ae::Aabb::new(ae::Vec2::new(500.0, 400.0), ae::Vec2::new(40.0, 40.0)),
        ambition_entity_catalog::placements::BossBrain::Dormant,
    );
    boss.status.sprite_metrics = Some(ambition_sprite_sheet::ActorSpriteMetrics {
        combat_offset: ae::Vec2::new(12.0, -3.0),
        ..Default::default()
    });
    boss.kin.facing = -1.0;
    let shaped = |unmirrored| {
        let view = crate::BossRef {
            unmirrored,
            ..boss.as_ref()
        };
        (view.drawn_side(), view.combat_offset())
    };
    assert_eq!(shaped(false), (-1.0, ae::Vec2::new(-12.0, -3.0)), "a boss that mirrors turns");
    assert_eq!(shaped(true), (1.0, ae::Vec2::new(12.0, -3.0)), "one that does not stays drawn");
}
