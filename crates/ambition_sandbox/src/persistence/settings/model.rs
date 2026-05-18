//! Pause-menu settings vocabulary: page enum, row enum, action enum,
//! outcome enum, and the `apply_action` controller dispatcher.
//!
//! This module is what the pause-menu renderer reads to decide which
//! rows to show and what to do when the user confirms / nudges them.
//! The actual data shapes (`UserSettings`, `VideoSettings`, etc.)
//! live in their per-category modules.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::window::{MonitorSelection, VideoModeSelection, WindowMode};

use super::audio::AudioSettings;
use super::gameplay::GameplaySettings;
use super::video::{ScreenShaderSettings, SerializableDisplayMode};
use super::UserSettings;
use crate::dev_tools::{
    apply_movement_profile, apply_player_body_profile, DeveloperTools, EditableMovementTuning,
};
use crate::ldtk_world::LdtkHotReloadState;
use crate::windowing::{DisplayModeKind, DisplayModeState};
use crate::SandboxDevState;

/// Top-level settings page. The pause menu starts at `Top` (the
/// category list) and pushes onto a small stack when the user
/// confirms a category.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsPage {
    #[default]
    Top,
    Video,
    Shaders,
    Audio,
    Controls,
    Gameplay,
    Developer,
}

impl SettingsPage {
    pub fn title(self) -> &'static str {
        match self {
            Self::Top => "Settings",
            Self::Video => "Video",
            Self::Shaders => "Shaders",
            Self::Audio => "Audio",
            Self::Controls => "Controls",
            Self::Gameplay => "Gameplay",
            Self::Developer => "Developer",
        }
    }

    pub const ALL: &'static [Self] = &[
        Self::Top,
        Self::Video,
        Self::Shaders,
        Self::Audio,
        Self::Controls,
        Self::Gameplay,
        Self::Developer,
    ];
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
    OpenDeveloper,
    Back,

    // Video page.
    DisplayMode,
    CameraZoom,
    CameraAspect,
    CameraFraming,
    OpenShaders,
    Flashes,
    Colorblind,
    ShowFps,

    // Video > Shaders page.
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
    ShaderVignetteStrength,

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
    TouchControls,
    MenuTapMode,
    ResetControlFiltering,

    // Gameplay page.
    Difficulty,
    Assist,
    PlayerDamageMultiplier,
    GameplayFlashes,
    DebugHud,
    QuestHud,
    TraceAutoDump,

    // Developer page (F-key equivalents).
    DebugOverlay,
    SlowMotion,
    Inspector,
    WorldInspector,
    OverviewCamera,
    ShowHitboxes,
    MicroGrid,
    CameraFrame,
    PlayerBodyProfile,
    MovementProfile,
    LdtkAutoApply,
}

