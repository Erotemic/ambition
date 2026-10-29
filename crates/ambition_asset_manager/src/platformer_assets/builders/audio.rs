//! Audio builders: packed SFX bank + per-track music entries.

use crate::{
    AssetEntry, AssetKind, AssetManifest, MissingAssetPolicy,
    PreloadGroup,
};

use super::super::ids;
use super::super::MusicCatalogRow;

/// Music track entries, one per game-provided music row. Each resolves to a
/// pre-rendered OGG path (the explicit `asset_path` override, else the game's
/// generated `audio/music/generated/<id>/full.ogg` convention). A missing OGG
/// degrades to a placeholder at load time. Author cues with
/// `tools/ambition_music_renderer`; `scripts/regen_music_registry.py`
/// generates `music_registry.ron`.
pub(in super::super) fn extend_with_music_entries(
    manifest: &mut AssetManifest,
    music: &[MusicCatalogRow],
) {
    for track in music {
        let id = ids::music_track(&track.id);
        manifest.insert(
            AssetEntry::new(id, AssetKind::AudioClip, track.asset_path.clone())
                .with_missing_policy(MissingAssetPolicy::WarnAndPlaceholder)
                .with_preload_group(PreloadGroup::SandboxCore),
        );
    }
}
