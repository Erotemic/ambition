//! Data manifests for the sandbox.
//!
//! The goal of this module is to keep tuning/audio iteration data in RON while
//! still letting the current code synthesize assets at startup. `bevy_common_assets` registers
//! `SandboxDataSpec` as a real Bevy asset type; `load_embedded` gives us a
//! synchronous bootstrap path until the sandbox grows a loading state.
//!
//! Bevy resolves `ambition/sandbox.ron` relative to the sandbox crate asset
//! root (`crates/ambition_gameplay_core/assets`) when this package is run through
//! Cargo, so the embedded copy intentionally lives there too. World/room
//! authoring has moved to LDtk; this RON asset intentionally owns only
//! non-spatial sandbox tuning and generated-audio configuration.

use crate::engine_core as ae;
use bevy::asset::{Asset, AssetServer};
use bevy::prelude::{Commands, Handle, Res, Resource};
use bevy::reflect::TypePath;
use serde::Deserialize;
#[cfg(test)]
use std::collections::HashSet;

pub const SANDBOX_DATA_ASSET: &str = "ambition/sandbox.ron";

#[derive(Clone, Debug, Deserialize, Asset, TypePath, Resource)]
pub struct SandboxDataSpec {
    pub abilities: ae::AbilitySet,
    pub tuning: ae::MovementTuning,
    pub audio: AudioSpec,
}

impl SandboxDataSpec {
    pub fn load_embedded() -> Self {
        ron::from_str(include_str!("../../assets/ambition/sandbox.ron"))
            .expect("embedded assets/ambition/sandbox.ron should parse")
    }
}

#[derive(Resource, Clone, Debug)]
pub struct SandboxDataAsset(pub Handle<SandboxDataSpec>);

/// Bevy startup system: register a `Handle<SandboxDataSpec>` so the
/// asset server keeps the underlying `.ron` alive (and emits hot
/// reload events under `bevy_dev_hot_reload`).
///
/// Resolves the path through the active
/// [`crate::assets::sandbox_assets::SandboxAssetCatalog`] when one is
/// installed. The catalog entry
/// [`crate::assets::sandbox_assets::ids::sandbox_data`] is required, so the
/// catalog never returns `Disabled` outside of `NoAssets`/`Headless`.
/// Falls back to the raw asset-path constant when no catalog resource
/// is present (visible-only init order / tests).
pub fn load_data_asset_handle(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    catalog: Option<Res<crate::assets::sandbox_assets::SandboxAssetCatalog>>,
) {
    let path = catalog
        .as_ref()
        .and_then(|c| c.path_for(&crate::assets::sandbox_assets::ids::sandbox_data()))
        .unwrap_or_else(|| SANDBOX_DATA_ASSET.to_string());
    commands.insert_resource(SandboxDataAsset(asset_server.load(path)));
}

