//! **A game gets to own its own art.** (Phase 6, recorded SDK leak #3)
//!
//! The external-consumer fixture records the gap in its own doc comment: "the
//! AssetServer file root must be pointed at the ENGINE's asset tree via
//! `actors_desktop_asset_root()` — consumer-owned art still has no home, and a
//! consumer that forgets this line gets bare boxes."
//!
//! A third party could therefore load the ENGINE's sprites or nothing. Its own
//! art had nowhere to live, because the mechanism that lets Ambition's content
//! crate own a world tree — a `game://` asset source whose reader falls back to
//! the shared engine tree — lived in `ambition_app`'s CLI module where nothing
//! outside the shell could reach it.
//!
//! That mechanism was never Ambition-specific. It is the answer to "my art
//! first, the engine's art if I did not author it", which is what every game
//! built on this engine wants and is exactly what the demos' generated sprite
//! trees rely on. It lives here now, where this crate's module docs already
//! promise "Bevy `AssetSource` wiring recommendations".
//!
//! ## Usage
//!
//! Registered BEFORE `AssetPlugin` is built — Bevy seals its sources when the
//! plugin builds, so this cannot be a plugin added later:
//!
//! ```ignore
//! app.register_asset_source(
//!     "game",
//!     ambition_asset_manager::consumer_source::layered_asset_source(
//!         my_game::my_asset_root(),                       // authored HERE
//!         ambition_asset_manager::actors_desktop_asset_root(), // engine's tree
//!     ),
//! );
//! ```
//!
//! Then a `game://sprites/whatever.png` path resolves out of the consumer's own
//! tree when it exists and the engine's when it does not.

use std::path::{Path, PathBuf};

use bevy::asset::io::{
    file::FileAssetReader, AssetReader, AssetReaderError, AssetSourceBuilder, PathStream, Reader,
};

/// Reads from an AUTHORED root first and a SHARED root second.
///
/// Ambition's own content crate owns `worlds/*.ldtk` while the tileset and
/// entity-sprite paths it names (`sprites/...`) are generated into the shared
/// engine tree. A source-level fallback preserves the game-owned root without
/// copying generated binaries into the content crate or emitting misleading
/// `Path not found` errors for assets that are genuinely present.
struct LayeredAssetReader {
    authored: FileAssetReader,
    shared: FileAssetReader,
}

impl LayeredAssetReader {
    fn new(authored_root: impl AsRef<Path>, shared_root: impl AsRef<Path>) -> Self {
        Self {
            authored: FileAssetReader::new(authored_root),
            shared: FileAssetReader::new(shared_root),
        }
    }
}

