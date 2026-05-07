//! User-facing settings for the sandbox.
//!
//! This module owns the data structures for video / audio / controls /
//! gameplay settings and the menu vocabulary that the pause menu
//! renders. The pause menu is a renderer/controller; per-setting
//! mutation lives next to the resource each setting owns
//! (`audio.rs` / `controls.rs` / `gameplay.rs` / `video.rs`).
//!
//! Architecture in one diagram:
//!
//! ```text
//!   pause_menu (renderer)
//!     |
//!     v
//!   SettingsItem  --enumerates rows-->  label_for, apply_action
//!     ^                                       |
//!     |                                       v
//!   SettingsPage::ALL                  UserSettings (Resource)
//!                                       |- video    -> VideoSettings
//!                                       |- audio    -> AudioSettings
//!                                       |- controls -> ControlSettings
//!                                       \- gameplay -> GameplaySettings
//! ```
//!
//! See `docs/settings_system.md` for how to add a new row or category.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::window::{MonitorSelection, VideoModeSelection, WindowMode};
use serde::{Deserialize, Serialize};

use crate::windowing::{DisplayModeKind, DisplayModeState};

pub mod audio;
pub mod controls;
pub mod gameplay;
pub mod persistence;
pub mod video;

pub use audio::AudioSettings;
pub use controls::{
    update_trigger_edge, ControlSettings, ControllerProfileId, DashInputMode, TriggerEdgeState,
};
pub use gameplay::{AssistMode, Difficulty, GameplaySettings};
pub use video::{
    CameraZoomPreset, ColorblindMode, FlashIntensity, SerializableDisplayMode, VideoSettings,
};

/// Aggregate user settings resource. Inserted at sandbox startup; the
/// pause menu mutates it through `apply_action`. Future persistence
/// can serialize this to disk via `serde`.
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
        self.audio.clamp_all();
        self.controls.clamp_all();
        self.gameplay.clamp_all();
    }
}

/// Top-level settings page. The pause menu starts at `Top` (the
/// category list) and pushes onto a small stack when the user
/// confirms a category.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsPage {
    #[default]
    Top,
    Video,
    Audio,
    Controls,
    Gameplay,
}

impl SettingsPage {
    pub fn title(self) -> &'static str {
        match self {
            Self::Top => "Settings",
            Self::Video => "Video",
            Self::Audio => "Audio",
            Self::Controls => "Controls",
            Self::Gameplay => "Gameplay",
        }
    }
}

/// One row on the active page. Each row knows how to render its label
/// and how to react to a `SettingsAction`.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsItem {
    // Top page: the category headers.
    OpenVideo,
    OpenAudio,
    OpenControls,
    OpenGameplay,
    Back,

    // Video page.
    DisplayMode,
    CameraZoom,
    Flashes,
    Colorblind,

    // Audio page.
    MasterVolume,
    MusicVolume,
    SfxVolume,
    Mute,

    // Controls page.
    KeyboardPreset,
    ControllerProfile,
    LeftStickDeadzone,
    RightStickDeadzone,
    TriggerPress,
    TriggerRelease,
    DpadMenuNav,
    InvertAimY,
    DashInputMode,
    ResetControlFiltering,

    // Gameplay page.
    Difficulty,
    Assist,
    PlayerDamageMultiplier,
    GameplayFlashes,
    TraceAutoDump,
}

