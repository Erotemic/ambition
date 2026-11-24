//! User-facing persisted settings data.
//!
//! This module owns typed, serializable settings. Menu/page/rendering IR lives
//! above this crate and mutates these shapes through explicit helpers.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub mod audio;
// The input-domain config (deadzones / trigger hysteresis / burst mode /
// controller + keyboard-preset vocabulary) lives in the `ambition_input` crate.
pub use ambition_input::settings as controls;
pub mod gameplay;
pub mod persistence;
pub mod platform_paths;
pub mod video;

pub use audio::AudioSettings;
pub use controls::{
    update_trigger_edge, BurstInputMode, ControlSettings, GameplayEdgeState, MenuPointerPress,
    MenuTapMode, RightStickMode, TriggerEdgeState,
};
pub use gameplay::{AssistMode, GameplaySettings};
pub use video::{
    profile_override_from_env, seed_profile_for_gpu, ActorGlowQuality, BackgroundTextureBudget, DarknessQuality, CameraAspectPolicy, GlowBudget,
    DetectedGpuClass, GameplayViewShape, ParallaxBudget, ParticleBudget, PortalCaptureBudget, RasterBudget,
    ResolvedVisualQuality, ScreenShaderSettings, ShaderBudget, SpriteTextureBudget,
    TextureResolutionScale, VideoSettings, VisualQualityBudget, VisualQualityProfile,
    VisualQualitySettings, MAX_SCALE_FACTOR_ENV, MSAA_ENV, QUALITY_PROFILE_ENV,
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

    #[test]
    fn a_binding_override_survives_the_settings_file() {
        // A remap that does not persist is not a remap. This is the whole
        // reason `ambition_input` depends on `bevy_input` with its `serialize`
        // feature: `KeyCode` and `GamepadButton` go to disk as themselves,
        // rather than through a name table that could drift from the enum.
        use bevy::prelude::{GamepadButton, KeyCode};

        let mut s = UserSettings::default();
        s.controls
            .set_binding_override(ambition_input::BindingOverride::key("Jump", KeyCode::KeyJ));
        s.controls
            .set_binding_override(ambition_input::BindingOverride::button(
                "Special",
                GamepadButton::North,
            ));
        let restored: UserSettings =
            serde_json::from_str(&serde_json::to_string(&s).expect("serialize"))
                .expect("deserialize");
        assert_eq!(
            restored.controls.binding_overrides,
            s.controls.binding_overrides
        );
    }
    #[test]
    fn touch_sizes_persist_and_old_settings_keep_the_default_sizes() {
        let mut settings = UserSettings::default();
        settings.controls.touch_left_scale = 1.7;
        settings.controls.touch_right_scale = 2.3;
        let mut stored = serde_json::to_value(&settings).unwrap();
        assert_eq!(serde_json::from_value::<UserSettings>(stored.clone()).unwrap(), settings);
        let controls = stored["controls"].as_object_mut().unwrap();
        controls.remove("touch_left_scale");
        controls.remove("touch_right_scale");
        let restored: UserSettings = serde_json::from_value(stored).unwrap();
        assert_eq!(restored.controls.touch_left_scale, UserSettings::default().controls.touch_left_scale);
        assert_eq!(restored.controls.touch_right_scale, UserSettings::default().controls.touch_right_scale);
    }

}
