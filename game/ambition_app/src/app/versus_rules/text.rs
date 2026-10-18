//! The lines the versus stage shows, as data in its pack
//! (`assets/data/versus_text.ron`): the score line, the round calls and the
//! launcher row.
//!
//! The stage declares each line and the holes it fills ([`VersusText`]), so
//! the pack's compile refuses a line the file leaves out, a line no code
//! shows, and a hole no code fills, each named by its key.

use ambition_platformer2d::content::{lowered_shown_text, shown_text_schema, SchemaRegistration, ShownText, TextKey, TextKeys};

/// The authored FILE kind: the lines the stage shows.
pub const VERSUS_TEXT_SCHEMA: &str = "versus_text";

/// Each line the versus stage shows.
pub struct VersusText;

impl TextKeys for VersusText {
    const KEYS: &'static [TextKey] = &[
        TextKey::with("hud.rounds", &["score", "to_win"]),
        TextKey::with("hud.team_score", &["team", "wins"]),
        TextKey::plain("hud.score_separator"),
        TextKey::with("announce.round", &["number"]),
        TextKey::plain("announce.fight"),
        TextKey::with("announce.ko", &["team"]),
        TextKey::plain("announce.double_ko"),
        TextKey::with("announce.match_won", &["winner"]),
        TextKey::plain("launcher.label"),
        TextKey::plain("launcher.description"),
        TextKey::plain("launcher.preparing"),
    ];
}

/// The schema, for the versus pack's `content_pack! { schemas: [..] }`.
pub fn versus_text_schema() -> SchemaRegistration {
    shown_text_schema::<VersusText>(
        VERSUS_TEXT_SCHEMA,
        super::super::versus::VERSUS_EXPERIENCE,
        "The lines the versus stage shows: the score line, the round calls and the launcher row.",
    )
}

/// The lines, as the versus pack states them.
pub fn versus_text() -> &'static ShownText {
    lowered_shown_text(super::super::versus_fighters::PACK.prepared(), VERSUS_TEXT_SCHEMA)
        .expect("the versus pack states its lines (`data/versus_text.ron`)")
}

/// The line `key`.
pub fn line(key: &str) -> String {
    versus_text().line(key).to_string()
}
