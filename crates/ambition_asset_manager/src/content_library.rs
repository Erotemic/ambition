//! Explicit content libraries and their immutable runtime file selections.

use std::{collections::BTreeMap, path::{Component, Path, PathBuf}};
use serde::{Deserialize, Serialize};

use crate::art_publication::Selection;

pub const SCHEMA: &str = "ambition.content_library.v1";

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryManifest {
    pub schema: String,
    pub id: String,
    pub version: String,
    pub asset_roots: BTreeMap<String, String>,
    #[serde(default)]
    pub provenance: Option<String>,
    #[serde(default)]
    pub catalog: Option<String>,
    #[serde(default)]
    pub embedded: Vec<String>,
    #[serde(default)]
    pub publication: Option<String>,
    #[serde(default)]
    pub producers: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug)]
pub struct ContentLibrary {
    pub manifest: LibraryManifest,
    pub root: PathBuf,
    pub catalog: crate::AssetManifest,
    embedded: Option<&'static [EmbeddedLibraryFile]>,
}

pub struct EmbeddedLibrary {
    pub manifest: &'static str,
    pub catalog: &'static str,
    pub files: &'static [EmbeddedLibraryFile],
}

#[derive(Debug, PartialEq)]
pub struct EmbeddedLibraryFile {
    pub path: &'static str,
    pub bytes: Option<&'static [u8]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LibraryFileData {
    Native(PathBuf),
    Embedded(Option<&'static [u8]>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryFile {
    pub library: String,
    pub logical: String,
    pub data: LibraryFileData,
    pub generation: Option<String>,
    address: String,
}

impl LibraryFile {
    /// The address that keeps this file's admitted bytes.
    pub fn asset_path(&self) -> &str { &self.address }

    pub fn read(&self) -> Result<Vec<u8>, String> {
        match &self.data {
            LibraryFileData::Native(path) => std::fs::read(path).map_err(|error| format!("{}: {error}", path.display())),
            LibraryFileData::Embedded(Some(bytes)) => Ok(bytes.to_vec()),
            LibraryFileData::Embedded(None) => Err(format!("Asset requires the platform reader: content://{}/{}", self.library, self.logical)),
        }
    }

    pub fn native_path(&self) -> Option<&Path> {
        match &self.data { LibraryFileData::Native(path) => Some(path), _ => None }
    }
}

/// The libraries one composition selected. An empty set is valid.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "bevy", derive(bevy::prelude::Resource))]
pub struct ContentLibraries {
    libraries: BTreeMap<String, ContentLibrary>,
}

/// The files admitted together from one set of library publications.
#[derive(Clone, Debug)]
pub struct LibrarySnapshot {
    pub libraries: ContentLibraries,
    pub files: std::sync::Arc<BTreeMap<String, LibraryFile>>,
}

fn relative(value: &str) -> bool {
    !value.is_empty() && !value.contains('\\') && Path::new(value).components()
        .all(|part| matches!(part, Component::Normal(_)))
}

fn validate(manifest: &LibraryManifest) -> Result<(), String> {
    if manifest.schema != SCHEMA || manifest.id.is_empty()
            || !manifest.id.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
            || manifest.version.is_empty()
            || manifest.asset_roots.values().any(|root| !relative(root))
            || manifest.provenance.as_ref().is_some_and(|root| !relative(root))
            || manifest.catalog.as_ref().is_some_and(|path| !relative(path))
            || manifest.embedded.iter().any(|path| !relative(path))
            || manifest.publication.as_ref().is_some_and(|root| !relative(root)) {
            return Err(format!("Invalid content library: {}", manifest.id));
        }
    Ok(())
}

impl ContentLibrary {
    pub fn read(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
        let manifest: LibraryManifest = serde_json::from_str(&text)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        validate(&manifest)?;
        let root = path.parent().unwrap_or(Path::new(".")).canonicalize()
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let catalog = match &manifest.catalog {
            Some(path) => serde_json::from_str(&std::fs::read_to_string(root.join(path))
                .map_err(|error| format!("{}: {error}", root.join(path).display()))?)
                .map_err(|error| format!("{}: {error}", root.join(path).display()))?,
            None => crate::AssetManifest::default(),
        };
        Ok(Self { manifest, root, catalog, embedded: None })
    }

    pub fn from_embedded(library: &'static EmbeddedLibrary) -> Result<Self, String> {
        let manifest: LibraryManifest = serde_json::from_str(library.manifest).map_err(|error| error.to_string())?;
        let catalog = serde_json::from_str(library.catalog).map_err(|error| error.to_string())?;
        validate(&manifest)?;
        let mut paths = std::collections::BTreeSet::new();
        for file in library.files {
            if !relative(file.path) || !paths.insert(file.path) {
                return Err(format!("Invalid or duplicate embedded library path: {}", file.path));
            }
        }
        Ok(Self { manifest, root: PathBuf::new(), catalog, embedded: Some(library.files) })
    }

