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
    texts_reusing(entries, &[])
}

/// [`texts`], taking the `(key, text)` of `previous` for each
/// file whose text is the same: a text is kept for the life of the process,
/// so a read of an unchanged tree keeps nothing more.
pub(crate) fn texts_reusing(
    entries: Vec<(String, PathBuf)>,
    previous: &[(&'static str, &'static str)],
) -> Vec<(&'static str, &'static str)> {
    let previous: std::collections::HashMap<&str, (&'static str, &'static str)> =
        previous.iter().map(|row| (row.0, *row)).collect();
    entries
        .into_iter()
        .filter_map(|(key, path)| match std::fs::read_to_string(&path) {
            Ok(text) => Some(match previous.get(key.as_str()) {
                Some(row) if row.1 == text => *row,
                _ => (kept(key), kept(text)),
            }),
            Err(error) => {
                tracing::warn!(target: "ambition_platformer2d::sprites", "{}: {error}", path.display());
                None
            }
        })
        .collect()
}
