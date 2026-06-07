//! Shared settings-menu intermediate representation (IR).
//!
//! [`SettingsMenuModel`] is the single, renderer-agnostic description of the
//! settings UI: a list of categories (Video / Audio / Controls / Gameplay),
//! each holding a list of options with a *live* value label and a [`kind`]
//! that tells a renderer how to draw the control (toggle / cycle / slider /
//! action). It is built once from [`UserSettings`] via [`settings_menu_model`]
//! and mutated through [`apply_settings_option`].
//!
//! [`kind`]: SettingsOption::kind
//!
//! ## Why an IR
//!
//! Two settings surfaces exist in the sandbox: the bevy-UI pause menu
//! (`crate::pause_menu`, which reads [`crate::persistence::settings::model::SettingsItem`]) and the
//! 3D OoT cube's System face (`crate::menu::model`). They used to each re-author
//! the option set, so they drifted. This IR is the shared source of truth the
//! cube renders from (the pause menu migrates onto it as a follow-up).
//!
//! ## Persistence
//!
//! [`apply_settings_option`] only mutates [`UserSettings`]; it never touches
//! disk. The existing `save_settings_on_change` system persists `settings.ron`
//! whenever the resource changes, so mutating it is the whole job. The mutate
//! helpers (`audio.nudge_*`, `CameraZoomPreset::next`/`prev`, the bool toggles)
//! are reused verbatim — this module adds no parallel persistence path.

use crate::persistence::settings::audio::AudioSettings;
use crate::persistence::settings::video::CameraZoomPreset;
use crate::persistence::settings::UserSettings;

/// Stable identity of a settings category. `Copy` so it can ride inside a
/// renderer's selection cursor / dispatched action without allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SettingsCategoryId {
    /// Display mode, camera, flashes, colorblind, FPS overlay — and the whole
    /// shader / post-process stack, appended after the basic rows. Shaders live
    /// UNDER Video (not a sibling category) so both frontends agree: the pause
    /// menu drills `Video > Shaders` as a subpage, and the cube's single-level
    /// System drill surfaces them flat in the Video screen after the basic rows.
    Video,
    Audio,
    Controls,
    Gameplay,
}

impl SettingsCategoryId {
    /// Every category, in display order.
    pub const ALL: [Self; 4] = [Self::Video, Self::Audio, Self::Controls, Self::Gameplay];

    /// Display name, matching `SettingsPage::title` in the pause menu.
    pub fn label(self) -> &'static str {
        match self {
            Self::Video => "Video",
            Self::Audio => "Audio",
            Self::Controls => "Controls",
            Self::Gameplay => "Gameplay",
        }
    }

    /// One-line category description (shown when a category row is focused).
    pub fn description(self) -> &'static str {
        match self {
            Self::Video => "Display mode, camera, flashes, colorblind, FPS, shaders.",
            Self::Audio => "Master / music / SFX volume and mute.",
            Self::Controls => "Sticks, triggers, dash, touch, and menu input.",
            Self::Gameplay => "Difficulty, assist, damage, and HUD overlays.",
        }
    }
}

/// Stable identity of a single settings option. `Copy` so it rides inside a
/// renderer's cursor / dispatched action. Each id maps to exactly one field
/// (or pair of fields) of [`UserSettings`], plus the [`SettingsOptionId::Close`]
/// pseudo-option that closes the menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SettingsOptionId {
    // Video.
    DisplayMode,
    CameraZoom,
    CameraAspect,
    CameraFraming,
    Flashes,
    Colorblind,
    ShowFps,
    // Shaders (the whole `Video > Shaders` subpage from the pause menu).
    ShaderStrength,
    ShaderCrtStrength,
    ShaderCrtScanlines,
    ShaderCrtMask,
    ShaderCrtCurvature,
    ShaderCrtBloom,
    ShaderCrtChroma,
    ShaderFilmGrainStrength,
    ShaderFilmGrainSize,
    ShaderFilmGrainFps,
    ShaderFilmGrainLumaBias,
    ShaderRobotDeathStrength,
    ShaderRobotStatic,
    ShaderRobotTear,
    ShaderRobotDesaturate,
    ShaderRobotScanlines,
    ShaderUnderwaterStrength,
    ShaderUnderwaterDistortion,
    ShaderDeepDreamStrength,
    ShaderVignetteStrength,
    // Audio.
    MasterVolume,
    MusicVolume,
    SfxVolume,
    Mute,
    // Controls.
    KeyboardPreset,
    ControllerProfile,
    LeftStickDeadzone,
    RightStickDeadzone,
    TriggerPress,
    TriggerRelease,
    DpadMenuNav,
    InvertAimY,
    DashInputMode,
    TouchControls,
    MenuTapMode,
    ResetControlFiltering,
    // Gameplay.
    Difficulty,
    Assist,
    PlayerDamage,
    DebugHud,
    QuestHud,
    TraceAutoDump,
    /// Suppress gameplay + menu input while the OS window is unfocused (opt-in
    /// guard against background input bleed). Default OFF.
    PauseInputUnfocused,
    // Menu-level action (not under any category).
    Close,
}

