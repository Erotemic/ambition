//! Resolve logical art paths from one accepted publication selection.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use serde::Deserialize;

pub const SCHEMA: &str = "ambition.art_publication.v1";

#[derive(Clone, Debug, Deserialize)]
pub struct Member {
    pub path: String,
    pub sha256: String,
    pub role: String,
    pub tier: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Selection {
    pub schema: String,
    pub generation: Option<String>,
    pub files: BTreeMap<String, Member>,
    #[serde(default)]
    pub roots: Vec<String>,
    /// The accepted bundle of each publication unit, by unit name
    /// (`producer:music`, `target:hand_saint`, `atlas:full`).
    #[serde(default)]
    pub bundles: BTreeMap<String, String>,
    #[serde(skip)]
    root: PathBuf,
}

fn safe(path: &str) -> bool {
    !path.is_empty() && !path.contains('\\') && Path::new(path).components().all(|part| matches!(part, Component::Normal(_)))
}

pub fn managed(path: &Path) -> bool {
    path.components().next().is_some_and(|part| matches!(part.as_os_str().to_str(),
        Some("sprites" | "sprites_0_5x" | "sprites_0_25x" | "sprites_potato" | "sprite_packs")))
}

impl Selection {
    /// The selection of a store that has published nothing.
    pub fn empty(root: &Path) -> Self {
        Self { schema: SCHEMA.to_owned(), generation: None, files: BTreeMap::new(), roots: Vec::new(),
            bundles: BTreeMap::new(), root: root.to_path_buf() }
    }

    /// Read the selection of the store at `root`.
    ///
    /// An absent `published.json` gives an empty selection: the generated
    /// content is not made yet, and the game runs without it. A selection
    /// that is present and not valid is an error.
    pub fn read(root: &Path) -> Result<Self, String> {
        let path = root.join("published.json");
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Self::empty(root)),
            Err(error) => return Err(format!("{}: {error}", path.display())),
        };
        let mut selection: Self = serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", path.display()))?;
        if selection.schema != SCHEMA || selection.files.iter().any(|(name, member)| !safe(name) || !safe(&member.path)) || selection.roots.iter().any(|root| !safe(root)) {
            return Err(format!("Invalid art publication: {}", path.display()));
        }
        selection.root = root.to_path_buf();
        Ok(selection)
    }

    pub fn root(&self) -> &Path { &self.root }

    /// True when this selection names no generation (nothing is published).
    pub fn is_empty(&self) -> bool { self.generation.is_none() && self.files.is_empty() }

    pub fn manages(&self, path: &Path) -> bool {
        self.files.contains_key(&path.to_string_lossy().replace('\\', "/"))
            || managed(path) || self.roots.iter().any(|root| path.starts_with(root))
    }

    pub fn path(&self, logical: &str) -> Option<PathBuf> {
        self.files.get(logical).map(|member| self.root.join(&member.path))
    }

    pub fn logical_path(&self, path: &Path) -> Option<&str> {
        self.files.iter().find(|(_, member)| self.root.join(&member.path) == path).map(|(name, _)| name.as_str())
    }

    pub fn entries(&self) -> impl Iterator<Item = (&str, PathBuf)> {
        self.files.iter().map(|(name, member)| (name.as_str(), self.root.join(&member.path)))
    }
}
