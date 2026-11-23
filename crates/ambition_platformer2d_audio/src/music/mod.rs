//! Ambition-game music adapters over the `ambition_audio` music core.
//!
//! Game facts select the neutral [`ambition_audio::music::MusicIntent`].




mod intent;

#[cfg(test)]
mod tests;

#[cfg(test)]
use intent::{
    boss_phase_key, resolve_adaptive_directive, resolve_boss_directive, resolve_directive_for_binding,
    resolve_room_directive, LARGE_BRUTE_DELAY_SECONDS,
};
#[cfg(test)]
use intent::{boss_claim, encounter_claim, room_claim};

#[cfg(test)]
pub(crate) use ambition_audio::music::EncounterMusicBinding;
pub use intent::{compute_music_intent, release_narrative_music_on_room_change};
