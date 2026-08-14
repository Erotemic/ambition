//! Menu backend selection vocabulary.
//!
//! This is presentation/menu state, not actor-domain state. It selects which
//! compiled unified-menu frontend is active for the inventory/system overlay.
//!
//! `InventoryUiBackend` selects which compiled menu frontend renders. The
//! `*_BACKEND_ENABLED` consts mirror build features so selection collapses
//! gracefully when a backend is absent.

use bevy::prelude::Resource;

/// Build-time switch for the flat Bevy-UI menu backend.
///
/// The normal visible desktop/Android personas enable this feature so both
/// platforms exercise the same menu stack. Focused diagnostics / minimal builds
/// can leave it off, and backend selection will gracefully collapse to any other
/// compiled backend instead of installing hidden Bevy-UI systems.
pub const BEVY_UI_MENU_BACKEND_ENABLED: bool = cfg!(feature = "bevy_ui_menu");

/// Build-time switch for the experimental 3D cube menu backend.
///
/// The normal visible desktop/Android personas enable this feature so both
/// platforms exercise the same menu stack. Minimal/headless builds can leave it
/// off, and backend selection will gracefully collapse to any other compiled
/// backend.
///
/// ⛔ **NEVER ON THE WEB** (Jon, 2026-08-14: *"there is an issue with
/// kaleidoscope in web"*). The browser gets the flat Bevy-UI menu, full stop.
///
/// ⚠ **the `target_arch` term is not redundant with the feature.** The browser
/// personas already leave `kaleidoscope_menu` out of their feature lists, and
/// that was not enough on its own: Cargo features are additive and unify across
/// a build, so any `--features` composition, a `--use-default-features` web
/// build, or a future dependency that forwards the flag turns the cube back on
/// silently. Answering the question at the SELECTION — which is what this
/// constant is for, per its neighbours' doc — makes the cube unreachable on wasm
/// however the feature arrives, and collapses a saved `LunexKaleidoscope`
/// setting to the grid through [`InventoryUiBackend::effective`].
pub const KALEIDOSCOPE_MENU_BACKEND_ENABLED: bool =
    cfg!(feature = "kaleidoscope_menu") && !cfg!(target_arch = "wasm32");

/// Which inventory frontend renders. The 3D cube remains the default when its
/// feature is installed; otherwise builds fall back to the flat Bevy-UI backend
/// when available. If a saved setting names a backend that is not compiled into
/// this build, [`InventoryUiBackend::effective`] collapses it to an available
/// backend before any systems run.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
pub enum InventoryUiBackend {
    Grid,
    LunexKaleidoscope,
}

impl Default for InventoryUiBackend {
    fn default() -> Self {
        if KALEIDOSCOPE_MENU_BACKEND_ENABLED {
            Self::LunexKaleidoscope
        } else {
            Self::Grid
        }
    }
}

impl InventoryUiBackend {
    pub fn is_available(self) -> bool {
        match self {
            Self::Grid => BEVY_UI_MENU_BACKEND_ENABLED,
            Self::LunexKaleidoscope => KALEIDOSCOPE_MENU_BACKEND_ENABLED,
        }
    }

    pub fn effective(self) -> Self {
        if self.is_available() {
            self
        } else if KALEIDOSCOPE_MENU_BACKEND_ENABLED {
            Self::LunexKaleidoscope
        } else {
            Self::Grid
        }
    }

    pub fn label(self) -> &'static str {
        match self.effective() {
            Self::Grid => "Grid",
            Self::LunexKaleidoscope => "Cube",
        }
    }

    pub fn next(self) -> Self {
        match self.effective() {
            Self::Grid if KALEIDOSCOPE_MENU_BACKEND_ENABLED => Self::LunexKaleidoscope,
            Self::LunexKaleidoscope if BEVY_UI_MENU_BACKEND_ENABLED => Self::Grid,
            Self::Grid | Self::LunexKaleidoscope => self.effective(),
        }
    }

    pub fn unavailable_note(self) -> &'static str {
        match (
            BEVY_UI_MENU_BACKEND_ENABLED,
            KALEIDOSCOPE_MENU_BACKEND_ENABLED,
        ) {
            (true, true) => "",
            (true, false) => " (cube backend disabled)",
            (false, true) => " (grid backend disabled)",
            (false, false) => " (all menu backends disabled)",
        }
    }
}
