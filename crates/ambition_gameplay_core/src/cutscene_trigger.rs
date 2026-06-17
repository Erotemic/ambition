//! The cutscene TRIGGER channel — a presentation-neutral request queue.
//!
//! Gameplay systems (a boss dying, a room entry, a dialogue node) decide *that*
//! a cutscene should play by pushing its id here; the cutscene PLAYBACK runtime
//! (overlay, script player) lives in [`crate::presentation::cutscene`] and drains
//! this queue. Splitting the trigger out of the presentation module lets sim
//! code request a cutscene without depending on the renderer — the same
//! request-channel seam used for VFX/SFX.

use bevy::prelude::*;

/// Trigger queue: anyone can push a cutscene id and the cutscene runtime picks
/// it up. Cleaner than Bevy events for the simple "fire once when X happens"
/// pattern.
#[derive(Resource, Default)]
pub struct CutsceneTriggerQueue(pub Vec<String>);

impl CutsceneTriggerQueue {
    pub fn request(&mut self, id: impl Into<String>) {
        self.0.push(id.into());
    }
}
