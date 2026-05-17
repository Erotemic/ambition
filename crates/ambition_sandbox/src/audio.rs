//! Audio runtime for the Ambition sandbox.
//!
//! All audio playback in the sandbox is **authored**: pre-rendered OGG
//! music tracks loaded through the asset manager catalog, and SFX served
//! from the packed `.sfxbank` (also catalog-routed). Kira owns the
//! backend, channels, fades, and looping. The old runtime fundsp
//! procedural music generator + SFX synthesizer was retired; see
//! `docs/fundsp_audio.md` for the historical note.
//!
//! Realtime DSP/effects (underwater muffle, low-pass filtering,
//! reverb) live in [`environment`]. Today only a Kira-friendly
//! channel-attenuation fallback is wired up because `bevy_kira_audio`
//! 0.25 does not expose track-level effect insertion or the
//! underlying `kira::AudioManager`; search for
//! `TODO: kira_underwater_filter_backend` for the exact swap points.
//! Reverb / heavier coloration remains future work.

use ambition_engine as ae;
use ambition_sfx::SfxId;
#[cfg(feature = "audio")]
use ambition_sfx::{self as sfx, SfxProvider};
#[cfg(feature = "audio")]
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
#[cfg(feature = "audio")]
use bevy_kira_audio::prelude::{
    AudioChannel, AudioControl, AudioEasing, AudioSource as KiraAudioSource, AudioTween, Frame,
    StaticSoundData, StaticSoundSettings,
};
#[cfg(feature = "audio")]
use std::io::Cursor;
#[cfg(feature = "audio")]
use std::sync::Arc;
#[cfg(feature = "audio")]
use std::time::Duration;

use crate::data::SoundCueKey;
#[cfg(feature = "audio")]
use crate::data::AudioSpec;

#[cfg(feature = "audio")]
mod bank_asset;
pub mod environment;
mod render;
mod runtime;
#[cfg(feature = "audio")]
mod web_unlock;

#[cfg(all(test, feature = "audio"))]
mod tests;

pub use environment::{AudioEnvironment, AudioEnvironmentMode};
pub use runtime::{SfxMessage, SoundCue, ORIGINAL_TRACK_ID};

#[cfg(feature = "audio")]
pub use bank_asset::{SfxBankAsset, SfxBankAssetPlugin};
#[cfg(feature = "audio")]
pub use environment::{
    apply_audio_environment, detect_audio_environment, smooth_audio_environment,
};
#[cfg(feature = "audio")]
pub use render::SfxBankHandleCache;
#[cfg(feature = "audio")]
pub use runtime::{
    amplitude_to_decibels, apply_encounter_music, audio_play_sfx_messages, set_radio_track,
    start_default_music_when_ready, switch_to_music_track, AudioLibrary, DefaultMusicStarted,
    MusicChannel, MusicPlaybackState, MusicTrackRuntime, RadioStationState, SfxChannel,
};
#[cfg(feature = "audio")]
pub use web_unlock::{AudioUnlockState, WebAudioUnlockPlugin};