impl SettingsItem {
    pub fn rows_for(page: SettingsPage) -> &'static [Self] {
        match page {
            SettingsPage::Top => &[
                Self::OpenVideo,
                Self::OpenAudio,
                Self::OpenControls,
                Self::OpenGameplay,
                Self::Back,
            ],
            SettingsPage::Video => &[
                Self::DisplayMode,
                Self::CameraZoom,
                Self::Flashes,
                Self::Colorblind,
                Self::Back,
            ],
            SettingsPage::Audio => &[
                Self::MasterVolume,
                Self::MusicVolume,
                Self::SfxVolume,
                Self::Mute,
                Self::Back,
            ],
            SettingsPage::Controls => &[
                Self::KeyboardPreset,
                Self::ControllerProfile,
                Self::LeftStickDeadzone,
                Self::RightStickDeadzone,
                Self::TriggerPress,
                Self::TriggerRelease,
                Self::DpadMenuNav,
                Self::InvertAimY,
                Self::DashInputMode,
                Self::ResetControlFiltering,
                Self::Back,
            ],
            SettingsPage::Gameplay => &[
                Self::Difficulty,
                Self::Assist,
                Self::PlayerDamageMultiplier,
                Self::GameplayFlashes,
                Self::TraceAutoDump,
                Self::Back,
            ],
        }
    }

    /// Label shown to the user for this row, given the current
    /// settings snapshot.
    pub fn label(self, settings: &UserSettings) -> String {
        match self {
            Self::OpenVideo => "Video >".into(),
            Self::OpenAudio => "Audio >".into(),
            Self::OpenControls => "Controls >".into(),
            Self::OpenGameplay => "Gameplay >".into(),
            Self::Back => "Back".into(),

            Self::DisplayMode => format!(
                "Display Mode: {}  < / >",
                DisplayModeKind::from(settings.video.display_mode).label()
            ),
            Self::CameraZoom => {
                format!("Camera Zoom: {}  < / >", settings.video.camera_zoom.label())
            }
            Self::Flashes => format!("Flashes: {}  < / >", settings.video.flashes.label()),
            Self::Colorblind => format!("Colorblind: {}  < / >", settings.video.colorblind.label()),

            Self::MasterVolume => format!(
                "Master Volume: {}%  < / >",
                AudioSettings::percent(settings.audio.master_volume)
            ),
            Self::MusicVolume => format!(
                "Music Volume: {}%  < / >",
                AudioSettings::percent(settings.audio.music_volume)
            ),
            Self::SfxVolume => format!(
                "SFX Volume: {}%  < / >",
                AudioSettings::percent(settings.audio.sfx_volume)
            ),
            Self::Mute => format!(
                "Mute: {}",
                if settings.audio.muted { "muted" } else { "off" }
            ),

            Self::KeyboardPreset => format!(
                "Keyboard Preset: {}  < / >",
                settings.controls.keyboard_preset_index
            ),
            Self::ControllerProfile => format!(
                "Controller: {}  < / >",
                settings.controls.controller_profile.label()
            ),
            Self::LeftStickDeadzone => format!(
                "L-Stick Deadzone: {}%  < / >",
                AudioSettings::percent(settings.controls.left_stick_deadzone)
            ),
            Self::RightStickDeadzone => format!(
                "R-Stick Deadzone: {}%  < / >",
                AudioSettings::percent(settings.controls.right_stick_deadzone)
            ),
            Self::TriggerPress => format!(
                "Trigger Press: {}%  < / >",
                AudioSettings::percent(settings.controls.trigger_press_threshold)
            ),
            Self::TriggerRelease => format!(
                "Trigger Release: {}%  < / >",
                AudioSettings::percent(settings.controls.trigger_release_threshold)
            ),
            Self::DpadMenuNav => format!(
                "D-Pad Menu Nav: {}",
                if settings.controls.dpad_menu_navigation {
                    "on"
                } else {
                    "off"
                }
            ),
            Self::InvertAimY => format!(
                "Invert Aim Y: {}",
                if settings.controls.invert_aim_y {
                    "on"
                } else {
                    "off"
                }
            ),
            Self::DashInputMode => format!(
                "Dash Input: {}  < / >",
                settings.controls.dash_input_mode.label()
            ),
            Self::ResetControlFiltering => "Reset Filter Defaults".into(),

            Self::Difficulty => format!(
                "Difficulty: {}  < / >",
                settings.gameplay.difficulty.label()
            ),
            Self::Assist => format!("Assist: {}", settings.gameplay.assist.label()),
            Self::PlayerDamageMultiplier => format!(
                "Player Damage: x{:.2}  < / >",
                settings.gameplay.player_damage_multiplier
            ),
            Self::GameplayFlashes => format!(
                "Flashes (gameplay): {}  < / >",
                settings.video.flashes.label()
            ),
            Self::TraceAutoDump => format!(
                "Trace Auto-Dump: {}",
                if settings.gameplay.trace_auto_dump {
                    "on"
                } else {
                    "off"
                }
            ),
        }
    }
}

