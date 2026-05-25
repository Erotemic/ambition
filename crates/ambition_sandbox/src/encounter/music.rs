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

/// Higher-priority music request from the boss-encounter system.
/// Boss encounters phase through Intro → Phase1 → Transition → Phase2 →
/// Enrage and each transition writes a `MusicRequested` event that
/// publishes here. This is separate from `EncounterMusicRequest`
/// because the regular encounter tick (see
/// `encounter/systems.rs`) unconditionally writes `desired_track`
/// every frame — including writing `None` when no regular
/// encounter is in flight. Without this split, the regular
/// encounter would clobber boss music to `None` on the very next
/// frame after the boss-encounter set its track, causing the
/// audio backend to flip back to the room default.
///
/// The audio backend (`apply_encounter_music`) prefers
/// `BossEncounterMusicRequest.desired_track` over
/// `EncounterMusicRequest.desired_track` over the room default.
#[derive(Resource, Default, Debug, Clone)]
pub struct BossEncounterMusicRequest {
    pub desired_track: Option<String>,
    pub last_applied: Option<String>,
}
