//! Window/display-mode controls for the Bevy sandbox.
//!
//! The game is authored around a 16:9 composition, but the window is resizable.
//! For now we avoid stretching or letterboxing: Bevy's default orthographic 2D
//! camera keeps one world unit close to one logical pixel, and the camera follow
//! system clamps using the current window size. Larger windows therefore reveal
//! more of the room instead of scaling the simulation oddly.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// User-facing display modes supported by the sandbox.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayModeKind {
    Windowed,
    Borderless,
    Fullscreen,
}

impl DisplayModeKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Windowed => "windowed",
            Self::Borderless => "borderless",
            Self::Fullscreen => "fullscreen",
        }
    }
}

/// Tracks the display mode we last requested.
#[derive(Resource, Clone, Copy, Debug)]
pub struct DisplayModeState {
    pub mode: DisplayModeKind,
}

impl Default for DisplayModeState {
    fn default() -> Self {
        Self {
            mode: DisplayModeKind::Windowed,
        }
    }
}

impl DisplayModeState {
    pub fn label(&self) -> &'static str {
        self.mode.label()
    }
}

/// Runtime display-mode hotkeys (developer convenience).
///
/// The primary user-facing surface is the pause menu's Settings page
/// (`crate::pause_menu`), which exposes Display Mode as a row that
/// cycles Windowed / Borderless / Fullscreen with Left/Right and
/// Confirm. These F-keys remain as a dev shortcut so you can flip
/// between modes without going through the menu while iterating:
///
/// - `F6`: windowed
/// - `F7`: borderless fullscreen
///
/// `F8` is reserved for the gameplay trace recorder dump (see
/// `crate::trace::handle_trace_hotkey`). Exclusive fullscreen used to
/// be on `F8` but is rarely useful for sandbox dev; the menu remains
/// the way to reach it.
///
/// The actual mode-application logic lives in
/// `crate::settings::apply_display_mode` so the menu and the hotkeys
/// stay in lock-step. Adding a new mode happens in one place.
pub fn window_mode_hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<DisplayModeState>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let requested = if keys.just_pressed(KeyCode::F6) {
        Some(DisplayModeKind::Windowed)
    } else if keys.just_pressed(KeyCode::F7) {
        Some(DisplayModeKind::Borderless)
    } else {
        None
    };
    if let Some(mode) = requested {
        crate::settings::apply_display_mode(mode, &mut state, &mut windows);
    }
}
