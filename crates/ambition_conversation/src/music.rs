//! **A track a conversation asked for**, and the room that bounds it.
//!
//! ⛔ **presentation-side ON PURPOSE, and that is the whole design decision.**
//! Every other gameplay-bearing Yarn command routes through
//! [`crate::ledger::NarrativeInputLedger`] because it writes rollback state from
//! `Update`, where a rewind would erase it. Music is not rollback state: nothing
//! in the simulation branches on which track is playing, and rewinding the
//! soundtrack would stutter it for the same reason rewinding the dialogue box
//! would. So this is its own channel, like `play_sfx`.
//!
//! ⚠ **it is bounded by the ROOM, not by the conversation.** A dialogue-claimed
//! track outlives the box that claimed it — that is the point, since the fight
//! it scores starts after the box closes — but a claim that also outlived the
//! ROOM would follow the player into unrelated rooms with no way to stop it.

use bevy::prelude::Resource;

/// The track a conversation asked to hear, if any.
///
/// Outranked by an encounter's own music (a live fight scores itself), and
/// outranks the radio and the room default.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct NarrativeMusicRequest {
    track: Option<String>,
}

impl NarrativeMusicRequest {
    pub fn track(&self) -> Option<&str> {
        self.track.as_deref()
    }

    /// Ask for `track`. An empty id clears the request, so authored content can
    /// hand the room its own music back without a second command.
    pub fn request(&mut self, track: &str) {
        self.track = (!track.is_empty()).then(|| track.to_string());
    }

    pub fn clear(&mut self) {
        self.track = None;
    }
}
