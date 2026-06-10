//! Audio runtime for the Ambition sandbox.
//!
//! All audio playback in the sandbox is **authored**: pre-rendered OGG
//! music tracks loaded through the asset manager catalog, and SFX served
//! from the packed `.sfxbank` (also catalog-routed). Kira owns the
//! backend, channels, fades, and looping. The old runtime fundsp
//! procedural music generator + SFX synthesizer was retired; see
//! `docs/archive/retired/fundsp-audio.md` for the historical note.
//!
//! Realtime DSP/effects (underwater muffle, low-pass filtering,
//! reverb) live in [`environment`]. Today only a Kira-friendly
//! channel-attenuation fallback is wired up because `bevy_kira_audio`
//! 0.25 does not expose track-level effect insertion or the
//! underlying `kira::AudioManager`; search for
//! `TODO: kira_underwater_filter_backend` for the exact swap points.
//! Reverb / heavier coloration remains future work.

use crate::engine_core as ae;
// `SfxMessage` now lives in the `ambition_sfx` crate (moved down so
// reusable mechanics request sound without naming a sandbox module).
// Re-export it here so the historical `crate::audio::SfxMessage` path
// — used across the sandbox and by the audio runtime consumer below —
// keeps resolving unchanged.
use ambition_sfx::SfxId;
pub use ambition_sfx::SfxMessage;
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

#[cfg(feature = "audio")]
use crate::runtime::data::AudioSpec;
use crate::runtime::data::SoundCueKey;

#[cfg(feature = "audio")]
mod bank_asset;
pub mod environment;
#[cfg(feature = "audio")]
mod plugin;
mod runtime;

#[cfg(all(test, feature = "audio"))]
mod tests;

pub use environment::{AudioEnvironment, AudioEnvironmentMode};
// SoundCue / SfxMessageCue / ORIGINAL_TRACK_ID live in `ambition_audio`
// (Kira-gated); headless paths use `SoundCueKey` from the data schema.

#[cfg(feature = "audio")]
pub use bank_asset::{SfxBankAsset, SfxBankAssetPlugin};
#[cfg(feature = "audio")]
pub use environment::{
    apply_audio_environment, detect_audio_environment, smooth_audio_environment,
};
#[cfg(feature = "audio")]
pub use plugin::SandboxAudioPlugin;
// The playback library + render cache + web unlock moved to the
// `ambition_audio` crate (Stage 20 / B1); re-exported so historical
// `crate::audio::…` paths keep resolving.
#[cfg(feature = "audio")]
pub use ambition_audio::library::{
    amplitude_to_decibels, set_radio_track, start_default_music_when_ready, switch_to_music_track,
    AudioLibrary, DefaultMusicStarted, MusicChannel, MusicPlaybackState, MusicTrackRuntime,
    RadioStationState, SfxChannel, SfxMessageCue, SoundCue, ORIGINAL_TRACK_ID,
};
#[cfg(feature = "audio")]
pub use ambition_audio::render::SfxBankHandleCache;
#[cfg(feature = "audio")]
pub use ambition_audio::web_unlock::{AudioUnlockState, WebAudioUnlockPlugin};
#[cfg(feature = "audio")]
pub use runtime::{apply_encounter_music, audio_play_sfx_messages};
