//! Window/display-mode state for the Bevy sandbox.
//!
//! Display mode is user-facing configuration owned by the Settings menu. It is
//! deliberately not part of the developer function-key deck.

pub use ambition_persistence::host::windowing::{DisplayModeKind, DisplayModeState};

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
