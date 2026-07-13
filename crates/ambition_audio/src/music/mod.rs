//! Adaptive music core: cue catalog, layered Kira channels, the
//! director (simple + adaptive cue playback), and its tuning. The
//! HOST supplies a [`crate::mix::MusicMix`] (synced from its settings)
//! and a [`state::MusicIntent`] (mapped from its game state) —
//! this module never reads game state directly.

use std::collections::HashMap;
use std::time::Duration;

use bevy::ecs::system::SystemParam;
use bevy::log::{debug, info, warn};
use bevy::prelude::*;
use bevy_kira_audio::prelude::{
    AudioChannel, AudioControl, AudioEasing, AudioSource as KiraAudioSource, AudioTween,
};

use crate::library::{
    amplitude_to_decibels, switch_to_music_track, AudioLibrary, MusicChannel, MusicPlaybackState,
};
use crate::mix::MusicMix;

pub const MUSIC_LOG_TARGET: &str = "ambition_music";
const MAX_LAYERS: usize = 6;

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

pub mod catalog;
pub mod channels;
pub mod director;
pub mod state;

pub use catalog::{
    EncounterMusicBinding, LoadedMusicCueAssets, MusicCueCatalog, MusicCueSpec, MusicLayerGainSpec,
    MusicLayerSourceSpec, MusicLayerSpec, MusicSectionSpec, MusicStateBalanceOverride,
    MusicStateSpec,
};
pub use channels::{
    MusicLayer0AChannel, MusicLayer0BChannel, MusicLayer1AChannel, MusicLayer1BChannel,
    MusicLayer2AChannel, MusicLayer2BChannel, MusicLayer3AChannel, MusicLayer3BChannel,
    MusicLayer4AChannel, MusicLayer4BChannel, MusicLayer5AChannel, MusicLayer5BChannel,
    MusicLayerChannels,
};
pub use director::{drive_music_director, load_music_cues};
use state::PendingMusicStateTransition;
pub use state::{AdaptiveCueDirective, MusicDirectorMode, MusicDirectorState, MusicIntent};

use channels::{LayerGains, MusicBank};

/// Minimal session-music driver for hosts WITHOUT the full adaptive director
/// (demo hosts, multi-game launchers): whenever the active audio selection
/// changes, stop playback and start the newly selected provider's default
/// track, looped. No selection — or a provider with no authored music — is
/// deliberate silence.
///
/// Provider-agnostic by construction: it reads only
/// [`crate::selection::ActiveAudioSelection`], so a new game's music plays
/// with zero host edits once its fragment is registered and its session
/// activates. Hosts running the full director do NOT add this system — the
/// director owns playback there.
pub fn drive_selected_session_music(
    selection: Res<crate::selection::ActiveAudioSelection>,
    asset_server: Res<AssetServer>,
    audio: Res<bevy_kira_audio::prelude::Audio>,
) {
    if !selection.is_changed() {
        return;
    }
    audio.stop();
    if let Some(music) = selection.music() {
        if let Some(track) = music.track(&music.default_track) {
            audio
                .play(asset_server.load(track.resolved_asset_path()))
                .looped();
        }
    }
}

/// Hard-stop all music playback and reset the director to its idle state.
///
/// Stops the base [`MusicChannel`] and every adaptive layer channel, resets the
/// [`MusicDirectorState`] to `Default` (mode `Idle`, no active cue, no
/// last-simple track), and clears the [`MusicPlaybackState`] active track. Used
/// by a host to enforce deterministic silence when leaving gameplay for a
/// frontend/title route — cached assets stay resident, but nothing is playing
/// and no stale director state can resurrect a previous session's music.
pub fn silence_music_backend(
    base_music_channel: &AudioChannel<MusicChannel>,
    layer_channels: &MusicLayerChannels,
    director: &mut MusicDirectorState,
    music_state: &mut MusicPlaybackState,
) {
    base_music_channel.stop();
    layer_channels.stop_all(0);
    *director = MusicDirectorState::default();
    music_state.active_track.clear();
}
