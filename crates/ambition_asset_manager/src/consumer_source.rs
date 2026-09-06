//! Layered asset source for consumer-owned game art with shared engine fallback.
//!
//! Register [`layered_asset_source`] before Bevy's `AssetPlugin`: reads first resolve
//! against the game's authored root, then the shared engine root. Metadata follows the
//! same layer that supplied the asset bytes. This lets external games own their asset tree
//! without copying generated engine assets into it.

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

    /// Metadata is read from the same layer that supplied the asset.
    /// An authored override without metadata does not inherit metadata for the
    /// shadowed shared asset.
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

    /// Merge both layers when listing a directory.
    /// Results are sorted and deduplicated so enumeration is deterministic and
    /// consistent with per-path fallback reads.
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

    /// Directory status comes from the layer that owns the path, matching
    /// [`Self::read`]. An authored file therefore shadows a shared directory of
    /// the same name.
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

    /// Metadata must come from the same layer that supplied the asset bytes.
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

    /// All operations resolve a path through the same owning layer.
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
