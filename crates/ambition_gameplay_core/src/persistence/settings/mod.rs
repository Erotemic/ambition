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
//! ## Submodule layout (post-2026-05-09 split)
//!
//! - [`audio`], [`controls`], [`gameplay`], [`video`] — per-category
//!   data + clamping / cycling helpers.
//! - [`model`] — pause-menu vocabulary
//!   ([`SettingsPage`], [`SettingsItem`], [`SettingsAction`],
//!   [`SettingsOutcome`], [`apply_action`], [`apply_display_mode`]).
//! - [`platform_paths`] — OS-conventional data-dir resolution shared
//!   by every persistence file in the sandbox.
//! - [`persistence`] — `settings.ron` load + save + the corresponding
//!   Bevy startup / change-watching systems.
//!
//! See `docs/systems/settings-system.md` for how to add a new row or category.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub mod audio;
// The input-domain config (deadzones / trigger hysteresis / dash mode /
// controller + keyboard-preset vocabulary) now lives in the `ambition_input`
// crate so the dependency points DOWN (persistence -> ambition_input). Re-exported
// as `controls` so existing `crate::persistence::settings::controls::…` and
// `super::controls::…` paths resolve unchanged (ADR 0019).
pub use ambition_input::settings as controls;
pub mod gameplay;
pub mod model;
pub mod persistence;
pub mod platform_paths;
pub mod video;

pub use audio::AudioSettings;
pub use controls::{
    update_trigger_edge, ControlSettings, DashInputMode, MenuPointerPress, MenuTapMode,
    TriggerEdgeState,
};
pub use gameplay::{AssistMode, GameplaySettings};
// Public IR surface used by renderers (the cube today; the pause menu migrates
// onto it next). The IR now lives in `crate::menu::ir`; these re-exports keep
// the historical `crate::persistence::settings::{SettingsOption, …}` paths
// resolving so external callers don't churn. `SettingsCategory` /
// `SettingsMenuModel` are reachable via `crate::menu::ir::settings` directly;
// only the names renderers currently name are re-exported here.
pub use crate::menu::ir::settings::{
    apply_settings_option, settings_menu_model, SettingsOption, SettingsOptionId,
    SettingsOptionKind,
};
pub use model::{
    apply_action, apply_display_mode, DevToggleSnapshot, SettingsAction, SettingsItem,
    SettingsOutcome, SettingsPage,
};
// The SYSTEM-menu IR layer (cube System face; the pause menu migrates onto it
// later). Sits on top of the settings IR above; now lives in
// `crate::menu::ir::system`. Only the names the cube currently uses are
// re-exported here (matching the settings-IR convenience list above); the rest
// of the IR vocabulary (`DevRow`, `LocaleId`, `RadioRow`, …) is reachable via
// `crate::menu::ir::system` directly.
pub use crate::menu::ir::system::{
    DevSnapshot, DevToggleId, RadioSnapshot, SystemMenuAction, SystemMenuEntryId, SystemMenuModel,
    SystemMenuTarget, SystemOptionId,
};
pub use video::{CameraAspectPolicy, ScreenShaderSettings, VideoSettings};

#[cfg(test)]
pub(crate) use gameplay::Difficulty;
#[cfg(test)]
pub(crate) use model::{next_display_mode, prev_display_mode};
#[cfg(test)]
pub(crate) use video::{FlashIntensity, SerializableDisplayMode};

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
        self.video.clamp_all();
        self.audio.clamp_all();
        self.controls.clamp_all();
        self.gameplay.clamp_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::windowing::DisplayModeKind;

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
