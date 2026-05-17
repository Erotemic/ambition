//! Audio runtime for the Ambition sandbox.
//!
//! All audio playback in the sandbox is **authored**: pre-rendered OGG
//! music tracks loaded through the asset manager catalog, and SFX served
//! from the packed `.sfxbank` (also catalog-routed). Kira owns the
//! backend, channels, fades, and looping. The old runtime fundsp
//! procedural music generator + SFX synthesizer was retired; see
//! `docs/fundsp_audio.md` for the historical note.
//!
//! Realtime DSP/effects (underwater muffle, low-pass filtering, reverb)
//! is not implemented today. Future effect work should land behind an
//! optional `audio_fx` feature gate alongside this module.

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
mod render;
mod runtime;
#[cfg(feature = "audio")]
mod web_unlock;

#[cfg(all(test, feature = "audio"))]
mod tests;

pub use runtime::{SfxMessage, SoundCue, ORIGINAL_TRACK_ID};

#[cfg(feature = "audio")]
pub use bank_asset::{SfxBankAsset, SfxBankAssetPlugin};
#[cfg(feature = "audio")]
pub use render::SfxBankHandleCache;
#[cfg(feature = "audio")]
pub use runtime::{
    amplitude_to_decibels, apply_audio_settings, apply_encounter_music, audio_play_sfx_messages,
    set_radio_track, start_default_music, switch_to_music_track, AudioLibrary, MusicChannel,
    MusicPlaybackState, MusicTrackRuntime, RadioStationState, SfxChannel,
};
#[cfg(feature = "audio")]
pub use web_unlock::WebAudioUnlockPlugin;
