//! The SYSTEM-menu intermediate representation (IR).
//!
//! [`SystemMenuModel`] is the renderer-agnostic description of the broadened
//! SYSTEM face: a flat, ordered list of top-level [`SystemMenuEntry`]s, each of
//! which drills into one *screen* (a curated settings category, the radio
//! station list, a language picker, or the developer toggles) or fires an
//! immediate [`SystemMenuAction`] (Reset Sandbox).
//!
//! It sits ON TOP of the existing [`super::settings::SettingsMenuModel`]: settings
//! screens reuse that IR's [`SettingsOption`]s and [`apply_settings_option`]
//! verbatim, so the cube and the pause menu can never drift on a setting's value
//! label / control kind / mutation. The new screens (Radio / Language /
//! Developer) add their own small option vocabularies that live here.
//!
//! ## Why a second IR layer
//!
//! The cube's System face needs more than the four settings categories: a radio
//! station audition list, a language stub, developer toggles, and a one-shot
//! sandbox reset — none of which are `UserSettings` fields. Rather than teach the
//! settings IR about resources it has no business touching, [`SystemMenuModel`]
//! composes the settings IR with those host screens into ONE tree the renderer
//! walks. The cube renders the flat top level as drill-in rows and each screen as
//! the same touch-styled option column.
//!
//! ## Dev-build gating
//!
//! The Developer and Reset Sandbox entries appear ONLY in dev builds, gated on
//! `cfg!(feature = "dev_tools")` — the same gate the rest of the sandbox's dev
//! tooling uses (`DeveloperTools` inspector, F-key dev hotkeys). In a non-dev
//! build [`SystemMenuModel::build`] omits them entirely, so there are no dead
//! rows and no references to dev-only code.

use super::settings::{settings_menu_model, SettingsOption, SettingsOptionId, SettingsOptionKind};
use crate::persistence::settings::UserSettings;

/// True in builds that ship the developer tooling. Matches the gate used by the
/// rest of the sandbox dev surface (`DeveloperTools` inspector, dev hotkeys), so
/// the Developer / Reset Sandbox entries appear in exactly the same builds.
pub const DEV_BUILD: bool = cfg!(feature = "dev_tools");

/// One developer toggle/cycle surfaced by the Developer screen. Each id maps to a
/// field (or pair) of `crate::dev::dev_tools::DeveloperTools`. Kept here (not in
/// `dev_tools.rs`) so the System IR owns the menu vocabulary; the cube applies
/// them through [`crate::menu::kaleidoscope_app`]'s dev-toggle path, which is the single
/// place that touches `DeveloperTools`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DevToggleId {
    // Global dev flags (sourced from `SandboxDevState`, not `DeveloperTools`).
    DebugOverlay,
    SlowMotion,
    // Inspectors.
    Inspector,
    WorldInspector,
    Gizmos,
    // Debug visuals.
    ShowHud,
    ShowHitboxes,
    HideSprites,
    PlaceholderSprites,
    FillDebugBoxes,
    MicroGrid,
    CameraFrame,
    // Camera.
    OverviewCamera,
    // Profiles (cycles).
    DebugViewMode,
    DebugArtMode,
    PlayerBodyProfile,
    MovementProfile,
    // LDtk hot-reload (sourced from `LdtkHotReloadState`, not `DeveloperTools`).
    LdtkAutoApply,
    // Menu frontend (sourced from `InventoryUiBackend`, not `DeveloperTools`): the
    // in-menu equivalent of the `\` hotkey, cycling Grid ↔ Cube. A toggle (two
    // states) so SELECT flips it; works from BOTH backends since the Developer
    // screen is shared.
    MenuBackend,
    // Portal transit visual effect (sourced from the portal presentation
    // crate's `PortalEffectSelection`, not `DeveloperTools`): cycles the
    // compiled-in effects (view cones / legacy masks / off) for in-session
    // A/B comparison and profiling.
    PortalEffect,
    // Ambient gravity direction (sourced from `BaseGravity`, not `DeveloperTools`):
    // cycles down → left → up → right. The in-menu equivalent of the `\` hotkey, so
    // sideways/inverted gravity is testable on mobile (no keyboard).
    Gravity,
}

