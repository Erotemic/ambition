//! Audio builders: packed SFX bank + per-track music entries.

use ambition_asset_manager::{
    AssetEntry, AssetKind, AssetLocation, AssetManifest, AssetSourceProfile, MissingAssetPolicy,
    PreloadGroup,
};

use super::super::ids;
use crate::content::data::AudioSpec;

/// Packed SFX bank entry. `WarnAndPlaceholder` matches the current
/// runtime contract: a missing bank degrades to procedural / silent
/// SFX instead of refusing to start.
///
/// `AMBITION_SFX_BANK_PATH` is honored as an explicit
/// `LooseFilesystem` `LocationCandidate` so dev workflows can point
/// the sandbox at a freshly-packed bank without re-publishing assets.
pub(in super::super) fn extend_with_sfx_bank_entry(manifest: &mut AssetManifest) {
    let mut entry = AssetEntry::new(ids::sfx_bank(), AssetKind::AudioBank, "audio/sfx.bank")
        .with_missing_policy(MissingAssetPolicy::WarnAndPlaceholder)
        .with_preload_group(PreloadGroup::SandboxCore);
    if let Ok(env_path) = std::env::var("AMBITION_SFX_BANK_PATH") {
        entry = entry.with_location(
            AssetSourceProfile::LooseFilesystem,
            AssetLocation::LocalPath(std::path::PathBuf::from(env_path)),
        );
    }
    manifest.insert(entry);
}

/// Music track entries — one per `MusicTrackSpec` in the audio spec
/// that has an `asset_path` (pre-rendered OGG). Tracks without
/// `asset_path` are skipped at both the catalog layer and the
/// `AudioLibrary` layer (the procedural fundsp music generator was
/// retired; see `docs/archive/retired/fundsp-audio.md` for the historical note).
/// Spec authors must add a pre-rendered OGG via
/// `tools/ambition_music_renderer` or remove the track from
/// `sandbox.ron`.
pub(in super::super) fn extend_with_music_entries(manifest: &mut AssetManifest, audio: &AudioSpec) {
    for track in &audio.music_tracks {
        let Some(asset_path) = track.asset_path.as_deref() else {
            continue;
        };
        let id = ids::music_track(&track.id);
        manifest.insert(
            AssetEntry::new(id, AssetKind::AudioClip, asset_path.to_string())
                .with_missing_policy(MissingAssetPolicy::WarnAndPlaceholder)
                .with_preload_group(PreloadGroup::SandboxCore),
        );
    }
}
