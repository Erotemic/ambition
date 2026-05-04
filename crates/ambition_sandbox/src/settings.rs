//! Settings catalog + per-setting value mutation.
//!
//! The pause menu (`crate::pause_menu`) is the renderer/controller for
//! the Settings page. This module owns the settings vocabulary: the
//! list of rows, what each row's current value looks like, and how
//! Left / Right / Confirm mutate the underlying resource.
//!
//! Adding a new setting:
//!
//! 1. Add a variant to [`SettingsItem`].
//! 2. Add it to [`SettingsItem::ALL`] in the desired display order.
//! 3. Implement the row's behavior:
//!    - `label_for(&self, &SettingsView)` — the row text.
//!    - `apply(&mut self, action, view, mut World/Resources)` — mutate
//!      the relevant resource on Left / Right / Confirm.
//!
//! New settings should keep their actual mutation logic close to the
//! resource they own (e.g. audio volume → `audio.rs`, control bindings
//! → `input.rs`). This module is the menu surface; it only routes
//! actions to handlers.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::window::{MonitorSelection, VideoModeSelection, WindowMode};

use crate::windowing::{DisplayModeKind, DisplayModeState};

/// Which settings row is active. Adding a new setting means adding a
/// variant here, appending to `ALL`, and updating `label` /
/// `handle_action`.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsItem {
    DisplayMode,
    Back,
}

/// Action a row can receive from the pause menu. The pause menu maps
/// `MoveLeft` / `MoveRight` / `Jump` / etc. on top of these. Adding a
/// new action shape (e.g. text entry) goes here so handlers can match
/// on it explicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsAction {
    /// Cycle the value backward (e.g. previous display mode).
    Prev,
    /// Cycle the value forward.
    Next,
    /// Activate / cycle the row, used when the user confirms with Jump.
    Confirm,
}

impl SettingsItem {
    pub const ALL: [Self; 2] = [Self::DisplayMode, Self::Back];

    pub fn static_label(self) -> &'static str {
        match self {
            Self::DisplayMode => "Display Mode",
            Self::Back => "Back",
        }
    }

    /// Render the row text including the current value. Each new
    /// setting that exposes a value extends the match here with a
    /// `format!(...)` showing the current value plus a Left/Right
    /// affordance.
    pub fn label(self, view: &SettingsView) -> String {
        match self {
            Self::DisplayMode => format!("Display Mode: {}  < / >", view.display_mode.label()),
            Self::Back => self.static_label().to_string(),
        }
    }
}

/// Read-only summary of the current values for each setting; the menu
/// renderer assembles this once per frame so individual `label` calls
/// stay borrow-free. New settings fields go here.
#[derive(Clone, Copy, Debug)]
pub struct SettingsView {
    pub display_mode: DisplayModeKind,
}

impl SettingsView {
    pub fn from_state(display: &DisplayModeState) -> Self {
        Self {
            display_mode: display.mode,
        }
    }
}

/// Outcome of dispatching a settings action. The pause menu may need
/// to know whether to pop back to the top page (`Back`) or keep
/// rendering the settings page (`Stay`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsOutcome {
    Stay,
    Back,
}

/// Apply a settings action to the relevant resource. Adding a new
/// setting extends this match with its own resource mutation. The
/// argument list grows as we add more setting types — that's the
/// whole point of the SettingsItem fan-out.
pub fn handle_action(
    item: SettingsItem,
    action: SettingsAction,
    display_state: &mut DisplayModeState,
    windows: &mut Query<&mut Window, With<PrimaryWindow>>,
) -> SettingsOutcome {
    match item {
        SettingsItem::DisplayMode => {
            let next = match action {
                SettingsAction::Prev => prev_display_mode(display_state.mode),
                SettingsAction::Next | SettingsAction::Confirm => {
                    next_display_mode(display_state.mode)
                }
            };
            apply_display_mode(next, display_state, windows);
            SettingsOutcome::Stay
        }
        SettingsItem::Back => match action {
            SettingsAction::Confirm => SettingsOutcome::Back,
            _ => SettingsOutcome::Stay,
        },
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
/// Settings menu and `crate::windowing::window_mode_hotkeys` so both
/// surfaces produce the same WindowMode mapping. Single source of
/// truth for "user wants to be in mode X".
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
    fn settings_item_all_lists_known_rows() {
        assert!(SettingsItem::ALL.contains(&SettingsItem::DisplayMode));
        assert!(SettingsItem::ALL.contains(&SettingsItem::Back));
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
        let view = SettingsView {
            display_mode: DisplayModeKind::Borderless,
        };
        let label = SettingsItem::DisplayMode.label(&view);
        assert!(label.contains("borderless"));
        assert_eq!(SettingsItem::Back.label(&view), "Back");
    }
}
