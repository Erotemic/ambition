//! Conversation-selected music is presentation state scoped to the current room.
//!
//! It bypasses [`crate::ledger::NarrativeInputLedger`] because simulation state
//! never depends on the soundtrack. A request may outlive the dialogue that
//! created it, but room transitions clear it.

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
