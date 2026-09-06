//! Desktop-specific platform setup.
//!
//! Mirror of [`super::android`] for desktop builds. Today this is a
//! no-op stub; future home for desktop-only platform work (Steam
//! Deck low-power detection, desktop-only diagnostics).

use bevy::prelude::*;

/// Bevy plugin for desktop-only setup.
pub struct DesktopPlatformPlugin;

impl Plugin for DesktopPlatformPlugin {
    fn build(&self, _app: &mut App) {}
}
