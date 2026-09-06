//! Window/display-mode controls for the Bevy sandbox.
//!
//! The game is authored around a 16:9 composition, but the window is resizable.
//! For now we avoid stretching or letterboxing: Bevy's default orthographic 2D
//! camera keeps one world unit close to one logical pixel, and the camera follow
//! system clamps using the current window size. Larger windows therefore reveal
//! more of the room instead of scaling the simulation oddly.

use bevy::prelude::Resource;

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

/// Cycle to the next display mode (Windowed → Borderless → Fullscreen → …).
/// Pure over the vocabulary so the settings-menu IR + the window-mode hotkey
/// path both step through the modes the same way.
pub fn next_display_mode(current: DisplayModeKind) -> DisplayModeKind {
    match current {
        DisplayModeKind::Windowed => DisplayModeKind::Borderless,
        DisplayModeKind::Borderless => DisplayModeKind::Fullscreen,
        DisplayModeKind::Fullscreen => DisplayModeKind::Windowed,
    }
}

/// Cycle to the previous display mode (inverse of [`next_display_mode`]).
pub fn prev_display_mode(current: DisplayModeKind) -> DisplayModeKind {
    match current {
        DisplayModeKind::Windowed => DisplayModeKind::Fullscreen,
        DisplayModeKind::Borderless => DisplayModeKind::Windowed,
        DisplayModeKind::Fullscreen => DisplayModeKind::Borderless,
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