impl AssetReader for LayeredAssetReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<Box<dyn Reader + 'a>, AssetReaderError> {
        match self.authored.read(path).await {
            Ok(reader) => Ok(Box::new(reader)),
            Err(AssetReaderError::NotFound(_)) => match self.shared.read(path).await {
                Ok(reader) => Ok(Box::new(reader)),
                Err(error) => Err(error),
            },
            Err(error) => Err(error),
        }
    }

    /// **Metadata follows the layer that supplied the ASSET.**
    ///
    /// This used to fall back independently, so a consumer that overrode
    /// `sprites/foo.png` and authored no `.meta` for it silently received the
    /// ENGINE's metadata for the engine's different `sprites/foo.png` — sampler
    /// settings, atlas layout and loader choice describing a file it is not
    /// describing (GPT 5.6, 2026-07-28). Wrong metadata is worse than missing
    /// metadata: missing means "use the defaults", wrong means "this image is
    /// something else".
    ///
    /// So the asset decides the layer, and the meta comes from there or not at
    /// all. Routing costs one extra open of a file the loader is about to read
    /// anyway, which is the honest price of asking the same question `read`
    /// answers rather than a second question that can disagree with it.
    async fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<dyn Reader + 'a>, AssetReaderError> {
        match self.authored.read(path).await {
            Ok(_) => Ok(Box::new(self.authored.read_meta(path).await?)),
            Err(AssetReaderError::NotFound(_)) => {
                Ok(Box::new(self.shared.read_meta(path).await?))
            }
            Err(error) => Err(error),
        }
    }

    /// **Both layers, merged** — a directory that exists in the authored tree
    /// used to HIDE the shared one entirely.
    ///
    /// The single-file case falls back per path, so `game://sprites/engine.png`
    /// stays readable; but enumerating `game://sprites` returned only the
    /// consumer's own entries, so anything that discovers assets by listing a
    /// folder saw a tree with the engine's sprites missing while every one of
    /// them was still individually loadable. An overlay whose listing disagrees
    /// with its reads is not an overlay (GPT 5.6, 2026-07-28).
    ///
    /// Sorted and deduplicated: a merged listing has to have SOME order, and
    /// filesystem order differs between machines — which is the class of
    /// nondeterminism ADR 0023 exists to keep out of this codebase.
    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        use futures_lite::StreamExt as _;

        let mut merged: std::collections::BTreeSet<PathBuf> = std::collections::BTreeSet::new();
        let mut found_any = false;
        let mut first_error = None;
        for layer in [&self.authored, &self.shared] {
            match layer.read_directory(path).await {
                Ok(mut entries) => {
                    found_any = true;
                    while let Some(entry) = entries.next().await {
                        merged.insert(entry);
                    }
                }
                Err(AssetReaderError::NotFound(_)) => {}
                Err(error) => {
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
            }
        }
        if let Some(error) = first_error {
            return Err(error);
        }
        if !found_any {
            return Err(AssetReaderError::NotFound(path.to_path_buf()));
        }
        Ok(Box::new(futures_lite::stream::iter(
            merged.into_iter().collect::<Vec<_>>(),
        )))
    }

    /// **The layer that owns the path answers**, so this cannot contradict
    /// [`Self::read`].
    ///
    /// This used to ask the shared layer whenever the authored layer said
    /// `false`. An authored FILE shadowed by a shared DIRECTORY of the same name
    /// then reported "directory" while `read` happily returned the file's bytes,
    /// which is a contradiction a caller has no way to resolve (GPT 5.6,
    /// 2026-07-28).
    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        if self.authored.is_directory(path).await.unwrap_or(false) {
            return Ok(true);
        }
        // Not a directory in the authored layer. If it is a FILE there, the
        // authored layer owns this path and the answer is false — the same
        // layer `read` will use.
        match self.authored.read(path).await {
            Ok(_) => return Ok(false),
            Err(AssetReaderError::NotFound(_)) => {}
            Err(error) => return Err(error),
        }
        self.shared.is_directory(path).await
    }
}

/// An asset source that resolves `authored_root` first and `shared_root` second.
///
/// When the two roots are EQUAL this returns the platform default unchanged, and
/// that is load-bearing rather than an optimisation. Both roots collapse to the
/// same relative `"assets"` in a packaged build (an Android APK, a Steam Deck
/// install, anything under `BEVY_ASSET_ROOT`) because the packager has already
/// merged the trees. The fallback reader is built from `FileAssetReader`, so
/// installing it there would shadow the platform's own reader with a filesystem
/// reader resolving against the process CWD — which can never see inside an APK,
/// and every load through this source would fail. With one root there is nothing
/// to fall back to, so the platform default IS the correct reader.
///
/// The authored root keeps its ordinary watcher/writer behaviour; only the
/// reader is layered.
pub fn layered_asset_source(
    authored_root: impl Into<PathBuf>,
    shared_root: impl Into<PathBuf>,
) -> AssetSourceBuilder {
    let authored_root: PathBuf = authored_root.into();
    let shared_root: PathBuf = shared_root.into();
    let authored_display = authored_root.to_string_lossy().into_owned();
    let builder = AssetSourceBuilder::platform_default(&authored_display, None);
    if authored_root == shared_root {
        return builder;
    }
    builder.with_reader(move || {
        Box::new(LayeredAssetReader::new(
            authored_root.clone(),
            shared_root.clone(),
        ))
    })
}


