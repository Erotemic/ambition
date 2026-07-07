//! The `settings_menu_model` builder — assembles the live `SettingsMenuModel`
//! from `UserSettings`. Split out of the settings IR god-module.

use super::*;
use ambition_persistence::settings::audio::AudioSettings;
use ambition_persistence::settings::video::CameraZoomPreset;
use ambition_persistence::settings::UserSettings;

pub fn settings_menu_model(settings: &UserSettings) -> SettingsMenuModel {
    use ambition_persistence::host::windowing::DisplayModeKind;
    use ambition_persistence::settings::controls::{
        ControllerProfileId, DashInputMode, MenuTapMode,
    };
    use ambition_persistence::settings::gameplay::Difficulty;
    use ambition_persistence::settings::video::{
        CameraAspectPolicy, CameraFramingPreset, ColorblindMode, FlashIntensity, FramePaceCap,
        VisualQualityProfile,
    };

    let v = &settings.video;
    let a = &settings.audio;
    let c = &settings.controls;
    let g = &settings.gameplay;

    let display_kind = DisplayModeKind::from(v.display_mode);
    let display_all = [
        DisplayModeKind::Windowed,
        DisplayModeKind::Borderless,
        DisplayModeKind::Fullscreen,
    ];
    let (di, dc) = enum_index(&display_all, display_kind);

    let mut video = SettingsCategory {
        id: SettingsCategoryId::Video,
        label: SettingsCategoryId::Video.label().to_string(),
        options: vec![
            cycle(
                SettingsOptionId::DisplayMode,
                "Display Mode",
                display_kind.label(),
                di,
                dc,
                "Windowed, borderless, or exclusive fullscreen.",
            ),
            {
                let (i, n) = enum_index(&CameraZoomPreset::ALL, v.camera_zoom);
                cycle(
                    SettingsOptionId::CameraZoom,
                    "Camera View",
                    v.camera_zoom.label(),
                    i,
                    n,
                    "Cycle the gameplay camera zoom preset.",
                )
            },
            {
                let (i, n) = enum_index(&CameraAspectPolicy::ALL, v.camera_aspect);
                cycle(
                    SettingsOptionId::CameraAspect,
                    "Camera Aspect",
                    v.camera_aspect.label(),
                    i,
                    n,
                    "How the viewport maps onto non-16:9 windows.",
                )
            },
            {
                let (i, n) = enum_index(&CameraFramingPreset::ALL, v.camera_framing);
                cycle(
                    SettingsOptionId::CameraFraming,
                    "Camera Framing",
                    v.camera_framing.label(),
                    i,
                    n,
                    "Where the player sits within the camera frame.",
                )
            },
            {
                let (i, n) = enum_index(&FlashIntensity::ALL, v.flashes);
                cycle(
                    SettingsOptionId::Flashes,
                    "Flashes",
                    v.flashes.label(),
                    i,
                    n,
                    "Limit or disable screen-flash effects.",
                )
            },
            {
                let (i, n) = enum_index(&ColorblindMode::ALL, v.colorblind);
                cycle(
                    SettingsOptionId::Colorblind,
                    "Colorblind",
                    v.colorblind.label(),
                    i,
                    n,
                    "Colorblind-friendly palette adjustment.",
                )
            },
            toggle(
                SettingsOptionId::ShowFps,
                "FPS Overlay",
                v.show_fps,
                "Toggle the on-screen frames-per-second counter.",
            ),
            {
                let (i, n) = enum_index(&FramePaceCap::ALL, v.frame_cap);
                cycle(
                    SettingsOptionId::FramePacing,
                    "Frame Cap",
                    v.frame_cap.label(),
                    i,
                    n,
                    "Cap the frame rate to save battery/heat (auto = display refresh; or 120/60/30/24).",
                )
            },
            {
                let (i, n) = enum_index(&VisualQualityProfile::ALL, v.quality.profile);
                cycle(
                    SettingsOptionId::VisualQuality,
                    "Quality Profile",
                    v.quality.profile.label(),
                    i,
                    n,
                    "Global visual quality budget for captures, textures, parallax, shaders, and particles.",
                )
            },
        ],
    };

    // Shaders: the whole `Video > Shaders` pause-menu subpage, ported 1:1 and
    // appended to the Video category (shaders live UNDER Video, not as a sibling).
    // Each row is a slider with the SAME step the pause menu nudges by
    // (UNIT_STEP 0.10 / FINE_STEP 0.05; grain size/fps use their own ranges) and
    // the SAME value label (`ScreenShaderSettings::percent`, or `px` / `fps`).
    use ambition_persistence::settings::video::ScreenShaderSettings as Shdr;
    let s = &v.shaders;
    let shader_options: Vec<SettingsOption> = vec![
        shader_unit_slider(
            SettingsOptionId::ShaderStrength,
            "Shader Strength",
            s.strength,
            Shdr::UNIT_STEP,
            "Global multiplier for the whole shader stack (0% = off).",
        ),
        shader_unit_slider(
            SettingsOptionId::ShaderCrtStrength,
            "CRT Strength",
            s.crt_strength,
            Shdr::UNIT_STEP,
            "Overall CRT treatment strength.",
        ),
        shader_unit_slider(
            SettingsOptionId::ShaderCrtScanlines,
            "CRT Scanlines",
            s.crt_scanlines,
            Shdr::FINE_STEP,
            "CRT beam scanline darkening.",
        ),
        shader_unit_slider(
            SettingsOptionId::ShaderCrtMask,
            "CRT Phosphor Mask",
            s.crt_mask,
            Shdr::FINE_STEP,
            "CRT RGB phosphor mask intensity.",
        ),
        shader_unit_slider(
            SettingsOptionId::ShaderCrtCurvature,
            "CRT Curvature",
            s.crt_curvature,
            Shdr::FINE_STEP,
            "CRT screen-curvature warp amount.",
        ),
        shader_unit_slider(
            SettingsOptionId::ShaderCrtBloom,
            "CRT Bloom",
            s.crt_bloom,
            Shdr::FINE_STEP,
            "CRT local glow / bloom.",
        ),
        shader_unit_slider(
            SettingsOptionId::ShaderCrtChroma,
            "CRT Chroma Split",
            s.crt_chroma,
            Shdr::FINE_STEP,
            "CRT chromatic-aberration split.",
        ),
        shader_unit_slider(
            SettingsOptionId::ShaderFilmGrainStrength,
            "Film Grain Strength",
            s.film_grain_strength,
            Shdr::FINE_STEP,
            "Film-grain noise strength.",
        ),
        slider(
            SettingsOptionId::ShaderFilmGrainSize,
            "Film Grain Size",
            s.film_grain_size,
            1.0,
            8.0,
            Shdr::GRAIN_SIZE_STEP,
            format!("{:.0}px", s.film_grain_size),
            "Output pixels per film-grain cell.",
        ),
        slider(
            SettingsOptionId::ShaderFilmGrainFps,
            "Film Grain Rate",
            s.film_grain_fps,
            1.0,
            60.0,
            Shdr::GRAIN_FPS_STEP,
            format!("{:.0} fps", s.film_grain_fps),
            "How often the film-grain seed changes.",
        ),
        shader_unit_slider(
            SettingsOptionId::ShaderFilmGrainLumaBias,
            "Film Grain Luma Bias",
            s.film_grain_luma_bias,
            Shdr::FINE_STEP,
            "Bias film grain toward darker / brighter areas.",
        ),
        shader_unit_slider(
            SettingsOptionId::ShaderRobotDeathStrength,
            "Robot Death Strength",
            s.robot_death_strength,
            Shdr::UNIT_STEP,
            "Robot-death static / glitch strength.",
        ),
        shader_unit_slider(
            SettingsOptionId::ShaderRobotStatic,
            "Robot Static",
            s.robot_static,
            Shdr::FINE_STEP,
            "Robot-death static noise amount.",
        ),
        shader_unit_slider(
            SettingsOptionId::ShaderRobotTear,
            "Robot Tear",
            s.robot_tear,
            Shdr::FINE_STEP,
            "Robot-death horizontal tearing.",
        ),
        shader_unit_slider(
            SettingsOptionId::ShaderRobotDesaturate,
            "Robot Desaturate",
            s.robot_desaturate,
            Shdr::FINE_STEP,
            "Robot-death desaturation.",
        ),
        shader_unit_slider(
            SettingsOptionId::ShaderRobotScanlines,
            "Robot Scanlines",
            s.robot_scanlines,
            Shdr::FINE_STEP,
            "Robot-death scanline overlay.",
        ),
        shader_unit_slider(
            SettingsOptionId::ShaderUnderwaterStrength,
            "Underwater Strength",
            s.underwater_strength,
            Shdr::UNIT_STEP,
            "Underwater ripple / tint strength.",
        ),
        shader_unit_slider(
            SettingsOptionId::ShaderUnderwaterDistortion,
            "Underwater Distortion",
            s.underwater_distortion,
            Shdr::FINE_STEP,
            "Underwater displacement amount.",
        ),
        shader_unit_slider(
            SettingsOptionId::ShaderDeepDreamStrength,
            "Deep Dream Strength",
            s.deep_dream_strength,
            Shdr::UNIT_STEP,
            "Full-screen deep-dream reference view strength.",
        ),
        shader_unit_slider(
            SettingsOptionId::ShaderVignetteStrength,
            "Vignette Strength",
            s.vignette_strength,
            Shdr::FINE_STEP,
            "Edge-darkening vignette strength.",
        ),
    ];
    // Shaders live UNDER Video: append the 20 shader rows after the basic Video
    // rows so the single screen carries both (no separate Shaders category).
    video.options.extend(shader_options);

    let audio = SettingsCategory {
        id: SettingsCategoryId::Audio,
        label: SettingsCategoryId::Audio.label().to_string(),
        options: vec![
            slider(
                SettingsOptionId::MasterVolume,
                "Master Volume",
                a.master_volume,
                0.0,
                1.0,
                AudioSettings::VOLUME_STEP,
                percent_label(a.master_volume),
                "Overall output volume.",
            ),
            slider(
                SettingsOptionId::MusicVolume,
                "Music Volume",
                a.music_volume,
                0.0,
                1.0,
                AudioSettings::VOLUME_STEP,
                percent_label(a.music_volume),
                "Background music volume.",
            ),
            slider(
                SettingsOptionId::SfxVolume,
                "SFX Volume",
                a.sfx_volume,
                0.0,
                1.0,
                AudioSettings::VOLUME_STEP,
                percent_label(a.sfx_volume),
                "Sound-effects volume.",
            ),
            toggle(
                SettingsOptionId::Mute,
                "Mute",
                a.muted,
                "Mute or unmute all game audio.",
            ),
        ],
    };

    let controls = SettingsCategory {
        id: SettingsCategoryId::Controls,
        label: SettingsCategoryId::Controls.label().to_string(),
        options: vec![
            {
                // The pause menu labels this with the raw preset index and cycles
                // it modulo `KeyboardPreset::presets().len()` (a fixed 4). Model it
                // as a Cycle over that fixed count, with the index itself as the
                // value label — byte-identical to the pause menu's
                // `format_cycle("Keyboard Preset", keyboard_preset_index)`.
                let count = ambition_input::KeyboardPreset::presets().len();
                cycle(
                    SettingsOptionId::KeyboardPreset,
                    "Keyboard Preset",
                    &c.keyboard_preset_index.to_string(),
                    c.keyboard_preset_index.min(count.saturating_sub(1)),
                    count,
                    "Active keyboard key-binding preset.",
                )
            },
            {
                let (i, n) = enum_index(&ControllerProfileId::ALL, c.controller_profile);
                cycle(
                    SettingsOptionId::ControllerProfile,
                    "Controller",
                    c.controller_profile.label(),
                    i,
                    n,
                    "Controller button-mapping profile.",
                )
            },
            slider(
                SettingsOptionId::LeftStickDeadzone,
                "L-Stick Deadzone",
                c.left_stick_deadzone,
                0.0,
                0.6,
                0.02,
                percent_label(c.left_stick_deadzone / 0.6),
                "Ignore left-stick input below this magnitude.",
            ),
            slider(
                SettingsOptionId::RightStickDeadzone,
                "R-Stick Deadzone",
                c.right_stick_deadzone,
                0.0,
                0.6,
                0.02,
                percent_label(c.right_stick_deadzone / 0.6),
                "Ignore right-stick input below this magnitude.",
            ),
            slider(
                SettingsOptionId::TriggerPress,
                "Trigger Press",
                c.trigger_press_threshold,
                0.05,
                1.0,
                0.05,
                percent_label(c.trigger_press_threshold),
                "How far a trigger must travel to register a press.",
            ),
            slider(
                SettingsOptionId::TriggerRelease,
                "Trigger Release",
                c.trigger_release_threshold,
                0.0,
                0.95,
                0.05,
                percent_label(c.trigger_release_threshold),
                "How far a trigger must back off to register a release.",
            ),
            toggle(
                SettingsOptionId::DpadMenuNav,
                "D-Pad Menu Nav",
                c.dpad_menu_navigation,
                "Use the D-pad to move the menu cursor.",
            ),
            toggle(
                SettingsOptionId::InvertAimY,
                "Invert Aim Y",
                c.invert_aim_y,
                "Invert the vertical aim axis.",
            ),
            {
                let (i, n) = enum_index(&DashInputMode::ALL, c.dash_input_mode);
                cycle(
                    SettingsOptionId::DashInputMode,
                    "Dash Input",
                    c.dash_input_mode.label(),
                    i,
                    n,
                    "How the dash action is triggered.",
                )
            },
            toggle(
                SettingsOptionId::TouchControls,
                "Touch Overlay",
                c.touch_controls_visible,
                "Show or hide the on-screen touch overlay (touch input stays active either way).",
            ),
            {
                let (i, n) = enum_index(&MenuTapMode::ALL, c.menu_tap_mode);
                cycle(
                    SettingsOptionId::MenuTapMode,
                    "Menu Tap",
                    c.menu_tap_mode.label(),
                    i,
                    n,
                    "Single- vs double-tap behaviour in menus.",
                )
            },
            // An Action row: restores deadzone / trigger / repeat values to their
            // defaults on confirm only (Prev/Next are no-ops), matching the pause
            // menu's `ResetControlFiltering` arm.
            SettingsOption {
                id: SettingsOptionId::ResetControlFiltering,
                label: "Reset Filter Defaults".to_string(),
                value_label: String::new(),
                kind: SettingsOptionKind::Action,
                description: "Restore stick/trigger/repeat filtering to defaults.".to_string(),
            },
        ],
    };

    let gameplay = SettingsCategory {
        id: SettingsCategoryId::Gameplay,
        label: SettingsCategoryId::Gameplay.label().to_string(),
        options: vec![
            {
                let (i, n) = enum_index(&Difficulty::ALL, g.difficulty);
                cycle(
                    SettingsOptionId::Difficulty,
                    "Difficulty",
                    g.difficulty.label(),
                    i,
                    n,
                    "Overall combat difficulty.",
                )
            },
            toggle(
                SettingsOptionId::Assist,
                "Assist",
                matches!(
                    g.assist,
                    ambition_persistence::settings::gameplay::AssistMode::On
                ),
                "Aim/traversal assists for accessibility.",
            ),
            slider(
                SettingsOptionId::PlayerDamage,
                "Player Damage",
                g.player_damage_multiplier,
                0.25,
                4.0,
                ambition_persistence::settings::gameplay::GameplaySettings::DAMAGE_STEP,
                format!("x{:.2}", g.player_damage_multiplier),
                "Scale the damage the player deals.",
            ),
            toggle(
                SettingsOptionId::DebugHud,
                "Debug HUD",
                g.debug_hud_visible,
                "Toggle the debug HUD overlay (state, timers).",
            ),
            toggle(
                SettingsOptionId::QuestHud,
                "Quest HUD",
                g.quest_hud_visible,
                "Toggle the quest objective HUD panel.",
            ),
            toggle(
                SettingsOptionId::TraceAutoDump,
                "Trace Auto-Dump",
                g.trace_auto_dump,
                "Automatically dump traces on key events.",
            ),
            toggle(
                SettingsOptionId::PauseInputUnfocused,
                "Pause Input When Unfocused",
                g.pause_input_when_unfocused,
                "Ignore gameplay + menu input while the window is in the background.",
            ),
            toggle(
                SettingsOptionId::PortalReverseFacing,
                "Portal Reverses Facing",
                g.portal_reverses_facing,
                "Reverse facing when a portal turns the controlled body back along the same wall.",
            ),
            {
                let (i, n) = g.movement_frame_mode_index();
                cycle(
                    SettingsOptionId::MovementFrameMode,
                    "Movement Frame",
                    ambition_persistence::settings::gameplay::GameplaySettings::frame_mode_label(
                        g.movement_frame_mode,
                    ),
                    i,
                    n,
                    "How raw locomotion input maps onto the controlled body under \
                     rotated gravity: body-relative assist or screen-directed.",
                )
            },
            {
                let (i, n) = g.aim_frame_mode_index();
                cycle(
                    SettingsOptionId::AimFrameMode,
                    "Aim Frame",
                    ambition_persistence::settings::gameplay::GameplaySettings::frame_mode_label(
                        g.aim_frame_mode,
                    ),
                    i,
                    n,
                    "How raw precision-aim input (blink steer, shots) maps onto the \
                     controlled body under rotated gravity. Defaults to screen-directed.",
                )
            },
        ],
    };

    SettingsMenuModel {
        categories: vec![video, audio, controls, gameplay],
    }
}