/// How a renderer should draw an option's control and what a directional step
/// means for it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SettingsOptionKind {
    /// A two-state on/off switch. The bool is the *current* state.
    Toggle(bool),
    /// A discrete enum cycled with prev/next. `index`/`count` describe the
    /// current position for a renderer that wants to draw a dot strip.
    Cycle { index: usize, count: usize },
    /// A continuous value normalised to `[min, max]` stepped by `step`.
    Slider {
        value: f32,
        min: f32,
        max: f32,
        step: f32,
    },
    /// A momentary action (Close Menu / Back). No value.
    Action,
}

/// One option row in the IR: its identity, label, live value label, control
/// kind, and a one-line description.
#[derive(Clone, Debug, PartialEq)]
pub struct SettingsOption {
    pub id: SettingsOptionId,
    pub label: String,
    pub value_label: String,
    pub kind: SettingsOptionKind,
    pub description: String,
}

/// One category and its options.
#[derive(Clone, Debug, PartialEq)]
pub struct SettingsCategory {
    pub id: SettingsCategoryId,
    pub label: String,
    pub options: Vec<SettingsOption>,
}

/// The whole settings UI as data. Build it with [`settings_menu_model`].
#[derive(Clone, Debug, PartialEq)]
pub struct SettingsMenuModel {
    pub categories: Vec<SettingsCategory>,
}

impl SettingsMenuModel {
    /// The category with the given id, if present.
    pub fn category(&self, id: SettingsCategoryId) -> Option<&SettingsCategory> {
        self.categories.iter().find(|c| c.id == id)
    }
}

fn on_off(value: bool) -> &'static str {
    if value {
        "ON"
    } else {
        "OFF"
    }
}

fn toggle(id: SettingsOptionId, label: &str, value: bool, description: &str) -> SettingsOption {
    SettingsOption {
        id,
        label: label.to_string(),
        value_label: on_off(value).to_string(),
        kind: SettingsOptionKind::Toggle(value),
        description: description.to_string(),
    }
}

fn cycle(
    id: SettingsOptionId,
    label: &str,
    value_label: &str,
    index: usize,
    count: usize,
    description: &str,
) -> SettingsOption {
    SettingsOption {
        id,
        label: label.to_string(),
        value_label: value_label.to_string(),
        kind: SettingsOptionKind::Cycle { index, count },
        description: description.to_string(),
    }
}

fn slider(
    id: SettingsOptionId,
    label: &str,
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    value_label: String,
    description: &str,
) -> SettingsOption {
    SettingsOption {
        id,
        label: label.to_string(),
        value_label,
        kind: SettingsOptionKind::Slider {
            value,
            min,
            max,
            step,
        },
        description: description.to_string(),
    }
}

fn percent_label(value: f32) -> String {
    format!("{}%", AudioSettings::percent(value))
}

/// `"NN%"` for a shader unit value, using the shader-specific percent rounding
/// (`ScreenShaderSettings::percent`, distinct from the audio one) so the IR's
/// value label is byte-identical to the pause menu's `format_shader_percent`.
fn shader_percent_label(value: f32) -> String {
    format!(
        "{}%",
        crate::persistence::settings::video::ScreenShaderSettings::percent(value)
    )
}

/// A 0..=1 shader-strength slider row. Replicates the pause menu's
/// `format_shader_percent` value label and the `nudge_unit` (clamp 0..1) step,
/// so the migrated `Shader*` rows behave identically in both frontends.
fn shader_unit_slider(
    id: SettingsOptionId,
    label: &str,
    value: f32,
    step: f32,
    description: &str,
) -> SettingsOption {
    slider(
        id,
        label,
        value,
        0.0,
        1.0,
        step,
        shader_percent_label(value),
        description,
    )
}

/// Position of `value` within `all` (defaulting to 0 if missing), for the
/// `Cycle { index, count }` dot strip.
fn enum_index<T: PartialEq + Copy>(all: &[T], value: T) -> (usize, usize) {
    (all.iter().position(|v| *v == value).unwrap_or(0), all.len())
}

