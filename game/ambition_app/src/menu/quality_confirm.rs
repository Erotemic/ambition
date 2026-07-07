//! Transactional visual-quality profile changes.
//!
//! Texture profile changes are expensive because they rebuild `GameAssets` and
//! rebind already-spawned presentation sprites. Keep the selected-but-unapplied
//! profile outside `UserSettings` so browsing quality choices cannot dirty or
//! persist settings until the user explicitly confirms.

use bevy::prelude::Resource;

use ambition_actors::persistence::settings::VisualQualityProfile;

/// App-local state for an in-flight visual-quality profile confirmation.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct VisualQualityConfirmState {
    pending: Option<VisualQualityProfile>,
}

impl VisualQualityConfirmState {
    /// The profile currently awaiting confirmation, if any.
    pub(crate) fn pending(&self) -> Option<VisualQualityProfile> {
        self.pending
    }

    /// Start or advance the pending choice without mutating persisted settings.
    pub(crate) fn step_from(&mut self, current: VisualQualityProfile, dir: i32) {
        let base = self.pending.unwrap_or(current);
        self.pending = Some(if dir < 0 { base.prev() } else { base.next() });
    }

    /// Discard the pending choice.
    pub(crate) fn cancel(&mut self) {
        self.pending = None;
    }

    /// Return the pending choice and clear the confirmation state.
    pub(crate) fn take_confirmed(&mut self) -> Option<VisualQualityProfile> {
        self.pending.take()
    }
}
