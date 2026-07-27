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

    async fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<dyn Reader + 'a>, AssetReaderError> {
        match self.authored.read_meta(path).await {
            Ok(reader) => Ok(Box::new(reader)),
            Err(AssetReaderError::NotFound(_)) => match self.shared.read_meta(path).await {
                Ok(reader) => Ok(Box::new(reader)),
                Err(error) => Err(error),
            },
            Err(error) => Err(error),
        }
    }

    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        match self.authored.read_directory(path).await {
            Ok(entries) => Ok(entries),
            Err(AssetReaderError::NotFound(_)) => self.shared.read_directory(path).await,
            Err(error) => Err(error),
        }
    }

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        match self.authored.is_directory(path).await {
            Ok(true) => Ok(true),
            Ok(false) | Err(AssetReaderError::NotFound(_)) => self.shared.is_directory(path).await,
            Err(error) => Err(error),
        }
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

