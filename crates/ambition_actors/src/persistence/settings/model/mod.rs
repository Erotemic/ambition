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

use super::video::FlashIntensity;
use super::UserSettings;
use crate::host::windowing::{DisplayModeKind, DisplayModeState};
use crate::ldtk_world::LdtkHotReloadState;
use ambition_dev_tools::dev_tools::{
    apply_movement_profile, apply_player_body_profile, DebugArtMode, DebugViewMode, DeveloperTools,
    EditableMovementTuning,
};
use ambition_dev_tools::SandboxDevState;

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
    PauseInputUnfocused,

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
    /// Map a pause-menu row to its shared-IR option, if the option is one the
    /// IR models. This is the single bridge between the pause menu's
    /// `SettingsItem` vocabulary and the renderer-agnostic
    /// [`SettingsOptionId`](ambition_settings_menu::settings::SettingsOptionId): rows that map are
    /// labelled / valued / applied through the shared IR
    /// ([`settings_menu_model`](ambition_settings_menu::settings::settings_menu_model) /
    /// [`apply_settings_option`](ambition_settings_menu::settings::apply_settings_option)) so the
    /// pause menu and the 3D cube cannot drift on the surface they share.
    ///
    /// Rows that return `None` are pause-menu-specific (the IR does not model
    /// them) and keep their own label/apply handling:
    ///   - the page-opener rows (`OpenVideo`, `OpenShaders`, … ) + `Back`,
    ///   - the action rows `ResetAllSettings`,
    ///   - the entire Developer page (rendered through `DevToggleSnapshot`; the
    ///     cube renders the matching dev toggles via the IR's `DevToggleId`
    ///     screen),
    ///   - `GameplayFlashes` (a *second* surface on `video.flashes` with the
    ///     distinct label "Flashes (gameplay)"; the IR exposes the field once
    ///     as `Flashes`).
    ///
    /// (The whole `Video > Shaders` subpage, `KeyboardPreset`, and
    /// `ResetControlFiltering` ALL map to the IR now — added in stage 3b — so
    /// they are no longer in the `None` set.)
    pub fn shared_option_id(self) -> Option<ambition_settings_menu::settings::SettingsOptionId> {
        use ambition_settings_menu::settings::SettingsOptionId as Id;
        Some(match self {
            // Video.
            Self::DisplayMode => Id::DisplayMode,
            Self::CameraZoom => Id::CameraZoom,
            Self::CameraAspect => Id::CameraAspect,
            Self::CameraFraming => Id::CameraFraming,
            Self::Flashes => Id::Flashes,
            Self::Colorblind => Id::Colorblind,
            Self::ShowFps => Id::ShowFps,
            // Audio.
            Self::MasterVolume => Id::MasterVolume,
            Self::MusicVolume => Id::MusicVolume,
            Self::SfxVolume => Id::SfxVolume,
            Self::Mute => Id::Mute,
            // Video > Shaders.
            Self::ShaderStrength => Id::ShaderStrength,
            Self::ShaderCrtStrength => Id::ShaderCrtStrength,
            Self::ShaderCrtScanlines => Id::ShaderCrtScanlines,
            Self::ShaderCrtMask => Id::ShaderCrtMask,
            Self::ShaderCrtCurvature => Id::ShaderCrtCurvature,
            Self::ShaderCrtBloom => Id::ShaderCrtBloom,
            Self::ShaderCrtChroma => Id::ShaderCrtChroma,
            Self::ShaderFilmGrainStrength => Id::ShaderFilmGrainStrength,
            Self::ShaderFilmGrainSize => Id::ShaderFilmGrainSize,
            Self::ShaderFilmGrainFps => Id::ShaderFilmGrainFps,
            Self::ShaderFilmGrainLumaBias => Id::ShaderFilmGrainLumaBias,
            Self::ShaderRobotDeathStrength => Id::ShaderRobotDeathStrength,
            Self::ShaderRobotStatic => Id::ShaderRobotStatic,
            Self::ShaderRobotTear => Id::ShaderRobotTear,
            Self::ShaderRobotDesaturate => Id::ShaderRobotDesaturate,
            Self::ShaderRobotScanlines => Id::ShaderRobotScanlines,
            Self::ShaderUnderwaterStrength => Id::ShaderUnderwaterStrength,
            Self::ShaderUnderwaterDistortion => Id::ShaderUnderwaterDistortion,
            Self::ShaderDeepDreamStrength => Id::ShaderDeepDreamStrength,
            Self::ShaderVignetteStrength => Id::ShaderVignetteStrength,
            // Controls.
            Self::KeyboardPreset => Id::KeyboardPreset,
            Self::ControllerProfile => Id::ControllerProfile,
            Self::LeftStickDeadzone => Id::LeftStickDeadzone,
            Self::RightStickDeadzone => Id::RightStickDeadzone,
            Self::TriggerPress => Id::TriggerPress,
            Self::TriggerRelease => Id::TriggerRelease,
            Self::DpadMenuNav => Id::DpadMenuNav,
            Self::InvertAimY => Id::InvertAimY,
            Self::DashInputMode => Id::DashInputMode,
            Self::TouchControls => Id::TouchControls,
            Self::MenuTapMode => Id::MenuTapMode,
            Self::ResetControlFiltering => Id::ResetControlFiltering,
            // Gameplay.
            Self::Difficulty => Id::Difficulty,
            Self::Assist => Id::Assist,
            Self::PlayerDamageMultiplier => Id::PlayerDamage,
            Self::DebugHud => Id::DebugHud,
            Self::QuestHud => Id::QuestHud,
            Self::TraceAutoDump => Id::TraceAutoDump,
            Self::PauseInputUnfocused => Id::PauseInputUnfocused,
            // Everything else is pause-menu-specific (see doc comment).
            _ => return None,
        })
    }

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
                Self::PauseInputUnfocused,
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
        // Stage 3a: rows that map to a shared-IR option derive their label +
        // value text from `settings_menu_model` so the pause menu and the cube
        // cannot drift on that surface. The pause menu's own `< / >` cycle
        // decoration is re-applied here from the option's `kind`.
        if let Some(id) = self.shared_option_id() {
            return pause_label_from_shared(id, settings);
        }
        match self {
            Self::OpenVideo
            | Self::OpenShaders
            | Self::OpenAudio
            | Self::OpenControls
            | Self::OpenGameplay
            | Self::OpenDeveloper
            | Self::Back => unreachable!("page-nav rows handled by page_nav_label above"),
            // Rows that map to a shared-IR option are handled by
            // `pause_label_from_shared` via the early `return` above; listing
            // them here (rather than a `_` wildcard) keeps the match exhaustive
            // so a *new* unmapped row can't silently fall through unlabelled.
            Self::DisplayMode
            | Self::CameraZoom
            | Self::CameraAspect
            | Self::CameraFraming
            | Self::Flashes
            | Self::Colorblind
            | Self::ShowFps
            | Self::MasterVolume
            | Self::MusicVolume
            | Self::SfxVolume
            | Self::Mute
            | Self::ControllerProfile
            | Self::LeftStickDeadzone
            | Self::RightStickDeadzone
            | Self::TriggerPress
            | Self::TriggerRelease
            | Self::DpadMenuNav
            | Self::InvertAimY
            | Self::DashInputMode
            | Self::TouchControls
            | Self::MenuTapMode
            | Self::Difficulty
            | Self::Assist
            | Self::PlayerDamageMultiplier
            | Self::DebugHud
            | Self::QuestHud
            | Self::TraceAutoDump
            | Self::PauseInputUnfocused
            // Stage 3b: the whole Shaders subpage + KeyboardPreset +
            // ResetControlFiltering now map to the shared IR too.
            | Self::ShaderStrength
            | Self::ShaderCrtStrength
            | Self::ShaderCrtScanlines
            | Self::ShaderCrtMask
            | Self::ShaderCrtCurvature
            | Self::ShaderCrtBloom
            | Self::ShaderCrtChroma
            | Self::ShaderFilmGrainStrength
            | Self::ShaderFilmGrainSize
            | Self::ShaderFilmGrainFps
            | Self::ShaderFilmGrainLumaBias
            | Self::ShaderRobotDeathStrength
            | Self::ShaderRobotStatic
            | Self::ShaderRobotTear
            | Self::ShaderRobotDesaturate
            | Self::ShaderRobotScanlines
            | Self::ShaderUnderwaterStrength
            | Self::ShaderUnderwaterDistortion
            | Self::ShaderDeepDreamStrength
            | Self::ShaderVignetteStrength
            | Self::KeyboardPreset
            | Self::ResetControlFiltering => {
                debug_assert!(self.shared_option_id().is_some());
                unreachable!("shared-IR rows handled by pause_label_from_shared above")
            }
            Self::ResetAllSettings => "Reset All Settings to Defaults".into(),

            Self::GameplayFlashes => {
                format_cycle("Flashes (gameplay)", settings.video.flashes.label())
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

/// Find a built [`SettingsOption`](ambition_settings_menu::settings::SettingsOption) by id in the
/// live shared-IR model. The id space is exhaustively built by
/// [`settings_menu_model`](ambition_settings_menu::settings::settings_menu_model) for every
/// category option, so any id a [`SettingsItem`] maps to is present (the only
/// id not produced by the model is `Close`, which no pause row maps to).
fn shared_option(
    id: ambition_settings_menu::settings::SettingsOptionId,
    settings: &UserSettings,
) -> ambition_settings_menu::settings::SettingsOption {
    let model = ambition_settings_menu::settings::settings_menu_model(settings);
    model
        .categories
        .iter()
        .flat_map(|c| c.options.iter())
        .find(|o| o.id == id)
        .cloned()
        .unwrap_or_else(|| panic!("shared IR has no option for {id:?}"))
}

/// Render a shared-IR option as a pause-menu row label. The label + value text
/// come verbatim from the IR (single source of truth shared with the cube);
/// the pause menu's own `< / >` cycle decoration is re-applied here from the
/// option's `kind` so cycle/slider rows still read "Label: value  < / >" and
/// toggle/action rows read "Label: value", matching the menu's UX shape.
pub(crate) fn pause_label_from_shared(
    id: ambition_settings_menu::settings::SettingsOptionId,
    settings: &UserSettings,
) -> String {
    use ambition_settings_menu::settings::SettingsOptionKind;
    let opt = shared_option(id, settings);
    match opt.kind {
        SettingsOptionKind::Cycle { .. } | SettingsOptionKind::Slider { .. } => {
            format_cycle(&opt.label, opt.value_label)
        }
        // An action row with no value (e.g. "Reset Filter Defaults") reads as a
        // bare label — no trailing "`: `" — matching the pause menu's static
        // action-row labels.
        SettingsOptionKind::Action if opt.value_label.is_empty() => opt.label,
        SettingsOptionKind::Toggle(_) | SettingsOptionKind::Action => {
            format!("{}: {}", opt.label, opt.value_label)
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
    pub player_body_profile: ambition_dev_tools::dev_tools::PlayerBodyProfile,
    pub movement_profile: ambition_dev_tools::dev_tools::MovementProfile,
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

/// Convert a pause-menu `SettingsAction` into the shared IR's signed step
/// direction (`-1` prev, `+1` next, `0` confirm/activate). Mirrors how
/// `apply_settings_option` reads `dir`: `<0` steps down, otherwise up, and `0`
/// (Confirm) advances like Next — matching the pause menu's own
/// `apply_cycle` "Confirm behaves like Next" rule.
fn settings_dir(action: SettingsAction) -> i32 {
    match action {
        SettingsAction::Prev => -1,
        SettingsAction::Next => 1,
        SettingsAction::Confirm => 0,
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
///
/// `_keyboard_preset_count` is retained for call-site compatibility but no
/// longer read: as of stage 3b `KeyboardPreset` routes through the shared IR,
/// which wraps the index modulo the fixed `KeyboardPreset::presets().len()`.
#[allow(clippy::too_many_arguments)]
pub fn apply_action(
    item: SettingsItem,
    action: SettingsAction,
    settings: &mut UserSettings,
    display_state: &mut DisplayModeState,
    windows: &mut Query<&mut Window, With<PrimaryWindow>>,
    _keyboard_preset_count: usize,
    dev_state: &mut SandboxDevState,
    developer: &mut DeveloperTools,
    editable_tuning: &mut EditableMovementTuning,
    ldtk_reload: &mut LdtkHotReloadState,
    live_movement_refs: Option<(
        &mut crate::actor::BodyKinematics,
        &crate::actor::BodyAbilities,
        &mut crate::actor::BodyDashState,
        &mut crate::actor::BodyJumpState,
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
    // Stage 3a: rows that map to a shared-IR option apply through the shared
    // `apply_settings_option` so the pause menu and the cube mutate
    // `UserSettings` identically. The IR is field-only; `DisplayMode` also
    // needs the live primary-window poke, which the pause menu still owns and
    // runs after the field update.
    if let Some(id) = item.shared_option_id() {
        ambition_settings_menu::settings::apply_settings_option(id, settings_dir(action), settings);
        if matches!(
            id,
            ambition_settings_menu::settings::SettingsOptionId::DisplayMode
        ) {
            let mode: DisplayModeKind = settings.video.display_mode.into();
            apply_display_mode(mode, display_state, windows);
        }
        return SettingsOutcome::Stay;
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

        // `GameplayFlashes` is a *second* surface on `video.flashes` (label
        // "Flashes (gameplay)") that the shared IR does not model; the IR
        // exposes the field once as `Flashes`, which is migrated. Keep this
        // arm so the gameplay-page row still nudges the same field.
        SettingsItem::GameplayFlashes => apply_cycle(
            action,
            &mut settings.video.flashes,
            FlashIntensity::prev,
            FlashIntensity::next,
        ),

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

        // Rows that map to a shared-IR option are applied via
        // `apply_settings_option` through the early `return` above; listed
        // explicitly (not a `_` wildcard) so a new unmapped row can't silently
        // become a no-op.
        SettingsItem::DisplayMode
        | SettingsItem::CameraZoom
        | SettingsItem::CameraAspect
        | SettingsItem::CameraFraming
        | SettingsItem::Flashes
        | SettingsItem::Colorblind
        | SettingsItem::ShowFps
        | SettingsItem::MasterVolume
        | SettingsItem::MusicVolume
        | SettingsItem::SfxVolume
        | SettingsItem::Mute
        | SettingsItem::ControllerProfile
        | SettingsItem::LeftStickDeadzone
        | SettingsItem::RightStickDeadzone
        | SettingsItem::TriggerPress
        | SettingsItem::TriggerRelease
        | SettingsItem::DpadMenuNav
        | SettingsItem::InvertAimY
        | SettingsItem::DashInputMode
        | SettingsItem::TouchControls
        | SettingsItem::MenuTapMode
        | SettingsItem::Difficulty
        | SettingsItem::Assist
        | SettingsItem::PlayerDamageMultiplier
        | SettingsItem::DebugHud
        | SettingsItem::QuestHud
        | SettingsItem::TraceAutoDump
        | SettingsItem::PauseInputUnfocused
        // Stage 3b migrations.
        | SettingsItem::ShaderStrength
        | SettingsItem::ShaderCrtStrength
        | SettingsItem::ShaderCrtScanlines
        | SettingsItem::ShaderCrtMask
        | SettingsItem::ShaderCrtCurvature
        | SettingsItem::ShaderCrtBloom
        | SettingsItem::ShaderCrtChroma
        | SettingsItem::ShaderFilmGrainStrength
        | SettingsItem::ShaderFilmGrainSize
        | SettingsItem::ShaderFilmGrainFps
        | SettingsItem::ShaderFilmGrainLumaBias
        | SettingsItem::ShaderRobotDeathStrength
        | SettingsItem::ShaderRobotStatic
        | SettingsItem::ShaderRobotTear
        | SettingsItem::ShaderRobotDesaturate
        | SettingsItem::ShaderRobotScanlines
        | SettingsItem::ShaderUnderwaterStrength
        | SettingsItem::ShaderUnderwaterDistortion
        | SettingsItem::ShaderDeepDreamStrength
        | SettingsItem::ShaderVignetteStrength
        | SettingsItem::KeyboardPreset
        | SettingsItem::ResetControlFiltering => {
            debug_assert!(item.shared_option_id().is_some());
            unreachable!("shared-IR rows applied via apply_settings_option above")
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

// `next_display_mode` / `prev_display_mode` moved to
// `ambition_persistence::host::windowing` (beside `DisplayModeKind`) at E1e so
// the settings-menu IR crate can step the modes without reaching up into
// gameplay-core. Re-exported so `crate::persistence::settings::model::*` callers
// and the model logic tests keep resolving.
pub use ambition_persistence::host::windowing::{next_display_mode, prev_display_mode};

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
mod model_logic_tests;
