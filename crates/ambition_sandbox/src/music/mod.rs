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
use crate::content::data::SandboxDataSpec;
use crate::encounter::{
    BossEncounterMusicRequest, EncounterMusicRequest, EncounterPhase, EncounterRegistry,
};
use crate::persistence::settings::UserSettings;
use crate::rooms::RoomMusicRequest;

pub const MUSIC_LOG_TARGET: &str = "ambition_music";
const MAX_LAYERS: usize = 6;
const MOB_LAB_ENCOUNTER_ID: &str = "goblin_encounter";
const FIRST_GOBLIN_CUE_ID: &str = "first_goblin_tune_v2";
const LARGE_BRUTE_DELAY_SECONDS: f32 = 3.5;

/// Relative volume for adaptive cues after user music volume.
///
/// Stacked layers sum hotter than the legacy single-channel procedural room
/// tracks, so keep the per-cue default conservative and let cue states shape
/// individual layer gains.
const ADAPTIVE_MUSIC_RELATIVE_VOLUME: f32 = 1.0;

/// Runtime gain smoothing for adaptive layer targets.
///
/// Keep this short enough that an intro-to-wave handoff reads as one continuous
/// cue instead of "intro ended, then another track faded in." Long musical
/// overlap is still controlled by the section crossfade constants below.
const STEM_GAIN_BLEND_SECONDS: f32 = 0.18;
const LOOP_SECTION_CROSSFADE_SECONDS: f32 = 1.70;

/// Intro -> first loop should feel like a continuous handoff rather than a
/// hard file switch. Transition-lab audits showed that the source material is
/// level-matched around a ~0.65s seam; shorter overlaps leave a measurable dip
/// before wave1 establishes its first-bar bed.
const INTRO_TO_LOOP_CROSSFADE_SECONDS: f32 = 0.65;
const OUTRO_CROSSFADE_SECONDS: f32 = 1.65;

/// Start room/radio music before the adaptive outro finishes so the return to
/// exploration does not leave a silent gap after encounter music fades.
const DEFAULT_RETURN_OVERLAP_SECONDS: f32 = 2.25;
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
