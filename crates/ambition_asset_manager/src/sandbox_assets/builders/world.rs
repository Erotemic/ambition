//! World / level data builders: LDtk project entries derived from
//! caller-provided world rows, plus the sandbox tuning RON. No world is named
//! here — the manifest rows carry identity, paths, and embedded candidates.

use crate::{
    AssetEntry, AssetKind, AssetLocation, AssetManifest, AssetSourceProfile, MissingAssetPolicy,
    PreloadGroup,
};

use super::super::ids;
use super::super::WorldCatalogRow;

/// LDtk world entries, one per installed world row. A `required` row (the
/// primary) gets `MissingAssetPolicy::Error` —
/// the game cannot run without it; optional secondaries get
/// `WarnAndPlaceholder` so the merge loader skips them silently when a
/// partial checkout lacks the file.
///
/// A row's `loose_path` becomes the explicit `LooseFilesystem` candidate so
/// the desktop hot-reload watcher (primary world only) has a `LocalPath` to
/// inotify; its `embedded_bevy_path` becomes the `EmbeddedBinary` candidate
/// the static profiles resolve to.
pub(in super::super) fn extend_with_world_entries(
    manifest: &mut AssetManifest,
    worlds: &[WorldCatalogRow],
) {
    for source in worlds {
        let mut entry = AssetEntry::new(
            source.id.clone(),
            AssetKind::LdtkProject,
            source.asset_path.as_str(),
        )
        .with_missing_policy(if source.required {
            MissingAssetPolicy::Error
        } else {
            MissingAssetPolicy::WarnAndPlaceholder
        })
        .with_preload_group(PreloadGroup::Bootstrap);
        if let Some(loose) = &source.loose_path {
            entry = entry.with_location(
                AssetSourceProfile::LooseFilesystem,
                AssetLocation::LocalPath(loose.clone()),
            );
        }
        if let Some(bevy_path) = source.embedded_bevy_path {
            entry = entry.with_location(
                AssetSourceProfile::EmbeddedBinary,
                AssetLocation::embedded(bevy_path),
            );
        }
        manifest.insert(entry);
    }
}

/// Sandbox tuning RON entry. Required — the game refuses to run without it. The
/// catalog entry exists so code that asks for the Bevy path under a non-static
/// profile gets a real answer.
pub(in super::super) fn extend_with_data_entries(manifest: &mut AssetManifest) {
    manifest.insert(
        AssetEntry::new(
            ids::sandbox_data(),
            AssetKind::RonData,
            "ambition/sandbox.ron",
        )
        .with_missing_policy(MissingAssetPolicy::Error)
        .with_preload_group(PreloadGroup::Bootstrap),
    );
}
