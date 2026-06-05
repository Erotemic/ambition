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
//! (`crate::pause_menu`, which reads [`super::model::SettingsItem`]) and the
//! 3D OoT cube's System face (`crate::oot_cube`). They used to each re-author
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

use super::audio::AudioSettings;
use super::video::CameraZoomPreset;
use super::UserSettings;

/// Stable identity of a settings category. `Copy` so it can ride inside a
/// renderer's selection cursor / dispatched action without allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SettingsCategoryId {
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
            Self::Video => "Display mode, camera, flashes, colorblind, FPS overlay.",
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
    // Audio.
    MasterVolume,
    MusicVolume,
    SfxVolume,
    Mute,
    // Controls.
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
    // Gameplay.
    Difficulty,
    Assist,
    PlayerDamage,
    DebugHud,
    QuestHud,
    TraceAutoDump,
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

fn toggle(
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

/// Position of `value` within `all` (defaulting to 0 if missing), for the
/// `Cycle { index, count }` dot strip.
fn enum_index<T: PartialEq + Copy>(all: &[T], value: T) -> (usize, usize) {
    (
        all.iter().position(|v| *v == value).unwrap_or(0),
        all.len(),
    )
}

/// Build the live settings menu model from the current [`UserSettings`]. This
/// is the single source of truth for the option set / grouping / value labels;
/// every renderer reads from here so they cannot drift.
pub fn settings_menu_model(settings: &UserSettings) -> SettingsMenuModel {
    use super::controls::{ControllerProfileId, DashInputMode, MenuTapMode};
    use super::gameplay::Difficulty;
    use super::video::{
        CameraAspectPolicy, CameraFramingPreset, ColorblindMode, FlashIntensity,
    };
    use crate::host::windowing::DisplayModeKind;

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

    let video = SettingsCategory {
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
                "Touch Controls",
                c.touch_controls_visible,
                "Show or hide the on-screen touch control pads.",
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
                matches!(g.assist, super::gameplay::AssistMode::On),
                "Aim/traversal assists for accessibility.",
            ),
            slider(
                SettingsOptionId::PlayerDamage,
                "Player Damage",
                g.player_damage_multiplier,
                0.25,
                4.0,
                super::gameplay::GameplaySettings::DAMAGE_STEP,
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
    use super::controls::{ControllerProfileId, DashInputMode, MenuTapMode};
    use super::gameplay::Difficulty;
    use super::video::{
        CameraAspectPolicy, CameraFramingPreset, ColorblindMode, FlashIntensity,
        SerializableDisplayMode,
    };
    use crate::host::windowing::DisplayModeKind;

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
                super::model::prev_display_mode(cur)
            } else {
                super::model::next_display_mode(cur)
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

        SettingsOptionId::MasterVolume => {
            settings.audio.nudge_master(s * AudioSettings::VOLUME_STEP)
        }
        SettingsOptionId::MusicVolume => settings.audio.nudge_music(s * AudioSettings::VOLUME_STEP),
        SettingsOptionId::SfxVolume => settings.audio.nudge_sfx(s * AudioSettings::VOLUME_STEP),
        SettingsOptionId::Mute => settings.audio.toggle_mute(),

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

        SettingsOptionId::Difficulty => cyc!(settings.gameplay.difficulty, Difficulty),
        SettingsOptionId::Assist => {
            settings.gameplay.assist = settings.gameplay.assist.toggle();
        }
        SettingsOptionId::PlayerDamage => settings
            .gameplay
            .nudge_player_damage(s * super::gameplay::GameplaySettings::DAMAGE_STEP),
        SettingsOptionId::DebugHud => tog!(settings.gameplay.debug_hud_visible),
        SettingsOptionId::QuestHud => tog!(settings.gameplay.quest_hud_visible),
        SettingsOptionId::TraceAutoDump => tog!(settings.gameplay.trace_auto_dump),

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
        assert!(apply_settings_option(SettingsOptionId::Close, 0, &mut settings));
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
