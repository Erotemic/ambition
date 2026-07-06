//! Window/display-mode controls for the Bevy sandbox.
//!
//! The game is authored around a 16:9 composition, but the window is resizable.
//! For now we avoid stretching or letterboxing: Bevy's default orthographic 2D
//! camera keeps one world unit close to one logical pixel, and the camera follow
//! system clamps using the current window size. Larger windows therefore reveal
//! more of the room instead of scaling the simulation oddly.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

pub use ambition_persistence::host::windowing::{DisplayModeKind, DisplayModeState};

/// Runtime display-mode hotkeys (developer convenience).
///
/// The primary user-facing surface is the pause menu's Settings page
/// (`ambition_gameplay_core::pause_menu`), which exposes Display Mode as a row that
/// cycles Windowed / Borderless / Fullscreen with Left/Right and
/// Confirm. These F-keys remain as a dev shortcut so you can flip
/// between modes without going through the menu while iterating:
///
/// - `F6`: windowed
/// - `F7`: borderless fullscreen
///
/// `F8` is reserved for the gameplay trace recorder dump (see
/// `ambition_gameplay_core::trace::handle_trace_hotkey`). Exclusive fullscreen used to
/// be on `F8` but is rarely useful for sandbox dev; the menu remains
/// the way to reach it.
///
/// The actual mode-application logic lives in
/// `crate::persistence::settings::apply_display_mode` so the menu and the hotkeys
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
        crate::persistence::settings::apply_display_mode(mode, &mut state, &mut windows);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_mode_kind_labels_are_distinct() {
        assert_ne!(
            DisplayModeKind::Windowed.label(),
            DisplayModeKind::Borderless.label()
        );
        assert_ne!(
            DisplayModeKind::Windowed.label(),
            DisplayModeKind::Fullscreen.label()
        );
        assert_ne!(
            DisplayModeKind::Borderless.label(),
            DisplayModeKind::Fullscreen.label()
        );
    }

    #[test]
    fn display_mode_state_default_is_windowed() {
        let s = DisplayModeState::default();
        assert_eq!(s.mode, DisplayModeKind::Windowed);
        assert_eq!(s.label(), "windowed");
    }

    #[test]
    fn display_mode_state_label_tracks_mode() {
        let s = DisplayModeState {
            mode: DisplayModeKind::Borderless,
        };
        assert_eq!(s.label(), "borderless");
    }
}
