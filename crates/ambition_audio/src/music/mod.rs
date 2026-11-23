//! Music selection, prepared cue data, and owned native playback.
//! The host supplies a
//! [`crate::mix::MusicMix`] (from its settings) and a [`state::MusicIntent`]
//! (from its game state); this module never reads game state directly.

use std::collections::HashMap;

use bevy::ecs::system::SystemParam;
use bevy::log::{debug, info, warn};
use bevy::prelude::*;
use bevy_kira_audio::prelude::{
    AudioControl, AudioInstance, AudioSource as KiraAudioSource, AudioTween, DynamicAudioChannels,
    PlaybackState,
};

use crate::library::{AudioLibrary, MusicPlaybackState, amplitude_to_decibels};
use crate::mix::MusicMix;

pub const MUSIC_LOG_TARGET: &str = "ambition_music";

const MAX_LAYERS: usize = crate::cue_spec::MAX_MUSIC_LAYERS;

const LOOP_SECTION_CROSSFADE_SECONDS: f32 = 1.70;

/// Intro to first loop should sound continuous, not like a file switch. The
/// source material is level-matched around a ~0.65s seam; shorter overlaps
/// leave a measurable dip before wave1's first bar.
const INTRO_TO_LOOP_CROSSFADE_SECONDS: f32 = 0.65;
const OUTRO_CROSSFADE_SECONDS: f32 = 1.65;

/// Start room/radio music before the adaptive outro finishes so the return to
/// exploration does not leave a silent gap after encounter music fades.
const DEFAULT_RETURN_OVERLAP_SECONDS: f32 = 2.25;
const MIN_TRANSITION_DELAY_SECONDS: f32 = 0.08;
const DEBUG_LOG_PERIOD_SECONDS: f32 = 1.0;

pub mod catalog;
pub mod commands;
pub mod diagnostics;
pub mod director;
pub mod playback;
pub mod state;

pub use catalog::{
    AdaptiveMusicCatalogAppExt, AdaptiveMusicCatalogError, AdaptiveMusicCatalogRegistry,
    BOSS_PHASE_KEYS, BossMusicBinding, BossPhaseState, EncounterMusicBinding, LoadedMusicCueAssets,
    MusicCueCatalog, MusicCueSpec, MusicLayerGainSpec, MusicLayerSourceSpec, MusicLayerSpec,
    MusicSectionSpec, MusicStateBalanceOverride, MusicStateSpec, RoomMusicBinding,
};
pub use commands::MusicPlaybackCommands;
pub use director::{drive_music_director, load_music_cues};
pub use playback::add_music_playback;
use state::PendingMusicStateTransition;
pub use state::{
    AdmittedMusic, MusicChoice, MusicClaim, MusicContinuity, MusicDirectorMode, MusicDirectorState,
    MusicDisposition, MusicEndingResult, MusicEntryPolicy, MusicGoal, MusicGoalPublication,
    MusicIntent, MusicReleaseReason, MusicSelection, MusicSilenceReason,
};

use crate::cue_program::{PreparedMusicCue, PreparedMusicSection, PreparedMusicState};

/// Cancel current music and reset the selection projection.
/// Submitted voice reservations remain owned until their terminal result.
pub fn silence_music_backend(
    layer_channels: &mut MusicPlaybackCommands,
    director: &mut MusicDirectorState,
    music_state: Option<&mut MusicPlaybackState>,
) {
    layer_channels.silence();
    *director = MusicDirectorState::default();
    if let Some(state) = music_state {
        state.silence();
    }
}

/// What the player hears now, named for a "now playing" line: the adaptive
/// cue and its section while a cue plays (a boss or an encounter claims the
/// music over the radio), else the simple track's display name. `None` is
/// silence.
///
/// A cue has no display name of its own; one named like a track uses the
/// track's, and any other is its id in title case (`galvanic_march` is
/// "Galvanic March").
pub fn now_playing_label(
    library: &AudioLibrary,
    playback: &MusicPlaybackState,
    director: Option<&MusicDirectorState>,
) -> Option<String> {
    let cue = director.filter(|d| {
        matches!(
            d.mode,
            MusicDirectorMode::AdaptiveIntro
                | MusicDirectorMode::AdaptiveLoop
                | MusicDirectorMode::AdaptiveOutro
        )
    });
    if let Some((director, cue_id)) = cue.and_then(|d| d.active_cue_id.as_deref().map(|id| (d, id)))
    {
        let name = library
            .track(cue_id)
            .map_or_else(|| title_case(cue_id), |t| t.display_name.clone());
        return Some(match director.current_section_id.as_deref() {
            Some(section) => format!("{name} \u{00b7} {}", title_case(section)),
            None => name,
        });
    }
    let track = playback.active_track();
    (!track.is_empty()).then(|| library.display_name(track).to_owned())
}

fn title_case(id: &str) -> String {
    id.split(['_', '-'])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}