impl SettingsItem {
    pub fn rows_for(page: SettingsPage) -> &'static [Self] {
        match page {
            SettingsPage::Top => &[
                Self::OpenVideo,
                Self::OpenAudio,
                Self::OpenControls,
                Self::OpenGameplay,
                Self::OpenDeveloper,
                Self::Back,
            ],
            SettingsPage::Video => &[
                Self::DisplayMode,
                Self::CameraZoom,
                Self::CameraAspect,
                Self::CameraFraming,
                Self::OpenShaders,
                Self::Flashes,
                Self::Colorblind,
                Self::ShowFps,
                Self::Back,
            ],
            SettingsPage::Shaders => &[
                Self::ShaderStrength,
                Self::ShaderCrtStrength,
                Self::ShaderCrtScanlines,
                Self::ShaderCrtMask,
                Self::ShaderCrtCurvature,
                Self::ShaderCrtBloom,
                Self::ShaderCrtChroma,
                Self::ShaderFilmGrainStrength,
                Self::ShaderFilmGrainSize,
                Self::ShaderFilmGrainFps,
                Self::ShaderFilmGrainLumaBias,
                Self::ShaderRobotDeathStrength,
                Self::ShaderRobotStatic,
                Self::ShaderRobotTear,
                Self::ShaderRobotDesaturate,
                Self::ShaderRobotScanlines,
                Self::ShaderUnderwaterStrength,
                Self::ShaderUnderwaterDistortion,
                Self::ShaderVignetteStrength,
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
                Self::TouchControls,
                Self::MenuTapMode,
                Self::ResetControlFiltering,
                Self::Back,
            ],
            SettingsPage::Gameplay => &[
                Self::Difficulty,
                Self::Assist,
                Self::PlayerDamageMultiplier,
                Self::GameplayFlashes,
                Self::DebugHud,
                Self::QuestHud,
                Self::TraceAutoDump,
                Self::Back,
            ],
            SettingsPage::Developer => &[
                Self::DebugOverlay,
                Self::SlowMotion,
                Self::Inspector,
                Self::WorldInspector,
                Self::OverviewCamera,
                Self::ShowHitboxes,
                Self::MicroGrid,
                Self::CameraFrame,
                Self::PlayerBodyProfile,
                Self::MovementProfile,
                Self::LdtkAutoApply,
                Self::Back,
            ],
        }
    }

    /// Label shown to the user for this row, given the current
    /// settings snapshot.
    pub fn label(self, settings: &UserSettings) -> String {
        self.label_with_dev(settings, DevToggleSnapshot::default())
    }

    /// Variant of [`label`](Self::label) that knows about the
    /// developer-page toggles. Use this when rendering the Developer
    /// page so the toggle state shows correctly; non-developer rows
    /// ignore the snapshot.
    pub fn label_with_dev(self, settings: &UserSettings, dev: DevToggleSnapshot) -> String {
        match self {
            Self::OpenVideo => "Video >".into(),
            Self::OpenShaders => "Shaders >".into(),
            Self::OpenAudio => "Audio >".into(),
            Self::OpenControls => "Controls >".into(),
            Self::OpenGameplay => "Gameplay >".into(),
            Self::OpenDeveloper => "Developer >".into(),
            Self::Back => "Back".into(),

            Self::DisplayMode => format!(
                "Display Mode: {}  < / >",
                DisplayModeKind::from(settings.video.display_mode).label()
            ),
            Self::CameraZoom => {
                format!("Camera View: {}  < / >", settings.video.camera_zoom.label())
            }
            Self::CameraAspect => {
                format!(
                    "Camera Aspect: {}  < / >",
                    settings.video.camera_aspect.label()
                )
            }
            Self::CameraFraming => {
                format!(
                    "Camera Framing: {}  < / >",
                    settings.video.camera_framing.label()
                )
            }
            Self::Flashes => format!("Flashes: {}  < / >", settings.video.flashes.label()),
            Self::Colorblind => format!("Colorblind: {}  < / >", settings.video.colorblind.label()),
            Self::ShowFps => format!(
                "FPS Overlay: {}",
                if settings.video.show_fps { "on" } else { "off" }
            ),
            Self::ShaderStrength => format!(
                "Shader Strength: {}%  < / >",
                settings.video.shaders.strength_percent()
            ),
            Self::ShaderCrtStrength => format!(
                "CRT Strength: {}%  < / >",
                ScreenShaderSettings::percent(settings.video.shaders.crt_strength)
            ),
            Self::ShaderCrtScanlines => format!(
                "CRT Scanlines: {}%  < / >",
                ScreenShaderSettings::percent(settings.video.shaders.crt_scanlines)
            ),
            Self::ShaderCrtMask => format!(
                "CRT Phosphor Mask: {}%  < / >",
                ScreenShaderSettings::percent(settings.video.shaders.crt_mask)
            ),
            Self::ShaderCrtCurvature => format!(
                "CRT Curvature: {}%  < / >",
                ScreenShaderSettings::percent(settings.video.shaders.crt_curvature)
            ),
            Self::ShaderCrtBloom => format!(
                "CRT Bloom: {}%  < / >",
                ScreenShaderSettings::percent(settings.video.shaders.crt_bloom)
            ),
            Self::ShaderCrtChroma => format!(
                "CRT Chroma Split: {}%  < / >",
                ScreenShaderSettings::percent(settings.video.shaders.crt_chroma)
            ),
            Self::ShaderFilmGrainStrength => format!(
                "Film Grain Strength: {}%  < / >",
                ScreenShaderSettings::percent(settings.video.shaders.film_grain_strength)
            ),
            Self::ShaderFilmGrainSize => format!(
                "Film Grain Size: {:.0}px  < / >",
                settings.video.shaders.film_grain_size
            ),
            Self::ShaderFilmGrainFps => format!(
                "Film Grain Rate: {:.0} fps  < / >",
                settings.video.shaders.film_grain_fps
            ),
            Self::ShaderFilmGrainLumaBias => format!(
                "Film Grain Luma Bias: {}%  < / >",
                ScreenShaderSettings::percent(settings.video.shaders.film_grain_luma_bias)
            ),
            Self::ShaderRobotDeathStrength => format!(
                "Robot Death Strength: {}%  < / >",
                ScreenShaderSettings::percent(settings.video.shaders.robot_death_strength)
            ),
            Self::ShaderRobotStatic => format!(
                "Robot Static: {}%  < / >",
                ScreenShaderSettings::percent(settings.video.shaders.robot_static)
            ),
            Self::ShaderRobotTear => format!(
                "Robot Tear: {}%  < / >",
                ScreenShaderSettings::percent(settings.video.shaders.robot_tear)
            ),
            Self::ShaderRobotDesaturate => format!(
                "Robot Desaturate: {}%  < / >",
                ScreenShaderSettings::percent(settings.video.shaders.robot_desaturate)
            ),
            Self::ShaderRobotScanlines => format!(
                "Robot Scanlines: {}%  < / >",
                ScreenShaderSettings::percent(settings.video.shaders.robot_scanlines)
            ),
            Self::ShaderUnderwaterStrength => format!(
                "Underwater Strength: {}%  < / >",
                ScreenShaderSettings::percent(settings.video.shaders.underwater_strength)
            ),
            Self::ShaderUnderwaterDistortion => format!(
                "Underwater Distortion: {}%  < / >",
                ScreenShaderSettings::percent(settings.video.shaders.underwater_distortion)
            ),
            Self::ShaderVignetteStrength => format!(
                "Vignette Strength: {}%  < / >",
                ScreenShaderSettings::percent(settings.video.shaders.vignette_strength)
            ),

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
            Self::TouchControls => format!(
                "Touch Controls: {}",
                if settings.controls.touch_controls_visible {
                    "on"
                } else {
                    "off"
                }
            ),
            Self::MenuTapMode => format!(
                "Menu Tap: {}  < / >",
                settings.controls.menu_tap_mode.label()
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
            Self::DebugHud => format!(
                "Debug HUD: {}",
                if settings.gameplay.debug_hud_visible {
                    "on"
                } else {
                    "off"
                }
            ),
            Self::QuestHud => format!(
                "Quest HUD: {}",
                if settings.gameplay.quest_hud_visible {
                    "on"
                } else {
                    "off"
                }
            ),
            Self::TraceAutoDump => format!(
                "Trace Auto-Dump: {}",
                if settings.gameplay.trace_auto_dump {
                    "on"
                } else {
                    "off"
                }
            ),

            Self::DebugOverlay => format!("Debug Overlay (F1): {}", on_off(dev.debug_overlay)),
            Self::SlowMotion => format!("Slow Motion (F2): {}", on_off(dev.slowmo)),
            Self::Inspector => format!("Inspector (F3): {}", on_off(dev.inspector)),
            Self::WorldInspector => {
                format!("World Inspector (F4): {}", on_off(dev.world_inspector))
            }
            Self::OverviewCamera => {
                format!("Overview Camera (F5): {}", on_off(dev.overview_camera))
            }
            Self::ShowHitboxes => {
                format!("Show Hitboxes: {}", on_off(dev.show_hitboxes))
            }
            Self::MicroGrid => {
                format!("Micro Grid (8px): {}", on_off(dev.micro_grid))
            }
            Self::CameraFrame => {
                format!("Camera Frame: {}", on_off(dev.camera_frame))
            }
            Self::PlayerBodyProfile => {
                format!("Player Body: {}  < / >", dev.player_body_profile.label())
            }
            Self::MovementProfile => {
                format!("Movement Profile: {}  < / >", dev.movement_profile.label())
            }
            Self::LdtkAutoApply => {
                format!("LDtk Auto-Reload (F12): {}", on_off(dev.ldtk_auto_apply))
            }
        }
    }
}