impl DevToggleId {
    /// Every developer toggle/cycle, grouped Global Flags → Inspectors → Debug
    /// Visuals → Camera → Profiles → LDtk, in display order. The first two
    /// (DebugOverlay/SlowMotion) and the trailing LdtkAutoApply are sourced from
    /// `SandboxDevState` / `LdtkHotReloadState` (not `DeveloperTools`); they mirror
    /// the pause-menu Developer page's F1 / F2 / F12 rows.
    pub const ALL: [Self; 21] = [
        // Pinned FIRST so it lands under the cursor the instant you drill into
        // Developer — the menu-frontend toggle is the one developers flip most.
        Self::MenuBackend,
        Self::DebugOverlay,
        Self::SlowMotion,
        Self::Inspector,
        Self::WorldInspector,
        Self::Gizmos,
        Self::ShowHud,
        Self::ShowHitboxes,
        Self::HideSprites,
        Self::PlaceholderSprites,
        Self::FillDebugBoxes,
        Self::MicroGrid,
        Self::CameraFrame,
        Self::OverviewCamera,
        Self::DebugViewMode,
        Self::DebugArtMode,
        Self::PlayerBodyProfile,
        Self::MovementProfile,
        Self::LdtkAutoApply,
        Self::PortalEffect,
        Self::Gravity,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::DebugOverlay => "Debug Overlay (F1)",
            Self::SlowMotion => "Slow Motion (F2)",
            Self::Inspector => "Inspector",
            Self::WorldInspector => "World Inspector",
            Self::Gizmos => "Gizmos",
            Self::ShowHud => "Debug HUD",
            Self::ShowHitboxes => "Custom Hitboxes",
            Self::HideSprites => "Hide Sprites",
            Self::PlaceholderSprites => "Placeholder Art",
            Self::FillDebugBoxes => "Fill Debug Boxes",
            Self::MicroGrid => "Micro Grid",
            Self::CameraFrame => "Camera Frame",
            Self::OverviewCamera => "Overview Camera",
            Self::DebugViewMode => "View Mode",
            Self::DebugArtMode => "Art Mode",
            Self::PlayerBodyProfile => "Body Profile",
            Self::MovementProfile => "Movement Profile",
            Self::LdtkAutoApply => "LDtk Auto-Reload (F12)",
            Self::MenuBackend => "Menu Backend",
            Self::PortalEffect => "Portal FX",
            Self::Gravity => "Gravity",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::DebugOverlay => "Toggle the F1 debug overlay (state, timers, gizmos).",
            Self::SlowMotion => "Toggle F2 slow-motion for inspecting fast motion.",
            Self::Inspector => "Show the reflected resource inspector windows.",
            Self::WorldInspector => "Show the full-world entity/resource inspector.",
            Self::Gizmos => "Master switch for Bevy gizmo overlays.",
            Self::ShowHud => "Toggle the debug HUD overlay.",
            Self::ShowHitboxes => "Draw the custom feature + player collision hitboxes.",
            Self::HideSprites => "Suppress sprite rendering so only gizmos show.",
            Self::PlaceholderSprites => "Replace textured sprites with sized rectangles.",
            Self::FillDebugBoxes => "Fill gizmo AABBs with a translucent tint.",
            Self::MicroGrid => "Draw an 8px subdivision grid over the tile grid.",
            Self::CameraFrame => "Draw the requested/actual camera frame rectangles.",
            Self::OverviewCamera => "Zoom out to inspect large or stitched areas.",
            Self::DebugViewMode => "Cycle the debug view preset.",
            Self::DebugArtMode => "Cycle the debug art preset.",
            Self::PlayerBodyProfile => "Cycle the player body-size feel preset.",
            Self::MovementProfile => "Cycle the movement tuning preset.",
            Self::LdtkAutoApply => "Auto-apply LDtk file changes on hot reload (F12).",
            Self::MenuBackend => {
                "Switch the menu frontend: Grid (flat) or Cube (3D). Same as the \\ key."
            }
            Self::PortalEffect => {
                "Cycle the portal transit visual (view cones / masks / off) for A/B profiling."
            }
            Self::Gravity => "Cycle ambient gravity: Down / Left / Up / Right. Same as the \\ key.",
        }
    }

    /// Whether this id is a cycle (vs a toggle). Cycles step value in place on
    /// LEFT/RIGHT; toggles flip on select.
    pub fn is_cycle(self) -> bool {
        matches!(
            self,
            Self::DebugViewMode
                | Self::DebugArtMode
                | Self::PlayerBodyProfile
                | Self::MovementProfile
                // MenuBackend cycles Grid ↔ Cube so its value label (the active
                // frontend name) shows in the row, like the other cycles.
                | Self::MenuBackend
                // PortalEffect cycles the compiled-in portal transit visuals
                // (view cones / masks / off) for A/B profiling.
                | Self::PortalEffect
                // Gravity cycles the ambient direction (down/left/up/right).
                | Self::Gravity
        )
    }
}

