//! Data manifests for the sandbox.
//!
//! The goal of this module is to keep tuning/audio iteration data in RON while
//! still letting the current code synthesize assets at startup. `bevy_common_assets` registers
//! `SandboxDataSpec` as a real Bevy asset type; `load_embedded` gives us a
//! synchronous bootstrap path until the sandbox grows a loading state.
//!
//! Bevy resolves `ambition/sandbox.ron` relative to the sandbox crate asset
//! root (`crates/ambition_sandbox/assets`) when this package is run through
//! Cargo, so the embedded copy intentionally lives there too. World/room
//! authoring has moved to LDtk; this RON asset intentionally owns only
//! non-spatial sandbox tuning and generated-audio configuration.

use ambition_engine as ae;
use bevy::asset::{Asset, AssetServer};
use bevy::prelude::{Commands, Handle, Res, Resource};
use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};
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
        ron::from_str(include_str!("../assets/ambition/sandbox.ron"))
            .expect("embedded assets/ambition/sandbox.ron should parse")
    }
}

#[derive(Resource, Clone, Debug)]
pub struct SandboxDataAsset(pub Handle<SandboxDataSpec>);

pub fn load_data_asset_handle(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(SandboxDataAsset(asset_server.load(SANDBOX_DATA_ASSET)));
}

// Spatial/world authoring moved to LDtk. This module intentionally contains
// only non-spatial sandbox tuning and generated-audio data.

#[derive(Clone, Debug, Deserialize)]
pub struct AudioSpec {
    pub sample_rate: u32,
    pub sfx: Vec<SfxSpec>,
    pub default_music_track: String,
    pub music_tracks: Vec<MusicTrackSpec>,
}

impl AudioSpec {
    pub fn validate(&self) -> Result<(), String> {
        if self.sample_rate < 8_000 {
            return Err(format!(
                "audio sample_rate must be at least 8000 Hz, got {}",
                self.sample_rate
            ));
        }
        if self.music_tracks.is_empty() {
            return Err("audio music_tracks must contain at least one track".to_string());
        }
        let mut ids = HashSet::new();
        for track in &self.music_tracks {
            if track.id.trim().is_empty() {
                return Err("music track id must not be empty".to_string());
            }
            if track.display_name.trim().is_empty() {
                return Err(format!("music track '{}' display_name is empty", track.id));
            }
            if !ids.insert(track.id.as_str()) {
                return Err(format!("duplicate music track id '{}'", track.id));
            }
            track.arrangement.validate().map_err(|error| {
                format!("music track '{}' arrangement is invalid: {error}", track.id)
            })?;
        }
        if self.track(&self.default_music_track).is_none() {
            return Err(format!(
                "default_music_track '{}' does not match any music_tracks id",
                self.default_music_track
            ));
        }
        Ok(())
    }

    pub fn track(&self, id: &str) -> Option<&MusicTrackSpec> {
        self.music_tracks.iter().find(|track| track.id == id)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct MusicTrackSpec {
    pub id: String,
    pub display_name: String,
    pub arrangement: MusicSpec,
    /// Optional pre-rendered OGG asset path (relative to the asset
    /// root). When `Some`, `AudioLibrary::new` loads this asset
    /// instead of running the procedural `render_lofi_theme` synth
    /// at startup. Authored via `tools/ambition_music_renderer` as a
    /// YAML cue → OGG; matches the `first_goblin_tune_v2` pattern
    /// already used for adaptive cues. The `arrangement` field stays
    /// for `duration_seconds()` reporting and as a fallback if the
    /// asset fails to load. Default `None` keeps the legacy
    /// procedural path for tracks not yet migrated.
    #[serde(default)]
    pub asset_path: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash)]
pub enum SoundCueKey {
    Jump,
    DoubleJump,
    Dash,
    Blink,
    PrecisionBlink,
    Slash,
    Hit,
    Pogo,
    Reset,
    Death,
    Respawn,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum WaveformSpec {
    Sine,
    Square,
    Triangle,
    Saw,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct SfxSpec {
    pub cue: SoundCueKey,
    pub waveform: WaveformSpec,
    pub frequency: f32,
    pub frequency_end: f32,
    pub duration: f32,
    pub volume: f32,
    pub attack: f32,
    pub release: f32,
    pub noise: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MusicSpec {
    pub bpm: f32,
    pub total_beats: f32,
    pub root_hz: f32,
    pub bass_root_hz: f32,
    pub key_root_hz: f32,
    pub master_gain: f32,
    pub lowpass_alpha: f32,
    pub tape_hiss: f32,
    pub lead: Vec<NoteSpec>,
    pub chords: Vec<[i32; 4]>,
    pub bass_roots: Vec<i32>,
    pub gains: MusicGainsSpec,
}

impl MusicSpec {
    pub fn duration_seconds(&self) -> f32 {
        self.total_beats.max(1.0) * 60.0 / self.bpm.max(1.0)
    }

    pub fn bar_count(&self) -> usize {
        (self.total_beats.max(1.0) / 4.0).ceil().max(1.0) as usize
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.bpm <= 0.0 {
            return Err(format!("bpm must be positive, got {}", self.bpm));
        }
        if self.total_beats <= 0.0 {
            return Err(format!(
                "total_beats must be positive, got {}",
                self.total_beats
            ));
        }
        if self.root_hz <= 0.0 || self.bass_root_hz <= 0.0 || self.key_root_hz <= 0.0 {
            return Err("root frequencies must be positive".to_string());
        }
        if self.chords.is_empty() {
            return Err("chords must not be empty".to_string());
        }
        if self.bass_roots.is_empty() {
            return Err("bass_roots must not be empty".to_string());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct NoteSpec {
    pub start: f32,
    pub duration: f32,
    pub semitone: i32,
    pub volume: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct MusicGainsSpec {
    pub chord_pad: f32,
    pub lead: f32,
    pub soft_keys: f32,
    pub bass: f32,
    pub drums: f32,
}

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
            root_hz: 220.0,
            bass_root_hz: 110.0,
            key_root_hz: 220.0,
            master_gain: 0.5,
            lowpass_alpha: 0.5,
            tape_hiss: 0.0,
            lead: Vec::new(),
            chords: vec![[0, 4, 7, 11]],
            bass_roots: vec![0],
            gains: MusicGainsSpec {
                chord_pad: 1.0,
                lead: 1.0,
                soft_keys: 1.0,
                bass: 1.0,
                drums: 1.0,
            },
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
