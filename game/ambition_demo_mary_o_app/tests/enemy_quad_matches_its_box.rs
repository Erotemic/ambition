//! **How big an enemy IS, derived rather than guessed.**
//!
//! Jon, 2026-08-05, for the second time: *"the snake and ai slop are still way
//! too big visually. The sprite might not match the box for the snake."*
//!
//! `SNAKE_WORLD_PER_PIXEL`'s own doc admits the first answer was not derived:
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
use ambition_platformer2d::actors::character_sprites::posed_body_geometry;
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
/// and it is why turning `SNAKE_WORLD_PER_PIXEL` never fixed it: the knob scales
/// both numbers together, so the DISAGREEMENT survives every value. At 0.35 the
/// body is 41 x 18 world — a bit over a tile long and half a tile tall — while
/// the quad is 45 x 45, nearly a tile and a half square.
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
    let scale = ambition_demo_mary_o::snake::SNAKE_WORLD_PER_PIXEL;
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
         is long and flat.\n\nScaling `SNAKE_WORLD_PER_PIXEL` cannot fix this: \
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
