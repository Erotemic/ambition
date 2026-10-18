//! The host's own pack (`assets/host/pack.ron`): the words of the host's own
//! surfaces (`data/host_text.ron`): the cover while the first frame is
//! prepared, and the pause menu's own words (each setting's label and
//! description are the settings' own).
//! And the title screen's look (`data/title_look.ron`, `title_screen::TitleLook`)
//! and the start cover's colours (`data/cover_look.ron`, `startup_loading::CoverLook`).
//!
//! These are the host's facts, not a game's, so they are a pack of the host
//! and no content crate is read for them. The pack is compiled at its first
//! use, through the same admission as every game's pack: an edit is shown at
//! the next start, with no Rust build, and a file that leaves out a line,
//! states a line the host never shows, or names a hole the host does not
//! fill is refused with the file and the key named.

use ambition_platformer2d::content::{
    lowered_shown_text, shown_text_schema, EmbeddedPack, SchemaRegistration, ShownText, TextKey, TextKeys,
};

/// `assets/host/pack.ron` and every source it declares, read off disk in a
/// development build and embedded under the app's `static_content` feature
/// (`build.rs`).
pub static HOST_PACK: EmbeddedPack = ambition_platformer2d::content_pack! {
    root: "assets/host",
    schemas: [host_text_schema, super::title_screen::title_look_schema, super::startup_loading::cover_look_schema],
};

/// The authored FILE kind: the words of the host's surfaces.
pub const HOST_TEXT_SCHEMA: &str = "host_text";

/// Each line the host shows.
pub struct HostText;

impl TextKeys for HostText {
    const KEYS: &'static [TextKey] = &[
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
        TextKey::plain("menu.page.items"),
        TextKey::plain("menu.page.map"),
        TextKey::plain("menu.page.quest"),
        TextKey::plain("menu.page.system"),
        TextKey::plain("menu.items.title"),
        TextKey::plain("menu.map.title"),
        TextKey::plain("menu.quest.title"),
        TextKey::plain("menu.system.title"),
        TextKey::with("menu.system.screen_title", &["screen"]),
        TextKey::plain("menu.map.placeholder"),
        TextKey::plain("menu.quest.placeholder"),
        TextKey::plain("menu.items.equip"),
        TextKey::plain("menu.items.use"),
        TextKey::plain("menu.items.selected"),
        TextKey::plain("menu.items.not_owned"),
        TextKey::plain("menu.items.equipped"),
        TextKey::plain("menu.items.activate_to_equip"),
        TextKey::plain("menu.items.activate_to_use"),
        TextKey::plain("menu.system.back"),
        TextKey::plain("menu.system.back_detail"),
        TextKey::with("menu.system.quality_apply", &["profile"]),
        TextKey::with("menu.system.quality_apply_detail", &["profile"]),
        TextKey::plain("menu.system.quality_cancel"),
        TextKey::plain("menu.system.quality_cancel_detail"),
        TextKey::plain("menu.system.radio_detail"),
        TextKey::plain("menu.system.locale_detail"),
        TextKey::plain("menu.system.locale_unavailable_detail"),
        TextKey::plain("menu.system.rebind_detail"),
        TextKey::with("menu.system.now_playing", &["name"]),
        TextKey::plain("menu.system.now_playing_nothing"),
        TextKey::plain("menu.system.missing_station"),
        TextKey::plain("menu.system.missing_locale"),
        TextKey::plain("menu.system.missing_rebind"),
        TextKey::plain("menu.system.missing_label"),
        TextKey::plain("menu.system.missing_detail"),
    ];
}

/// The schema, for the host pack's `content_pack! { schemas: [..] }`.
pub fn host_text_schema() -> SchemaRegistration {
    shown_text_schema::<HostText>(
        HOST_TEXT_SCHEMA,
        "ambition_host",
        "The words of the host's own surfaces: the start cover and the pause menu's own rows.",
    )
}

/// The host's lines, as its pack states them.
pub fn host_text() -> &'static ShownText {
    lowered_shown_text(HOST_PACK.prepared(), HOST_TEXT_SCHEMA)
        .expect("the host's pack states its lines (`data/host_text.ron`)")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shipped file states each line the host shows, and nothing else.
    #[test]
    fn the_host_file_states_each_line_the_host_shows() {
        let text = host_text();
        for key in HostText::KEYS {
            assert!(!text.line(key.key).is_empty(), "`{}` is empty", key.key);
        }
    }
}
