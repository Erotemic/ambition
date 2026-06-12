//! [`AssetManifest`] — the catalog of [`AssetEntry`] records.
//!
//! Each entry binds one [`crate::AssetId`] to:
//! - an [`crate::kind::AssetKind`],
//! - a *logical* path (the canonical relative path used by the loose-fs
//!   profile and as a default for embedded / installed sources),
//! - a list of [`crate::location::AssetLocation`] candidates tagged with
//!   the [`crate::profile::AssetSourceProfile`] that supplies them,
//! - missing/cache policy,
//! - preload-group tag,
//! - optional content hash (for integrity checks in
//!   `IpfsGatewayPlaceholder` or HTTP-served profiles),
//! - declared dependencies (other [`crate::AssetId`]s that must also be
//!   loaded — purely informational in the first slice).
//!
//! The manifest is intentionally Bevy-free. The resolver picks one
//! `AssetLocation` per `(id, profile)` pair; Bevy integration lives in
//! [`crate::bevy_integration`].

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::id::AssetId;
use crate::kind::AssetKind;
use crate::location::AssetLocation;
use crate::policy::{CachePolicy, MissingAssetPolicy};
use crate::preload::PreloadGroup;
use crate::profile::AssetSourceProfile;

/// One asset entry in the catalog.
///
/// `logical_path` is a single relative path used as the default for
/// loose-filesystem / embedded / installed sources when their entry in
/// `locations` doesn't override. Override per-source with an explicit
/// `LocationCandidate`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AssetEntry {
    pub id: AssetId,
    pub kind: AssetKind,
    /// Canonical relative path. Used by the resolver to synthesize a
    /// `BevyPath` / `Embedded` location when no explicit candidate is
    /// authored for the active source. Always lower-snake-case with
    /// forward slashes — same shape as Bevy's default asset paths.
    pub logical_path: String,
    /// Authored locations. The resolver picks the first one whose
    /// `source` is enabled for the active profile.
    #[serde(default)]
    pub locations: Vec<LocationCandidate>,
    #[serde(default)]
    pub missing_policy: MissingAssetPolicy,
    #[serde(default)]
    pub cache_policy: CachePolicy,
    /// Optional preload bucket. None = lazy / on-demand.
    #[serde(default)]
    pub preload_group: Option<PreloadGroup>,
    /// Optional content hash (e.g. SHA-256 hex). Surfaces to consumers
    /// that want to verify HTTP / IPFS bytes; not enforced here.
    #[serde(default)]
    pub content_hash: Option<String>,
    /// Other ids this asset depends on. Purely informational in the
    /// first slice — the resolver does not topologically order loads.
    /// Future: drive `bevy_asset_loader` dependency tracking.
    #[serde(default)]
    pub dependencies: Vec<AssetId>,
}

impl AssetEntry {
    /// Minimal constructor: id + kind + logical path. Defaults all
    /// policies; no explicit per-source overrides.
    pub fn new(id: impl Into<AssetId>, kind: AssetKind, logical_path: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind,
            logical_path: logical_path.into(),
            locations: Vec::new(),
            missing_policy: MissingAssetPolicy::default(),
            cache_policy: CachePolicy::default(),
            preload_group: None,
            content_hash: None,
            dependencies: Vec::new(),
        }
    }

    pub fn with_missing_policy(mut self, policy: MissingAssetPolicy) -> Self {
        self.missing_policy = policy;
        self
    }

    pub fn with_cache_policy(mut self, policy: CachePolicy) -> Self {
        self.cache_policy = policy;
        self
    }

    pub fn with_preload_group(mut self, group: PreloadGroup) -> Self {
        self.preload_group = Some(group);
        self
    }

    pub fn with_content_hash(mut self, hash: impl Into<String>) -> Self {
        self.content_hash = Some(hash.into());
        self
    }

    pub fn with_dependency(mut self, dep: impl Into<AssetId>) -> Self {
        self.dependencies.push(dep.into());
        self
    }

    /// Add an explicit per-source location override. Multiple calls
    /// stack; the resolver picks the first matching `source` for the
    /// active profile.
    pub fn with_location(mut self, source: AssetSourceProfile, location: AssetLocation) -> Self {
        self.locations.push(LocationCandidate { source, location });
        self
    }
}

/// One (source-kind, location) pair authored on an entry.
///
/// The resolver consults the active profile's
/// [`crate::profile::AssetProfile::preferred_sources`] order; the first
/// candidate whose `source` appears in that list wins.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LocationCandidate {
    pub source: AssetSourceProfile,
    pub location: AssetLocation,
}

/// Catalog of asset entries keyed by [`AssetId`].
///
/// Construct via [`AssetManifest::builder`] for ergonomic in-code
/// authoring, or deserialize from RON/JSON via serde.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AssetManifest {
    entries: HashMap<AssetId, AssetEntry>,
}

