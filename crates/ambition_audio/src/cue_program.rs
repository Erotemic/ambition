//! Validate authored music and resolve its sources, indices, and layer gains.

use std::collections::BTreeMap;

use bevy::prelude::Resource;

use crate::cue_spec::*;
use crate::music_media::{MusicMediaCatalog, PreparedMusicClock};

mod media;
mod references;
mod validation;
use validation::validation_errors;

pub const DAY_PHASE_KEYS: [&str; 4] = ["morning", "day", "evening", "night"];

/// A catalog can enter playback only through preparation.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct PreparedMusicCues {
    cues: BTreeMap<String, PreparedMusicCue>,
    encounter_bindings: Vec<EncounterMusicBinding>,
    room_bindings: Vec<RoomMusicBinding>,
    boss_bindings: Vec<BossMusicBinding>,
    room_phase_tracks: Vec<RoomPhaseTrack>,
    encounter_index: BTreeMap<String, usize>,
    boss_index: BTreeMap<String, usize>,
    room_index: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreparedMusicCue {
    pub id: String,
    pub media_identity: Option<PreparedMusicMediaIdentity>,
    pub sections: Vec<PreparedMusicSection>,
    pub states: Vec<PreparedMusicState>,
    pub outro_state: Option<String>,
    pub post_clear_bridge_state: Option<String>,
    section_index: BTreeMap<String, usize>,
    state_index: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreparedMusicMediaIdentity {
    pub cue_id: String,
    pub description_path: String,
    pub compiled_score_fingerprint: String,
    pub render_fingerprint: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreparedMusicSection {
    pub id: String,
    pub duration_seconds: f32,
    pub file_duration_seconds: Option<f64>,
    pub clock: PreparedMusicClock,
    pub markers: Vec<crate::music_media::MusicFrameMarker>,
    pub retained_material: Option<crate::music_media::MusicRetainedMaterial>,
    pub looped: bool,
    pub sources: Vec<PreparedMusicSource>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreparedMusicSource {
    pub layer_id: String,
    pub slot: usize,
    pub path: String,
    pub product: Option<crate::music_media::MusicEncodedProduct>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreparedMusicState {
    pub id: String,
    pub section_id: String,
    pub next_state: Option<String>,
    /// Includes the cue volume and any authored balance override.
    pub gains: [f32; MAX_MUSIC_LAYERS],
}

impl PreparedMusicCue {
    pub fn section(&self, id: &str) -> Option<&PreparedMusicSection> {
        self.section_index
            .get(id)
            .map(|index| &self.sections[*index])
    }

    pub fn state(&self, id: &str) -> Option<&PreparedMusicState> {
        self.state_index.get(id).map(|index| &self.states[*index])
    }
}

impl PreparedMusicCues {
    pub fn prepare(
        authored: AuthoredMusicCues,
        media: &MusicMediaCatalog,
    ) -> Result<Self, Vec<String>> {
        let mut errors = Vec::new();
        let mut linked = Vec::new();
        for cue in &authored.cues {
            match media::link(cue, media) {
                Ok(cue) => linked.push(cue),
                Err(error) => errors.push(format!("cue {:?}: {error}", cue.id)),
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        errors.extend(validation_errors(&authored, &linked));
        if !errors.is_empty() {
            return Err(errors);
        }
        let encounter_index = index(authored.encounter_bindings.iter().map(|b| &b.encounter_id));
        let room_index = index(authored.room_bindings.iter().map(|b| &b.room_id));
        let boss_index = index(authored.boss_bindings.iter().map(|b| &b.boss_id));
        Ok(Self {
            cues: linked
                .into_iter()
                .map(|cue| {
                    let cue = prepare_cue(cue);
                    (cue.id.clone(), cue)
                })
                .collect(),
            encounter_bindings: authored.encounter_bindings,
            room_bindings: authored.room_bindings,
            boss_bindings: authored.boss_bindings,
            room_phase_tracks: authored.room_phase_tracks,
            encounter_index,
            boss_index,
            room_index,
        })
    }

    pub fn cue(&self, id: &str) -> Option<&PreparedMusicCue> {
        self.cues.get(id)
    }
    pub fn cue_ids(&self) -> impl Iterator<Item = &str> {
        self.cues.keys().map(String::as_str)
    }
    pub fn cues(&self) -> impl Iterator<Item = &PreparedMusicCue> {
        self.cues.values()
    }
    pub fn encounter_bindings(&self) -> &[EncounterMusicBinding] {
        &self.encounter_bindings
    }
    pub fn room_bindings(&self) -> &[RoomMusicBinding] {
        &self.room_bindings
    }
    pub fn boss_bindings(&self) -> &[BossMusicBinding] {
        &self.boss_bindings
    }
    pub fn room_phase_tracks(&self) -> &[RoomPhaseTrack] {
        &self.room_phase_tracks
    }
    pub fn binding_for_encounter(&self, id: &str) -> Option<&EncounterMusicBinding> {
        self.encounter_index
            .get(id)
            .map(|i| &self.encounter_bindings[*i])
    }
    pub fn binding_for_room(&self, id: &str) -> Option<&RoomMusicBinding> {
        self.room_index.get(id).map(|i| &self.room_bindings[*i])
    }
    pub fn binding_for_boss(&self, id: &str) -> Option<&BossMusicBinding> {
        self.boss_index.get(id).map(|i| &self.boss_bindings[*i])
    }
    pub fn is_outro_state(&self, cue: &str, state: &str) -> bool {
        self.cue(cue)
            .is_some_and(|cue| cue.outro_state.as_deref() == Some(state))
    }
    pub fn room_phase_track(&self, room: &str, phase: &str) -> Option<&str> {
        self.room_phase_tracks
            .iter()
            .find(|row| row.room_id == room && row.phase == phase)
            .map(|row| row.track.as_str())
    }
    pub fn room_follows_the_day(&self, room: &str) -> bool {
        self.room_phase_tracks.iter().any(|row| row.room_id == room)
    }
}

fn index<'a>(ids: impl Iterator<Item = &'a String>) -> BTreeMap<String, usize> {
    ids.enumerate().map(|(i, id)| (id.clone(), i)).collect()
}

fn prepare_cue(linked: media::LinkedMusicCue) -> PreparedMusicCue {
    let cue = linked.spec;
    let slots = linked.slots;
    let section_index = index(linked.sections.iter().map(|section| &section.id));
    let state_index = index(cue.states.iter().map(|state| &state.id));
    let states = cue
        .states
        .iter()
        .map(|state| {
            let mut gains = [0.0; MAX_MUSIC_LAYERS];
            for gain in &state.gains {
                gains[slots[gain.layer_id.as_str()]] = gain.gain * cue.relative_volume;
            }
            // A mastered full mix retains the renderer's balance.
            if !linked.mastered_full {
                if let Some(balance) = cue
                    .runtime_balance_overrides
                    .iter()
                    .find(|balance| balance.state_id == state.id)
                {
                    for (layer, gain) in &balance.layer_gains {
                        gains[slots[layer.as_str()]] = gain * cue.relative_volume;
                    }
                }
            }
            PreparedMusicState {
                id: state.id.clone(),
                section_id: state.section_id.clone(),
                next_state: state.next_state.clone(),
                gains,
            }
        })
        .collect();
    PreparedMusicCue {
        id: cue.id.clone(),
        media_identity: linked.identity,
        sections: linked.sections,
        states,
        outro_state: cue.outro_state.clone(),
        post_clear_bridge_state: cue.post_clear_bridge_state.clone(),
        section_index,
        state_index,
    }
}

#[cfg(test)]
mod tests;
