//! App-local content sources over Bevy's native, web, and APK readers.

use std::{collections::{BTreeMap, BTreeSet}, path::{Component, Path, PathBuf}, sync::{Arc, Mutex}};
use bevy::asset::io::{AssetReader, AssetReaderError, AssetSourceBuilder, ErasedAssetReader, PathStream, Reader, VecReader};
use futures_lite::{StreamExt, io::AsyncReadExt};
use crate::content_library::{ContentLibraries, LibrarySnapshot, LibraryFile, LibraryFileData};

#[derive(Clone, Default, bevy::prelude::Resource)]
pub struct LibraryAssets(Arc<Mutex<LibraryState>>);

#[derive(Default)]
struct LibraryState {
    libraries: ContentLibraries,
    files: Arc<BTreeMap<String, LibraryFile>>,
}

impl LibraryAssets {
    pub fn replace(&self, snapshot: LibrarySnapshot) {
        *self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = LibraryState {
            libraries: snapshot.libraries, files: snapshot.files,
        };
    }

    /// The files and library definitions admitted by this App.
    pub fn snapshot(&self) -> LibrarySnapshot {
        let state = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        LibrarySnapshot { libraries: state.libraries.clone(), files: state.files.clone() }
    }

    fn files(&self) -> Arc<BTreeMap<String, LibraryFile>> {
        Arc::clone(&self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).files)
    }

    fn immutable(&self, path: &Path) -> Option<PathBuf> {
        let (library, physical) = path.to_str()?.split_once("/__art/")?;
        if !Path::new(physical).components().all(|part| matches!(part, Component::Normal(_))) { return None; }
        let state = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let library = state.libraries.get(library)?;
        if library.is_embedded() { return None; }
        Some(library.publication_root()?.join(physical))
    }
}

pub fn library_asset_source(root: &str, libraries: LibraryAssets, namespaced: bool) -> AssetSourceBuilder {
    let builder = match crate::exe_bundle::ExeBundle::of_running_exe() {
        Some(bundle) => crate::exe_bundle::exe_bundle_asset_source(bundle),
        None => AssetSourceBuilder::platform_default(root, None),
    };
    let mut reader = builder.reader;
    AssetSourceBuilder { reader: Box::new(move || Box::new(LibraryAssetReader {
        fallback: reader(), libraries: libraries.clone(), namespaced,
    })), ..builder }
}

struct LibraryAssetReader {
    fallback: Box<dyn ErasedAssetReader>,
    libraries: LibraryAssets,
    namespaced: bool,
}

impl LibraryAssetReader {
    fn key(&self, path: &Path) -> String {
        if self.namespaced { format!("content://{}", path.to_string_lossy()) }
        else { path.to_string_lossy().into_owned() }
    }

    fn lookup(&self, path: &Path) -> Result<Option<LibraryFile>, AssetReaderError> {
        let files = self.libraries.files();
        let key = self.key(path);
        if let Some(file) = files.get(&key) { return Ok(Some(file.clone())); }
        if !self.namespaced {
            let owners: BTreeSet<_> = files.values().filter(|file| file.logical == key).map(|file| file.library.as_str()).collect();
            if owners.len() > 1 {
                return Err(std::io::Error::other(format!("Ambiguous asset `{key}`; use a content:// library source: {owners:?}")).into());
            }
        }
        Ok(None)
    }

    async fn read_file<'a>(&'a self, file: LibraryFile) -> Result<Box<dyn Reader + 'a>, AssetReaderError> {
        match file.data {
            LibraryFileData::Native(physical) => opened(&physical),
            LibraryFileData::Embedded(Some(bytes)) => Ok(Box::new(VecReader::new(bytes.to_vec()))),
            LibraryFileData::Embedded(None) => {
                let path = Path::new("__libraries").join(&file.library).join(&file.logical);
                let mut reader = self.fallback.read(&path).await?;
                let mut bytes = Vec::new();
                AsyncReadExt::read_to_end(&mut reader, &mut bytes).await?;
                Ok(Box::new(VecReader::new(bytes)))
            }
        }
    }
}

impl AssetReader for LibraryAssetReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<Box<dyn Reader + 'a>, AssetReaderError> {
        if self.namespaced {
            if let Some(physical) = self.libraries.immutable(path) { return opened(&physical); }
        }
        if let Some(file) = self.lookup(path)? {
            if !self.namespaced {
                match self.fallback.read(path).await {
                    Ok(_) => return Err(std::io::Error::other(format!("Asset `{}` has both a game file and a library owner; use an explicit source", path.display())).into()),
                    Err(AssetReaderError::NotFound(_)) => {},
                    Err(error) => return Err(error),
                }
            }
            return self.read_file(file).await;
        }
        if self.namespaced { return Err(AssetReaderError::NotFound(path.to_path_buf())); }
        self.fallback.read(path).await
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<Box<dyn Reader + 'a>, AssetReaderError> {
        if self.lookup(path)?.is_some() {
            let mut name = path.as_os_str().to_owned();
            name.push(".meta");
            let meta = PathBuf::from(name);
            return match self.lookup(&meta)? {
                Some(file) => self.read_file(file).await,
                None => Err(AssetReaderError::NotFound(meta)),
            };
        }
        if self.namespaced { return Err(AssetReaderError::NotFound(path.to_path_buf())); }
        self.fallback.read_meta(path).await
    }

    async fn read_directory<'a>(&'a self, path: &'a Path) -> Result<Box<PathStream>, AssetReaderError> {
        let files = self.libraries.files();
        let mut entries = BTreeSet::new();
        for key in files.keys() {
            let logical = if self.namespaced { key.strip_prefix("content://") }
                else { (!key.contains("://")).then_some(key.as_str()) };
            if let Some(logical) = logical {
                if let Ok(under) = Path::new(logical).strip_prefix(path) {
                    if let Some(first) = under.components().next() { entries.insert(path.join(first.as_os_str())); }
                }
            }
        }
        if self.namespaced {
            if entries.is_empty() { return Err(AssetReaderError::NotFound(path.to_path_buf())); }
            return Ok(Box::new(futures_lite::stream::iter(entries)));
        }
        match self.fallback.read_directory(path).await {
            Ok(mut other) => while let Some(entry) = other.next().await { entries.insert(entry); },
            Err(AssetReaderError::NotFound(_)) if !entries.is_empty() => {},
            Err(error) => return Err(error),
        }
        Ok(Box::new(futures_lite::stream::iter(entries)))
    }

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        let files = self.libraries.files();
        let key = self.key(path);
        if files.contains_key(&key) { return Ok(false); }
        let prefix = if path.as_os_str().is_empty() { key.clone() } else { key.clone() + "/" };
        if files.keys().any(|name| name.starts_with(&prefix) && (self.namespaced || !name.contains("://"))) { return Ok(true); }
        if self.namespaced { return Ok(false); }
        self.fallback.is_directory(path).await
    }
}