impl AssetManifest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder() -> AssetManifestBuilder {
        AssetManifestBuilder::default()
    }

    /// Insert an entry. If `id` was already present the old entry is
    /// returned, mirroring `HashMap::insert`.
    pub fn insert(&mut self, entry: AssetEntry) -> Option<AssetEntry> {
        self.entries.insert(entry.id.clone(), entry)
    }

    /// Look up a single entry by id.
    pub fn get(&self, id: &AssetId) -> Option<&AssetEntry> {
        self.entries.get(id)
    }

    /// Iterate every (id, entry) pair. Order is not stable.
    pub fn iter(&self) -> impl Iterator<Item = (&AssetId, &AssetEntry)> {
        self.entries.iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return every entry tagged with the given preload group, sorted
    /// by id so the order is reproducible for tests + diagnostics.
    pub fn entries_in_group(&self, group: PreloadGroup) -> Vec<&AssetEntry> {
        let mut out: Vec<&AssetEntry> = self
            .entries
            .values()
            .filter(|e| e.preload_group == Some(group))
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    /// Return every required entry (missing-policy = Error). Sorted by
    /// id for reproducible diagnostics.
    pub fn required_entries(&self) -> Vec<&AssetEntry> {
        let mut out: Vec<&AssetEntry> = self
            .entries
            .values()
            .filter(|e| e.missing_policy.is_required())
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }
}

/// Ergonomic builder for in-code manifest authoring. Chain `.entry(...)`
/// calls and finish with `.build()`.
#[derive(Clone, Debug, Default)]
pub struct AssetManifestBuilder {
    manifest: AssetManifest,
}

impl AssetManifestBuilder {
    pub fn entry(mut self, entry: AssetEntry) -> Self {
        self.manifest.insert(entry);
        self
    }

    pub fn build(self) -> AssetManifest {
        self.manifest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> AssetManifest {
        AssetManifest::builder()
            .entry(
                AssetEntry::new(
                    "world.sandbox_ldtk",
                    AssetKind::LdtkProject,
                    "ambition/worlds/sandbox.ldtk",
                )
                .with_missing_policy(MissingAssetPolicy::Error)
                .with_preload_group(PreloadGroup::Bootstrap),
            )
            .entry(
                AssetEntry::new(
                    "audio.sfx_bank",
                    AssetKind::AudioBank,
                    "ambition/audio/sfx.bank",
                )
                .with_missing_policy(MissingAssetPolicy::WarnAndPlaceholder)
                .with_preload_group(PreloadGroup::SandboxCore),
            )
            .entry(
                AssetEntry::new(
                    "sprite.entity.chest_closed",
                    AssetKind::Image,
                    "sprites/entities/chest_closed.png",
                )
                .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
                .with_preload_group(PreloadGroup::SandboxCore),
            )
            .build()
    }

    #[test]
    fn lookup_by_id_returns_entry() {
        let m = fixture();
        let entry = m.get(&AssetId::new("world.sandbox_ldtk")).unwrap();
        assert_eq!(entry.kind, AssetKind::LdtkProject);
        assert_eq!(entry.logical_path, "ambition/worlds/sandbox.ldtk");
        assert_eq!(entry.preload_group, Some(PreloadGroup::Bootstrap));
    }

    #[test]
    fn lookup_unknown_id_is_none() {
        let m = fixture();
        assert!(m.get(&AssetId::new("does.not.exist")).is_none());
    }

    #[test]
    fn entries_in_group_returns_sorted_subset() {
        let m = fixture();
        let core = m.entries_in_group(PreloadGroup::SandboxCore);
        assert_eq!(core.len(), 2);
        let ids: Vec<_> = core.iter().map(|e| e.id.as_str()).collect();
        // Sorted alphabetically by id.
        assert_eq!(ids, vec!["audio.sfx_bank", "sprite.entity.chest_closed"]);
    }

    #[test]
    fn required_entries_excludes_optional() {
        let m = fixture();
        let req = m.required_entries();
        assert_eq!(req.len(), 1);
        assert_eq!(req[0].id.as_str(), "world.sandbox_ldtk");
    }

    #[test]
    fn insert_returns_previous_entry() {
        let mut m = AssetManifest::new();
        let first = AssetEntry::new("a", AssetKind::Other, "a.bin");
        let second = AssetEntry::new("a", AssetKind::Binary, "a.bin");
        assert!(m.insert(first.clone()).is_none());
        let prev = m.insert(second).unwrap();
        assert_eq!(prev, first);
    }

    #[test]
    fn json_round_trip_preserves_entry_fields() {
        let m = fixture();
        let json = serde_json::to_string(&m).unwrap();
        let back: AssetManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn builder_chains_into_manifest() {
        let m = AssetManifest::builder()
            .entry(AssetEntry::new("a", AssetKind::Other, "a.bin"))
            .entry(AssetEntry::new("b", AssetKind::Other, "b.bin"))
            .build();
        assert_eq!(m.len(), 2);
    }
}
