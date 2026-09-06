//! are all dramatically out of scale with each other. Alice and bob are great,
//! but characters like the vikings, or jeff hinter render as tiny little
//! characters and look out of place compared to the rest of the cast. The
//! character art needs to be rescaled (probably at the generator level, not via
//! some post-hoc fix)."*
//!
//! and the exception is in the same paragraph: *"Note the player robot v3
//! is supposed to be chibi and short compared to other humanoids."* So the rule
//! is NOT "everyone is the same height", and no ratchet here can be written
//! until somebody says which characters are meant to be alike. That decision is
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

/// The instrument. Prints every character's baked body, tallest first.
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

/// The instrument cannot rot silently.
///
/// it deliberately does NOT bound the spread.
#[test]
fn the_hall_scale_report_is_actually_measuring_something() {
    let rows = measured_bodies();
    if rows.is_empty() {
        eprintln!("[skip] no baked character sheets — run ./scripts/regen/sprites.sh");
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
