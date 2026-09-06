//! Host-supplied mix levels. The host (game) owns its settings model;
//! a host system mirrors the effective volumes in here each frame so
//! the director and playback systems never read game settings types.

use bevy::prelude::*;

/// Effective music volume (master x music), in [0, 1]. Synced from the
/// host's settings before the director runs.
#[derive(Resource, Clone, Copy, Debug)]
pub struct MusicMix {
    pub effective_music: f32,
}

impl Default for MusicMix {
    fn default() -> Self {
        Self {
            effective_music: 1.0,
        }
    }
}

impl MusicMix {
    pub fn effective_music(&self) -> f32 {
        self.effective_music
    }
}
