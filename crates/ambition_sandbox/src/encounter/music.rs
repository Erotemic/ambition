use bevy::prelude::Resource;

/// Music request from the encounter system to the audio backend.
/// The encounter writes `desired_track` (Some(track_id) while an
/// encounter is in flight, None when default music should resume);
/// the audio-feature-gated `apply_encounter_music` system in
/// `audio.rs` swaps the music channel only when the desired track
/// changes.
#[derive(Resource, Default, Debug, Clone)]
pub struct EncounterMusicRequest {
    pub desired_track: Option<String>,
    /// The track id we last applied so we can detect transitions
    /// (None ↔ Some(other) ↔ Some(other2)).
    pub last_applied: Option<String>,
}
