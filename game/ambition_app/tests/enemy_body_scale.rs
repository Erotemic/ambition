//! **How big each enemy's BODY is against the player's, measured rather than eyeballed.**
//!
//! Jon, twice: *"The snake and AI slop are still way too big visually, and the
//! sprite might not match the box for the snake."* (2026-08-05, second report.)
//!
//! ⛔ **the earlier attempts at this measured badly TWICE**, and both failures
//! were the same shape — a capture measures the DRAWN QUAD, which folds in the
//! camera, the pose, the frame padding and the animation state. A colour filter
//! ate the green warp pipes; the snake has two body states, so two captures
//! compared different animals. The 0.35 scale that landed was arithmetic, not a
//! derivation.
//!
//! ⭐ **so this asks the SHEET, at `world_per_pixel = 1.0`**, which answers in
//! sheet pixels — the unit the generator works in, and the same choice
//! `hall_scale_spread` makes for the same reason.
//!
//! ⭐ **and it reports BOTH numbers, because Jon's second clause is the lead.**
//! `PosedBodyGeometry` carries `collision` (the authored body box) and `render`
//! (the drawn quad). If those disagree, scaling one does not fix the other and
//! the visual size keeps looking wrong however many times it is nudged.
//!
//! Run the report:
//!
//! ```text
//! cargo test -p ambition_app --test app_it enemy_body_scale -- --ignored --nocapture
//! ```

use ambition_platformer2d::actors::character_sprites::posed_body_geometry;
use ambition_platformer2d::sprite_sheet::character::CharacterAnim;

/// The player is the ruler: "too big" is a statement about a RATIO, and this is
/// its denominator.
const REFERENCE: &str = "player_robot_v3";

/// The sheet targets Jon named, plus the reference.
const SUBJECTS: &[&str] = &["player_robot_v3", "solid_snake", "ai_slop", "mary_o_v2"];

fn measured(target: &str) -> Option<(f32, f32, f32, f32)> {
    let g = posed_body_geometry(target, CharacterAnim::Idle, 1.0)?;
    Some((g.collision.x, g.collision.y, g.render.x, g.render.y))
}

/// **The instrument.** Prints each subject's body box and drawn quad in sheet
/// pixels, and both as a multiple of the player's.
#[test]
#[ignore]
fn print_enemy_bodies_against_the_player() {
    let Some((ref_w, ref_h, _, _)) = measured(REFERENCE) else {
        println!("[skip] no baked sheet for {REFERENCE} — run ./regen_sprites.sh");
        return;
    };
    println!(
        "{:>18} {:>13} {:>13} {:>8} {:>8}",
        "target", "collision", "render", "x_vs_p", "y_vs_p"
    );
    for target in SUBJECTS {
        match measured(target) {
            Some((cw, ch, rw, rh)) => println!(
                "{target:>18} {:>6.0}x{:<6.0} {:>6.0}x{:<6.0} {:>7.2}x {:>7.2}x",
                cw,
                ch,
                rw,
                rh,
                cw / ref_w,
                ch / ref_h
            ),
            None => println!("{target:>18} (no baked sheet)"),
        }
    }
}

/// **The instrument cannot rot silently.**
///
/// ⛔ a measurement whose scan quietly stops finding anything reports the
/// success condition — the failure this repo has hit with a portrait checker, a
/// regen census and a rollback ratchet. So the population is asserted even
/// though no ratio is: what counts as "too big" is Jon's call, and a limit here
/// would be a taxonomy invented by a test.
#[test]
fn the_enemy_body_report_is_actually_measuring_something() {
    // ⚠ SKIP, not fail, when the baked art is absent: sheets are generated and
    // gitignored, so a clean checkout has none, and a source test that goes red
    // because a working tree was not rendered teaches people to ignore it.
    let Some((ref_w, ref_h, _, _)) = measured(REFERENCE) else {
        eprintln!("[skip] no baked character sheets — run ./regen_sprites.sh");
        return;
    };
    assert!(
        ref_w > 0.0 && ref_h > 0.0,
        "the reference body measured {ref_w}x{ref_h}, which is not a measurement"
    );
    for target in SUBJECTS {
        let Some((cw, ch, rw, rh)) = measured(target) else {
            panic!(
                "`{target}` has no baked sheet while `{REFERENCE}` does, so the \
                 report above would print one row and look complete"
            );
        };
        assert!(
            cw > 0.0 && ch > 0.0 && rw > 0.0 && rh > 0.0,
            "`{target}` measured a zero-sized body or quad"
        );
    }
}