    pub fn is_embedded(&self) -> bool { self.embedded.is_some() }

    pub fn publication_root(&self) -> Option<PathBuf> {
        self.manifest.publication.as_ref().map(|path| self.root.join(path))
    }

    pub fn selection(&self) -> Result<Option<Selection>, String> {
        if self.embedded.is_some() { return Ok(None); }
        self.publication_root().map(|root| Selection::read(&root)).transpose()
    }

    /// Published products own their declared roots. Loose copies have no authority.
    pub fn files(&self) -> Result<BTreeMap<String, LibraryFile>, String> {
        if let Some(files) = self.embedded {
            return Ok(files.iter().map(|file| (file.path.to_string(), LibraryFile {
                library: self.manifest.id.clone(), logical: file.path.to_string(),
                data: LibraryFileData::Embedded(file.bytes), generation: None,
                address: format!("content://{}/{}", self.manifest.id, file.path),
            })).collect());
        }
        let selected = self.selection()?;
        let mut files = BTreeMap::new();
        for (role, relative) in &self.manifest.asset_roots {
            let root = self.root.join(relative);
            if !root.is_dir() { continue; }
            for (logical, physical) in regular_files(&root, if role == "authored" { None } else { selected.as_ref() })? {
                if role != "authored" && selected.as_ref().is_some_and(|selection| selection.manages(Path::new(&logical))) {
                    continue;
                }
                let file = LibraryFile { library: self.manifest.id.clone(), logical: logical.clone(),
                    data: LibraryFileData::Native(physical), generation: None,
                    address: format!("content://{}/{logical}", self.manifest.id) };
                if let Some(previous) = files.insert(logical.clone(), file) {
                    return Err(format!("Library `{}` declares `{logical}` twice: {:?} and {:?}",
                        self.manifest.id, previous.native_path(), files[&logical].native_path()));
                }
            }
        }
        if let Some(selected) = selected {
            for (logical, physical) in selected.entries() {
                if files.contains_key(logical) { return Err(format!("Authored asset {logical:?} collides with its published product")); }
                files.insert(logical.to_owned(), LibraryFile { library: self.manifest.id.clone(),
                    logical: logical.to_owned(), data: LibraryFileData::Native(physical), generation: selected.generation.clone(),
                    address: format!("content://{}/__art/{}", self.manifest.id, selected.files[logical].path) });
            }
        }
        Ok(files)
    }
}

impl ContentLibraries {
    pub fn snapshot(&self) -> Result<LibrarySnapshot, String> {
        Ok(LibrarySnapshot { libraries: self.clone(), files: std::sync::Arc::new(self.files()?) })
    }

    pub fn insert(&mut self, library: ContentLibrary) -> Result<(), String> {
        if let Some(previous) = self.libraries.get(&library.manifest.id) {
            if previous.root == library.root && previous.manifest == library.manifest
                && previous.catalog == library.catalog && previous.embedded == library.embedded {
                return Ok(());
            }
            return Err(format!("Library `{}` has two selections: {} and {}", library.manifest.id,
                previous.root.display(), library.root.display()));
        }
        self.libraries.insert(library.manifest.id.clone(), library);
        Ok(())
    }

    pub fn iter(&self) -> impl Iterator<Item = &ContentLibrary> { self.libraries.values() }

    pub fn get(&self, id: &str) -> Option<&ContentLibrary> { self.libraries.get(id) }

    pub fn asset_manifest(&self) -> Result<crate::AssetManifest, String> {
        let mut catalog = crate::AssetManifest::default();
        let mut owners = BTreeMap::new();
        for library in self.iter() {
            for (id, entry) in library.catalog.iter() {
                if let Some(previous) = owners.insert(id.clone(), library.manifest.id.clone()) {
                    return Err(format!("Asset role `{id}` is declared by `{previous}` and `{}`", library.manifest.id));
                }
                let mut entry = entry.clone();
                if !entry.logical_path.contains("://") {
                    entry.logical_path = format!("content://{}/{}", library.manifest.id, entry.logical_path);
                }
                catalog.insert(entry);
            }
        }
        Ok(catalog)
    }

    pub fn bind_catalog(&self, catalog: &mut crate::AssetManifest) -> Result<(), String> {
        let files = self.files()?;
        let replacements = catalog.iter().map(|(_, entry)| {
            let mut entry = entry.clone();
            let file = files.get(&entry.logical_path);
            if let Some(file) = file {
                entry.logical_path = format!("content://{}/{}", file.library, file.logical);
                if matches!(file.data, LibraryFileData::Embedded(Some(_))) {
                    let path = entry.logical_path.clone();
                    entry = entry.with_location(crate::AssetSourceProfile::EmbeddedBinary,
                        crate::AssetLocation::BevyPath(path));
                }
            }
            entry
        }).collect::<Vec<_>>();
        for entry in replacements { catalog.insert(entry); }
        Ok(())
    }

