//! Resolve checked pack assets from the pack's selected libraries.

use std::collections::{BTreeMap, BTreeSet};
use ambition_asset_manager::content_library::{ContentLibraries, LibraryFile};
use ambition_content_pack::{AssetSource, Severity};

pub(crate) struct LibraryAssetSource {
    base: Box<dyn AssetSource>,
    files: BTreeMap<String, LibraryFile>,
    libraries: Vec<String>,
}

impl LibraryAssetSource {
    pub(crate) fn new(base: Box<dyn AssetSource>, libraries: &ContentLibraries, advisory: bool) -> Result<Self, String> {
        let mut available = ContentLibraries::default();
        for library in libraries.iter() {
            if advisory && library.publication_root().is_some_and(|root| !root.join("published.json").exists()) { continue; }
            available.insert(library.clone())?;
        }
        Ok(Self { base, files: available.files()?, libraries: libraries.iter().map(|library| library.manifest.id.clone()).collect() })
    }
}

impl AssetSource for LibraryAssetSource {
    fn label(&self) -> String { format!("{}; selected libraries: {}", self.base.label(), self.libraries.join(", ")) }
    fn contains(&self, path: &str) -> bool {
        if let Some(path) = path.strip_prefix("game://") { return self.base.contains(path); }
        if path.starts_with("content://") { return self.files.get(path).is_some_and(|file| file.native_path().is_some_and(std::path::Path::is_file)); }
        let owners: BTreeSet<_> = self.files.values().filter(|file| file.logical == path).map(|file| &file.library).collect();
        let game = self.base.contains(path);
        if !path.contains("://") && (owners.len() > 1 || (game && !owners.is_empty())) { return false; }
        self.files.get(path).is_some_and(|file| file.native_path().is_some_and(std::path::Path::is_file)) || game
    }
    fn severity(&self) -> Severity { self.base.severity() }
}