#[cfg(test)]
mod tests {
    use super::*;
    use futures_lite::{future::block_on, StreamExt as _};

    /// Two roots under one temp dir: `authored/` and `shared/`.
    fn roots() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().expect("a temp dir");
        let authored = dir.path().join("authored");
        let shared = dir.path().join("shared");
        std::fs::create_dir_all(&authored).unwrap();
        std::fs::create_dir_all(&shared).unwrap();
        (dir, authored, shared)
    }

    fn write(root: &Path, relative: &str, bytes: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    /// **A listing is the union.** The authored tree owning a directory used to
    /// hide the shared one entirely, so `game://sprites` showed a consumer's one
    /// file and none of the engine's — every one of which was still individually
    /// readable through the same source.
    #[test]
    fn listing_a_directory_shows_both_layers() {
        let (_dir, authored, shared) = roots();
        write(&authored, "sprites/mine.png", "mine");
        write(&shared, "sprites/engine.png", "engine");
        let reader = LayeredAssetReader::new(&authored, &shared);

        let listed: Vec<String> = block_on(async {
            let mut entries = reader.read_directory(Path::new("sprites")).await.unwrap();
            let mut out = Vec::new();
            while let Some(entry) = entries.next().await {
                out.push(entry.file_name().unwrap().to_string_lossy().into_owned());
            }
            out
        });
        assert_eq!(listed, vec!["engine.png", "mine.png"], "sorted union");
    }

    /// **Metadata cannot come from a layer that did not supply the bytes.** A
    /// consumer overriding an engine sprite and authoring no `.meta` used to
    /// receive the ENGINE's meta — sampler, atlas and loader settings for a
    /// different image.
    #[test]
    fn metadata_follows_the_layer_that_supplied_the_asset() {
        let (_dir, authored, shared) = roots();
        write(&authored, "sprites/foo.png", "consumer art");
        write(&shared, "sprites/foo.png", "engine art");
        write(&shared, "sprites/foo.png.meta", "engine meta");
        let reader = LayeredAssetReader::new(&authored, &shared);

        block_on(async {
            let meta = reader.read_meta(Path::new("sprites/foo.png")).await;
            assert!(
                matches!(meta, Err(AssetReaderError::NotFound(_))),
                "the authored layer supplied the asset and authored no meta, so \
                 there is no meta — inheriting the engine's would describe a \
                 different picture"
            );
            // ...and the layer with no override still answers for its own.
            let meta = reader.read_meta(Path::new("sprites/engine_only.png")).await;
            assert!(matches!(meta, Err(AssetReaderError::NotFound(_))));
        });
        write(&shared, "sprites/engine_only.png", "engine art");
        write(&shared, "sprites/engine_only.png.meta", "engine meta");
        let reader = LayeredAssetReader::new(&authored, &shared);
        block_on(async {
            assert!(
                reader
                    .read_meta(Path::new("sprites/engine_only.png"))
                    .await
                    .is_ok(),
                "the shared layer supplied the asset, so its meta is the right meta"
            );
        });
    }

    /// **`is_directory` cannot contradict `read`.** An authored FILE shadowed by
    /// a shared DIRECTORY of the same name used to report "directory" while
    /// `read` returned the file's bytes.
    #[test]
    fn a_path_is_answered_by_the_layer_that_owns_it() {
        let (_dir, authored, shared) = roots();
        write(&authored, "thing", "i am a file");
        write(&shared, "thing/inner.png", "i am inside a directory");
        write(&shared, "engine_only/inner.png", "only the engine has this");
        let reader = LayeredAssetReader::new(&authored, &shared);

        block_on(async {
            assert!(
                reader.read(Path::new("thing")).await.is_ok(),
                "the authored file is what `read` returns"
            );
            assert!(
                !reader.is_directory(Path::new("thing")).await.unwrap(),
                "so it must not also be a directory"
            );
            // A path only the SHARED layer has is still answered from there.
            assert!(
                reader
                    .is_directory(Path::new("engine_only"))
                    .await
                    .unwrap(),
                "a directory the consumer never authored is still a directory"
            );
        });
    }
}
