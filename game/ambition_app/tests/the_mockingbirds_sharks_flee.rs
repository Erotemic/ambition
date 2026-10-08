//! The Mockingbird's sharks, as the shipped host draws them (Jon, 2026-10-08).
//!
//! The sharks drift toward the Mockingbird, and its sky runs past them faster
//! than they drift. So through the air they fly away from it and lose ground.
//! They are drawn that way: each one faces away from it, and each one swims
//! (its `idle` row, in a loop) for as long as it is a platform.
//!
//! Measured before: each shark faced the Mockingbird (a sheet faced its
//! platform's own heading), and each one played its row once and held its
//! last frame.
//!
//! The rules themselves are held in `ambition_render`
//! (`moving_platforms::tests`). This arm holds that the shipped room reaches
//! them: its sky scroll, its sharks' look and its art.

use ambition_app::app::VisibleRenderMode;
use ambition_content::bosses::mockingbird::{MOCKINGBIRD_ID, SHARK_SHEET};
use ambition_platformer2d::boss_encounter::BossConfig;
use ambition_platformer2d::game_shell::ShellCommand;
use ambition_platformer2d::render::rendering::moving_platforms::MovingPlatformVisual;
use ambition_platformer2d::sprite_sheet::character::{CharacterAnim, CharacterAnimator};
use bevy::prelude::*;

const SKY: &str = "mockingbird_sky";

/// `(index, x, flipped, frame, row)` of each platform drawn as a sheet.
fn drawn_sharks(app: &mut App) -> Vec<(usize, f32, bool, usize, Option<usize>)> {
    let world = app.world_mut();
    let mut q = world.query::<(&MovingPlatformVisual, &Transform, &Sprite, &CharacterAnimator)>();
    let mut sharks: Vec<_> = q
        .iter(world)
        .filter(|(visual, ..)| visual.sheet.as_deref() == Some(SHARK_SHEET))
        .map(|(visual, transform, sprite, animator)| {
            (visual.index, transform.translation.x, sprite.flip_x, animator.frame, animator.drawn_row())
        })
        .collect();
    sharks.sort_by_key(|shark| shark.0);
    sharks
}

#[test]
fn the_sharks_face_away_from_the_mockingbird_and_keep_swimming() {
    let mut app = ambition_app::app::build_visible_app_with(VisibleRenderMode::NoWindow, true, |app| {
        app.insert_resource(ambition_app::app::StartRoomOverride(SKY.to_string()));
        app.insert_resource(ambition_app::app::StartRoomMustResolve);
    });
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    for _ in 0..30 {
        app.update();
    }
    app.world_mut().write_message(ShellCommand::GoTo(
        ambition_content::provider::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    // The room is live and the sharks' art has loaded: they are drawn as
    // their sheet.
    let mut sharks = Vec::new();
    for _ in 0..1800 {
        app.update();
        sharks = drawn_sharks(&mut app);
        if sharks.len() >= 9 {
            break;
        }
    }
    assert!(sharks.len() >= 9, "premise: the sky draws its nine sharks as their sheet ({})", sharks.len());

    // Premises: the room is the scrolling sky, and the Mockingbird holds the
    // side the sharks drift toward.
    let (scroll, room_w) = {
        let world = app.world_mut();
        let definition = ambition_platformer2d::world::rooms::sole_live_room_definition(world).expect("a live room");
        let mut q = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let spec = q.iter(world).next().expect("rooms").spec(definition);
        (spec.metadata.visual_profile.sky_scroll_px_s, spec.world.size.x)
    };
    assert_eq!(scroll, Some(-800), "premise: the sky runs left");
    let bird_x = {
        let world = app.world_mut();
        let mut q = world.query::<(&BossConfig, &ambition_platformer2d::engine_core::BodyKinematics)>();
        q.iter(world)
            .find(|(config, _)| config.behavior.id == MOCKINGBIRD_ID)
            .map(|(_, kin)| kin.pos.x)
            .expect("premise: the Mockingbird is in its sky")
    };
    assert!(bird_x < room_w * 0.5, "premise: the Mockingbird holds the left side ({bird_x} of {room_w})");

    // The sheet is drawn facing right, so a shark that is not flipped faces
    // right: away from the Mockingbird at the left.
    for (index, _, flipped, ..) in &sharks {
        assert!(!flipped, "shark {index} faces the Mockingbird");
    }

    // Three idle cycles: each shark draws its idle row and starts it again.
    let idle = {
        let world = app.world_mut();
        let mut q = world.query::<(&MovingPlatformVisual, &CharacterAnimator)>();
        let (_, animator) = q.iter(world).next().expect("a shark");
        (animator.spec.slot_for_anim(CharacterAnim::Idle), animator.spec.clip_seconds(CharacterAnim::Idle))
    };
    let mut frames: std::collections::BTreeMap<usize, Vec<usize>> = Default::default();
    for _ in 0..(3.0 * idle.1 * 60.0) as usize {
        app.update();
        for (index, _, flipped, frame, row) in drawn_sharks(&mut app) {
            assert!(!flipped, "shark {index} turned to face the Mockingbird");
            assert_eq!(row, Some(idle.0), "shark {index} does not draw its idle row");
            frames.entry(index).or_default().push(frame);
        }
    }
    for (index, seen) in &frames {
        let restarts = seen.windows(2).filter(|pair| pair[1] < pair[0]).count();
        assert!(restarts >= 2, "shark {index} started its row again {restarts} time(s) in three cycles: {seen:?}");
    }
}
