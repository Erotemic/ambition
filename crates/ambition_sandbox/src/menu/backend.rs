//! Menu backend selection vocabulary (lib-side).
//!
//! `InventoryUiBackend` is the small shared resource that selects which
//! menu frontend renders (the bevy_ui grid vs the 3D cube). It lives in
//! the machinery lib because lib-side consumers read it (the Map tab's
//! input gating, the System-IR rebuild); the host backends themselves
//! moved up to `ambition_app::menu` (Stage 20 menu split). The two
//! `*_BACKEND_ENABLED` consts mirror the build features so backend
//! selection collapses gracefully when a backend is compiled out.

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
pub const KALEIDOSCOPE_MENU_BACKEND_ENABLED: bool = cfg!(feature = "kaleidoscope_menu");

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