/// Build the live settings menu model from the current [`UserSettings`]. This
/// is the single source of truth for the option set / grouping / value labels;
/// every renderer reads from here so they cannot drift.
pub fn settings_menu_model(settings: &UserSettings) -> SettingsMenuModel {
    use crate::host::windowing::DisplayModeKind;
    use crate::persistence::settings::controls::{ControllerProfileId, DashInputMode, MenuTapMode};
    use crate::persistence::settings::gameplay::Difficulty;
    use crate::persistence::settings::video::{
        CameraAspectPolicy, CameraFramingPreset, ColorblindMode, FlashIntensity,
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
        ],
    };

    // Shaders: the whole `Video > Shaders` pause-menu subpage, ported 1:1 and
    // appended to the Video category (shaders live UNDER Video, not as a sibling).
    // Each row is a slider with the SAME step the pause menu nudges by
    // (UNIT_STEP 0.10 / FINE_STEP 0.05; grain size/fps use their own ranges) and
    // the SAME value label (`ScreenShaderSettings::percent`, or `px` / `fps`).
    use crate::persistence::settings::video::ScreenShaderSettings as Shdr;
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
                    crate::persistence::settings::gameplay::AssistMode::On
                ),
                "Aim/traversal assists for accessibility.",
            ),
            slider(
                SettingsOptionId::PlayerDamage,
                "Player Damage",
                g.player_damage_multiplier,
                0.25,
                4.0,
                crate::persistence::settings::gameplay::GameplaySettings::DAMAGE_STEP,
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
        ],
    };

    SettingsMenuModel {
        categories: vec![video, audio, controls, gameplay],
    }
}

/// A close-menu pseudo-option, for renderers that surface "Close Menu" as a
/// top-level row outside any category (the cube does). Kept here so its label /
/// description live with the rest of the IR vocabulary.
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
        CameraAspectPolicy, CameraFramingPreset, ColorblindMode, FlashIntensity,
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

        SettingsOptionId::Close => return true,
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_has_the_four_categories_in_order() {
        let model = settings_menu_model(&UserSettings::default());
        let ids: Vec<_> = model.categories.iter().map(|c| c.id).collect();
        assert_eq!(ids, SettingsCategoryId::ALL.to_vec());
        for cat in &model.categories {
            assert!(!cat.options.is_empty(), "{:?} has options", cat.id);
            assert_eq!(cat.label, cat.id.label());
        }
    }

    #[test]
    fn value_labels_are_live() {
        let mut settings = UserSettings::default();
        settings.video.show_fps = false;
        let model = settings_menu_model(&settings);
        let fps = model
            .category(SettingsCategoryId::Video)
            .unwrap()
            .options
            .iter()
            .find(|o| o.id == SettingsOptionId::ShowFps)
            .unwrap();
        assert_eq!(fps.value_label, "OFF");
        assert!(matches!(fps.kind, SettingsOptionKind::Toggle(false)));
    }

    #[test]
    fn apply_toggles_a_bool_field() {
        let mut settings = UserSettings::default();
        let before = settings.gameplay.quest_hud_visible;
        let closed = apply_settings_option(SettingsOptionId::QuestHud, 0, &mut settings);
        assert!(!closed);
        assert_ne!(settings.gameplay.quest_hud_visible, before);
    }

    #[test]
    fn apply_steps_a_slider_and_cycles_an_enum() {
        let mut settings = UserSettings::default();
        let v0 = settings.audio.master_volume;
        apply_settings_option(SettingsOptionId::MasterVolume, 1, &mut settings);
        assert!(settings.audio.master_volume >= v0);

        let z0 = settings.video.camera_zoom;
        apply_settings_option(SettingsOptionId::CameraZoom, 1, &mut settings);
        assert_eq!(settings.video.camera_zoom, z0.next());
    }

    #[test]
    fn close_option_reports_close() {
        let mut settings = UserSettings::default();
        assert!(apply_settings_option(
            SettingsOptionId::Close,
            0,
            &mut settings
        ));
    }

    #[test]
    fn slider_value_label_is_percent() {
        let model = settings_menu_model(&UserSettings::default());
        let master = model
            .category(SettingsCategoryId::Audio)
            .unwrap()
            .options
            .iter()
            .find(|o| o.id == SettingsOptionId::MasterVolume)
            .unwrap();
        assert!(master.value_label.ends_with('%'));
        assert!(matches!(master.kind, SettingsOptionKind::Slider { .. }));
    }
}
