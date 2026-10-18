//! The lines Ambition shows, as data in its pack (`data/ambition_text.ron`):
//! its launcher row. The host's cover is the host's (`ambition_app`'s
//! `host_text.ron`), not the game's.
//!
//! The game declares each line and the holes it fills ([`AmbitionText`]), so
//! the pack's compile refuses a line the file leaves out, a line no code
//! shows, and a hole no code fills, each named by its key.

use ambition_content_pack::{
    lowered_shown_text, shown_text_schema, PreparedContentPack, SchemaRegistration, ShownText, TextKey, TextKeys,
};

/// The authored FILE kind: the lines Ambition shows.
pub const AMBITION_TEXT_SCHEMA: &str = "ambition_text";

/// Each line Ambition shows.
pub struct AmbitionText;

impl TextKeys for AmbitionText {
    const KEYS: &'static [TextKey] = &[
        TextKey::plain("launcher.label"),
        TextKey::plain("launcher.description"),
        TextKey::plain("launcher.preparing"),
    ];
}

/// The schema, for Ambition's pack.
pub fn ambition_text_schema() -> SchemaRegistration {
    shown_text_schema::<AmbitionText>(
        AMBITION_TEXT_SCHEMA,
        super::AMBITION_CAPABILITY,
        "The lines Ambition shows: its launcher row.",
    )
}

/// The lines `pack` states, or `None` when it declares no `ambition_text`
/// source.
pub fn lowered_ambition_text(pack: &PreparedContentPack) -> Option<ShownText> {
    lowered_shown_text(pack, AMBITION_TEXT_SCHEMA).cloned()
}
