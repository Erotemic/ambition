//! Procedural audio for Ambition sandbox feedback and music.
//!
//! The sandbox renders procedural sound effects and declarative lo-fi music
//! into in-memory Kira static sound assets at visible startup. Kira owns the
//! playback backend, channels, fades, and looping; the RON data remains the
//! source of truth for cue shapes and music arrangements.

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
use fundsp::audiounit::AudioUnit;
#[cfg(feature = "audio")]
use fundsp::prelude as dsp;
#[cfg(feature = "audio")]
use std::io::Cursor;
#[cfg(feature = "audio")]
use std::sync::Arc;
#[cfg(feature = "audio")]
use std::time::Duration;

use crate::data::SoundCueKey;
#[cfg(feature = "audio")]
use crate::data::{AudioSpec, MusicSpec, MusicTrackSpec, NoteSpec, SfxSpec, WaveformSpec};

mod render;
mod runtime;

#[cfg(all(test, feature = "audio"))]
mod tests;

pub use runtime::{SfxMessage, SoundCue, ORIGINAL_TRACK_ID};

#[cfg(feature = "audio")]
pub use render::{
    render_music_preview, render_music_preview_wav_bytes, wav_bytes_from_rendered_audio,
    RenderedAudio, SfxBankHandleCache,
};
#[cfg(feature = "audio")]
pub use runtime::{
    amplitude_to_decibels, apply_audio_settings, apply_encounter_music, audio_play_sfx_messages,
    set_radio_track, start_default_music, switch_to_music_track, AudioLibrary, MusicChannel,
    MusicPlaybackState, MusicTrackRuntime, RadioStationState, SfxChannel,
};
