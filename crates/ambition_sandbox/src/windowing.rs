//! Window/display-mode controls for the Bevy sandbox.
//!
//! The game is authored around a 16:9 composition, but the window is resizable.
//! For now we avoid stretching or letterboxing: Bevy's default orthographic 2D
//! camera keeps one world unit close to one logical pixel, and the camera follow
//! system clamps using the current window size. Larger windows therefore reveal
//! more of the room instead of scaling the simulation oddly.

use bevy::prelude::*;
use bevy::window::{MonitorSelection, PrimaryWindow, VideoModeSelection, WindowMode};

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

/// Runtime display-mode hotkeys.
///
/// - F6: windowed
/// - F7: borderless fullscreen on the current monitor
/// - F8: exclusive fullscreen using the monitor's current video mode
pub fn window_mode_hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<DisplayModeState>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let requested = if keys.just_pressed(KeyCode::F6) {
        Some(DisplayModeKind::Windowed)
    } else if keys.just_pressed(KeyCode::F7) {
        Some(DisplayModeKind::Borderless)
    } else if keys.just_pressed(KeyCode::F8) {
        Some(DisplayModeKind::Fullscreen)
    } else {
        None
    };

    let Some(mode) = requested else {
        return;
    };
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
