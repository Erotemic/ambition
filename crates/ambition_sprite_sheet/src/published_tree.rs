//! A development build's read of the published sprite tree.
//!
//! A build without `cfg(embedded_sprites)` embeds no sprite file (`build.rs`),
//! so each `baked_*` table is read from the tree in the source tree the first
//! time it is asked for. An edit to the tree is seen after a restart, with no
//! crate compiled; the sheet manifests can also be published again while the
//! game runs (`baked_sheet_rons::publish_sheet_generation`).

use std::path::{Path, PathBuf};

/// The folder that holds the published tree (`assets/sprites*`,
/// `assets/sprite_packs`).
pub(crate) fn owner() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../ambition_platformer2d_actor_monolith"))
}

/// Text that lives as long as the process, as an embedded table's text does.
pub(crate) fn kept(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

/// Each `(key, path)` as `(key, text)`. A file that cannot be read is left
/// out with a warning: the publisher wrote the list, so it is a file removed
/// while the game started.
pub(crate) fn texts(entries: Vec<(String, PathBuf)>) -> Vec<(&'static str, &'static str)> {
    entries
        .into_iter()
        .filter_map(|(key, path)| match std::fs::read_to_string(&path) {
            Ok(text) => Some((kept(key), kept(text))),
            Err(error) => {
                tracing::warn!(target: "ambition_platformer2d::sprites", "{}: {error}", path.display());
                None
            }
        })
        .collect()
}
