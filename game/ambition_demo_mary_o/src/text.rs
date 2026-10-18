//! The lines Mary-O shows, as data in her pack (`assets/data/mary_o_text.ron`):
//! the HUD labels, the level cards and the launcher row.
//!
//! The demo declares each line and the holes it fills ([`MaryOText`]), so the
//! pack's compile refuses a line the file leaves out, a line no code shows,
//! and a hole no code fills, each named by its key.

use ambition_platformer2d::content::{lowered_shown_text, shown_text_schema, SchemaRegistration, ShownText, TextKey, TextKeys};

/// The authored FILE kind: the lines the demo shows.
pub const MARY_O_TEXT_SCHEMA: &str = "mary_o_text";

/// Each line Mary-O shows.
pub struct MaryOText;

impl TextKeys for MaryOText {
    const KEYS: &'static [TextKey] = &[
        TextKey::plain("hud.score"),
        TextKey::plain("hud.coins"),
        TextKey::plain("hud.time"),
        TextKey::plain("hud.lives"),
        TextKey::with("card.course_clear", &["score"]),
        TextKey::with("card.intro", &["title", "lives"]),
        TextKey::with("card.intro_untitled", &["lives"]),
        TextKey::plain("launcher.label"),
        TextKey::plain("launcher.description"),
        TextKey::plain("launcher.preparing"),
    ];
}

/// The schema, for the pack's `content_pack! { schemas: [..] }`.
pub fn mary_o_text_schema() -> SchemaRegistration {
    shown_text_schema::<MaryOText>(
        MARY_O_TEXT_SCHEMA,
        "mary_o_demo",
        "The lines the Mary-O demo shows: the HUD labels, the level cards and the launcher row.",
    )
}

/// The lines, as the demo's pack states them.
pub fn mary_o_text() -> &'static ShownText {
    lowered_shown_text(crate::pack::PACK.prepared(), MARY_O_TEXT_SCHEMA)
        .expect("the demo's pack states its lines (`data/mary_o_text.ron`)")
}

/// The line `key`.
pub fn line(key: &str) -> String {
    mary_o_text().line(key).to_string()
}