/// A locale row in the Language stub. Only [`LocaleId::English`] is selectable;
/// the rest are listed (so the slot reads as "more coming") but disabled. Real
/// i18n is a later foundational pass — see `TODO.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LocaleId {
    English,
    Spanish,
    French,
    German,
    Japanese,
}

impl LocaleId {
    pub const ALL: [Self; 5] = [
        Self::English,
        Self::Spanish,
        Self::French,
        Self::German,
        Self::Japanese,
    ];

    /// The locale's display name (in its own language, the convention for a
    /// language picker).
    pub fn display_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Spanish => "Español",
            Self::French => "Français",
            Self::German => "Deutsch",
            Self::Japanese => "日本語",
        }
    }

    /// Only English is wired today; the rest are placeholders.
    pub fn is_available(self) -> bool {
        matches!(self, Self::English)
    }
}

/// Identity of a top-level SYSTEM row, in display order. `Copy` so it rides a
/// renderer's cursor / dispatched action without allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SystemMenuEntryId {
    Radio,
    Video,
    Audio,
    Controls,
    Gameplay,
    Language,
    /// Reset every persisted settings/dev resource back to defaults. Always
    /// present (mirrors the pause menu's Top-page `ResetAllSettings`).
    ResetAllSettings,
    /// Quit the application to the desktop. Always present, immediate action
    /// (no drill screen), placed after Reset All Settings. Mirrors the pause
    /// menu's Top-page `Quit` (which is removed in a later phase).
    Quit,
    /// Dev-build only.
    Developer,
    /// Dev-build only.
    ResetSandbox,
}

impl SystemMenuEntryId {
    pub fn label(self) -> &'static str {
        match self {
            Self::Radio => "Radio",
            Self::Video => "Video",
            Self::Audio => "Audio",
            Self::Controls => "Controls",
            Self::Gameplay => "Gameplay",
            Self::Language => "Language",
            Self::ResetAllSettings => "Reset All Settings",
            Self::Quit => "Quit to Desktop",
            Self::Developer => "Developer",
            Self::ResetSandbox => "Reset Sandbox",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Radio => "Pick the sandbox radio station (music plays as you browse).",
            Self::Video => "Display, FPS, camera zoom, and the shader / post-process stack.",
            Self::Audio => "Master / music / SFX volume and mute.",
            Self::Controls => "Touch overlay, dash input, and stick deadzone.",
            Self::Gameplay => "Debug and quest HUD overlays.",
            Self::Language => "Interface language (English only for now).",
            Self::ResetAllSettings => "Restore every setting and developer tool to its default.",
            Self::Quit => "Exit the game and return to the desktop.",
            Self::Developer => "Developer inspectors, debug visuals, and feel profiles.",
            Self::ResetSandbox => "Wipe the save and respawn at the start room.",
        }
    }
}

/// A non-settings option row that lives only inside a System screen (Radio /
/// Language / Developer). Settings rows reuse [`SettingsOptionId`] instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SystemOptionId {
    /// Select a radio station by track index (auditions it, keeps the menu open).
    Radio(usize),
    /// Select a locale (only English is enabled).
    Locale(LocaleId),
    /// Toggle/cycle a developer tool.
    Dev(DevToggleId),
}

/// A momentary, screen-less SYSTEM action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SystemMenuAction {
    /// Wipe the save and respawn at the start room, then close the menu.
    ResetSandbox,
    /// Reset every persisted settings/dev resource to defaults, then close the
    /// menu. Mirrors the pause menu's `SettingsItem::ResetAllSettings`.
    ResetAllSettings,
    /// Quit the application to the desktop (writes `AppExit::Success`), then
    /// close the menu. Mirrors the pause menu's Top-page `Quit`.
    Quit,
}

