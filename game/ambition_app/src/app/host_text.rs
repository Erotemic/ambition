//! The words of the host's own surfaces (`assets/data/host_text.ron`): the
//! cover while the first frame is prepared.
//!
//! These are the host's words, not a game's, so they are a file of the host
//! and no content crate is read for them. The file is read once, at the
//! first use: an edit is shown at the next start, with no Rust build. The
//! host declares each line and the holes it fills ([`HostText`]); a file
//! that leaves out a line, states a line the host never shows, or names a
//! hole the host does not fill is refused with the file and the key named.

use std::sync::OnceLock;

use ambition_platformer2d::content::{PackText, ShownText, TextKey, TextKeys};

/// The file, read off disk in a development build and embedded under the
/// app's `static_content` feature.
pub static HOST_TEXT_FILE: PackText = ambition_platformer2d::content_text!("assets/data/host_text.ron");

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
    ];
}

/// The host's lines. A file that is refused stops the host with each
/// problem named: the host has no words of its own to fall back on.
pub fn host_text() -> &'static ShownText {
    static TEXT: OnceLock<ShownText> = OnceLock::new();
    TEXT.get_or_init(|| {
        let file = HOST_TEXT_FILE.file;
        let text = HOST_TEXT_FILE.text().unwrap_or_else(|error| panic!("{file}: {error}"));
        ShownText::checked(&text, HostText::KEYS)
            .unwrap_or_else(|problems| panic!("{file} is refused:\n  {}", problems.join("\n  ")))
    })
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
