//! Resolve logical art paths from one accepted publication selection.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use serde::Deserialize;

pub const SCHEMA: &str = "ambition.art_publication.v1";
pub const ADDRESS_PREFIX: &str = "__art/";

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
    #[serde(skip)]
    root: PathBuf,
}

fn safe(path: &str) -> bool {
    !path.is_empty() && !path.contains('\\') && Path::new(path).components().all(|part| matches!(part, Component::Normal(_)))
}

pub fn store_for_assets(assets: &Path) -> PathBuf {
    std::env::var_os("AMBITION_ART_STORE").map(PathBuf::from)
        .unwrap_or_else(|| assets.parent().unwrap_or(assets).join("art_publication"))
}

pub fn managed(path: &Path) -> bool {
    path.components().next().is_some_and(|part| matches!(part.as_os_str().to_str(),
        Some("sprites" | "sprites_0_5x" | "sprites_0_25x" | "sprites_potato" | "sprite_packs")))
}

impl Selection {
    pub fn read(root: &Path) -> Result<Self, String> {
        let path = root.join("published.json");
        let bytes = std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let mut selection: Self = serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", path.display()))?;
        if selection.schema != SCHEMA || selection.files.iter().any(|(name, member)| !safe(name) || !safe(&member.path)) {
            return Err(format!("Invalid art publication: {}", path.display()));
        }
        selection.root = root.to_path_buf();
        Ok(selection)
    }

    pub fn root(&self) -> &Path { &self.root }

    pub fn manages(&self, path: &Path) -> bool {
        managed(path) || path.components().next().is_some_and(|part|
            self.roots.iter().any(|root| part.as_os_str() == std::ffi::OsStr::new(root)))
    }

    pub fn path(&self, logical: &str) -> Option<PathBuf> {
        self.files.get(logical).map(|member| self.root.join(&member.path))
    }

    pub fn address(&self, logical: &str) -> Option<String> {
        self.files.get(logical).map(|member| format!("{ADDRESS_PREFIX}{}", member.path))
    }

    pub fn logical_path(&self, path: &Path) -> Option<&str> {
        self.files.iter().find(|(_, member)| self.root.join(&member.path) == path).map(|(name, _)| name.as_str())
    }

    pub fn entries(&self) -> impl Iterator<Item = (&str, PathBuf)> {
        self.files.iter().map(|(name, member)| (name.as_str(), self.root.join(&member.path)))
    }
}