/// What a top-level [`SystemMenuEntry`] does when selected.
#[derive(Clone, Debug, PartialEq)]
pub enum SystemMenuTarget {
    /// Drill into a curated settings category: its option rows are
    /// [`SettingsOption`]s pulled live from the settings IR.
    Settings(Vec<SettingsOption>),
    /// Drill into the radio station list.
    Radio(Vec<RadioRow>),
    /// Drill into the language picker.
    Language(Vec<LocaleRow>),
    /// Drill into the developer toggles.
    Developer(Vec<DevRow>),
    /// Fire an immediate action (no screen).
    Action(SystemMenuAction),
}

/// One radio station row in the Radio screen.
#[derive(Clone, Debug, PartialEq)]
pub struct RadioRow {
    /// Track index into the audio library (what `set_radio_track` resolves).
    pub index: usize,
    pub label: String,
    /// Whether this is the currently-playing station.
    pub active: bool,
}

/// One locale row in the Language screen.
#[derive(Clone, Debug, PartialEq)]
pub struct LocaleRow {
    pub id: LocaleId,
    pub label: String,
    pub available: bool,
    pub active: bool,
}

/// One developer toggle/cycle row in the Developer screen.
#[derive(Clone, Debug, PartialEq)]
pub struct DevRow {
    pub id: DevToggleId,
    pub label: String,
    pub value_label: String,
    pub kind: SettingsOptionKind,
}

/// One top-level SYSTEM row.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemMenuEntry {
    pub id: SystemMenuEntryId,
    pub label: String,
    pub description: String,
    pub target: SystemMenuTarget,
}

/// The whole SYSTEM menu as data: an ordered list of top-level entries. Build it
/// with [`SystemMenuModel::build`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SystemMenuModel {
    pub entries: Vec<SystemMenuEntry>,
}

/// A live snapshot of the radio station list, passed into [`SystemMenuModel::build`]
/// so the IR stays ECS-free. The cube fills this from `RadioStationState` +
/// `AudioLibrary` (under the `audio` feature); in audio-less builds it is empty
/// and the Radio screen lists nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RadioSnapshot {
    /// `(track_index, display_name)` in library order.
    pub stations: Vec<(usize, String)>,
    /// The currently-active track index, if any.
    pub active: Option<usize>,
}

/// A live snapshot of the developer toggles, passed into [`SystemMenuModel::build`]
/// so the IR does not depend on `DeveloperTools` directly. The cube fills this from
/// the `DeveloperTools` resource.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DevSnapshot {
    pub values: Vec<(DevToggleId, bool, String)>,
}

impl DevSnapshot {
    /// `(toggle, on)` for a bool toggle.
    pub fn toggle(id: DevToggleId, on: bool) -> (DevToggleId, bool, String) {
        (id, on, if on { "ON" } else { "OFF" }.to_string())
    }
    /// `(cycle, value_label)` for a cycle.
    pub fn cycle(id: DevToggleId, value_label: impl Into<String>) -> (DevToggleId, bool, String) {
        (id, false, value_label.into())
    }
}

