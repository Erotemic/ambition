//! User-facing persisted settings data.
//!
//! This module owns typed, serializable settings. Menu/page/rendering IR lives
//! above this crate and mutates these shapes through explicit helpers.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub mod audio;
// The input-domain config (deadzones / trigger hysteresis / dash mode /
// controller + keyboard-preset vocabulary) lives in the `ambition_input` crate.
pub use ambition_input::settings as controls;
pub mod gameplay;
pub mod persistence;
pub mod platform_paths;
pub mod video;

pub use audio::AudioSettings;
pub use controls::{
    update_trigger_edge, ControlSettings, DashInputMode, MenuPointerPress, MenuTapMode,
    TriggerEdgeState,
};
pub use gameplay::{AssistMode, GameplaySettings};
pub use video::{
    BackgroundTextureBudget, CameraAspectPolicy, ParallaxBudget, ParticleBudget,
    PortalCaptureBudget, ScreenShaderSettings, ShaderBudget, SpriteTextureBudget,
    TextureResolutionScale, VideoSettings, VisualQualityBudget, VisualQualityProfile,
    VisualQualitySettings,
};

#[cfg(test)]
pub(crate) use gameplay::Difficulty;
#[cfg(test)]
pub(crate) use video::FlashIntensity;

/// Aggregate user settings resource.
#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct UserSettings {
    #[serde(default)]
    pub video: VideoSettings,
    #[serde(default)]
    pub audio: AudioSettings,
    #[serde(default)]
    pub controls: ControlSettings,
    #[serde(default)]
    pub gameplay: GameplaySettings,
}

impl UserSettings {
    /// Re-clamp every value into its valid range. Useful right after
    /// loading from disk in case the file was hand-edited.
    pub fn clamp_all(&mut self) {
        self.video.clamp_all();
        self.audio.clamp_all();
        self.controls.clamp_all();
        self.gameplay.clamp_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_settings_serde_round_trip() {
        let s = UserSettings::default();
        let serialized = serde_json::to_string(&s).expect("serialize");
        let restored: UserSettings = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(s, restored);
    }
}
