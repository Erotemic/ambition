//! Audio data schema: the authored (RON) shapes for procedural SFX and
//! pre-rendered music. Kira-free: parse and validation only; playback is
//! behind this crate's `kira` feature.
//!
//! SFX and music use two registries ([`SfxRegistry`] from
//! `sfx_registry.ron`, [`MusicRegistry`] from `music_registry.ron`) because
//! their authorship differs. SFX specs are hand-tuned synthesis; the music
//! registry is generated from the rendered-OGG asset tree (see
//! `scripts/regen_music_registry.py` and `scripts/regen/music.sh`). So the
//! generated list never churns hand-authored sound design, and neither lives
//! in `platformer_defaults.ron`.

use ambition_sfx::SfxId;
use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};

/// Procedural SFX-synthesis registry, authored in `sfx_registry.ron`.
///
/// Hand-tuned sound design: the synth `sample_rate` and one [`SfxSpec`] per
/// cue. Separate from [`MusicRegistry`].
#[derive(Clone, Debug, Deserialize, PartialEq, Resource)]
#[serde(deny_unknown_fields)]
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
        let mut ids = BTreeSet::new();
        for spec in &self.sfx {
            let id = spec.sfx_id()?;
            if !ids.insert(id) {
                return Err(format!("duplicate procedural SFX id {id}"));
            }
        }
        Ok(())
    }

    /// The [`SfxId`]s this registry authorizes through its procedural cue
    /// specs (kira-free). A provider that authors a cue authorizes its id, so
    /// provider-relative playback can gate an [`ambition_sfx::SfxMessage`]
    /// without the synth handle table. No cues means no procedural ids.
    pub fn authorized_cue_ids(&self) -> BTreeSet<SfxId> {
        self.sfx
            .iter()
            .filter_map(|spec| spec.sfx_id().ok())
            .collect()
    }

    /// Provider-authored procedural definition for `id`, if this registry owns
    /// it. Playback uses it directly, so a cue cannot render another
    /// provider's sound.
    pub fn spec_for_id(&self, id: SfxId) -> Option<&SfxSpec> {
        self.sfx.iter().find(|spec| spec.sfx_id().ok() == Some(id))
    }
}

impl SoundCueKey {
    /// The stable [`SfxId`] a procedural cue resolves to. Mirrors the
    /// consumer's `SoundCue::sfx_id` table, in the kira-free layer, so
    /// provider authority does not need the playback crate.
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
            Self::Land => ids::PLAYER_LAND,
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
    Land,
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

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SfxSpec {
    /// Compatibility shorthand for the engine's common typed gameplay cues.
    /// Exactly one of `cue` or `id` must be authored.
    #[serde(default)]
    pub cue: Option<SoundCueKey>,
    /// Open provider-local identity for menu, shell, content, and future-game
    /// cues that do not belong in the engine's fixed convenience enum.
    #[serde(default)]
    pub id: Option<String>,
    pub waveform: WaveformSpec,
    pub frequency: f32,
    pub frequency_end: f32,
    pub duration: f32,
    /// Loudness trim in `[0, 1]`: a fraction of the renderer's procedural
    /// reference level, in the RMS domain.
    ///
    /// Not a peak amplitude. Every [`WaveformSpec`] swings ±1, so a peak value
    /// would mean different loudness per waveform and make noisy cues
    /// quieter. As a loudness trim, `0.5` is half the reference level for any
    /// cue, and equal values are equally loud.
    ///
    /// The reference and normalization are in
    /// `crate::render::PROCEDURAL_CUE_REFERENCE_RMS_DBFS`.
    pub volume: f32,
    pub attack: f32,
    pub release: f32,
    pub noise: f32,
}

impl SfxSpec {
    pub fn sfx_id(&self) -> Result<SfxId, String> {
        match (self.cue, self.id.as_deref()) {
            (Some(cue), None) => Ok(cue.sfx_id()),
            (None, Some(id)) if !id.trim().is_empty() => Ok(SfxId::new(id)),
            (None, Some(_)) => Err("procedural SFX id must not be empty".to_owned()),
            (None, None) => Err("procedural SFX must author either cue or id".to_owned()),
            (Some(_), Some(_)) => Err("procedural SFX must not author both cue and id".to_owned()),
        }
    }
}

/// Music-cue registry, authored in `music_registry.ron`.
///
/// Generated by `scripts/regen_music_registry.py` from the rendered-OGG tree
/// (`audio/music/generated/*/full.ogg`), through `scripts/regen/music.sh`.
/// Hand edits are overwritten; change the generator's denylist or display-name
/// map instead. The format is only ids, so it can be generated: the OGG sets
/// the length, and the music director owns looping and crossfade.
#[derive(Clone, Debug, Deserialize, PartialEq, Resource)]
#[serde(deny_unknown_fields)]
pub struct MusicRegistry {
    /// Track id played at startup, or when no radio station is selected.
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
/// `asset_path` is optional; the default is the conventional
/// `audio/music/generated/{id}/full.ogg`. Set it only for off-convention
/// assets (for example an adaptive cue's section mix).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MusicTrack {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub asset_path: Option<String>,
    /// Play once and stop, instead of looping.
    ///
    /// A sting (a death cue, a course-clear fanfare) is written to end, but
    /// the music channel loops by default. Marking the cue uses the length in
    /// the file, instead of each caller stopping it on a timer.
    ///
    /// The tier stays claimed after the sting ends, so silence follows, not
    /// the level theme.
    #[serde(default)]
    pub one_shot: bool,
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

#[cfg(test)]
mod open_sfx_id_tests {
    use super::*;

    fn open_spec(id: &str) -> SfxSpec {
        SfxSpec {
            cue: None,
            id: Some(id.to_owned()),
            waveform: WaveformSpec::Triangle,
            frequency: 440.0,
            frequency_end: 660.0,
            duration: 0.1,
            volume: 0.5,
            attack: 0.0,
            release: 0.02,
            noise: 0.0,
        }
    }

    #[test]
    fn providers_can_author_open_procedural_ids() {
        let registry = SfxRegistry {
            sample_rate: 44_100,
            sfx: vec![open_spec("ui.menu.move")],
        };
        registry.validate().unwrap();
        let id = SfxId::new("ui.menu.move");
        assert!(registry.authorized_cue_ids().contains(&id));
        assert_eq!(registry.spec_for_id(id), registry.sfx.first());
    }

    #[test]
    fn identity_is_unambiguous_and_unique() {
        let both = SfxRegistry {
            sample_rate: 44_100,
            sfx: vec![SfxSpec {
                cue: Some(SoundCueKey::Jump),
                id: Some("also.jump".to_owned()),
                ..open_spec("ignored")
            }],
        };
        assert!(both.validate().unwrap_err().contains("both cue and id"));

        let duplicate = SfxRegistry {
            sample_rate: 44_100,
            sfx: vec![open_spec("same"), open_spec("same")],
        };
        assert!(duplicate.validate().unwrap_err().contains("duplicate"));
    }
}
