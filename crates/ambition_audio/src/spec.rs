//! Audio data schema: the authored (RON) shapes for SFX cues and
//! pre-rendered music tracks. Kira-free — parse/validation only; the
//! playback runtime lives behind this crate's `kira` feature.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

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
    /// Pre-rendered OGG asset path (relative to the asset root).
    /// `AudioLibrary::new` loads this asset at startup. Authored via
    /// `tools/ambition_music_renderer` as a YAML cue → OGG. The
    /// runtime requires every live track to set this — tracks left
    /// at `None` are skipped at startup with a warning (the fundsp
    /// procedural music generator was retired; see
    /// `docs/archive/retired/fundsp-audio.md`). The `arrangement` field is retained
    /// as documentation of how each OGG was authored and for
    /// `duration_seconds()` reporting.
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

/// Minimal duration descriptor for a pre-rendered music track.
///
/// Originally this was a full `MusicSpec` describing the fundsp
/// procedural synth path (chord progressions, lead notes, bass
/// roots, per-section gains). That synth path was retired (see
/// `docs/archive/retired/fundsp-audio.md`) and OGGs are now authored
/// via `tools/ambition_music_renderer` from YAML cues. The only
/// runtime-meaningful values left were `bpm` + `total_beats` —
/// everything else was inert documentation that drifted from the
/// real OGGs. Trimmed 2026-05-23 to just those two fields so the
/// `sandbox.ron` track list stops carrying ~30-80 lines of dead
/// data per entry.
///
/// Authoritative arrangement details for each track live in the
/// renderer score: `tools/ambition_music_renderer/scores/active/<id>.music.yaml`.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct MusicSpec {
    pub bpm: f32,
    pub total_beats: f32,
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
        Ok(())
    }
}