fn opened(path: &Path) -> Result<Box<dyn Reader>, AssetReaderError> {
    std::fs::File::open(path).map(|file| Box::new(OpenedFile(file)) as Box<dyn Reader>)
        .map_err(|error| if error.kind() == std::io::ErrorKind::NotFound { AssetReaderError::NotFound(path.to_path_buf()) } else { error.into() })
}
// An owned file handle keeps an in-flight read independent of later selections.
pub(crate) struct OpenedFile(pub(crate) std::fs::File);

impl futures_lite::io::AsyncRead for OpenedFile {
    fn poll_read(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>, buffer: &mut [u8])
        -> std::task::Poll<std::io::Result<usize>> {
        let _ = cx;
        std::task::Poll::Ready(std::io::Read::read(&mut self.0, buffer))
    }
}
impl futures_lite::io::AsyncSeek for OpenedFile {
    fn poll_seek(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>, position: std::io::SeekFrom)
        -> std::task::Poll<std::io::Result<u64>> {
        let _ = cx;
        std::task::Poll::Ready(std::io::Seek::seek(&mut self.0, position))
    }
}
impl Reader for OpenedFile {
    fn seekable(&mut self) -> Result<&mut dyn bevy::asset::io::SeekableReader, bevy::asset::io::ReaderNotSeekableError> {
        Ok(self)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::content_library::{ContentLibrary, SCHEMA};

    fn library(root: &Path, id: &str, bytes: &[u8]) -> ContentLibrary {
        let root = root.join(id);
        std::fs::create_dir_all(root.join("runtime")).unwrap();
        std::fs::write(root.join("runtime/body.bin"), bytes).unwrap();
        std::fs::write(root.join("library.json"), serde_json::json!({
            "schema": SCHEMA, "id": id, "version": "1", "asset_roots":{"generated":"runtime"}
        }).to_string()).unwrap();
        ContentLibrary::read(root.join("library.json")).unwrap()
    }

    async fn bytes(reader: &dyn ErasedAssetReader, path: &str) -> Result<Vec<u8>, AssetReaderError> {
        let mut input = reader.read(Path::new(path)).await?;
        let mut bytes = Vec::new();
        AsyncReadExt::read_to_end(&mut input, &mut bytes).await?;
        Ok(bytes)
    }

    #[test]
    fn readers_keep_app_scope_and_refuse_implicit_overrides() {
        futures_lite::future::block_on(async {
            let root = tempfile::tempdir().unwrap();
            let game = root.path().join("game");
            std::fs::create_dir_all(&game).unwrap();
            std::fs::write(game.join("own.bin"), b"game").unwrap();
            let state = LibraryAssets::default();
            let mut default = library_asset_source(game.to_str().unwrap(), state.clone(), false);
            let default = (default.reader)();
            let mut qualified = library_asset_source(game.to_str().unwrap(), state.clone(), true);
            let qualified = (qualified.reader)();
            assert_eq!(bytes(default.as_ref(), "own.bin").await.unwrap(), b"game");
            assert!(bytes(default.as_ref(), "body.bin").await.is_err());
            let mut selected = ContentLibraries::default();
            selected.insert(library(root.path(), "first", b"first")).unwrap();
            state.replace(selected.snapshot().unwrap());
            assert_eq!(bytes(default.as_ref(), "body.bin").await.unwrap(), b"first");
            let mut retained = qualified.read(Path::new("first/body.bin")).await.unwrap();
            selected.insert(library(root.path(), "second", b"second")).unwrap();
            state.replace(selected.snapshot().unwrap());
            assert!(bytes(default.as_ref(), "body.bin").await.is_err());
            assert_eq!(bytes(qualified.as_ref(), "second/body.bin").await.unwrap(), b"second");
            let mut first = Vec::new();
            AsyncReadExt::read_to_end(&mut retained, &mut first).await.unwrap();
            assert_eq!(first, b"first");
            let other = LibraryAssets::default();
            let mut reader = library_asset_source(game.to_str().unwrap(), other, false);
            assert!(bytes((reader.reader)().as_ref(), "body.bin").await.is_err());
            let mut selected = ContentLibraries::default();
            selected.insert(library(root.path(), "first", b"first")).unwrap();
            state.replace(selected.snapshot().unwrap());
            std::fs::write(game.join("body.bin"), b"own").unwrap();
            assert!(bytes(default.as_ref(), "body.bin").await.is_err());
            assert_eq!(bytes(qualified.as_ref(), "first/body.bin").await.unwrap(), b"first");
        });
    }
}
