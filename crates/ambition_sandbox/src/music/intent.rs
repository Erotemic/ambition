//! Ambition content → neutral music intent.
//!
//! This module is the *game-specific* half of the music split. It reads
//! Ambition gameplay resources (encounter phases, boss/encounter/room track
//! requests, the radio station, the sandbox default-track config) and resolves
//! them into the content-agnostic [`MusicIntent`] that the director consumes.
//!
//! The director ([`super::director`]) names no encounter, boss, room, or track
//! and imports none of these gameplay modules — it only looks up cue/state and
//! track ids that *this* module decided on. That inversion is the reusable-crate
//! seam: drop the director into another game and supply a different
//! `compute_music_intent` system.

use bevy::prelude::*;

use crate::audio::RadioStationState;
use crate::encounter::{
    BossEncounterMusicRequest, EncounterMusicRequest, EncounterPhase, EncounterRegistry,
};
use crate::rooms::RoomMusicRequest;
use crate::runtime::data::SandboxDataSpec;

use super::catalog::{EncounterMusicBinding, MusicCueCatalog};
use super::state::{AdaptiveCueDirective, MusicDirectorMode, MusicDirectorState, MusicIntent};

/// Delay after wave 2 starts before the music promotes to its "reinforced"
/// (large-brute) state. Content tuning — owned here, not by the director.
pub(super) const LARGE_BRUTE_DELAY_SECONDS: f32 = 3.5;

/// Resolve this frame's [`MusicIntent`] from Ambition gameplay state.
///
/// Runs before [`super::drive_music_director`] each frame and is the only
/// system that bridges encounter/room/content gameplay into the music layer.
#[allow(clippy::too_many_arguments)]
pub fn compute_music_intent(
    catalog: Option<Res<MusicCueCatalog>>,
    director: Option<Res<MusicDirectorState>>,
    encounters: Res<EncounterRegistry>,
    mut encounter_music: ResMut<EncounterMusicRequest>,
    mut boss_music: ResMut<BossEncounterMusicRequest>,
    room_music: Res<RoomMusicRequest>,
    radio: Option<Res<RadioStationState>>,
    sandbox_data: Res<SandboxDataSpec>,
    mut intent: ResMut<MusicIntent>,
) {
    let adaptive = match (catalog.as_ref(), director.as_ref()) {
        (Some(catalog), Some(director)) => {
            resolve_adaptive_directive(catalog, &encounters, director)
        }
        _ => None,
    };

    let candidates = simple_track_candidates(
        &room_music,
        radio.as_deref(),
        &sandbox_data,
        &encounter_music,
        &boss_music,
    );

    // Mirror the resolved priority winner back into the request resources'
    // `last_applied` so the (currently unscheduled) `apply_encounter_music`
    // fallback stays consistent if it is ever re-enabled. The director itself
    // never touches these gameplay resources.
    if let Some(top) = candidates.first().cloned() {
        encounter_music.last_applied = Some(top.clone());
        boss_music.last_applied = Some(top);
    }

    intent.adaptive = adaptive;
    intent.simple_track_candidates = candidates;
}

/// Build the simple-track priority list. Priority: boss-encounter music >
/// regular encounter music > radio > room default > sandbox default. The
/// director plays the first id that exists in its `AudioLibrary`, so this
/// stays a pure list of candidate ids (no audio backend access here).
fn simple_track_candidates(
    room_music: &RoomMusicRequest,
    radio: Option<&RadioStationState>,
    sandbox_data: &SandboxDataSpec,
    encounter_music: &EncounterMusicRequest,
    boss_music: &BossEncounterMusicRequest,
) -> Vec<String> {
    let mut candidates = Vec::new();
    if let Some(track) = &boss_music.desired_track {
        candidates.push(track.clone());
    }
    if let Some(track) = &encounter_music.desired_track {
        candidates.push(track.clone());
    }
    if let Some(track) = radio.and_then(|radio| radio.selected_track()) {
        candidates.push(track.to_string());
    }
    if let Some(track) = &room_music.desired_track {
        candidates.push(track.clone());
    }
    candidates.push(sandbox_data.audio.default_music_track.clone());
    candidates
}

/// Iterate the catalog's encounter bindings and return the first live
/// adaptive directive. (Iterating bindings rather than hardcoding an
/// encounter id lets future boss / mini-boss cues drop in by adding a
/// binding.)
pub(super) fn resolve_adaptive_directive(
    catalog: &MusicCueCatalog,
    encounters: &EncounterRegistry,
    director: &MusicDirectorState,
) -> Option<AdaptiveCueDirective> {
    for binding in &catalog.encounter_bindings {
        if let Some(directive) = resolve_directive_for_binding(binding, encounters, director) {
            return Some(directive);
        }
    }
    None
}

/// Resolve a single encounter binding's adaptive cue directive.
/// Returns:
/// - `Some(StopNow)` when the encounter no longer exists but the
///   binding's cue is still active.
/// - `Some(Play { cue_id, state_id })` when the encounter is in a
///   phase mapped to a cue state.
/// - `None` when the encounter is unknown / inactive AND its cue is
///   not playing — the binding doesn't claim audio this frame.
pub(super) fn resolve_directive_for_binding(
    binding: &EncounterMusicBinding,
    encounters: &EncounterRegistry,
    director: &MusicDirectorState,
) -> Option<AdaptiveCueDirective> {
    let cue_active = director.active_cue_id.as_deref() == Some(binding.cue_id.as_str());
    let Some(encounter) = encounters.get(&binding.encounter_id) else {
        if cue_active {
            return Some(AdaptiveCueDirective::StopNow);
        }
        return None;
    };

    match encounter.phase {
        EncounterPhase::Starting { .. } => Some(AdaptiveCueDirective::Play {
            cue_id: binding.cue_id.clone(),
            state_id: binding.starting_state.clone(),
        }),
        EncounterPhase::Active { wave_index, .. } => {
            let mut state_id = binding
                .wave_states
                .get(wave_index)
                .or_else(|| binding.wave_states.last())
                .cloned()
                .unwrap_or_else(|| binding.starting_state.clone());
            if wave_index == 1 && encounter.run.wave_elapsed >= LARGE_BRUTE_DELAY_SECONDS {
                if let Some(reinforced) = &binding.wave2_reinforced_state {
                    state_id = reinforced.clone();
                }
            }
            Some(AdaptiveCueDirective::Play {
                cue_id: binding.cue_id.clone(),
                state_id,
            })
        }
        EncounterPhase::Cleared => Some(AdaptiveCueDirective::Play {
            cue_id: binding.cue_id.clone(),
            state_id: binding.cleared_state.clone(),
        }),
        EncounterPhase::Inactive => {
            // The encounter often resets to Inactive immediately after clear;
            // if this adaptive cue is already active, continue into its outro
            // instead of hard-cutting to room music.
            if cue_active
                && director.mode != MusicDirectorMode::AdaptiveFinished
                && director.mode != MusicDirectorMode::Idle
            {
                Some(AdaptiveCueDirective::Play {
                    cue_id: binding.cue_id.clone(),
                    state_id: binding.cleared_state.clone(),
                })
            } else {
                None
            }
        }
        EncounterPhase::Failed => Some(AdaptiveCueDirective::StopNow),
    }
}
