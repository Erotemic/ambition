//! The lines Sanic shows, as data in his pack (`assets/data/sanic_text.ron`):
//! the HUD labels, the act cards, the signs and the launcher row.
//!
//! The demo declares each line and the holes it fills ([`SanicText`]), so the
//! pack's compile refuses a line the file leaves out, a line no code shows,
//! and a hole no code fills, each named by its key.

use ambition_platformer2d::content::{lowered_shown_text, shown_text_schema, SchemaRegistration, ShownText, TextKey, TextKeys};

/// The authored FILE kind: the lines the demo shows.
pub const SANIC_TEXT_SCHEMA: &str = "sanic_text";

/// Each line Sanic shows.
pub struct SanicText;

impl TextKeys for SanicText {
    const KEYS: &'static [TextKey] = &[
        TextKey::plain("hud.rings"),
        TextKey::plain("hud.time"),
        TextKey::with("card.act_title", &["name", "number"]),
        TextKey::with("card.act_clear", &["time", "rings", "score"]),
        TextKey::plain("sign.loop"),
        TextKey::plain("sign.finish"),
        TextKey::plain("launcher.label"),
        TextKey::plain("launcher.description"),
        TextKey::plain("launcher.preparing"),
    ];
}

/// The schema, for the pack's `content_pack! { schemas: [..] }`.
pub fn sanic_text_schema() -> SchemaRegistration {
    shown_text_schema::<SanicText>(
        SANIC_TEXT_SCHEMA,
        "sanic_demo",
        "The lines the Sanic demo shows: the HUD labels, the act cards, the signs and the launcher row.",
    )
}

/// The lines, as the demo's pack states them.
pub fn sanic_text() -> &'static ShownText {
    lowered_shown_text(crate::pack::PACK.prepared(), SANIC_TEXT_SCHEMA)
        .expect("the demo's pack states its lines (`data/sanic_text.ron`)")
}

/// The line `key`.
pub fn line(key: &str) -> String {
    sanic_text().line(key).to_string()
}
