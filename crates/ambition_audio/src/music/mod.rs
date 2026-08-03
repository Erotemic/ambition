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
    AdaptiveMusicCatalogAppExt, AdaptiveMusicCatalogError, AdaptiveMusicCatalogRegistry,
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
    music_state.silence();
}

/// Does the base track SURVIVE this change of audio context?
///
/// ⭐ **The same song, chosen by a different owner, is still the same song.**
/// Frontend audio is keyed by the ACTIVATION that selected it, so walking from
/// the startup cards to the launcher is a new `Frontend(_)` owner — and every
/// context-change path then stopped the channel and started the identical title
/// track again from zero, because `FrontendAudioProfile` names one title theme
/// for the whole provider. The audible result was the title music restarting on
/// the handoff, which nobody had declared and nobody wanted. (Jon, 2026-08-03.)
///
/// Arbitrating by IDENTITY rather than by owner needs no route to name another
/// route: a screen declares its own track, and playing a track that is already
/// playing is a no-op. A per-route "continue what the last one played" flag
/// would make continuity a property of the PAIR of screens, which is the
/// coupling that makes adding a third screen a question.
///
/// ⚠ **frontend to frontend only, deliberately.** A gameplay session handing
/// back to a title screen must still stop and reset even when the base track
/// happens to match: what it is carrying — adaptive layers, a director mid-cue,
/// a room request — is exactly what the title screen is not, and only the reset
/// path clears it.
///
/// ⚠ this predicate lives HERE, next to the silencer, because two separate
/// systems perform this reset (the context-change reset and the frontend policy
/// application). One rule, applied at both, rather than the same condition
/// written twice and drifting.
/// ⛔ **it is NOT a comparison of owners, and the first two attempts were.**
/// Measured with the play counter rather than reasoned about: one handoff from
/// the startup cards to the launcher produced SIX generations — silence,
/// silence, play, silence, silence, play — because the audio owner passes
/// through `None` between activations. Any rule of the form "the previous owner
/// and the next owner are both frontends" is false at exactly the moment it is
/// asked. The stable fact is the PROFILE: `FrontendAudioProfile` is composed
/// once for the whole host and does not blink out between two of its screens.
pub fn title_theme_keeps_playing(
    title_track: Option<&str>,
    incoming_owner: Option<ambition_sfx::AudioContextOwner>,
    music_state: &MusicPlaybackState,
) -> bool {
    if matches!(
        incoming_owner,
        Some(ambition_sfx::AudioContextOwner::Gameplay(_))
    ) {
        return false;
    }
    match title_track {
        Some(track) => !track.is_empty() && music_state.active_track() == track,
        None => false,
    }
}
