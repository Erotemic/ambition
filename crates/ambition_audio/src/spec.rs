//! Audio data schema: the authored (RON) shapes for procedural SFX and
//! pre-rendered music. Kira-free — parse/validation only; the playback
//! runtime lives behind this crate's `kira` feature.
//!
//! SFX and music are deliberately split into two registries
//! ([`SfxRegistry`] ← `sfx_registry.ron`, [`MusicRegistry`] ←
//! `music_registry.ron`): they are different concerns with different
//! authorship. SFX specs are hand-tuned procedural synthesis; the music
//! registry is a *generated* projection of the rendered-OGG asset tree
//! (see `scripts/regen_music_registry.py` + `regen_music.sh`). Keeping
//! them apart means the auto-generated music list never churns the
//! hand-authored sound-design data, and neither lives inside the
//! gameplay-tuning `sandbox.ron`.

use ambition_sfx::SfxId;
use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};

/// Procedural SFX-synthesis registry, authored in `sfx_registry.ron`.
///
/// Hand-tuned sound design: the synth `sample_rate` plus one [`SfxSpec`]
/// per cue. Deliberately separate from [`MusicRegistry`] — different
/// concern, different file.
#[derive(Clone, Debug, Deserialize, PartialEq, Resource)]
pub struct SfxRegistry {
    pub sample_rate: u32,
    pub sfx: Vec<SfxSpec>,
}

impl SfxRegistry {
    pub fn validate(&self) -> Result<(), String> {
        if self.sample_rate < 8_000 {
            return Err(format!(
                "audio sample_rate must be at least 8000 Hz, got {}",
                self.sample_rate
            ));
        }
        Ok(())
    }

    /// The [`SfxId`]s this registry authorizes through its procedural cue
    /// specs. This is the *authority* projection of the registry (kira-free):
    /// a provider that authors a cue authorizes the id that cue resolves to,
    /// so provider-relative playback can gate an [`ambition_sfx::SfxMessage`]
    /// without the resident synth handle table. A registry with no cues
    /// authorizes no procedural ids — deliberate silence for that path.
    pub fn authorized_cue_ids(&self) -> BTreeSet<SfxId> {
        self.sfx.iter().map(|spec| spec.cue.sfx_id()).collect()
    }
}

impl SoundCueKey {
    /// The stable [`SfxId`] a procedural cue resolves to. Mirrors the
    /// consumer-side `SoundCue::sfx_id` table but lives in the kira-free data
    /// layer so provider authority can be derived without the playback crate.
    pub fn sfx_id(self) -> SfxId {
        use ambition_sfx::ids;
        match self {
            Self::Jump => ids::PLAYER_JUMP,
            Self::DoubleJump => ids::PLAYER_DOUBLE_JUMP,
            Self::Dash => ids::PLAYER_DASH,
            Self::Blink => ids::PLAYER_BLINK,
            Self::PrecisionBlink => ids::PLAYER_PRECISION_BLINK,
            Self::Slash => ids::PLAYER_SLASH,
            Self::Hit => ids::PLAYER_HIT,
            Self::Pogo => ids::PLAYER_POGO,
            Self::Reset => ids::PLAYER_RESET,
            Self::Death => ids::PLAYER_DEATH,
            Self::Respawn => ids::PLAYER_RESPAWN,
        }
    }
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

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum WaveformSpec {
    Sine,
    Square,
    Triangle,
    Saw,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
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

/// Music-cue registry, authored in `music_registry.ron`.
///
/// **This file is GENERATED** by `scripts/regen_music_registry.py` from
/// the rendered-OGG asset tree (`audio/music/generated/*/full.ogg`), wired
/// into `regen_music.sh`. Hand-edits get overwritten on the next render —
/// adjust the generator's denylist / display-name map instead. The format
/// is intentionally trivial (just ids) precisely so it can be generated:
/// there is no tempo/arrangement metadata because the OGG is what plays
/// and the runtime music director owns looping/crossfade.
#[derive(Clone, Debug, Deserialize, PartialEq, Resource)]
pub struct MusicRegistry {
    /// Track id played at startup / when no radio station is selected.
    pub default_track: String,
    pub tracks: Vec<MusicTrack>,
}

impl MusicRegistry {
    pub fn validate(&self) -> Result<(), String> {
        if self.tracks.is_empty() {
            return Err("music registry must contain at least one track".to_string());
        }
        let mut ids = HashSet::new();
        for track in &self.tracks {
            if track.id.trim().is_empty() {
                return Err("music track id must not be empty".to_string());
            }
            if track.display_name.trim().is_empty() {
                return Err(format!("music track '{}' display_name is empty", track.id));
            }
            if !ids.insert(track.id.as_str()) {
                return Err(format!("duplicate music track id '{}'", track.id));
            }
        }
        if self.track(&self.default_track).is_none() {
            return Err(format!(
                "default_track '{}' does not match any registered track id",
                self.default_track
            ));
        }
        Ok(())
    }

    pub fn track(&self, id: &str) -> Option<&MusicTrack> {
        self.tracks.iter().find(|track| track.id == id)
    }
}

/// One playable music track: a pointer to a pre-rendered OGG.
///
/// `asset_path` is optional — when omitted it defaults to the conventional
/// `audio/music/generated/{id}/full.ogg`, which covers every plain
/// renderer cue. Set it explicitly only for off-convention assets (e.g. an
/// adaptive cue's section mix). No arrangement/tempo metadata: that data
/// was vestigial (the OGG dictates length), and dropping it is what lets
/// the registry be generated from `id` alone.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct MusicTrack {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub asset_path: Option<String>,
}

impl MusicTrack {
    /// Asset path the `AudioLibrary` should load: the explicit override if
    /// set, else the conventional generated path derived from `id`.
    pub fn resolved_asset_path(&self) -> String {
        self.asset_path
            .clone()
            .unwrap_or_else(|| format!("audio/music/generated/{}/full.ogg", self.id))
    }
}
