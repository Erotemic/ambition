//! `apply_settings_option` + `close_menu_option` — mutating `UserSettings` in
//! response to a settings-option nudge. Split out of the settings IR god-module.

use super::*;
use crate::persistence::settings::video::CameraZoomPreset;
use crate::persistence::settings::UserSettings;

pub fn close_menu_option() -> SettingsOption {
    SettingsOption {
        id: SettingsOptionId::Close,
        label: "Close Menu".to_string(),
        value_label: String::new(),
        kind: SettingsOptionKind::Action,
        description: "Close this menu and return to the game.".to_string(),
    }
}

/// Apply a directional step to the option identified by `id`. `dir` is the
/// signed step direction: `+1` = next/increment/toggle-on-press, `-1` =
/// prev/decrement, `0` = confirm/activate (toggles flip, cycles advance,
/// sliders step up). Only mutates [`UserSettings`]; persistence is automatic
/// via change detection. Returns `true` if the option was [`SettingsOptionId::Close`]
/// so the caller can fold the menu shut (the only outcome a renderer can't
/// express by re-reading the model).
pub fn apply_settings_option(id: SettingsOptionId, dir: i32, settings: &mut UserSettings) -> bool {
    use crate::host::windowing::DisplayModeKind;
    use crate::persistence::settings::controls::{ControllerProfileId, DashInputMode, MenuTapMode};
    use crate::persistence::settings::gameplay::Difficulty;
    use crate::persistence::settings::video::{
        CameraAspectPolicy, CameraFramingPreset, ColorblindMode, FlashIntensity, FramePaceCap,
        ScreenShaderSettings, SerializableDisplayMode,
    };

    // Cycle helper: dir<0 -> prev, otherwise next (confirm advances like next).
    macro_rules! cyc {
        ($field:expr, $ty:ty) => {{
            $field = if dir < 0 {
                <$ty>::prev($field)
            } else {
                <$ty>::next($field)
            };
        }};
    }
    // Toggle helper: any press/step flips the bool.
    macro_rules! tog {
        ($field:expr) => {{
            $field = !$field;
        }};
    }
    // Slider step magnitude with the option's own step; dir 0 nudges up.
    let s = if dir < 0 { -1.0 } else { 1.0 };

    match id {
        SettingsOptionId::DisplayMode => {
            let cur = DisplayModeKind::from(settings.video.display_mode);
            let next = if dir < 0 {
                crate::persistence::settings::model::prev_display_mode(cur)
            } else {
                crate::persistence::settings::model::next_display_mode(cur)
            };
            settings.video.display_mode = SerializableDisplayMode::from(next);
        }
        SettingsOptionId::CameraZoom => cyc!(settings.video.camera_zoom, CameraZoomPreset),
        SettingsOptionId::CameraAspect => cyc!(settings.video.camera_aspect, CameraAspectPolicy),
        SettingsOptionId::CameraFraming => {
            cyc!(settings.video.camera_framing, CameraFramingPreset)
        }
        SettingsOptionId::Flashes => cyc!(settings.video.flashes, FlashIntensity),
        SettingsOptionId::Colorblind => cyc!(settings.video.colorblind, ColorblindMode),
        SettingsOptionId::ShowFps => tog!(settings.video.show_fps),
        SettingsOptionId::FramePacing => cyc!(settings.video.frame_cap, FramePaceCap),

        // Shaders. Each nudge replicates the pause menu's `nudge_shader_unit` /
        // `nudge_shader_range` with the SAME step (UNIT_STEP / FINE_STEP, or the
        // grain ranges). Confirm (dir 0) steps up, matching the pause menu's
        // "Confirm behaves like Next" rule.
        SettingsOptionId::ShaderStrength => settings
            .video
            .shaders
            .nudge_strength(s * ScreenShaderSettings::UNIT_STEP),
        SettingsOptionId::ShaderCrtStrength => ScreenShaderSettings::nudge_unit(
            &mut settings.video.shaders.crt_strength,
            s * ScreenShaderSettings::UNIT_STEP,
        ),
        SettingsOptionId::ShaderCrtScanlines => ScreenShaderSettings::nudge_unit(
            &mut settings.video.shaders.crt_scanlines,
            s * ScreenShaderSettings::FINE_STEP,
        ),
        SettingsOptionId::ShaderCrtMask => ScreenShaderSettings::nudge_unit(
            &mut settings.video.shaders.crt_mask,
            s * ScreenShaderSettings::FINE_STEP,
        ),
        SettingsOptionId::ShaderCrtCurvature => ScreenShaderSettings::nudge_unit(
            &mut settings.video.shaders.crt_curvature,
            s * ScreenShaderSettings::FINE_STEP,
        ),
        SettingsOptionId::ShaderCrtBloom => ScreenShaderSettings::nudge_unit(
            &mut settings.video.shaders.crt_bloom,
            s * ScreenShaderSettings::FINE_STEP,
        ),
        SettingsOptionId::ShaderCrtChroma => ScreenShaderSettings::nudge_unit(
            &mut settings.video.shaders.crt_chroma,
            s * ScreenShaderSettings::FINE_STEP,
        ),
        SettingsOptionId::ShaderFilmGrainStrength => ScreenShaderSettings::nudge_unit(
            &mut settings.video.shaders.film_grain_strength,
            s * ScreenShaderSettings::FINE_STEP,
        ),
        SettingsOptionId::ShaderFilmGrainSize => ScreenShaderSettings::nudge_range(
            &mut settings.video.shaders.film_grain_size,
            s * ScreenShaderSettings::GRAIN_SIZE_STEP,
            1.0,
            8.0,
        ),
        SettingsOptionId::ShaderFilmGrainFps => ScreenShaderSettings::nudge_range(
            &mut settings.video.shaders.film_grain_fps,
            s * ScreenShaderSettings::GRAIN_FPS_STEP,
            1.0,
            60.0,
        ),
        SettingsOptionId::ShaderFilmGrainLumaBias => ScreenShaderSettings::nudge_unit(
            &mut settings.video.shaders.film_grain_luma_bias,
            s * ScreenShaderSettings::FINE_STEP,
        ),
        SettingsOptionId::ShaderRobotDeathStrength => ScreenShaderSettings::nudge_unit(
            &mut settings.video.shaders.robot_death_strength,
            s * ScreenShaderSettings::UNIT_STEP,
        ),
        SettingsOptionId::ShaderRobotStatic => ScreenShaderSettings::nudge_unit(
            &mut settings.video.shaders.robot_static,
            s * ScreenShaderSettings::FINE_STEP,
        ),
        SettingsOptionId::ShaderRobotTear => ScreenShaderSettings::nudge_unit(
            &mut settings.video.shaders.robot_tear,
            s * ScreenShaderSettings::FINE_STEP,
        ),
        SettingsOptionId::ShaderRobotDesaturate => ScreenShaderSettings::nudge_unit(
            &mut settings.video.shaders.robot_desaturate,
            s * ScreenShaderSettings::FINE_STEP,
        ),
        SettingsOptionId::ShaderRobotScanlines => ScreenShaderSettings::nudge_unit(
            &mut settings.video.shaders.robot_scanlines,
            s * ScreenShaderSettings::FINE_STEP,
        ),
        SettingsOptionId::ShaderUnderwaterStrength => ScreenShaderSettings::nudge_unit(
            &mut settings.video.shaders.underwater_strength,
            s * ScreenShaderSettings::UNIT_STEP,
        ),
        SettingsOptionId::ShaderUnderwaterDistortion => ScreenShaderSettings::nudge_unit(
            &mut settings.video.shaders.underwater_distortion,
            s * ScreenShaderSettings::FINE_STEP,
        ),
        SettingsOptionId::ShaderDeepDreamStrength => {
            ScreenShaderSettings::nudge_unit(
                &mut settings.video.shaders.deep_dream_strength,
                s * ScreenShaderSettings::UNIT_STEP,
            );
            // Pause-menu side-effect: enabling deep-dream while the global
            // strength is off auto-arms the master strength so the effect shows.
            if settings.video.shaders.deep_dream_strength > 0.001
                && settings.video.shaders.strength <= 0.001
            {
                settings.video.shaders.strength = 1.0;
            }
        }
        SettingsOptionId::ShaderVignetteStrength => ScreenShaderSettings::nudge_unit(
            &mut settings.video.shaders.vignette_strength,
            s * ScreenShaderSettings::FINE_STEP,
        ),

        SettingsOptionId::MasterVolume => {
            settings.audio.nudge_master(s * AudioSettings::VOLUME_STEP)
        }
        SettingsOptionId::MusicVolume => settings.audio.nudge_music(s * AudioSettings::VOLUME_STEP),
        SettingsOptionId::SfxVolume => settings.audio.nudge_sfx(s * AudioSettings::VOLUME_STEP),
        SettingsOptionId::Mute => settings.audio.toggle_mute(),

        SettingsOptionId::KeyboardPreset => {
            // Cycle the preset index modulo the fixed preset count, matching the
            // pause menu's `keyboard_preset_count`-driven wrap (count is always
            // `KeyboardPreset::presets().len()`).
            let len = ambition_input::KeyboardPreset::presets().len();
            if len != 0 {
                let cur = settings.controls.keyboard_preset_index;
                settings.controls.keyboard_preset_index = if dir < 0 {
                    (cur + len - 1) % len
                } else {
                    (cur + 1) % len
                };
            }
        }
        SettingsOptionId::ControllerProfile => {
            cyc!(settings.controls.controller_profile, ControllerProfileId)
        }
        SettingsOptionId::LeftStickDeadzone => {
            settings.controls.left_stick_deadzone =
                (settings.controls.left_stick_deadzone + s * 0.02).clamp(0.0, 0.6);
        }
        SettingsOptionId::RightStickDeadzone => {
            settings.controls.right_stick_deadzone =
                (settings.controls.right_stick_deadzone + s * 0.02).clamp(0.0, 0.6);
        }
        SettingsOptionId::TriggerPress => {
            settings.controls.trigger_press_threshold =
                (settings.controls.trigger_press_threshold + s * 0.05).clamp(0.05, 1.0);
            settings.controls.clamp_all();
        }
        SettingsOptionId::TriggerRelease => {
            settings.controls.trigger_release_threshold =
                (settings.controls.trigger_release_threshold + s * 0.05).clamp(0.0, 0.95);
            settings.controls.clamp_all();
        }
        SettingsOptionId::DpadMenuNav => tog!(settings.controls.dpad_menu_navigation),
        SettingsOptionId::InvertAimY => tog!(settings.controls.invert_aim_y),
        SettingsOptionId::DashInputMode => cyc!(settings.controls.dash_input_mode, DashInputMode),
        SettingsOptionId::TouchControls => tog!(settings.controls.touch_controls_visible),
        SettingsOptionId::MenuTapMode => cyc!(settings.controls.menu_tap_mode, MenuTapMode),
        SettingsOptionId::ResetControlFiltering => {
            // Confirm-only (dir 0): a stray prev/next nudge must NOT wipe the
            // user's filtering, matching the pause menu's Confirm-gated arm.
            if dir == 0 {
                settings.controls.reset_filtering_to_defaults();
            }
        }

        SettingsOptionId::Difficulty => cyc!(settings.gameplay.difficulty, Difficulty),
        SettingsOptionId::Assist => {
            settings.gameplay.assist = settings.gameplay.assist.toggle();
        }
        SettingsOptionId::PlayerDamage => settings.gameplay.nudge_player_damage(
            s * crate::persistence::settings::gameplay::GameplaySettings::DAMAGE_STEP,
        ),
        SettingsOptionId::DebugHud => tog!(settings.gameplay.debug_hud_visible),
        SettingsOptionId::QuestHud => tog!(settings.gameplay.quest_hud_visible),
        SettingsOptionId::TraceAutoDump => tog!(settings.gameplay.trace_auto_dump),
        SettingsOptionId::PauseInputUnfocused => {
            tog!(settings.gameplay.pause_input_when_unfocused)
        }
        SettingsOptionId::PortalReverseFacing => {
            tog!(settings.gameplay.portal_reverses_facing)
        }
        SettingsOptionId::InputFrameMode => settings.gameplay.cycle_input_frame_mode(dir),

        SettingsOptionId::Close => return true,
    }
    false
}