/// Action a row can receive from the pause menu controller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsAction {
    /// Cycle backward / decrement.
    Prev,
    /// Cycle forward / increment.
    Next,
    /// Activate. May toggle or open a sub-page depending on the row.
    Confirm,
}

/// Menu controller outcome: stay on the current page, push to a sub-
/// page, or pop back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsOutcome {
    Stay,
    OpenPage(SettingsPage),
    PopPage,
}

/// Apply `action` to `item`, mutating the relevant fields of
/// `settings`. Returns the outcome the caller (the pause menu
/// controller) should follow.
///
/// `display_state` and `windows` are only required for the display-
/// mode row because applying the change touches the live primary
/// window.
pub fn apply_action(
    item: SettingsItem,
    action: SettingsAction,
    settings: &mut UserSettings,
    display_state: &mut DisplayModeState,
    windows: &mut Query<&mut Window, With<PrimaryWindow>>,
    keyboard_preset_count: usize,
) -> SettingsOutcome {
    match item {
        SettingsItem::OpenVideo => {
            if matches!(action, SettingsAction::Confirm) {
                return SettingsOutcome::OpenPage(SettingsPage::Video);
            }
        }
        SettingsItem::OpenAudio => {
            if matches!(action, SettingsAction::Confirm) {
                return SettingsOutcome::OpenPage(SettingsPage::Audio);
            }
        }
        SettingsItem::OpenControls => {
            if matches!(action, SettingsAction::Confirm) {
                return SettingsOutcome::OpenPage(SettingsPage::Controls);
            }
        }
        SettingsItem::OpenGameplay => {
            if matches!(action, SettingsAction::Confirm) {
                return SettingsOutcome::OpenPage(SettingsPage::Gameplay);
            }
        }
        SettingsItem::Back => {
            if matches!(action, SettingsAction::Confirm) {
                return SettingsOutcome::PopPage;
            }
        }

        SettingsItem::DisplayMode => {
            let current: DisplayModeKind = settings.video.display_mode.into();
            let next = match action {
                SettingsAction::Prev => prev_display_mode(current),
                SettingsAction::Next | SettingsAction::Confirm => next_display_mode(current),
            };
            apply_display_mode(next, display_state, windows);
            settings.video.display_mode = SerializableDisplayMode::from(next);
        }
        SettingsItem::CameraZoom => match action {
            SettingsAction::Prev => settings.video.camera_zoom = settings.video.camera_zoom.prev(),
            SettingsAction::Next | SettingsAction::Confirm => {
                settings.video.camera_zoom = settings.video.camera_zoom.next()
            }
        },
        SettingsItem::Flashes | SettingsItem::GameplayFlashes => match action {
            SettingsAction::Prev => settings.video.flashes = settings.video.flashes.prev(),
            SettingsAction::Next | SettingsAction::Confirm => {
                settings.video.flashes = settings.video.flashes.next()
            }
        },
        SettingsItem::Colorblind => match action {
            SettingsAction::Prev => settings.video.colorblind = settings.video.colorblind.prev(),
            SettingsAction::Next | SettingsAction::Confirm => {
                settings.video.colorblind = settings.video.colorblind.next()
            }
        },

        SettingsItem::MasterVolume => match action {
            SettingsAction::Prev => settings.audio.nudge_master(-AudioSettings::VOLUME_STEP),
            SettingsAction::Next => settings.audio.nudge_master(AudioSettings::VOLUME_STEP),
            SettingsAction::Confirm => settings.audio.nudge_master(AudioSettings::VOLUME_STEP),
        },
        SettingsItem::MusicVolume => match action {
            SettingsAction::Prev => settings.audio.nudge_music(-AudioSettings::VOLUME_STEP),
            SettingsAction::Next => settings.audio.nudge_music(AudioSettings::VOLUME_STEP),
            SettingsAction::Confirm => settings.audio.nudge_music(AudioSettings::VOLUME_STEP),
        },
        SettingsItem::SfxVolume => match action {
            SettingsAction::Prev => settings.audio.nudge_sfx(-AudioSettings::VOLUME_STEP),
            SettingsAction::Next => settings.audio.nudge_sfx(AudioSettings::VOLUME_STEP),
            SettingsAction::Confirm => settings.audio.nudge_sfx(AudioSettings::VOLUME_STEP),
        },
        SettingsItem::Mute => {
            if matches!(
                action,
                SettingsAction::Confirm | SettingsAction::Next | SettingsAction::Prev
            ) {
                settings.audio.toggle_mute();
            }
        }

        SettingsItem::KeyboardPreset => {
            if keyboard_preset_count == 0 {
                return SettingsOutcome::Stay;
            }
            let len = keyboard_preset_count;
            settings.controls.keyboard_preset_index = match action {
                SettingsAction::Prev => (settings.controls.keyboard_preset_index + len - 1) % len,
                SettingsAction::Next | SettingsAction::Confirm => {
                    (settings.controls.keyboard_preset_index + 1) % len
                }
            };
        }
        SettingsItem::ControllerProfile => match action {
            SettingsAction::Prev => {
                settings.controls.controller_profile = settings.controls.controller_profile.prev();
            }
            SettingsAction::Next | SettingsAction::Confirm => {
                settings.controls.controller_profile = settings.controls.controller_profile.next();
            }
        },
        SettingsItem::LeftStickDeadzone => {
            let delta = match action {
                SettingsAction::Prev => -0.02,
                SettingsAction::Next | SettingsAction::Confirm => 0.02,
            };
            settings.controls.left_stick_deadzone =
                (settings.controls.left_stick_deadzone + delta).clamp(0.0, 0.6);
        }
        SettingsItem::RightStickDeadzone => {
            let delta = match action {
                SettingsAction::Prev => -0.02,
                SettingsAction::Next | SettingsAction::Confirm => 0.02,
            };
            settings.controls.right_stick_deadzone =
                (settings.controls.right_stick_deadzone + delta).clamp(0.0, 0.6);
        }
        SettingsItem::TriggerPress => {
            let delta = match action {
                SettingsAction::Prev => -0.05,
                SettingsAction::Next | SettingsAction::Confirm => 0.05,
            };
            settings.controls.trigger_press_threshold =
                (settings.controls.trigger_press_threshold + delta).clamp(0.05, 1.0);
            settings.controls.clamp_all();
        }
        SettingsItem::TriggerRelease => {
            let delta = match action {
                SettingsAction::Prev => -0.05,
                SettingsAction::Next | SettingsAction::Confirm => 0.05,
            };
            settings.controls.trigger_release_threshold =
                (settings.controls.trigger_release_threshold + delta).clamp(0.0, 0.95);
            settings.controls.clamp_all();
        }
        SettingsItem::DpadMenuNav => {
            if matches!(
                action,
                SettingsAction::Confirm | SettingsAction::Next | SettingsAction::Prev
            ) {
                settings.controls.dpad_menu_navigation = !settings.controls.dpad_menu_navigation;
            }
        }
        SettingsItem::InvertAimY => {
            if matches!(
                action,
                SettingsAction::Confirm | SettingsAction::Next | SettingsAction::Prev
            ) {
                settings.controls.invert_aim_y = !settings.controls.invert_aim_y;
            }
        }
        SettingsItem::DashInputMode => match action {
            SettingsAction::Prev => {
                settings.controls.dash_input_mode = settings.controls.dash_input_mode.prev();
            }
            SettingsAction::Next | SettingsAction::Confirm => {
                settings.controls.dash_input_mode = settings.controls.dash_input_mode.next();
            }
        },
        SettingsItem::ResetControlFiltering => {
            if matches!(action, SettingsAction::Confirm) {
                settings.controls.reset_filtering_to_defaults();
            }
        }

        SettingsItem::Difficulty => match action {
            SettingsAction::Prev => {
                settings.gameplay.difficulty = settings.gameplay.difficulty.prev()
            }
            SettingsAction::Next | SettingsAction::Confirm => {
                settings.gameplay.difficulty = settings.gameplay.difficulty.next()
            }
        },
        SettingsItem::Assist => {
            if matches!(
                action,
                SettingsAction::Confirm | SettingsAction::Next | SettingsAction::Prev
            ) {
                settings.gameplay.assist = settings.gameplay.assist.toggle();
            }
        }
        SettingsItem::PlayerDamageMultiplier => match action {
            SettingsAction::Prev => settings
                .gameplay
                .nudge_player_damage(-GameplaySettings::DAMAGE_STEP),
            SettingsAction::Next | SettingsAction::Confirm => settings
                .gameplay
                .nudge_player_damage(GameplaySettings::DAMAGE_STEP),
        },
        SettingsItem::TraceAutoDump => {
            if matches!(
                action,
                SettingsAction::Confirm | SettingsAction::Next | SettingsAction::Prev
            ) {
                settings.gameplay.trace_auto_dump = !settings.gameplay.trace_auto_dump;
            }
        }
    }
    SettingsOutcome::Stay
}

