//! The lines Ambition shows, as data in its pack (`data/ambition_text.ron`):
//! its launcher row and the cover while its first frame is prepared.
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
        TextKey::plain("loading.title"),
        TextKey::plain("loading.subtitle"),
        TextKey::plain("loading.preparing"),
        TextKey::plain("loading.building"),
        TextKey::plain("loading.loading"),
        TextKey::with("loading.progress", &["headline", "settled", "total"]),
        TextKey::with("loading.item", &["label"]),
        TextKey::plain("loading.presentation"),
        TextKey::plain("loading.ready"),
        TextKey::plain("loading.finalizing"),
        TextKey::plain("loading.asset_failed"),
        TextKey::with("loading.failed_list", &["assets"]),
        TextKey::plain("loading.preparation_failed"),
    ];
}

/// The schema, for Ambition's pack.
pub fn ambition_text_schema() -> SchemaRegistration {
    shown_text_schema::<AmbitionText>(
        AMBITION_TEXT_SCHEMA,
        super::AMBITION_CAPABILITY,
        "The lines Ambition shows: its launcher row and the cover while its first frame is prepared.",
    )
}

/// The lines `pack` states, or `None` when it declares no `ambition_text`
/// source.
pub fn lowered_ambition_text(pack: &PreparedContentPack) -> Option<ShownText> {
    lowered_shown_text(pack, AMBITION_TEXT_SCHEMA).cloned()
}
