use bevy::prelude::Vec2;

use super::anim::CharacterAnim;
use super::sheets::{sprite_render_size, ROBOT_SHEET};

#[test]
fn sprite_render_size_uses_max_collision_axis() {
    // Tall narrow body: render height tracks collision.y (the
    // larger axis), scaled by collision_scale.
    let collision = Vec2::new(28.0, 46.0);
    let size = sprite_render_size(ROBOT_SHEET, collision);
    let expected_height = 46.0 * ROBOT_SHEET.collision_scale;
    assert!((size.y - expected_height).abs() < 1e-3);
}

#[test]
fn sprite_render_size_clamps_at_minimum_eight() {
    // Tiny collision boxes hit the 8.0 floor so micro-entities
    // (debris-sized actors) still render visibly.
    let collision = Vec2::new(2.0, 1.0);
    let size = sprite_render_size(ROBOT_SHEET, collision);
    let expected_height = 8.0 * ROBOT_SHEET.collision_scale;
    assert!((size.y - expected_height).abs() < 1e-3);
}

#[test]
fn sprite_render_size_preserves_frame_aspect() {
    // Width tracks the frame's source aspect, not the collision
    // box, so cropped non-square frames don't get distorted.
    let collision = Vec2::new(28.0, 46.0);
    let size = sprite_render_size(ROBOT_SHEET, collision);
    let expected_aspect = ROBOT_SHEET.frame_width as f32 / ROBOT_SHEET.frame_height as f32;
    let actual_aspect = size.x / size.y;
    assert!(
        (actual_aspect - expected_aspect).abs() < 1e-3,
        "expected aspect {expected_aspect}, got {actual_aspect}"
    );
}

#[test]
fn flat_index_zero_for_first_frame_of_first_row() {
    let idx = ROBOT_SHEET.flat_index(CharacterAnim::Idle, 0);
    assert_eq!(idx, 0);
}

#[test]
fn frame_count_positive_for_every_row() {
    for (anim, _) in ROBOT_SHEET.rows {
        assert!(
            ROBOT_SHEET.frame_count(*anim) > 0,
            "anim {:?} has zero frames",
            anim
        );
    }
}

#[test]
fn flat_index_clamps_to_last_frame_of_row() {
    // Asking for frame past the end of a row clamps to the last
    // valid frame; this avoids out-of-bounds atlas reads when the
    // animation cursor overshoots due to a long delta-t.
    let last = ROBOT_SHEET.flat_index(CharacterAnim::Idle, 9_999);
    let expected = ROBOT_SHEET.frame_count(CharacterAnim::Idle) - 1;
    assert_eq!(last, expected);
}

#[test]
fn robot_sheet_has_fly_row() {
    // The generator's `hover` row is the source of the Fly visual.
    // If a future sheet regen drops or reorders hover, this test
    // catches it before runtime indexes a non-existent row.
    assert_eq!(ROBOT_SHEET.frame_count(CharacterAnim::Fly), 8);
    assert!((ROBOT_SHEET.frame_duration(CharacterAnim::Fly) - 0.078).abs() < 1e-4);
    // Hover is the LAST row in the regenerated sheet, so its frames
    // sit after every other row in atlas-flat-index space.
    let fly_first = ROBOT_SHEET.flat_index(CharacterAnim::Fly, 0);
    let dash_last = ROBOT_SHEET.flat_index(
        CharacterAnim::Dash,
        ROBOT_SHEET.frame_count(CharacterAnim::Dash),
    );
    assert!(
        fly_first > dash_last,
        "Fly row must follow Dash; fly_first={fly_first} dash_last={dash_last}"
    );
}

#[test]
fn frame_duration_positive_for_every_row() {
    // Zero or negative duration would wedge the animation cursor
    // (advance_anim divides by it). Pin the contract.
    for (anim, _) in ROBOT_SHEET.rows {
        assert!(
            ROBOT_SHEET.frame_duration(*anim) > 0.0,
            "anim {:?} has non-positive duration",
            anim
        );
    }
}