    /// Resolve qualified identities. Unqualified references must have one owner.
    pub fn files(&self) -> Result<BTreeMap<String, LibraryFile>, String> {
        let mut files = BTreeMap::new();
        let mut aliases: BTreeMap<String, LibraryFile> = BTreeMap::new();
        let mut ambiguous = std::collections::BTreeSet::new();
        for library in self.iter() {
            for (logical, file) in library.files()? {
                files.insert(format!("content://{}/{logical}", library.manifest.id), file.clone());
                files.insert(file.asset_path().to_owned(), file.clone());
                if aliases.insert(logical.clone(), file).is_some() {
                    ambiguous.insert(logical);
                }
            }
        }
        files.extend(aliases.into_iter().filter(|(logical, _)| !ambiguous.contains(logical)));
        Ok(files)
    }

    pub fn resolve(&self, name: &str) -> Result<LibraryFile, String> {
        let files = self.files()?;
        if let Some(file) = files.get(name) { return Ok(file.clone()); }
        let owners: Vec<_> = files.values().filter(|file| file.logical == name)
            .map(|file| file.library.as_str()).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
        if owners.len() > 1 {
            Err(format!("Unqualified asset `{name}` has multiple owners: {}. Use an explicit library source.", owners.join(", ")))
        } else { Err(format!("No selected library supplies `{name}`")) }
    }
}

fn regular_files(root: &Path, selected: Option<&Selection>) -> Result<Vec<(String, PathBuf)>, String> {
    fn visit(root: &Path, directory: &Path, files: &mut Vec<(String, PathBuf)>, selected: Option<&Selection>) -> Result<(), String> {
        for entry in std::fs::read_dir(directory).map_err(|error| format!("{}: {error}", directory.display()))? {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            if entry.file_name().to_string_lossy().starts_with('.') { continue; }
            if selected.is_some_and(|selection| selection.manages(path.strip_prefix(root).unwrap())) { continue; }
            if entry.file_type().map_err(|error| error.to_string())?.is_dir() {
                visit(root, &path, files, selected)?;
            } else if path.is_file() {
                files.push((path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"), path));
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    visit(root, root, &mut files, selected)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library(root: &Path, id: &str, path: &str) -> ContentLibrary {
        let root = root.join(id);
        std::fs::create_dir_all(root.join("authored")).unwrap();
        std::fs::write(root.join("authored").join(path), id).unwrap();
        std::fs::write(root.join("library.json"), serde_json::json!({
            "schema": SCHEMA, "id": id, "version": "1", "asset_roots":{"generated":"authored"}
        }).to_string()).unwrap();
        ContentLibrary::read(root.join("library.json")).unwrap()
    }

    #[test]
    fn independent_compositions_select_only_their_own_library_files() {
        let root = tempfile::tempdir().unwrap();
        let mut first = ContentLibraries::default();
        first.insert(library(root.path(), "first", "body.ron")).unwrap();
        let mut second = ContentLibraries::default();
        second.insert(library(root.path(), "second", "body.ron")).unwrap();
        assert_eq!(std::fs::read_to_string(first.files().unwrap()["body.ron"].native_path().unwrap()).unwrap(), "first");
        assert_eq!(std::fs::read_to_string(second.files().unwrap()["body.ron"].native_path().unwrap()).unwrap(), "second");
        assert!(ContentLibraries::default().files().unwrap().is_empty());
        assert!(!first.files().unwrap().contains_key("content://second/body.ron"));
    }

    #[test]
    fn two_libraries_cannot_silently_replace_an_unqualified_asset() {
        let root = tempfile::tempdir().unwrap();
        let mut selected = ContentLibraries::default();
        selected.insert(library(root.path(), "first", "body.ron")).unwrap();
        selected.insert(library(root.path(), "second", "body.ron")).unwrap();
        assert!(selected.resolve("content://first/body.ron").is_ok());
        assert!(selected.resolve("content://second/body.ron").is_ok());
        let error = selected.resolve("body.ron").unwrap_err();
        assert!(error.contains("first") && error.contains("second") && error.contains("body.ron"));
    }
}

/// The package stores each library under its own identity.
pub fn package_path(path: &str) -> std::borrow::Cow<'_, str> {
    match path.strip_prefix("content://") {
        Some(path) => std::borrow::Cow::Owned(format!("__libraries/{path}")),
        None => std::borrow::Cow::Borrowed(path),
    }
}