fn on_off(value: bool) -> &'static str {
    if value {
        "on"
    } else {
        "off"
    }
}

/// Snapshot of the developer-page toggles, sampled from the live
/// resources (`DeveloperTools`, `LdtkHotReloadState`)
/// so the pause-menu renderer can label rows without holding `Res`
/// handles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DevToggleSnapshot {
    pub debug_overlay: bool,
    pub slowmo: bool,
    pub inspector: bool,
    pub world_inspector: bool,
    pub overview_camera: bool,
    pub show_hitboxes: bool,
    pub micro_grid: bool,
    pub camera_frame: bool,
    pub player_body_profile: crate::dev_tools::PlayerBodyProfile,
    pub movement_profile: crate::dev_tools::MovementProfile,
    pub ldtk_auto_apply: bool,
}

impl DevToggleSnapshot {
    pub fn capture(
        dev_state: &SandboxDevState,
        developer: &DeveloperTools,
        ldtk_reload: &LdtkHotReloadState,
    ) -> Self {
        Self {
            debug_overlay: dev_state.debug_enabled(),
            slowmo: dev_state.slowmo,
            inspector: developer.inspector_visible,
            world_inspector: developer.world_inspector_visible,
            overview_camera: developer.overview_camera,
            show_hitboxes: developer.show_feature_hitboxes,
            micro_grid: developer.show_micro_grid,
            camera_frame: developer.show_camera_frame,
            player_body_profile: developer.player_body_profile,
            movement_profile: developer.movement_profile,
            ldtk_auto_apply: ldtk_reload.auto_apply,
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
#[allow(clippy::too_many_arguments)]
pub fn apply_action(
    item: SettingsItem,
    action: SettingsAction,
    settings: &mut UserSettings,
    display_state: &mut DisplayModeState,
    windows: &mut Query<&mut Window, With<PrimaryWindow>>,
    keyboard_preset_count: usize,
    dev_state: &mut SandboxDevState,
    developer: &mut DeveloperTools,
    editable_tuning: &mut EditableMovementTuning,
    ldtk_reload: &mut LdtkHotReloadState,
    authority_player: Option<&mut ambition_engine::Player>,
) -> SettingsOutcome {
    match item {
        SettingsItem::OpenVideo => {
            if matches!(action, SettingsAction::Confirm) {
                return SettingsOutcome::OpenPage(SettingsPage::Video);
            }
        }
        SettingsItem::OpenShaders => {
            if matches!(action, SettingsAction::Confirm) {
                return SettingsOutcome::OpenPage(SettingsPage::Shaders);
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
        SettingsItem::OpenDeveloper => {
            if matches!(action, SettingsAction::Confirm) {
                return SettingsOutcome::OpenPage(SettingsPage::Developer);
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
        SettingsItem::CameraAspect => match action {
            SettingsAction::Prev => {
                settings.video.camera_aspect = settings.video.camera_aspect.prev()
            }
            SettingsAction::Next | SettingsAction::Confirm => {
                settings.video.camera_aspect = settings.video.camera_aspect.next()
            }
        },
        SettingsItem::CameraFraming => match action {
            SettingsAction::Prev => {
                settings.video.camera_framing = settings.video.camera_framing.prev()
            }
            SettingsAction::Next | SettingsAction::Confirm => {
                settings.video.camera_framing = settings.video.camera_framing.next()
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
        SettingsItem::TouchControls => {
            if matches!(
                action,
                SettingsAction::Confirm | SettingsAction::Next | SettingsAction::Prev
            ) {
                settings.controls.touch_controls_visible =
                    !settings.controls.touch_controls_visible;
            }
        }
        SettingsItem::MenuTapMode => match action {
            SettingsAction::Prev => {
                settings.controls.menu_tap_mode = settings.controls.menu_tap_mode.prev();
            }
            SettingsAction::Next | SettingsAction::Confirm => {
                settings.controls.menu_tap_mode = settings.controls.menu_tap_mode.next();
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
        SettingsItem::DebugHud => {
            if matches!(
                action,
                SettingsAction::Confirm | SettingsAction::Next | SettingsAction::Prev
            ) {
                settings.gameplay.debug_hud_visible = !settings.gameplay.debug_hud_visible;
            }
        }
        SettingsItem::ShowFps => {
            if matches!(
                action,
                SettingsAction::Confirm | SettingsAction::Next | SettingsAction::Prev
            ) {
                settings.video.show_fps = !settings.video.show_fps;
            }
        }
        SettingsItem::ShaderStrength => match action {
            SettingsAction::Prev => settings
                .video
                .shaders
                .nudge_strength(-ScreenShaderSettings::UNIT_STEP),
            SettingsAction::Next | SettingsAction::Confirm => settings
                .video
                .shaders
                .nudge_strength(ScreenShaderSettings::UNIT_STEP),
        },
        SettingsItem::ShaderCrtStrength => nudge_shader_unit(
            action,
            &mut settings.video.shaders.crt_strength,
            ScreenShaderSettings::UNIT_STEP,
        ),
        SettingsItem::ShaderCrtScanlines => nudge_shader_unit(
            action,
            &mut settings.video.shaders.crt_scanlines,
            ScreenShaderSettings::FINE_STEP,
        ),
        SettingsItem::ShaderCrtMask => nudge_shader_unit(
            action,
            &mut settings.video.shaders.crt_mask,
            ScreenShaderSettings::FINE_STEP,
        ),
        SettingsItem::ShaderCrtCurvature => nudge_shader_unit(
            action,
            &mut settings.video.shaders.crt_curvature,
            ScreenShaderSettings::FINE_STEP,
        ),
        SettingsItem::ShaderCrtBloom => nudge_shader_unit(
            action,
            &mut settings.video.shaders.crt_bloom,
            ScreenShaderSettings::FINE_STEP,
        ),
        SettingsItem::ShaderCrtChroma => nudge_shader_unit(
            action,
            &mut settings.video.shaders.crt_chroma,
            ScreenShaderSettings::FINE_STEP,
        ),
        SettingsItem::ShaderFilmGrainStrength => nudge_shader_unit(
            action,
            &mut settings.video.shaders.film_grain_strength,
            ScreenShaderSettings::FINE_STEP,
        ),
        SettingsItem::ShaderFilmGrainSize => nudge_shader_range(
            action,
            &mut settings.video.shaders.film_grain_size,
            ScreenShaderSettings::GRAIN_SIZE_STEP,
            1.0,
            8.0,
        ),
        SettingsItem::ShaderFilmGrainFps => nudge_shader_range(
            action,
            &mut settings.video.shaders.film_grain_fps,
            ScreenShaderSettings::GRAIN_FPS_STEP,
            1.0,
            60.0,
        ),
        SettingsItem::ShaderFilmGrainLumaBias => nudge_shader_unit(
            action,
            &mut settings.video.shaders.film_grain_luma_bias,
            ScreenShaderSettings::FINE_STEP,
        ),
        SettingsItem::ShaderRobotDeathStrength => nudge_shader_unit(
            action,
            &mut settings.video.shaders.robot_death_strength,
            ScreenShaderSettings::UNIT_STEP,
        ),
        SettingsItem::ShaderRobotStatic => nudge_shader_unit(
            action,
            &mut settings.video.shaders.robot_static,
            ScreenShaderSettings::FINE_STEP,
        ),
        SettingsItem::ShaderRobotTear => nudge_shader_unit(
            action,
            &mut settings.video.shaders.robot_tear,
            ScreenShaderSettings::FINE_STEP,
        ),
        SettingsItem::ShaderRobotDesaturate => nudge_shader_unit(
            action,
            &mut settings.video.shaders.robot_desaturate,
            ScreenShaderSettings::FINE_STEP,
        ),
        SettingsItem::ShaderRobotScanlines => nudge_shader_unit(
            action,
            &mut settings.video.shaders.robot_scanlines,
            ScreenShaderSettings::FINE_STEP,
        ),
        SettingsItem::ShaderUnderwaterStrength => nudge_shader_unit(
            action,
            &mut settings.video.shaders.underwater_strength,
            ScreenShaderSettings::UNIT_STEP,
        ),
        SettingsItem::ShaderUnderwaterDistortion => nudge_shader_unit(
            action,
            &mut settings.video.shaders.underwater_distortion,
            ScreenShaderSettings::FINE_STEP,
        ),
        SettingsItem::ShaderVignetteStrength => nudge_shader_unit(
            action,
            &mut settings.video.shaders.vignette_strength,
            ScreenShaderSettings::FINE_STEP,
        ),
        SettingsItem::QuestHud => {
            if matches!(
                action,
                SettingsAction::Confirm | SettingsAction::Next | SettingsAction::Prev
            ) {
                settings.gameplay.quest_hud_visible = !settings.gameplay.quest_hud_visible;
            }
        }
        SettingsItem::TraceAutoDump => {
            if matches!(
                action,
                SettingsAction::Confirm | SettingsAction::Next | SettingsAction::Prev
            ) {
                settings.gameplay.trace_auto_dump = !settings.gameplay.trace_auto_dump;
            }
        }

        SettingsItem::DebugOverlay => {
            if is_toggle_action(action) {
                dev_state.debug = !dev_state.debug;
            }
        }
        SettingsItem::SlowMotion => {
            if is_toggle_action(action) {
                dev_state.slowmo = !dev_state.slowmo;
            }
        }
        SettingsItem::Inspector => {
            if is_toggle_action(action) {
                developer.inspector_visible = !developer.inspector_visible;
            }
        }
        SettingsItem::WorldInspector => {
            if is_toggle_action(action) {
                developer.world_inspector_visible = !developer.world_inspector_visible;
            }
        }
        SettingsItem::OverviewCamera => {
            if is_toggle_action(action) {
                developer.overview_camera = !developer.overview_camera;
            }
        }
        SettingsItem::ShowHitboxes => {
            if is_toggle_action(action) {
                let next = !developer.show_feature_hitboxes;
                developer.show_feature_hitboxes = next;
                developer.show_player_hitbox = next;
            }
        }
        SettingsItem::MicroGrid => {
            if is_toggle_action(action) {
                developer.show_micro_grid = !developer.show_micro_grid;
            }
        }
        SettingsItem::CameraFrame => {
            if is_toggle_action(action) {
                developer.show_camera_frame = !developer.show_camera_frame;
            }
        }
        SettingsItem::PlayerBodyProfile => match action {
            SettingsAction::Prev => {
                developer.player_body_profile = developer.player_body_profile.prev();
                if let Some(player) = authority_player {
                    apply_player_body_profile(player, developer.player_body_profile);
                }
            }
            SettingsAction::Next | SettingsAction::Confirm => {
                developer.player_body_profile = developer.player_body_profile.next();
                if let Some(player) = authority_player {
                    apply_player_body_profile(player, developer.player_body_profile);
                }
            }
        },
        SettingsItem::MovementProfile => match action {
            SettingsAction::Prev => {
                developer.movement_profile = developer.movement_profile.prev();
                apply_movement_profile(editable_tuning, developer.movement_profile, authority_player);
            }
            SettingsAction::Next | SettingsAction::Confirm => {
                developer.movement_profile = developer.movement_profile.next();
                apply_movement_profile(editable_tuning, developer.movement_profile, authority_player);
            }
        },
        SettingsItem::LdtkAutoApply => {
            if is_toggle_action(action) {
                ldtk_reload.auto_apply = !ldtk_reload.auto_apply;
                ldtk_reload.last_status = format!(
                    "LDtk auto-apply {}",
                    if ldtk_reload.auto_apply {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
            }
        }
    }
    SettingsOutcome::Stay
}


/// Slider cap for the player-damage multiplier. The underlying field
/// clamps to `[0.25, 4.0]` (see `GameplaySettings::nudge_player_damage`),
/// but exposing the full range on a 0..1 slider would mean "default"
/// (1.0) lives at the 25% mark, which reads as "I should slide right".
/// Capping the slider at 2.0 gives a 0..1 bar where 0.5 ≈ default and
/// 1.0 ≈ glass-cannon mode; users who want higher have to nudge with
/// keyboard `>`. Matches the spirit of the existing `< / >` nudge UX.
pub const PLAYER_DAMAGE_SLIDER_MAX: f32 = 2.0;

impl SettingsItem {
    /// Read the row's value as a normalized `0.0..=1.0` slider position.
    /// Returns `None` for rows that aren't scalar percent-style settings
    /// (enums, toggles, navigation rows, non-percent ranges). Used by
    /// the touch slider widget; keyboard `<` / `>` continues to drive
    /// the same value through `apply_action`.
    pub fn normalized_value(self, settings: &UserSettings) -> Option<f32> {
        match self {
            Self::ShaderStrength => Some(settings.video.shaders.strength),
            Self::ShaderCrtStrength => Some(settings.video.shaders.crt_strength),
            Self::ShaderCrtScanlines => Some(settings.video.shaders.crt_scanlines),
            Self::ShaderCrtMask => Some(settings.video.shaders.crt_mask),
            Self::ShaderCrtCurvature => Some(settings.video.shaders.crt_curvature),
            Self::ShaderCrtBloom => Some(settings.video.shaders.crt_bloom),
            Self::ShaderCrtChroma => Some(settings.video.shaders.crt_chroma),
            Self::ShaderFilmGrainStrength => Some(settings.video.shaders.film_grain_strength),
            Self::ShaderFilmGrainLumaBias => Some(settings.video.shaders.film_grain_luma_bias),
            Self::ShaderRobotDeathStrength => Some(settings.video.shaders.robot_death_strength),
            Self::ShaderRobotStatic => Some(settings.video.shaders.robot_static),
            Self::ShaderRobotTear => Some(settings.video.shaders.robot_tear),
            Self::ShaderRobotDesaturate => Some(settings.video.shaders.robot_desaturate),
            Self::ShaderRobotScanlines => Some(settings.video.shaders.robot_scanlines),
            Self::ShaderUnderwaterStrength => Some(settings.video.shaders.underwater_strength),
            Self::ShaderUnderwaterDistortion => Some(settings.video.shaders.underwater_distortion),
            Self::ShaderVignetteStrength => Some(settings.video.shaders.vignette_strength),
            Self::MasterVolume => Some(settings.audio.master_volume),
            Self::MusicVolume => Some(settings.audio.music_volume),
            Self::SfxVolume => Some(settings.audio.sfx_volume),
            // Stick deadzones top out at 0.6 internally; expose them as
            // 0..1 on the slider so the bar represents "drag fraction
            // of allowed range" rather than a misleading 0..100% the
            // engine never honors.
            Self::LeftStickDeadzone => Some(settings.controls.left_stick_deadzone / 0.6),
            Self::RightStickDeadzone => Some(settings.controls.right_stick_deadzone / 0.6),
            // Trigger press clamps to [0.05, 1.0]; map back to a
            // 0..1 slider that represents the live press level.
            Self::TriggerPress => {
                Some(((settings.controls.trigger_press_threshold - 0.05) / 0.95).clamp(0.0, 1.0))
            }
            // Trigger release clamps to [0.0, 0.95]; map back to a
            // 0..1 slider.
            Self::TriggerRelease => {
                Some((settings.controls.trigger_release_threshold / 0.95).clamp(0.0, 1.0))
            }
            Self::PlayerDamageMultiplier => Some(
                (settings.gameplay.player_damage_multiplier
                    / PLAYER_DAMAGE_SLIDER_MAX)
                    .clamp(0.0, 1.0),
            ),
            _ => None,
        }
    }

    /// Write a normalized `0.0..=1.0` slider position back to the row's
    /// underlying value. Returns `true` if the row accepted the write
    /// (i.e. is a slider row). Inverse of [`normalized_value`].
    pub fn try_set_normalized(self, settings: &mut UserSettings, value: f32) -> bool {
        let v = value.clamp(0.0, 1.0);
        match self {
            Self::ShaderStrength => settings.video.shaders.strength = v,
            Self::ShaderCrtStrength => settings.video.shaders.crt_strength = v,
            Self::ShaderCrtScanlines => settings.video.shaders.crt_scanlines = v,
            Self::ShaderCrtMask => settings.video.shaders.crt_mask = v,
            Self::ShaderCrtCurvature => settings.video.shaders.crt_curvature = v,
            Self::ShaderCrtBloom => settings.video.shaders.crt_bloom = v,
            Self::ShaderCrtChroma => settings.video.shaders.crt_chroma = v,
            Self::ShaderFilmGrainStrength => settings.video.shaders.film_grain_strength = v,
            Self::ShaderFilmGrainLumaBias => settings.video.shaders.film_grain_luma_bias = v,
            Self::ShaderRobotDeathStrength => settings.video.shaders.robot_death_strength = v,
            Self::ShaderRobotStatic => settings.video.shaders.robot_static = v,
            Self::ShaderRobotTear => settings.video.shaders.robot_tear = v,
            Self::ShaderRobotDesaturate => settings.video.shaders.robot_desaturate = v,
            Self::ShaderRobotScanlines => settings.video.shaders.robot_scanlines = v,
            Self::ShaderUnderwaterStrength => settings.video.shaders.underwater_strength = v,
            Self::ShaderUnderwaterDistortion => settings.video.shaders.underwater_distortion = v,
            Self::ShaderVignetteStrength => settings.video.shaders.vignette_strength = v,
            Self::MasterVolume => settings.audio.master_volume = v,
            Self::MusicVolume => settings.audio.music_volume = v,
            Self::SfxVolume => settings.audio.sfx_volume = v,
            Self::LeftStickDeadzone => settings.controls.left_stick_deadzone = v * 0.6,
            Self::RightStickDeadzone => settings.controls.right_stick_deadzone = v * 0.6,
            Self::TriggerPress => settings.controls.trigger_press_threshold = 0.05 + v * 0.95,
            Self::TriggerRelease => settings.controls.trigger_release_threshold = v * 0.95,
            Self::PlayerDamageMultiplier => {
                settings.gameplay.player_damage_multiplier =
                    v * PLAYER_DAMAGE_SLIDER_MAX;
            }
            _ => return false,
        }
        // Re-run the per-category clamps so any cross-field invariants
        // (trigger press > release, audio mute, etc.) stay healthy.
        settings.video.shaders.clamp_all();
        settings.audio.clamp_all();
        settings.controls.clamp_all();
        settings.gameplay.clamp_all();
        true
    }
}

fn nudge_shader_unit(action: SettingsAction, value: &mut f32, step: f32) {
    match action {
        SettingsAction::Prev => ScreenShaderSettings::nudge_unit(value, -step),
        SettingsAction::Next | SettingsAction::Confirm => {
            ScreenShaderSettings::nudge_unit(value, step);
        }
    }
}

fn nudge_shader_range(action: SettingsAction, value: &mut f32, step: f32, min: f32, max: f32) {
    match action {
        SettingsAction::Prev => ScreenShaderSettings::nudge_range(value, -step, min, max),
        SettingsAction::Next | SettingsAction::Confirm => {
            ScreenShaderSettings::nudge_range(value, step, min, max);
        }
    }
}

fn is_toggle_action(action: SettingsAction) -> bool {
    matches!(
        action,
        SettingsAction::Confirm | SettingsAction::Next | SettingsAction::Prev
    )
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