// Spatial/world authoring moved to LDtk. This module intentionally contains
// only non-spatial sandbox tuning; the generated-audio data schema
// (`AudioSpec` & friends) moved DOWN into the `ambition_audio` crate.
// Re-exported so every `crate::runtime::data::AudioSpec` path resolves
// unchanged.
pub use ambition_audio::spec::{
    AudioSpec, MusicSpec, MusicTrackSpec, SfxSpec, SoundCueKey, WaveformSpec,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_sandbox_data_parses_and_audio_validates() {
        let spec = SandboxDataSpec::load_embedded();
        spec.audio
            .validate()
            .expect("embedded audio spec validates");
    }

    #[test]
    fn embedded_music_tracks_are_unique_and_default_resolves() {
        let spec = SandboxDataSpec::load_embedded();
        let mut ids = HashSet::new();
        for track in &spec.audio.music_tracks {
            assert!(ids.insert(track.id.as_str()), "duplicate id {}", track.id);
        }
        assert!(spec.audio.track(&spec.audio.default_music_track).is_some());
    }

    fn synthetic_audio(tracks: Vec<MusicTrackSpec>, default: &str) -> AudioSpec {
        AudioSpec {
            sample_rate: 44_100,
            sfx: Vec::new(),
            default_music_track: default.into(),
            music_tracks: tracks,
        }
    }

    fn synthetic_arrangement() -> MusicSpec {
        MusicSpec {
            bpm: 72.0,
            total_beats: 32.0,
        }
    }

    fn synthetic_track(id: &str) -> MusicTrackSpec {
        MusicTrackSpec {
            id: id.into(),
            display_name: format!("{id} display"),
            arrangement: synthetic_arrangement(),
            asset_path: None,
        }
    }

    /// Duplicate track ids must be rejected — the audio system uses
    /// the id as a switch key, so a duplicate would shadow whichever
    /// track the player asked for at runtime.
    #[test]
    fn validate_rejects_duplicate_track_ids() {
        let audio = synthetic_audio(
            vec![synthetic_track("alpha"), synthetic_track("alpha")],
            "alpha",
        );
        let err = audio.validate().expect_err("duplicate ids must fail");
        assert!(err.contains("duplicate"), "got: {err}");
    }

    /// Missing music_tracks (empty list) must fail — nothing to play.
    #[test]
    fn validate_rejects_empty_music_tracks() {
        let audio = synthetic_audio(Vec::new(), "alpha");
        let err = audio.validate().expect_err("empty music_tracks must fail");
        assert!(err.contains("music_tracks"), "got: {err}");
    }

    /// Missing default_music_track id (no track matches) must fail —
    /// the audio system would otherwise try to play a non-existent
    /// track at startup.
    #[test]
    fn validate_rejects_missing_default_track() {
        let audio = synthetic_audio(vec![synthetic_track("alpha")], "ghost");
        let err = audio.validate().expect_err("missing default must fail");
        assert!(err.contains("default_music_track"), "got: {err}");
    }

    /// Empty display_name must fail — the pause-menu music selector
    /// surfaces it in the UI; an empty value would render as a blank
    /// line and confuse the player.
    #[test]
    fn validate_rejects_empty_display_name() {
        let mut track = synthetic_track("alpha");
        track.display_name = String::new();
        let audio = synthetic_audio(vec![track], "alpha");
        let err = audio.validate().expect_err("empty display_name must fail");
        assert!(err.contains("display_name"), "got: {err}");
    }

    /// Empty track id must fail — id is used as a switch key in the
    /// audio system; an empty key collides with "no track selected".
    #[test]
    fn validate_rejects_empty_track_id() {
        let mut track = synthetic_track("");
        track.id = String::new();
        let audio = synthetic_audio(vec![track], "");
        let err = audio.validate().expect_err("empty id must fail");
        assert!(err.contains("id"), "got: {err}");
    }

    /// Invalid arrangement (zero bpm) must fail — `duration_seconds`
    /// would otherwise return infinity, breaking music playback.
    #[test]
    fn validate_rejects_arrangement_with_zero_bpm() {
        let mut track = synthetic_track("alpha");
        track.arrangement.bpm = 0.0;
        let audio = synthetic_audio(vec![track], "alpha");
        let err = audio
            .validate()
            .expect_err("zero bpm arrangement must fail");
        assert!(err.contains("bpm"), "got: {err}");
    }

    #[test]
    fn embedded_music_tracks_include_original_and_long_default() {
        let spec = SandboxDataSpec::load_embedded();
        let original = spec
            .audio
            .track("original_lofi_loop")
            .expect("original track exists");
        let long = spec
            .audio
            .track("long_lofi_drift")
            .expect("long track exists");
        assert_eq!(spec.audio.default_music_track, "long_lofi_drift");
        assert!((original.arrangement.duration_seconds() - (32.0 * 60.0 / 72.0)).abs() < 0.01);
        let long_duration = long.arrangement.duration_seconds();
        assert!(long_duration > original.arrangement.duration_seconds() * 3.0);
        assert!(
            (90.0..=120.0).contains(&long_duration),
            "long duration was {long_duration}"
        );
    }
}