/// The settings-option ids surfaced by each settings category on the SYSTEM
/// face, in pause-menu page order. This now mirrors the OLD pause menu's pages
/// 1:1 for the player-facing set (Phase C1 parity): Video carries the basic
/// display/camera/accessibility rows plus the whole shader subpage; Controls
/// carries every stick/trigger/dash/menu row; Gameplay carries
/// difficulty/assist/damage plus the HUD + trace toggles. Nothing the pause
/// menu shows is dropped here, so the System face is at full parity once the
/// pause menu is deleted.
fn curated_options(id: SystemMenuEntryId) -> &'static [SettingsOptionId] {
    match id {
        // Video carries its basic rows PLUS the whole shader subpage appended
        // after them — shaders live UNDER Video (the cube's single-level System
        // drill surfaces them flat in this one screen, mirroring the pause menu's
        // `Video > Shaders` subpage). Every shader the pause menu's Shaders page
        // exposes is reachable here.
        SystemMenuEntryId::Video => &[
            SettingsOptionId::DisplayMode,
            SettingsOptionId::CameraZoom,
            SettingsOptionId::CameraAspect,
            SettingsOptionId::CameraFraming,
            SettingsOptionId::Flashes,
            SettingsOptionId::Colorblind,
            SettingsOptionId::ShowFps,
            SettingsOptionId::FramePacing,
            SettingsOptionId::ShaderStrength,
            SettingsOptionId::ShaderCrtStrength,
            SettingsOptionId::ShaderCrtScanlines,
            SettingsOptionId::ShaderCrtMask,
            SettingsOptionId::ShaderCrtCurvature,
            SettingsOptionId::ShaderCrtBloom,
            SettingsOptionId::ShaderCrtChroma,
            SettingsOptionId::ShaderFilmGrainStrength,
            SettingsOptionId::ShaderFilmGrainSize,
            SettingsOptionId::ShaderFilmGrainFps,
            SettingsOptionId::ShaderFilmGrainLumaBias,
            SettingsOptionId::ShaderRobotDeathStrength,
            SettingsOptionId::ShaderRobotStatic,
            SettingsOptionId::ShaderRobotTear,
            SettingsOptionId::ShaderRobotDesaturate,
            SettingsOptionId::ShaderRobotScanlines,
            SettingsOptionId::ShaderUnderwaterStrength,
            SettingsOptionId::ShaderUnderwaterDistortion,
            SettingsOptionId::ShaderDeepDreamStrength,
            SettingsOptionId::ShaderVignetteStrength,
        ],
        SystemMenuEntryId::Audio => &[
            SettingsOptionId::MasterVolume,
            SettingsOptionId::MusicVolume,
            SettingsOptionId::SfxVolume,
            SettingsOptionId::Mute,
        ],
        SystemMenuEntryId::Controls => &[
            SettingsOptionId::KeyboardPreset,
            SettingsOptionId::ControllerProfile,
            SettingsOptionId::LeftStickDeadzone,
            SettingsOptionId::RightStickDeadzone,
            SettingsOptionId::TriggerPress,
            SettingsOptionId::TriggerRelease,
            SettingsOptionId::DpadMenuNav,
            SettingsOptionId::InvertAimY,
            SettingsOptionId::DashInputMode,
            SettingsOptionId::TouchControls,
            SettingsOptionId::MenuTapMode,
            SettingsOptionId::ResetControlFiltering,
        ],
        SystemMenuEntryId::Gameplay => &[
            SettingsOptionId::Difficulty,
            SettingsOptionId::Assist,
            SettingsOptionId::PlayerDamage,
            SettingsOptionId::DebugHud,
            SettingsOptionId::QuestHud,
            SettingsOptionId::TraceAutoDump,
            SettingsOptionId::PauseInputUnfocused,
        ],
        _ => &[],
    }
}

