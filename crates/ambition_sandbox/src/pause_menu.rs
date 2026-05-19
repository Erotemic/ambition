//! Pause menu overlay (UI shell + navigation).
//!
//! `GameMode::Paused` already gates gameplay. This module is the
//! visible side: a translucent overlay with a small action menu and a
//! focused selection that responds to keyboard / gamepad through the
//! `Menu*` actions on `crate::input::SandboxAction`.
//!
//! The menu is structured as a stack of pages (`SettingsPage`). The
//! top page lists Resume / Settings / Music / Inventory / Quit; the
//! Settings entry pushes onto a category page (Video / Audio /
//! Controls / Gameplay), which then push to the actual setting rows.
//!
//! When `audio` is disabled the Music row is replaced with a
//! placeholder and the navigation system uses the audio-free path so
//! `--no-default-features --features input` still compiles.

use bevy::app::AppExit;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
#[cfg(feature = "audio")]
use bevy_kira_audio::prelude::AudioChannel;

#[cfg(feature = "audio")]
use crate::audio::{
    set_radio_track, AudioLibrary, MusicChannel, MusicPlaybackState, RadioStationState,
};
use crate::dev::dev_tools::DeveloperTools;
use crate::game_mode::GameMode;
use crate::input::{KeyboardPreset, MenuControlFrame, MenuInputFrame};
use crate::inventory::InventoryUiState;
use crate::ldtk_world::LdtkHotReloadState;
use crate::settings::{
    apply_action as handle_settings_action, DevToggleSnapshot, SettingsAction, SettingsItem,
    SettingsOutcome, SettingsPage, UserSettings,
};
use crate::ui_nav::visible_row_index;
#[cfg(feature = "input")]
use crate::ui_nav::{apply_vertical_scroll, resolve_selectable_row_interaction, RowPointerOutcome};
use crate::host::windowing::DisplayModeState;
use crate::SandboxDevState;

/// Re-export the settings-row component so other modules that want to
/// query menu rows by tag don't need to remember which module owns it.
pub use crate::settings::SettingsItem as MenuSettingsItem;

mod input;
mod model;
mod pointer;
mod ui;

#[cfg(test)]
mod tests;

#[cfg(feature = "input")]
pub use self::input::{pause_menu_navigate, pause_menu_toggle};
pub use self::model::{
    DevToggleParams, DevToggleView, PauseMenuItem, PauseMenuPage, PauseMenuRoot,
    PauseMenuSettingsPanel, PauseMenuState, PauseMenuTopPanel, SettingsRowLabel,
    SettingsRowSliderFill, SettingsRowSliderTrack, SettingsRowSlot, SettingsScrollbarThumb,
    SettingsScrollbarTrack,
};
#[cfg(feature = "input")]
pub use self::pointer::pause_menu_pointer_input;
#[cfg(feature = "input")]
pub use self::ui::{settings_scrollbar_drag_input, settings_slider_drag_input};
pub use self::ui::{spawn_pause_menu, sync_pause_menu, sync_settings_panel_rows, SettingsTitle};
