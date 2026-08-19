//! **How big an enemy IS, derived rather than guessed.**
//!
//! Jon, 2026-08-05, for the second time: *"the snake and ai slop are still way
//! too big visually. The sprite might not match the box for the snake."*
//!
//! The scale's own doc admits the first answer was not derived:
//! *"`0.35` is chosen by ARITHMETIC, honestly labelled: it is a 30% reduction on
//! the value Jon asked to shrink, and nothing more. Turning this knob again is a
//! taste call best made by whoever is looking at the running game."* Jon has now
//! looked twice. So this stops turning the knob and states the target instead.
//!
//! ⚠ **and it measures a NUMBER the sim publishes, not pixels in a capture.**
//! The two previous attempts both measured badly — a colour filter ate the green
//! warp pipes, and a snake has two body states so two captures compared
//! different animals. `posed_body_geometry` has neither failure mode.

use ambition_demo_mary_o::snake::SNAKE_SHEET_TARGET;
use ambition_platformer2d::character_sprites::posed_body_geometry;
use ambition_platformer2d::sprite_sheet::character::CharacterAnim;

/// One Mary-O tile, in world units. Her own standing height is 48 — one and a
/// half of these — which is the scale every enemy is read against.
const TILE: f32 = 32.0;

/// **How much taller than its own body a drawn enemy may be.**
///
/// ⭐ **the measurement that changed the question.** The snake's sheet publishes
/// a body of **117 x 52 px** — a long, flat animal — inside a **128 x 128**
/// frame, and `PosedBodyGeometry::render` is *"the whole sheet frame"*. So at
/// any scale the drawn quad is SQUARE while the creature is 2.25:1, and the
/// sprite stands about 2.5x taller than the box it collides with.
///
/// That is Jon's *"the sprite might not match the box for the snake"*, exactly,
/// and it is why turning the scale never fixed it: the knob scales
/// both numbers together, so the DISAGREEMENT survives every value.
///
/// ⚠ **the sizes this used to quote (41 x 18 body, 45 x 45 quad, at 0.35) are
/// GONE, and the ratio is why this test did not notice.** The scale derives from
/// Mary-O's width and she became one brick on 2026-08-18, so it is 0.182 now and
/// the body is 21.3 x 9.5 — half of what this paragraph described. The overhang
/// is 2.46x either way, because a ratio is scale-invariant. ⇒ read the numbers
/// off the run, not off this comment.
///
/// ⚠ **this asserts the RATIO, not a size.** How big a snake should be is a
/// taste call for whoever is looking at the running game; that its picture and
/// its body should describe the same animal is not.
///
/// ⚠ **a RATCHET at the measured value, not a target.** The disagreement is real
/// and unfixed — fixing it is either an art-pipeline crop or sizing the quad
/// from the body — so this pins today's number so it cannot grow, and fails when
/// it shrinks so the constant cannot go stale behind a fix.
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

/// **The AI Slop's BOX now has its sheet's shape — the half that was fixable.**
///
/// Its sibling above ratchets a disagreement it cannot fix: the snake's quad is
/// the whole 128x128 sheet FRAME, so closing that gap needs an art-pipeline crop
/// or a quad sized from the body. The slop's was different in kind — the sheet
/// published a 1.54:1 body and `AI_SLOP_HALF` splatted a 1:1 SQUARE over it, so
/// the box was 28 tall where the creature is 18, and no value of any scale knob
/// could have fixed a shape.
///
/// ⭐ **width is the anchor and the height is now a measurement.** A slop patrols
/// a corridor; how much of it it occupies is what a room is authored against, and
/// 28 is what it occupied before — so this cost no level a re-author while the
/// height stopped being a claim.
///
/// ⚠ **this asserts the SHAPE, not the size**, exactly like the snake's. How big
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

    // ⛔ **ask the SIZING FUNCTION, not the sheet.** The first draft of this
    // recomputed the height from `sheet.collision` itself and was therefore green
    // against the splat it exists to prevent — a guard passing through its own
    // arithmetic rather than through the code. Probed: stubbing
    // `ai_slop_half_size` back to the square now fails here.
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
    // ⭐ the number this replaced, stated so the change is legible: the splat put
    // 28 x 28 on a creature whose art is 28 x ~18.
    assert!(
        height < width,
        "an AI Slop is a wide low blob; its box measured {width:.1} x {height:.1}"
    );
}

/// **A LIVE slop is the size its character authors — asked of the BODY, not of
/// the arithmetic.**
///
/// ⛔⛔ **its sibling above cannot see this defect and says so in its own
/// comment.** `the_ai_slops_box_has_the_shape_its_sheet_publishes` asserts
/// against `ai_slop_half_size()` — the FUNCTION — deliberately, after a first
/// draft that recomputed from the sheet and was green against the splat it
/// existed to prevent. Asking the function proves the arithmetic. It cannot
/// prove that the arithmetic reaches an entity, and it did not: measured
/// 2026-08-18, `tag_mary_o_ai_slop` wrote `CenteredAabb.half_size`, which is a
/// DERIVED MIRROR, while `BodyKinematics.size` — the authority every later
/// re-derivation reads — kept the size the spawn gave it. The authored value
/// reached the mirror for two ticks and never reached the body.
///
/// ⇒ **the same lesson the human-grab defect taught one layer up: a test that
/// starts downstream of the wiring cannot see the wiring.** This one starts at
/// a booted app.
///
/// ⚠ **it asserts AGREEMENT, not a number.** How big a slop should be is a
/// taste call for whoever is looking at the running game (Jon ruled on
/// 2026-08-19 that the authored constant wins); that the number a character
/// authors is the number its body has is not.
#[test]
fn a_live_ai_slop_wears_the_size_its_character_authors() {
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    for _ in 0..400 {
        app.update();
    }

    let authored = ambition_demo_mary_o::ai_slop::ai_slop_half_size();
    let mut q = app.world_mut().query::<(
        &ambition_platformer2d::engine_core::BodyKinematics,
        &ambition_platformer2d::actors::features::CenteredAabb,
        &ambition_demo_mary_o::ai_slop::AiSlop,
    )>();
    let live: Vec<(bevy::prelude::Vec2, bevy::prelude::Vec2)> = q
        .iter(app.world())
        .map(|(kin, aabb, _)| (kin.size, aabb.half_size))
        .collect();

    // ⛔ **the zero floor.** A run that tagged no slop at all would otherwise
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