impl SystemMenuModel {
    /// Build the live SYSTEM menu. `radio` / `dev` are host snapshots (see their
    /// docs); pass defaults where those subsystems are absent (audio-less / non-dev
    /// builds). Developer + Reset Sandbox are included only in dev builds
    /// ([`DEV_BUILD`]).
    pub fn build(settings: &UserSettings, radio: &RadioSnapshot, dev: &DevSnapshot) -> Self {
        let model = settings_menu_model(settings);
        let settings_entry = |id: SystemMenuEntryId| -> SystemMenuEntry {
            let wanted = curated_options(id);
            // Pull each wanted option's LIVE IR entry (value label + kind) so the
            // System face can never drift from the pause menu's settings.
            let options: Vec<SettingsOption> = wanted
                .iter()
                .filter_map(|want| {
                    model
                        .categories
                        .iter()
                        .flat_map(|c| c.options.iter())
                        .find(|o| o.id == *want)
                        .cloned()
                })
                .collect();
            SystemMenuEntry {
                id,
                label: id.label().to_string(),
                description: id.description().to_string(),
                target: SystemMenuTarget::Settings(options),
            }
        };

        let mut entries = Vec::new();

        // Radio.
        let radio_rows: Vec<RadioRow> = radio
            .stations
            .iter()
            .map(|(index, name)| RadioRow {
                index: *index,
                label: name.clone(),
                active: radio.active == Some(*index),
            })
            .collect();
        entries.push(SystemMenuEntry {
            id: SystemMenuEntryId::Radio,
            label: SystemMenuEntryId::Radio.label().to_string(),
            description: SystemMenuEntryId::Radio.description().to_string(),
            target: SystemMenuTarget::Radio(radio_rows),
        });

        // Settings categories (curated subsets). Shaders are no longer a sibling
        // entry: they ride UNDER Video (see `curated_options`).
        entries.push(settings_entry(SystemMenuEntryId::Video));
        entries.push(settings_entry(SystemMenuEntryId::Audio));
        entries.push(settings_entry(SystemMenuEntryId::Controls));
        entries.push(settings_entry(SystemMenuEntryId::Gameplay));

        // Language (stub).
        let locale_rows: Vec<LocaleRow> = LocaleId::ALL
            .iter()
            .map(|id| LocaleRow {
                id: *id,
                label: id.display_name().to_string(),
                available: id.is_available(),
                // English is the only active locale today.
                active: matches!(id, LocaleId::English),
            })
            .collect();
        entries.push(SystemMenuEntry {
            id: SystemMenuEntryId::Language,
            label: SystemMenuEntryId::Language.label().to_string(),
            description: SystemMenuEntryId::Language.description().to_string(),
            target: SystemMenuTarget::Language(locale_rows),
        });

        // Reset All Settings: an immediate Action entry, ALWAYS present (it
        // mirrors the pause menu's Top-page `ResetAllSettings`, which is not
        // dev-gated). Placed just before the dev-only entries so it sits near
        // Reset Sandbox.
        entries.push(SystemMenuEntry {
            id: SystemMenuEntryId::ResetAllSettings,
            label: SystemMenuEntryId::ResetAllSettings.label().to_string(),
            description: SystemMenuEntryId::ResetAllSettings
                .description()
                .to_string(),
            target: SystemMenuTarget::Action(SystemMenuAction::ResetAllSettings),
        });

        // Quit to Desktop: always present, immediate action, placed right after
        // Reset All Settings (and before the dev-only entries).
        entries.push(SystemMenuEntry {
            id: SystemMenuEntryId::Quit,
            label: SystemMenuEntryId::Quit.label().to_string(),
            description: SystemMenuEntryId::Quit.description().to_string(),
            target: SystemMenuTarget::Action(SystemMenuAction::Quit),
        });

        // Developer + Reset Sandbox: DEV-BUILD GATED.
        if DEV_BUILD {
            let dev_rows: Vec<DevRow> = DevToggleId::ALL
                .iter()
                .map(|id| {
                    let (on, value_label) = dev
                        .values
                        .iter()
                        .find(|(d, _, _)| d == id)
                        .map(|(_, on, label)| (*on, label.clone()))
                        .unwrap_or((false, if id.is_cycle() { "—" } else { "OFF" }.to_string()));
                    let kind = if id.is_cycle() {
                        SettingsOptionKind::Cycle { index: 0, count: 1 }
                    } else {
                        SettingsOptionKind::Toggle(on)
                    };
                    DevRow {
                        id: *id,
                        label: id.label().to_string(),
                        value_label,
                        kind,
                    }
                })
                .collect();
            entries.push(SystemMenuEntry {
                id: SystemMenuEntryId::Developer,
                label: SystemMenuEntryId::Developer.label().to_string(),
                description: SystemMenuEntryId::Developer.description().to_string(),
                target: SystemMenuTarget::Developer(dev_rows),
            });
            entries.push(SystemMenuEntry {
                id: SystemMenuEntryId::ResetSandbox,
                label: SystemMenuEntryId::ResetSandbox.label().to_string(),
                description: SystemMenuEntryId::ResetSandbox.description().to_string(),
                target: SystemMenuTarget::Action(SystemMenuAction::ResetSandbox),
            });
        }

        SystemMenuModel { entries }
    }

    /// The entry with the given id, if present (absent dev entries return `None`
    /// in non-dev builds).
    pub fn entry(&self, id: SystemMenuEntryId) -> Option<&SystemMenuEntry> {
        self.entries.iter().find(|e| e.id == id)
    }
}

#[cfg(test)]
mod tests;
