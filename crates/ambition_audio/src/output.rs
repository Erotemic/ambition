//! Runtime selection of the final audio-output side effect.
//!
//! Audio ownership, provider-relative resolution, and playback evidence are
//! useful in automated hosts, but those hosts must never emit literal sound.
//! [`AudioOutputMode::Recording`] keeps the real decision path active while
//! suppressing only the final Kira `play` command. Windowed applications use
//! [`AudioOutputMode::Device`] by default.

use bevy::prelude::Resource;

/// Where accepted audio playback decisions are delivered.
#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AudioOutputMode {
    /// Deliver accepted playback to the real Kira device backend.
    #[default]
    Device,
    /// Record normal playback state/evidence without issuing device playback.
    Recording,
}

impl AudioOutputMode {
    /// Whether accepted playback should issue a literal backend `play` command.
    pub const fn emits_to_device(self) -> bool {
        matches!(self, Self::Device)
    }
}

/// Resolve an optional resource to the backwards-compatible device default.
pub fn emits_to_device(mode: Option<&AudioOutputMode>) -> bool {
    match mode {
        Some(mode) => mode.emits_to_device(),
        None => true,
    }
}
