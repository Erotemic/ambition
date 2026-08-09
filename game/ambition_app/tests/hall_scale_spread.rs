//! **How tall each hall character actually STANDS, measured rather than eyeballed.**
//!
//! Jon, in his observations: *"In the hall of characters, the humanoid characters
//! are all dramatically out of scale with each other. Alice and bob are great,
//! but characters like the vikings, or jeff hinter render as tiny little
//! characters and look out of place compared to the rest of the cast. The
//! character art needs to be rescaled (probably at the generator level, not via
//! some post-hoc fix)."*
//!
//! ⚠ **and the exception is in the same paragraph**: *"Note the player robot v3
//! is supposed to be chibi and short compared to other humanoids."* So the rule
//! is NOT "everyone is the same height", and no ratchet here can be written
//! until somebody says which characters are meant to be alike. That decision is
//! Jon's; this is the number it needs.
//!
//! ⭐ **the same shape as `enemy_quad_matches_its_box`**: ask
//! `posed_body_geometry` at `world_per_pixel = 1.0` so the answer comes back in
//! SHEET PIXELS, which is the unit the generator works in — the one Jon named as
//! where the fix belongs. A capture would measure the drawn quad instead, which
//! folds in the camera, the pose and the frame padding, and two previous
//! attempts at a sprite-size question measured badly for exactly that reason.
//!
//! Run the report:
//!
//! ```text
//! cargo test -p ambition_app --test hall_scale_spread -- --ignored --nocapture
//! ```

use ambition_app::app::{build_visible_app, VisibleRenderMode};
use ambition_platformer2d::character::CharacterCatalog;
use ambition_platformer2d::character_sprites::posed_body_geometry;
use ambition_platformer2d::sprite_sheet::character::CharacterAnim;

/// `sprites/<stem>_spritesheet.png` → `<stem>`, the target a sheet is baked under.
fn sheet_target(spritesheet: &str) -> Option<&str> {
    spritesheet
        .rsplit('/')
        .next()?
        .strip_suffix("_spritesheet.png")
}

/// Every catalog character's baked body, in sheet pixels.
fn measured_bodies() -> Vec<(String, f32, f32)> {
    let app = build_visible_app(VisibleRenderMode::NoWindow, true);
    let catalog = app
        .world()
        .get_resource::<CharacterCatalog>()
        .expect("the composed host has an assembled character catalog");
    let mut rows: Vec<(String, f32, f32)> = catalog
        .iter()
        .filter_map(|(id, entry)| {
            let target = sheet_target(&entry.spritesheet)?;
            let geometry = posed_body_geometry(target, CharacterAnim::Idle, 1.0)?;
            Some((id.clone(), geometry.collision.x, geometry.collision.y))
        })
        .collect();
    rows.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    rows
}

/// **The instrument.** Prints every character's baked body, tallest first.
#[test]
#[ignore]
fn print_how_tall_every_character_stands() {
    let rows = measured_bodies();
    println!("{:>4}  {:>9}  {}", "px_h", "px_w", "character");
    for (id, w, h) in &rows {
        println!("{h:>7.1}  {w:>7.1}  {id}");
    }
    if let (Some(tallest), Some(shortest)) = (rows.first(), rows.last()) {
        println!(
            "\n{} characters measured; tallest {} at {:.1}px, shortest {} at {:.1}px, spread {:.2}x",
            rows.len(),
            tallest.0,
            tallest.2,
            shortest.0,
            shortest.2,
            tallest.2 / shortest.2.max(1.0),
        );
    }
}

/// **The instrument cannot rot silently.**
///
/// ⛔ a measurement whose scan quietly stops finding anything reports the
/// success condition — the failure mode this repo has hit with a portrait
/// checker, a regen census and a rollback ratchet. So the population is asserted
/// even though the spread is not: if `posed_body_geometry` stops answering, or
/// the catalog stops naming sheets this way, the report above would print an
/// empty table and look fine.
///
/// ⚠ it deliberately does NOT bound the spread. Jon named an exception in the
/// same breath as the complaint — robot v3 is meant to be short — so a limit
/// here would be a taxonomy invented by a test rather than decided by him.
#[test]
fn the_hall_scale_report_is_actually_measuring_something() {
    let rows = measured_bodies();
    // ⚠ **SKIP, not fail, when the baked art is absent.** Sheets are generated
    // and gitignored, so a clean checkout has none — and a source test that goes
    // red because a working tree was not rendered is a false alarm that teaches
    // people to ignore it. (GPT 5.6, 2026-08-05, about the sibling gauntlet test
    // this lesson cost.)
    if rows.is_empty() {
        eprintln!("[skip] no baked character sheets — run ./regen_sprites.sh");
        return;
    }
    assert!(
        rows.len() >= 40,
        "only {} character bodies resolved from a tree that HAS baked art — the \
         scan is broken and the report it feeds would print a near-empty table \
         that looks like agreement",
        rows.len()
    );
    assert!(
        rows.iter().all(|(_, w, h)| *w > 0.0 && *h > 0.0),
        "a character resolved a zero-sized body, which is not a measurement"
    );
}