pub fn next_display_mode(current: DisplayModeKind) -> DisplayModeKind {
    match current {
        DisplayModeKind::Windowed => DisplayModeKind::Borderless,
        DisplayModeKind::Borderless => DisplayModeKind::Fullscreen,
        DisplayModeKind::Fullscreen => DisplayModeKind::Windowed,
    }
}

pub fn prev_display_mode(current: DisplayModeKind) -> DisplayModeKind {
    match current {
        DisplayModeKind::Windowed => DisplayModeKind::Fullscreen,
        DisplayModeKind::Borderless => DisplayModeKind::Windowed,
        DisplayModeKind::Fullscreen => DisplayModeKind::Borderless,
    }
}

/// Apply a `DisplayModeKind` to the primary window. Shared between the
/// settings menu and `crate::windowing::window_mode_hotkeys` so both
/// surfaces produce the same `WindowMode` mapping.
pub fn apply_display_mode(
    mode: DisplayModeKind,
    state: &mut DisplayModeState,
    windows: &mut Query<&mut Window, With<PrimaryWindow>>,
) {
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    window.mode = match mode {
        DisplayModeKind::Windowed => WindowMode::Windowed,
        DisplayModeKind::Borderless => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
        DisplayModeKind::Fullscreen => {
            WindowMode::Fullscreen(MonitorSelection::Current, VideoModeSelection::Current)
        }
    };
    state.mode = mode;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_for_top_includes_categories() {
        let rows = SettingsItem::rows_for(SettingsPage::Top);
        assert!(rows.contains(&SettingsItem::OpenVideo));
        assert!(rows.contains(&SettingsItem::OpenAudio));
        assert!(rows.contains(&SettingsItem::OpenControls));
        assert!(rows.contains(&SettingsItem::OpenGameplay));
        assert!(rows.contains(&SettingsItem::Back));
    }

    #[test]
    fn cycle_display_mode_forward_and_back_returns_to_start() {
        let start = DisplayModeKind::Windowed;
        let one = next_display_mode(start);
        let two = next_display_mode(one);
        let back = next_display_mode(two);
        assert_eq!(back, start);
        assert_eq!(prev_display_mode(start), DisplayModeKind::Fullscreen);
    }

    #[test]
    fn label_includes_current_display_mode() {
        let mut s = UserSettings::default();
        s.video.display_mode = SerializableDisplayMode::Borderless;
        let label = SettingsItem::DisplayMode.label(&s);
        assert!(label.contains("borderless"));
        assert_eq!(SettingsItem::Back.label(&s), "Back");
    }

    #[test]
    fn user_settings_serde_round_trip() {
        let s = UserSettings::default();
        let serialized = serde_json::to_string(&s).expect("serialize");
        let restored: UserSettings = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(s, restored);
    }
}
