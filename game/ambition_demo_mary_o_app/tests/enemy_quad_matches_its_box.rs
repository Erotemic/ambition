//! Checks that authored sprite rendering and collision describe the same body.
//!
//! The snake uses legacy whole-frame rendering while its sheet publishes a much
//! flatter body rectangle. The test ratchets that ratio so scale changes cannot
//! hide the geometry disagreement; regenerating per-frame render rectangles is
//! the intended fix.

use ambition_demo_mary_o::snake::SNAKE_SHEET_TARGET;
use ambition_platformer2d::character_sprites::posed_body_geometry;
use ambition_platformer2d::sprite_sheet::character::CharacterAnim;

/// One Mary-O tile, in world units. Her own standing height is 48 — one and a
/// half of these — which is the scale every enemy is read against.
const TILE: f32 = 32.0;

/// Current legacy whole-frame/body-height ratio for the snake. It is scale
/// invariant and should decrease when the sheet gains per-frame render geometry.
const QUAD_OVERHANG_LIMIT: f32 = 2.47;

#[test]
fn the_snakes_picture_and_its_body_describe_the_same_animal() {
    // At `world_per_pixel = 1.0` the geometry comes back in SHEET PIXELS, which
    // is what makes the scale solvable instead of guessable.
    let walking = posed_body_geometry(SNAKE_SHEET_TARGET, CharacterAnim::Idle, 1.0)
        .expect("the snake's sheet publishes body metrics");
    let scale = ambition_demo_mary_o::snake::snake_world_per_pixel();
    let world = walking.collision * scale;
    let quad = walking.render * scale;

    eprintln!(
        "[snake] sheet_px collision={:?} render={:?}",
        walking.collision, walking.render
    );
    eprintln!(
        "[snake] at world_per_pixel={scale}: collision={world:?} ({:.2} tiles tall), quad={quad:?}",
        world.y / TILE
    );
    eprintln!(
        "[snake] the picture is {:.2}x the body's height, and {:.2}x its width",
        quad.y / world.y.max(0.001),
        quad.x / world.x.max(0.001)
    );

    let overhang = quad.y / world.y.max(0.001);
    assert!(
        overhang <= QUAD_OVERHANG_LIMIT && overhang > QUAD_OVERHANG_LIMIT - 0.15,
        "the snake is DRAWN {:.1} world tall and COLLIDES {:.1} world tall — \
         {overhang:.2}x. Its sheet body is {:?} px inside a {:?} px frame, and \
         the quad is the whole frame, so the picture is square while the animal \
         is long and flat.\n\nScaling `snake_body_width` cannot fix this: \
         the knob multiplies both numbers, so the disagreement survives every \
         value — which is why two previous attempts at Jon's report did not \
         land. Either the sheet frame is cropped to the animal, or the quad is \
         sized from the BODY rather than the frame.",
        quad.y,
        world.y,
        walking.collision,
        walking.render,
    );
}

/// The AI Slop's BOX now has its sheet's shape — the half that was fixable.
///
/// Its sibling above ratchets a disagreement it cannot fix: the snake's quad is the whole
/// 128x128 sheet FRAME, so closing that gap needs an art-pipeline crop or a quad sized from the
/// body.
///
/// this asserts the SHAPE, not the size, exactly like the snake's. How big
/// a slop should be is a taste call for whoever is looking at the running game;
/// that its box and its picture describe the same animal is not.
#[test]
fn the_ai_slops_box_has_the_shape_its_sheet_publishes() {
    let sheet = posed_body_geometry(
        ambition_demo_mary_o::ai_slop::AI_SLOP_SHEET_TARGET,
        CharacterAnim::Idle,
        1.0,
    )
    .expect("the AI Slop's sheet publishes body metrics");
    let sheet_aspect = sheet.collision.x / sheet.collision.y;
    assert!(
        sheet_aspect > 1.2,
        "this test exists because the slop is WIDER than it is tall ({sheet_aspect:.2}:1); \
         if the art became square the square box was never the bug"
    );

    // Exercise the production sizing function rather than recomputing its
    // arithmetic in the test.
    let half = ambition_demo_mary_o::ai_slop::ai_slop_half_size();
    let (width, height) = (half.x * 2.0, half.y * 2.0);
    assert!(
        (width - ambition_demo_mary_o::ai_slop::AI_SLOP_BODY_WIDTH).abs() < 0.01,
        "the derived body is {width:.1} wide, not the {:.1} it authors — width is \
         the anchor and every level is placed against it",
        ambition_demo_mary_o::ai_slop::AI_SLOP_BODY_WIDTH
    );
    assert!(
        (height - width).abs() > TILE * 0.25,
        "the derived body came out near-square ({width:.1} x {height:.1}), which \
         means the derivation is not doing anything and the splat could come back \
         unnoticed"
    );
    // the number this replaced, stated so the change is legible: the splat put
    // 28 x 28 on a creature whose art is 28 x ~18.
    assert!(
        height < width,
        "an AI Slop is a wide low blob; its box measured {width:.1} x {height:.1}"
    );
}

/// A LIVE slop is the size its character authors — asked of the BODY, not of
/// the arithmetic.
///
/// Asking the function proves the arithmetic. The authored value reached the mirror for two ticks
/// and never reached the body.
///
///  the same lesson the human-grab defect taught one layer up: a test that
/// starts downstream of the wiring cannot see the wiring. This one starts at
/// a booted app.
#[test]
fn a_live_ai_slop_wears_the_size_its_character_authors() {
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    for _ in 0..400 {
        app.update();
    }

    let authored = ambition_demo_mary_o::ai_slop::ai_slop_half_size();
    let mut q = app.world_mut().query::<(
        &ambition_platformer2d::engine_core::BodyKinematics,
        &ambition_platformer2d::combat::components::CenteredAabb,
        &ambition_demo_mary_o::ai_slop::AiSlop,
    )>();
    let live: Vec<(bevy::prelude::Vec2, bevy::prelude::Vec2)> = q
        .iter(app.world())
        .map(|(kin, aabb, _)| (kin.size, aabb.half_size))
        .collect();

    // the zero floor. A run that tagged no slop at all would otherwise
    // agree with every assertion below by reading no subjects.
    assert!(
        !live.is_empty(),
        "no tagged AI Slop is alive after 400 ticks, so this measured nothing"
    );

    for (kin_size, half) in &live {
        assert!(
            (kin_size.x - authored.x * 2.0).abs() < 0.5
                && (kin_size.y - authored.y * 2.0).abs() < 0.5,
            "a live slop's BODY is {:.1} x {:.1} but its character authors \
             {:.1} x {:.1} — the authored size is reaching a mirror and not the \
             authority (`BodyKinematics.size`), which is what every reset and \
             re-derivation reads",
            kin_size.x,
            kin_size.y,
            authored.x * 2.0,
            authored.y * 2.0
        );
        assert!(
            (half.x - authored.x).abs() < 0.5 && (half.y - authored.y).abs() < 0.5,
            "a live slop's COLLISION BOX is {:.1} x {:.1} (half-extents) but its \
             character authors {:.1} x {:.1}",
            half.x,
            half.y,
            authored.x,
            authored.y
        );
    }
}
