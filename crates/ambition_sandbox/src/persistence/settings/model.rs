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
use super::controls::{ControllerProfileId, DashInputMode, MenuTapMode};
use super::gameplay::{Difficulty, GameplaySettings};
use super::video::{
    CameraAspectPolicy, CameraFramingPreset, CameraZoomPreset, ColorblindMode, FlashIntensity,
    ScreenShaderSettings, SerializableDisplayMode,
};
use super::UserSettings;
use crate::dev::dev_tools::{
    apply_movement_profile, apply_player_body_profile, DebugArtMode, DebugViewMode, DeveloperTools,
    EditableMovementTuning,
};
use crate::host::windowing::{DisplayModeKind, DisplayModeState};
use crate::ldtk_world::LdtkHotReloadState;
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

    #[allow(dead_code)] // Iterator-friendly handle on every page; reserved for future docs/tests.
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
    /// Reset every persisted resource (user settings + developer
    /// tools) back to their default values. Surfaced on the Top
    /// page so it can recover from any sub-page nudge that broke
    /// the build (rare, but the only way out otherwise is to
    /// hand-edit the .ron files on disk).
    ResetAllSettings,
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
    ShaderDeepDreamStrength,
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
    DebugViewMode,
    DebugArtMode,
    ShowHitboxes,
    FillDebugBoxes,
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
                Self::ResetAllSettings,
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
                Self::ShaderDeepDreamStrength,
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
                Self::DebugViewMode,
                Self::DebugArtMode,
                Self::ShowHitboxes,
                Self::FillDebugBoxes,
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
    ///
    /// Convenience wrapper around [`label_with_dev`](Self::label_with_dev);
    /// the snapshot defaults to all-off for callers that don't render
    /// the developer page.
    #[cfg(test)]
    pub fn label(self, settings: &UserSettings) -> String {
        self.label_with_dev(settings, DevToggleSnapshot::default())
    }

    /// Variant of `label` that knows about the developer-page
    /// toggles. Use this when rendering the Developer page so the
    /// toggle state shows correctly; non-developer rows ignore the
    /// snapshot.
    pub fn label_with_dev(self, settings: &UserSettings, dev: DevToggleSnapshot) -> String {
        // Page-navigation rows have static labels held in
        // `PAGE_NAV_ROWS`; everything else has dynamic content so
        // falls through to the per-variant match below.
        if let Some(label) = page_nav_label(self) {
            return label.into();
        }
        match self {
            Self::OpenVideo
            | Self::OpenShaders
            | Self::OpenAudio
            | Self::OpenControls
            | Self::OpenGameplay
            | Self::OpenDeveloper
            | Self::Back => unreachable!("page-nav rows handled by page_nav_label above"),
            Self::ResetAllSettings => "Reset All Settings to Defaults".into(),

            Self::DisplayMode => format_cycle(
                "Display Mode",
                DisplayModeKind::from(settings.video.display_mode).label(),
            ),
            Self::CameraZoom => format_cycle("Camera View", settings.video.camera_zoom.label()),
            Self::CameraAspect => {
                format_cycle("Camera Aspect", settings.video.camera_aspect.label())
            }
            Self::CameraFraming => {
                format_cycle("Camera Framing", settings.video.camera_framing.label())
            }
            Self::Flashes => format_cycle("Flashes", settings.video.flashes.label()),
            Self::Colorblind => format_cycle("Colorblind", settings.video.colorblind.label()),
            Self::ShowFps => format_toggle("FPS Overlay", settings.video.show_fps),
            Self::ShaderStrength => {
                format_shader_percent("Shader Strength", settings.video.shaders.strength)
            }
            Self::ShaderCrtStrength => {
                format_shader_percent("CRT Strength", settings.video.shaders.crt_strength)
            }
            Self::ShaderCrtScanlines => {
                format_shader_percent("CRT Scanlines", settings.video.shaders.crt_scanlines)
            }
            Self::ShaderCrtMask => {
                format_shader_percent("CRT Phosphor Mask", settings.video.shaders.crt_mask)
            }
            Self::ShaderCrtCurvature => {
                format_shader_percent("CRT Curvature", settings.video.shaders.crt_curvature)
            }
            Self::ShaderCrtBloom => {
                format_shader_percent("CRT Bloom", settings.video.shaders.crt_bloom)
            }
            Self::ShaderCrtChroma => {
                format_shader_percent("CRT Chroma Split", settings.video.shaders.crt_chroma)
            }
            Self::ShaderFilmGrainStrength => format_shader_percent(
                "Film Grain Strength",
                settings.video.shaders.film_grain_strength,
            ),
            Self::ShaderFilmGrainSize => format_cycle(
                "Film Grain Size",
                format!("{:.0}px", settings.video.shaders.film_grain_size),
            ),
            Self::ShaderFilmGrainFps => format_cycle(
                "Film Grain Rate",
                format!("{:.0} fps", settings.video.shaders.film_grain_fps),
            ),
            Self::ShaderFilmGrainLumaBias => format_shader_percent(
                "Film Grain Luma Bias",
                settings.video.shaders.film_grain_luma_bias,
            ),
            Self::ShaderRobotDeathStrength => format_shader_percent(
                "Robot Death Strength",
                settings.video.shaders.robot_death_strength,
            ),
            Self::ShaderRobotStatic => {
                format_shader_percent("Robot Static", settings.video.shaders.robot_static)
            }
            Self::ShaderRobotTear => {
                format_shader_percent("Robot Tear", settings.video.shaders.robot_tear)
            }
            Self::ShaderRobotDesaturate => {
                format_shader_percent("Robot Desaturate", settings.video.shaders.robot_desaturate)
            }
            Self::ShaderRobotScanlines => {
                format_shader_percent("Robot Scanlines", settings.video.shaders.robot_scanlines)
            }
            Self::ShaderUnderwaterStrength => format_shader_percent(
                "Underwater Strength",
                settings.video.shaders.underwater_strength,
            ),
            Self::ShaderUnderwaterDistortion => format_shader_percent(
                "Underwater Distortion",
                settings.video.shaders.underwater_distortion,
            ),
            Self::ShaderDeepDreamStrength => format_shader_percent(
                "Deep Dream Strength",
                settings.video.shaders.deep_dream_strength,
            ),
            Self::ShaderVignetteStrength => format_shader_percent(
                "Vignette Strength",
                settings.video.shaders.vignette_strength,
            ),

            Self::MasterVolume => {
                format_audio_percent("Master Volume", settings.audio.master_volume)
            }
            Self::MusicVolume => format_audio_percent("Music Volume", settings.audio.music_volume),
            Self::SfxVolume => format_audio_percent("SFX Volume", settings.audio.sfx_volume),
            Self::Mute => format!(
                "Mute: {}",
                if settings.audio.muted { "muted" } else { "off" }
            ),

            Self::KeyboardPreset => {
                format_cycle("Keyboard Preset", settings.controls.keyboard_preset_index)
            }
            Self::ControllerProfile => {
                format_cycle("Controller", settings.controls.controller_profile.label())
            }
            Self::LeftStickDeadzone => {
                format_audio_percent("L-Stick Deadzone", settings.controls.left_stick_deadzone)
            }
            Self::RightStickDeadzone => {
                format_audio_percent("R-Stick Deadzone", settings.controls.right_stick_deadzone)
            }
            Self::TriggerPress => {
                format_audio_percent("Trigger Press", settings.controls.trigger_press_threshold)
            }
            Self::TriggerRelease => format_audio_percent(
                "Trigger Release",
                settings.controls.trigger_release_threshold,
            ),
            Self::DpadMenuNav => {
                format_toggle("D-Pad Menu Nav", settings.controls.dpad_menu_navigation)
            }
            Self::InvertAimY => format_toggle("Invert Aim Y", settings.controls.invert_aim_y),
            Self::DashInputMode => {
                format_cycle("Dash Input", settings.controls.dash_input_mode.label())
            }
            Self::TouchControls => {
                format_toggle("Touch Overlay", settings.controls.touch_controls_visible)
            }
            Self::MenuTapMode => format_cycle("Menu Tap", settings.controls.menu_tap_mode.label()),
            Self::ResetControlFiltering => "Reset Filter Defaults".into(),

            Self::Difficulty => format_cycle("Difficulty", settings.gameplay.difficulty.label()),
            Self::Assist => format!("Assist: {}", settings.gameplay.assist.label()),
            Self::PlayerDamageMultiplier => format_cycle(
                "Player Damage",
                format!("x{:.2}", settings.gameplay.player_damage_multiplier),
            ),
            Self::GameplayFlashes => {
                format_cycle("Flashes (gameplay)", settings.video.flashes.label())
            }
            Self::DebugHud => format_toggle("Debug HUD", settings.gameplay.debug_hud_visible),
            Self::QuestHud => format_toggle("Quest HUD", settings.gameplay.quest_hud_visible),
            Self::TraceAutoDump => {
                format_toggle("Trace Auto-Dump", settings.gameplay.trace_auto_dump)
            }

            Self::DebugOverlay => format_toggle("Debug Overlay (F1)", dev.debug_overlay),
            Self::SlowMotion => format_toggle("Slow Motion (F2)", dev.slowmo),
            Self::Inspector => format_toggle("Inspector (F3)", dev.inspector),
            Self::WorldInspector => format_toggle("World Inspector (F4)", dev.world_inspector),
            Self::OverviewCamera => format_toggle("Overview Camera (F5)", dev.overview_camera),
            Self::DebugViewMode => format_cycle("Debug View", dev.debug_view_mode.label()),
            Self::DebugArtMode => format_cycle("Debug Art", dev.debug_art_mode.label()),
            Self::ShowHitboxes => format_toggle("Custom Hitboxes", dev.show_hitboxes),
            Self::FillDebugBoxes => format_toggle("Debug Fills", dev.fill_debug_boxes),
            Self::MicroGrid => format_toggle("Micro Grid (8px)", dev.micro_grid),
            Self::CameraFrame => format_toggle("Camera Frame", dev.camera_frame),
            Self::PlayerBodyProfile => format_cycle("Player Body", dev.player_body_profile.label()),
            Self::MovementProfile => format_cycle("Movement Profile", dev.movement_profile.label()),
            Self::LdtkAutoApply => format_toggle("LDtk Auto-Reload (F12)", dev.ldtk_auto_apply),
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

/// `Label: <value>  < / >` — the shared cycle/nudge row format.
/// Use this instead of an inline `format!("…: {} < / >", …)` so the
/// "value-on-the-right + arrows" UX shape stays uniform across pages.
/// Display values that already render the way the row should read
/// (e.g. an enum's `label()` method) pass through directly.
fn format_cycle(label: &str, value: impl std::fmt::Display) -> String {
    format!("{label}: {value}  < / >")
}

/// `Label: NN%  < / >` — the shared format for every 0..1 shader slider
/// row. Pulled out of the 17 near-identical `Shader*` arms in
/// [`SettingsItem::label_with_dev`] so adding a new shader slider is a
/// one-line label change rather than a five-line `format!` boilerplate.
fn format_shader_percent(label: &str, value: f32) -> String {
    format_cycle(label, format!("{}%", ScreenShaderSettings::percent(value)))
}

/// `Label: NN%  < / >` — same shape as [`format_shader_percent`] but
/// using [`AudioSettings::percent`] (also clamping to 0..1 but at the
/// audio settings layer). Used by the volume sliders and the controller
/// deadzone / trigger threshold rows.
fn format_audio_percent(label: &str, value: f32) -> String {
    format_cycle(label, format!("{}%", AudioSettings::percent(value)))
}

/// `Label: on|off` — the shared format for every two-state toggle row
/// that wraps a plain `bool` field. Routed through [`on_off`] so the
/// developer toggles, gameplay toggles, and controls toggles all read
/// the same in the menu.
fn format_toggle(label: &str, value: bool) -> String {
    format!("{label}: {}", on_off(value))
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
    pub debug_view_mode: DebugViewMode,
    pub debug_art_mode: DebugArtMode,
    pub show_hitboxes: bool,
    pub hide_sprites: bool,
    pub placeholder_sprites: bool,
    pub fill_debug_boxes: bool,
    pub micro_grid: bool,
    pub camera_frame: bool,
    pub player_body_profile: crate::dev::dev_tools::PlayerBodyProfile,
    pub movement_profile: crate::dev::dev_tools::MovementProfile,
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
            debug_view_mode: developer.debug_view_mode,
            debug_art_mode: developer.debug_art_mode,
            show_hitboxes: developer.show_feature_hitboxes,
            hide_sprites: developer.hide_sprites,
            placeholder_sprites: developer.placeholder_sprites,
            fill_debug_boxes: developer.fill_debug_boxes,
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

/// Authored descriptor for a navigation-style row. Each entry pairs
/// the row with its static label and the outcome `apply_action`
/// should produce on `Confirm`. Keeps the page-nav match arms
/// collapsed to one table lookup so adding a new sub-page is a
/// one-row change.
struct PageNavRow {
    item: SettingsItem,
    label: &'static str,
    outcome: SettingsOutcome,
}

const PAGE_NAV_ROWS: &[PageNavRow] = &[
    PageNavRow {
        item: SettingsItem::OpenVideo,
        label: "Video >",
        outcome: SettingsOutcome::OpenPage(SettingsPage::Video),
    },
    PageNavRow {
        item: SettingsItem::OpenShaders,
        label: "Shaders >",
        outcome: SettingsOutcome::OpenPage(SettingsPage::Shaders),
    },
    PageNavRow {
        item: SettingsItem::OpenAudio,
        label: "Audio >",
        outcome: SettingsOutcome::OpenPage(SettingsPage::Audio),
    },
    PageNavRow {
        item: SettingsItem::OpenControls,
        label: "Controls >",
        outcome: SettingsOutcome::OpenPage(SettingsPage::Controls),
    },
    PageNavRow {
        item: SettingsItem::OpenGameplay,
        label: "Gameplay >",
        outcome: SettingsOutcome::OpenPage(SettingsPage::Gameplay),
    },
    PageNavRow {
        item: SettingsItem::OpenDeveloper,
        label: "Developer >",
        outcome: SettingsOutcome::OpenPage(SettingsPage::Developer),
    },
    PageNavRow {
        item: SettingsItem::Back,
        label: "Back",
        outcome: SettingsOutcome::PopPage,
    },
];

/// Run `on()` exactly when the row received a settling action
/// (Confirm / Prev / Next all collapse to the same "the user pressed
/// the row" semantic for toggles). Skips Stay/Refresh/etc.
fn apply_toggle<F: FnOnce()>(action: SettingsAction, on: F) {
    if matches!(
        action,
        SettingsAction::Confirm | SettingsAction::Next | SettingsAction::Prev
    ) {
        on();
    }
}

/// Drive a `prev()` / `next()` cycle row: `Prev` runs `prev`,
/// everything else runs `next`. The two function pointers come from
/// the field's own enum (`CameraZoomPreset::prev` etc.).
fn apply_cycle<T: Copy>(action: SettingsAction, field: &mut T, prev: fn(T) -> T, next: fn(T) -> T) {
    *field = match action {
        SettingsAction::Prev => prev(*field),
        SettingsAction::Next | SettingsAction::Confirm => next(*field),
    };
}

/// Resolve a slider-style `Prev` / `Next` press into a signed step.
/// Confirm is treated as Next so a tap-without-direction still nudges.
fn nudge_delta(action: SettingsAction, step: f32) -> f32 {
    match action {
        SettingsAction::Prev => -step,
        SettingsAction::Next | SettingsAction::Confirm => step,
    }
}

/// Look up the `SettingsOutcome` for a page-navigation row, if `item`
/// is one of those rows. Non-navigation items return `None` so the
/// main `apply_action` match can dispatch them.
fn page_nav_outcome(item: SettingsItem) -> Option<SettingsOutcome> {
    PAGE_NAV_ROWS
        .iter()
        .find(|row| row.item == item)
        .map(|row| row.outcome)
}

/// Look up the static label for a page-navigation row.
fn page_nav_label(item: SettingsItem) -> Option<&'static str> {
    PAGE_NAV_ROWS
        .iter()
        .find(|row| row.item == item)
        .map(|row| row.label)
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
    live_movement_refs: Option<(
        &mut crate::player::BodyKinematics,
        &crate::player::PlayerAbilities,
        &mut crate::player::PlayerDashState,
        &mut crate::player::PlayerJumpState,
    )>,
) -> SettingsOutcome {
    // Page-navigation rows (Open* + Back) share identical behavior:
    // `Confirm` → push/pop the page in `PAGE_NAV_ROWS`. Dispatch them
    // through the table so adding a new sub-page is a one-row change
    // and the per-variant match arms stay focused on rows with
    // genuinely distinct logic.
    if matches!(action, SettingsAction::Confirm) {
        if let Some(outcome) = page_nav_outcome(item) {
            return outcome;
        }
    }
    match item {
        // Page-navigation rows handled by `page_nav_outcome` above.
        SettingsItem::OpenVideo
        | SettingsItem::OpenShaders
        | SettingsItem::OpenAudio
        | SettingsItem::OpenControls
        | SettingsItem::OpenGameplay
        | SettingsItem::OpenDeveloper
        | SettingsItem::Back => {}
        SettingsItem::ResetAllSettings => {
            // Only react to Confirm — Prev/Next would let a stray
            // d-pad nudge wipe everything on the highlighted row.
            if matches!(action, SettingsAction::Confirm) {
                *settings = UserSettings::default();
                *developer = DeveloperTools::default();
                // Keep dependent state coherent with the new dev
                // defaults: editable movement tuning is derived from
                // the active movement profile.
                apply_movement_profile(
                    editable_tuning,
                    developer.movement_profile,
                    live_movement_refs.map(|(_, a, d, j)| (a, d, j)),
                );
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
        SettingsItem::CameraZoom => apply_cycle(
            action,
            &mut settings.video.camera_zoom,
            CameraZoomPreset::prev,
            CameraZoomPreset::next,
        ),
        SettingsItem::CameraAspect => apply_cycle(
            action,
            &mut settings.video.camera_aspect,
            CameraAspectPolicy::prev,
            CameraAspectPolicy::next,
        ),
        SettingsItem::CameraFraming => apply_cycle(
            action,
            &mut settings.video.camera_framing,
            CameraFramingPreset::prev,
            CameraFramingPreset::next,
        ),
        SettingsItem::Flashes | SettingsItem::GameplayFlashes => apply_cycle(
            action,
            &mut settings.video.flashes,
            FlashIntensity::prev,
            FlashIntensity::next,
        ),
        SettingsItem::Colorblind => apply_cycle(
            action,
            &mut settings.video.colorblind,
            ColorblindMode::prev,
            ColorblindMode::next,
        ),

        SettingsItem::MasterVolume => settings
            .audio
            .nudge_master(nudge_delta(action, AudioSettings::VOLUME_STEP)),
        SettingsItem::MusicVolume => settings
            .audio
            .nudge_music(nudge_delta(action, AudioSettings::VOLUME_STEP)),
        SettingsItem::SfxVolume => settings
            .audio
            .nudge_sfx(nudge_delta(action, AudioSettings::VOLUME_STEP)),
        SettingsItem::Mute => apply_toggle(action, || settings.audio.toggle_mute()),

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
        SettingsItem::ControllerProfile => apply_cycle(
            action,
            &mut settings.controls.controller_profile,
            ControllerProfileId::prev,
            ControllerProfileId::next,
        ),
        SettingsItem::LeftStickDeadzone => {
            settings.controls.left_stick_deadzone =
                (settings.controls.left_stick_deadzone + nudge_delta(action, 0.02)).clamp(0.0, 0.6);
        }
        SettingsItem::RightStickDeadzone => {
            settings.controls.right_stick_deadzone = (settings.controls.right_stick_deadzone
                + nudge_delta(action, 0.02))
            .clamp(0.0, 0.6);
        }
        SettingsItem::TriggerPress => {
            settings.controls.trigger_press_threshold = (settings.controls.trigger_press_threshold
                + nudge_delta(action, 0.05))
            .clamp(0.05, 1.0);
            settings.controls.clamp_all();
        }
        SettingsItem::TriggerRelease => {
            settings.controls.trigger_release_threshold =
                (settings.controls.trigger_release_threshold + nudge_delta(action, 0.05))
                    .clamp(0.0, 0.95);
            settings.controls.clamp_all();
        }
        SettingsItem::DpadMenuNav => apply_toggle(action, || {
            settings.controls.dpad_menu_navigation = !settings.controls.dpad_menu_navigation;
        }),
        SettingsItem::InvertAimY => apply_toggle(action, || {
            settings.controls.invert_aim_y = !settings.controls.invert_aim_y;
        }),
        SettingsItem::DashInputMode => apply_cycle(
            action,
            &mut settings.controls.dash_input_mode,
            DashInputMode::prev,
            DashInputMode::next,
        ),
        SettingsItem::TouchControls => apply_toggle(action, || {
            settings.controls.touch_controls_visible = !settings.controls.touch_controls_visible;
        }),
        SettingsItem::MenuTapMode => apply_cycle(
            action,
            &mut settings.controls.menu_tap_mode,
            MenuTapMode::prev,
            MenuTapMode::next,
        ),
        SettingsItem::ResetControlFiltering => {
            if matches!(action, SettingsAction::Confirm) {
                settings.controls.reset_filtering_to_defaults();
            }
        }

        SettingsItem::Difficulty => apply_cycle(
            action,
            &mut settings.gameplay.difficulty,
            Difficulty::prev,
            Difficulty::next,
        ),
        SettingsItem::Assist => apply_toggle(action, || {
            settings.gameplay.assist = settings.gameplay.assist.toggle();
        }),
        SettingsItem::PlayerDamageMultiplier => settings
            .gameplay
            .nudge_player_damage(nudge_delta(action, GameplaySettings::DAMAGE_STEP)),
        SettingsItem::DebugHud => apply_toggle(action, || {
            settings.gameplay.debug_hud_visible = !settings.gameplay.debug_hud_visible;
        }),
        SettingsItem::ShowFps => apply_toggle(action, || {
            settings.video.show_fps = !settings.video.show_fps;
        }),
        SettingsItem::ShaderStrength => settings
            .video
            .shaders
            .nudge_strength(nudge_delta(action, ScreenShaderSettings::UNIT_STEP)),
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
        SettingsItem::ShaderDeepDreamStrength => {
            nudge_shader_unit(
                action,
                &mut settings.video.shaders.deep_dream_strength,
                ScreenShaderSettings::UNIT_STEP,
            );
            if settings.video.shaders.deep_dream_strength > 0.001
                && settings.video.shaders.strength <= 0.001
            {
                settings.video.shaders.strength = 1.0;
            }
        }
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

        SettingsItem::DebugOverlay => apply_toggle(action, || {
            dev_state.debug = !dev_state.debug;
        }),
        SettingsItem::SlowMotion => apply_toggle(action, || {
            dev_state.slowmo = !dev_state.slowmo;
        }),
        SettingsItem::Inspector => apply_toggle(action, || {
            developer.inspector_visible = !developer.inspector_visible;
        }),
        SettingsItem::WorldInspector => apply_toggle(action, || {
            developer.world_inspector_visible = !developer.world_inspector_visible;
        }),
        SettingsItem::OverviewCamera => apply_toggle(action, || {
            developer.overview_camera = !developer.overview_camera;
        }),
        SettingsItem::DebugViewMode => match action {
            SettingsAction::Prev => {
                developer.apply_debug_view_mode(developer.debug_view_mode.prev(), true);
            }
            SettingsAction::Next | SettingsAction::Confirm => {
                developer.apply_debug_view_mode(developer.debug_view_mode.next(), true);
            }
        },
        SettingsItem::DebugArtMode => match action {
            SettingsAction::Prev => {
                developer.apply_debug_art_mode(developer.debug_art_mode.prev());
            }
            SettingsAction::Next | SettingsAction::Confirm => {
                developer.apply_debug_art_mode(developer.debug_art_mode.next());
            }
        },
        SettingsItem::ShowHitboxes => apply_toggle(action, || {
            developer.mark_debug_view_custom();
            let next = !developer.show_feature_hitboxes;
            developer.show_feature_hitboxes = next;
            developer.show_player_hitbox = next;
        }),
        SettingsItem::FillDebugBoxes => apply_toggle(action, || {
            developer.mark_debug_view_custom();
            developer.fill_debug_boxes = !developer.fill_debug_boxes;
        }),
        SettingsItem::MicroGrid => apply_toggle(action, || {
            developer.mark_debug_view_custom();
            developer.show_micro_grid = !developer.show_micro_grid;
        }),
        SettingsItem::CameraFrame => apply_toggle(action, || {
            developer.mark_debug_view_custom();
            developer.show_camera_frame = !developer.show_camera_frame;
        }),
        SettingsItem::PlayerBodyProfile => match action {
            SettingsAction::Prev => {
                developer.player_body_profile = developer.player_body_profile.prev();
                if let Some((kinematics, _, _, _)) = live_movement_refs {
                    apply_player_body_profile(kinematics, developer.player_body_profile);
                }
            }
            SettingsAction::Next | SettingsAction::Confirm => {
                developer.player_body_profile = developer.player_body_profile.next();
                if let Some((kinematics, _, _, _)) = live_movement_refs {
                    apply_player_body_profile(kinematics, developer.player_body_profile);
                }
            }
        },
        SettingsItem::MovementProfile => match action {
            SettingsAction::Prev => {
                developer.movement_profile = developer.movement_profile.prev();
                apply_movement_profile(
                    editable_tuning,
                    developer.movement_profile,
                    live_movement_refs.map(|(_, a, d, j)| (a, d, j)),
                );
            }
            SettingsAction::Next | SettingsAction::Confirm => {
                developer.movement_profile = developer.movement_profile.next();
                apply_movement_profile(
                    editable_tuning,
                    developer.movement_profile,
                    live_movement_refs.map(|(_, a, d, j)| (a, d, j)),
                );
            }
        },
        SettingsItem::LdtkAutoApply => apply_toggle(action, || {
            ldtk_reload.auto_apply = !ldtk_reload.auto_apply;
            ldtk_reload.last_status = format!(
                "LDtk auto-apply {}",
                if ldtk_reload.auto_apply {
                    "enabled"
                } else {
                    "disabled"
                }
            );
        }),
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
            Self::ShaderDeepDreamStrength => Some(settings.video.shaders.deep_dream_strength),
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
                (settings.gameplay.player_damage_multiplier / PLAYER_DAMAGE_SLIDER_MAX)
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
            Self::ShaderDeepDreamStrength => {
                settings.video.shaders.deep_dream_strength = v;
                if v > 0.001 && settings.video.shaders.strength <= 0.001 {
                    settings.video.shaders.strength = 1.0;
                }
            }
            Self::ShaderVignetteStrength => settings.video.shaders.vignette_strength = v,
            Self::MasterVolume => settings.audio.master_volume = v,
            Self::MusicVolume => settings.audio.music_volume = v,
            Self::SfxVolume => settings.audio.sfx_volume = v,
            Self::LeftStickDeadzone => settings.controls.left_stick_deadzone = v * 0.6,
            Self::RightStickDeadzone => settings.controls.right_stick_deadzone = v * 0.6,
            Self::TriggerPress => settings.controls.trigger_press_threshold = 0.05 + v * 0.95,
            Self::TriggerRelease => settings.controls.trigger_release_threshold = v * 0.95,
            Self::PlayerDamageMultiplier => {
                settings.gameplay.player_damage_multiplier = v * PLAYER_DAMAGE_SLIDER_MAX;
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
/// settings menu and `crate::host::windowing::window_mode_hotkeys` so both
/// surfaces produce the same `WindowMode` mapping.
///
/// On wasm the underlying `winit` window-mode transitions are either
/// no-ops or require a user gesture to satisfy the browser fullscreen
/// API; cycling the setting via menu Confirm has been observed to lose
/// the canvas's keyboard focus and produce an "input doesn't work"
/// state. Short-circuit on wasm: update the menu state but don't touch
/// `window.mode` — the canvas stays in its current display mode and
/// keyboard focus is preserved.
pub fn apply_display_mode(
    mode: DisplayModeKind,
    state: &mut DisplayModeState,
    windows: &mut Query<&mut Window, With<PrimaryWindow>>,
) {
    state.mode = mode;
    #[cfg(target_arch = "wasm32")]
    {
        // Don't poke `winit`'s WindowMode on wasm; tracking only.
        let _ = windows;
        return;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let Ok(mut window) = windows.single_mut() else {
            return;
        };
        window.mode = match mode {
            DisplayModeKind::Windowed => WindowMode::Windowed,
            DisplayModeKind::Borderless => {
                WindowMode::BorderlessFullscreen(MonitorSelection::Current)
            }
            DisplayModeKind::Fullscreen => {
                WindowMode::Fullscreen(MonitorSelection::Current, VideoModeSelection::Current)
            }
        };
    }
}

#[cfg(test)]
mod model_logic_tests {
    //! Pure-logic coverage for the settings model: the menu's page graph,
    //! row tables, action routing, and label formatting. `apply_action`
    //! itself is World-coupled (it pokes the primary `Window`), but the
    //! helpers it delegates to are pure and are where the navigation bugs
    //! would actually live, so they are what we pin here.
    use super::*;

    #[test]
    fn display_mode_cycle_is_a_three_step_loop() {
        let start = DisplayModeKind::Windowed;
        let a = next_display_mode(start);
        let b = next_display_mode(a);
        let c = next_display_mode(b);
        assert_eq!(c, start, "three Next presses return to the start");
        assert_ne!(start, a);
        assert_ne!(a, b);
        assert_ne!(start, b);
    }

    #[test]
    fn display_mode_prev_inverts_next() {
        for kind in [
            DisplayModeKind::Windowed,
            DisplayModeKind::Borderless,
            DisplayModeKind::Fullscreen,
        ] {
            assert_eq!(prev_display_mode(next_display_mode(kind)), kind);
            assert_eq!(next_display_mode(prev_display_mode(kind)), kind);
        }
    }

    #[test]
    fn page_nav_outcome_opens_each_subpage_and_back_pops() {
        use SettingsItem as I;
        use SettingsOutcome::{OpenPage, PopPage};
        assert_eq!(
            page_nav_outcome(I::OpenVideo),
            Some(OpenPage(SettingsPage::Video))
        );
        assert_eq!(
            page_nav_outcome(I::OpenShaders),
            Some(OpenPage(SettingsPage::Shaders))
        );
        assert_eq!(
            page_nav_outcome(I::OpenAudio),
            Some(OpenPage(SettingsPage::Audio))
        );
        assert_eq!(
            page_nav_outcome(I::OpenControls),
            Some(OpenPage(SettingsPage::Controls))
        );
        assert_eq!(
            page_nav_outcome(I::OpenGameplay),
            Some(OpenPage(SettingsPage::Gameplay))
        );
        assert_eq!(
            page_nav_outcome(I::OpenDeveloper),
            Some(OpenPage(SettingsPage::Developer))
        );
        assert_eq!(page_nav_outcome(I::Back), Some(PopPage));
        // A content row (cycles a value) is not a page-navigation row.
        assert_eq!(page_nav_outcome(I::DisplayMode), None);
    }

    #[test]
    fn page_nav_label_present_iff_nav_row() {
        assert_eq!(page_nav_label(SettingsItem::OpenVideo), Some("Video >"));
        assert_eq!(page_nav_label(SettingsItem::Back), Some("Back"));
        assert_eq!(page_nav_label(SettingsItem::DisplayMode), None);
    }

    #[test]
    fn every_page_has_rows_terminated_by_back_with_no_dupes() {
        for &page in SettingsPage::ALL {
            let rows = SettingsItem::rows_for(page);
            assert!(!rows.is_empty(), "{page:?} has no rows");
            assert_eq!(
                *rows.last().unwrap(),
                SettingsItem::Back,
                "{page:?} should end with a Back row",
            );
            for (i, a) in rows.iter().enumerate() {
                for b in &rows[i + 1..] {
                    assert_ne!(a, b, "{page:?} lists {a:?} twice");
                }
            }
        }
    }

    #[test]
    fn top_page_exposes_reset_and_subpage_entries() {
        let top = SettingsItem::rows_for(SettingsPage::Top);
        for required in [
            SettingsItem::OpenVideo,
            SettingsItem::OpenAudio,
            SettingsItem::OpenControls,
            SettingsItem::OpenGameplay,
            SettingsItem::OpenDeveloper,
            SettingsItem::ResetAllSettings,
        ] {
            assert!(top.contains(&required), "Top page missing {required:?}");
        }
    }

    #[test]
    fn page_titles_are_unique_and_nonempty() {
        let titles: Vec<&str> = SettingsPage::ALL.iter().map(|p| p.title()).collect();
        assert_eq!(titles.len(), SettingsPage::ALL.len());
        for t in &titles {
            assert!(!t.is_empty(), "empty page title");
        }
        for (i, a) in titles.iter().enumerate() {
            for b in &titles[i + 1..] {
                assert_ne!(a, b, "duplicate page title {a:?}");
            }
        }
    }

    #[test]
    fn apply_cycle_routes_prev_and_next() {
        let dec = |x: i32| x - 1;
        let inc = |x: i32| x + 1;
        let mut v = 10;
        apply_cycle(SettingsAction::Next, &mut v, dec, inc);
        assert_eq!(v, 11);
        apply_cycle(SettingsAction::Prev, &mut v, dec, inc);
        assert_eq!(v, 10);
        apply_cycle(SettingsAction::Confirm, &mut v, dec, inc);
        assert_eq!(v, 11, "Confirm advances like Next");
    }

    #[test]
    fn nudge_delta_signs_match_direction() {
        assert_eq!(nudge_delta(SettingsAction::Next, 0.25), 0.25);
        assert_eq!(nudge_delta(SettingsAction::Confirm, 0.25), 0.25);
        assert_eq!(nudge_delta(SettingsAction::Prev, 0.25), -0.25);
    }

    #[test]
    fn format_helpers_have_the_expected_shape() {
        assert_eq!(
            format_cycle("Camera View", "Wide"),
            "Camera View: Wide  < / >"
        );
        assert_eq!(format_toggle("FPS Overlay", true), "FPS Overlay: on");
        assert_eq!(format_toggle("FPS Overlay", false), "FPS Overlay: off");
    }
}
