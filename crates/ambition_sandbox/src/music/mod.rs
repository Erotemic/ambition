//! Sandbox music adapters over the `ambition_audio` music core.
//!
//! Game-side music glue: [`intent`] (encounter / room / radio state ->
//! [`MusicIntent`]), authored goblin cue data, and settings ->
//! [`ambition_audio::MusicMix`] sync.

#![cfg(feature = "audio")]

use bevy::prelude::*;

use crate::persistence::settings::UserSettings;

mod first_goblin;
mod intent;

#[cfg(test)]
mod tests;

#[cfg(test)]
use intent::{
    resolve_adaptive_directive, resolve_directive_for_binding, LARGE_BRUTE_DELAY_SECONDS,
};

#[cfg(test)]
pub(crate) use ambition_audio::music::EncounterMusicBinding;
pub use ambition_audio::music::{
    drive_music_director, load_music_cues, AdaptiveCueDirective, LoadedMusicCueAssets,
    MusicCueCatalog, MusicCueSpec, MusicDirectorMode, MusicDirectorState, MusicIntent,
    MusicLayer0AChannel, MusicLayer0BChannel, MusicLayer1AChannel, MusicLayer1BChannel,
    MusicLayer2AChannel, MusicLayer2BChannel, MusicLayer3AChannel, MusicLayer3BChannel,
    MusicLayer4AChannel, MusicLayer4BChannel, MusicLayer5AChannel, MusicLayer5BChannel,
    MusicLayerChannels, MusicLayerGainSpec, MusicLayerSourceSpec, MusicLayerSpec, MusicSectionSpec,
    MusicStateSpec, MUSIC_LOG_TARGET,
};
pub use first_goblin::{
    ambition_music_cue_catalog, first_goblin_tune_v2_spec, FIRST_GOBLIN_CUE_ID,
    MOB_LAB_ENCOUNTER_ID,
};
pub use intent::compute_music_intent;

/// Mirror the user's effective music volume into the audio crate's
/// [`ambition_audio::MusicMix`] each frame, BEFORE the director runs,
/// so the reusable music core never reads the sandbox settings model.
pub fn sync_music_mix(settings: Res<UserSettings>, mut mix: ResMut<ambition_audio::MusicMix>) {
    mix.effective_music = settings.audio.effective_music();
}
