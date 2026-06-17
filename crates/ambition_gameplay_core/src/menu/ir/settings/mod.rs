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
//! The settings menu used to be re-authored independently by each surface (a
//! since-removed bevy-UI pause menu and the OoT cube's System face), so the two
//! drifted. This IR is now the single source of truth every renderer reads from
//! — the cube's System face ([`crate::menu::ir::system`]) and the bevy-UI grid
//! both build from this one model, so they cannot drift again.
//!
//! ## Persistence
//!
//! [`apply_settings_option`] only mutates [`UserSettings`]; it never touches
//! disk. The existing `save_settings_on_change` system persists `settings.ron`
//! whenever the resource changes, so mutating it is the whole job. The mutate
//! helpers (`audio.nudge_*`, `CameraZoomPreset::next`/`prev`, the bool toggles)
//! are reused verbatim — this module adds no parallel persistence path.

use crate::persistence::settings::audio::AudioSettings;

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
    FramePacing,
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

pub(super) fn on_off(value: bool) -> &'static str {
    if value {
        "ON"
    } else {
        "OFF"
    }
}

pub(super) fn toggle(
    id: SettingsOptionId,
    label: &str,
    value: bool,
    description: &str,
) -> SettingsOption {
    SettingsOption {
        id,
        label: label.to_string(),
        value_label: on_off(value).to_string(),
        kind: SettingsOptionKind::Toggle(value),
        description: description.to_string(),
    }
}

pub(super) fn cycle(
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

pub(super) fn slider(
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

pub(super) fn percent_label(value: f32) -> String {
    format!("{}%", AudioSettings::percent(value))
}

/// `"NN%"` for a shader unit value, using the shader-specific percent rounding
/// (`ScreenShaderSettings::percent`, distinct from the audio one) so the IR's
/// value label is byte-identical to the pause menu's `format_shader_percent`.
pub(super) fn shader_percent_label(value: f32) -> String {
    format!(
        "{}%",
        crate::persistence::settings::video::ScreenShaderSettings::percent(value)
    )
}

/// A 0..=1 shader-strength slider row. Replicates the pause menu's
/// `format_shader_percent` value label and the `nudge_unit` (clamp 0..1) step,
/// so the migrated `Shader*` rows behave identically in both frontends.
pub(super) fn shader_unit_slider(
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
pub(super) fn enum_index<T: PartialEq + Copy>(all: &[T], value: T) -> (usize, usize) {
    (all.iter().position(|v| *v == value).unwrap_or(0), all.len())
}

mod apply;
mod build;

#[cfg(test)]
mod tests;

pub use apply::{apply_settings_option, close_menu_option};
pub use build::settings_menu_model;
