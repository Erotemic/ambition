//! The lines Smash shows, as data in its pack (`assets/data/smash_text.ron`):
//! the match's cards, the select screen, the pause offer and the launcher row.
//!
//! The demo declares each line and the holes it fills ([`SmashText`]), so the
//! pack's compile refuses a line the file leaves out, a line no code shows,
//! and a hole no code fills, each named by its key.

use ambition_platformer2d::content::{lowered_shown_text, shown_text_schema, SchemaRegistration, ShownText, TextKey, TextKeys};

/// The authored FILE kind: the lines the demo shows.
pub const SMASH_TEXT_SCHEMA: &str = "smash_text";

/// Each line Smash shows.
pub struct SmashText;

impl TextKeys for SmashText {
    const KEYS: &'static [TextKey] = &[
        TextKey::with("banner.winner", &["name"]),
        TextKey::plain("banner.draw"),
        TextKey::plain("banner.no_contest"),
        TextKey::plain("announce.sudden_death"),
        TextKey::plain("announce.go"),
        TextKey::plain("pause.exit_label"),
        TextKey::plain("pause.exit_detail"),
        TextKey::plain("prompt.choose"),
        TextKey::plain("launcher.label"),
        TextKey::plain("launcher.description"),
        TextKey::plain("launcher.preparing"),
        TextKey::plain("select.title"),
        TextKey::plain("select.back"),
        TextKey::plain("select.start"),
        TextKey::plain("select.random"),
        TextKey::with("select.slot", &["number"]),
        TextKey::with("select.stage", &["stage"]),
        TextKey::with("select.stocks", &["stocks"]),
        TextKey::plain("select.not_playing"),
        TextKey::with("select.controller", &["number"]),
        TextKey::plain("select.cpu"),
        TextKey::plain("select.keyboard"),
        TextKey::with("select.pad", &["number"]),
        TextKey::with("select.pad_missing", &["number"]),
        TextKey::plain("select.needs_two"),
        TextKey::plain("select.drag_tokens"),
        TextKey::with("select.refused", &["refusal"]),
        TextKey::plain("select.ready"),
    ];
}

/// The schema, for the pack's `content_pack! { schemas: [..] }`.
pub fn smash_text_schema() -> SchemaRegistration {
    shown_text_schema::<SmashText>(
        SMASH_TEXT_SCHEMA,
        "smash_demo",
        "The lines the Smash demo shows: the match's cards, the select screen, the pause offer and the launcher row.",
    )
}

/// The lines, as the demo's pack states them.
pub fn smash_text() -> &'static ShownText {
    lowered_shown_text(crate::smash_pack::PACK.prepared(), SMASH_TEXT_SCHEMA)
        .expect("the demo's pack states its lines (`data/smash_text.ron`)")
}

/// The line `key`.
pub fn line(key: &str) -> String {
    smash_text().line(key).to_string()
}
