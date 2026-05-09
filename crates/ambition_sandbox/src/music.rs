//! Unified music director for room, encounter, and adaptive cue playback.
//!
//! This is the first pass at replacing the special-case generated goblin
//! music path with a generic cue model. Gameplay code should request music by
//! intent: room default, encounter override, and cue state. The director then
//! resolves that into either a simple `AudioLibrary` track or an adaptive cue
//! made of file-backed sections/layers.
//!
//! A simple room track is conceptually a one-section/one-layer cue. In this
//! overlay the legacy procedural room tracks still live in `AudioLibrary`, but
//! selection and priority are owned here rather than split between
//! `audio::apply_encounter_music` and `generated_music.rs`. The adaptive
//! goblin cue is now just data in `MusicCueCatalog`.

#![cfg(feature = "audio")]

use std::collections::HashMap;
use std::time::Duration;

use bevy::ecs::system::SystemParam;
use bevy::log::{debug, info, warn};
use bevy::prelude::*;
use bevy_kira_audio::prelude::{
    AudioChannel, AudioControl, AudioEasing, AudioSource as KiraAudioSource, AudioTween,
};

use crate::audio::{
    amplitude_to_decibels, switch_to_music_track, AudioLibrary, MusicChannel, MusicPlaybackState,
    RadioStationState,
};
use crate::data::SandboxDataSpec;
use crate::encounter::{EncounterMusicRequest, EncounterPhase, EncounterRegistry};
use crate::rooms::RoomMusicRequest;
use crate::settings::UserSettings;

pub const MUSIC_LOG_TARGET: &str = "ambition_music";
const MAX_LAYERS: usize = 6;
const MOB_LAB_ENCOUNTER_ID: &str = "mob_lab";
const FIRST_GOBLIN_CUE_ID: &str = "first_goblin_tune_v2";
const LARGE_BRUTE_DELAY_SECONDS: f32 = 3.5;

/// Relative volume for adaptive cues after user music volume.
///
/// Stacked layers sum hotter than the legacy single-channel procedural room
/// tracks, so keep the per-cue default conservative and let cue states shape
/// individual layer gains.
const ADAPTIVE_MUSIC_RELATIVE_VOLUME: f32 = 1.0;
const STEM_GAIN_BLEND_SECONDS: f32 = 1.05;
const LOOP_SECTION_CROSSFADE_SECONDS: f32 = 1.70;
const INTRO_TO_LOOP_CROSSFADE_SECONDS: f32 = 1.25;
const OUTRO_CROSSFADE_SECONDS: f32 = 1.65;
const DEFAULT_RETURN_OVERLAP_SECONDS: f32 = 1.35;
const MIN_TRANSITION_DELAY_SECONDS: f32 = 0.08;
const LAYER_START_FADE_MS: u64 = 0;
const DEBUG_LOG_PERIOD_SECONDS: f32 = 1.0;

// Two banks of six layer channels. This keeps the current Kira backend simple
// while letting the director crossfade a new section over an old section.

mod catalog;
mod channels;
mod director;
mod first_goblin;
mod state;

#[cfg(test)]
mod tests;

pub use catalog::{
    EncounterMusicBinding, LoadedMusicCueAssets, MusicCueCatalog, MusicCueSpec, MusicLayerGainSpec,
    MusicLayerSourceSpec, MusicLayerSpec, MusicSectionSpec, MusicStateSpec,
};
pub use channels::{
    MusicLayer0AChannel, MusicLayer0BChannel, MusicLayer1AChannel, MusicLayer1BChannel,
    MusicLayer2AChannel, MusicLayer2BChannel, MusicLayer3AChannel, MusicLayer3BChannel,
    MusicLayer4AChannel, MusicLayer4BChannel, MusicLayer5AChannel, MusicLayer5BChannel,
    MusicLayerChannels,
};
pub use director::{drive_music_director, load_music_cues};
pub use state::{MusicDirectorMode, MusicDirectorState};

use catalog::MusicSourceKey;
use channels::{LayerGains, MusicBank};
use state::{AdaptiveCueDirective, PendingMusicStateTransition};

#[cfg(test)]
use director::{resolve_adaptive_directive, resolve_directive_for_binding};
