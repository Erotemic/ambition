//! Desktop-specific platform setup.
//!
//! Mirror of [`super::android`] for desktop builds. Today this only
//! sets the initial [`super::power::PowerProfile`] to `Performance`
//! (desktop users almost always have wall power). Future home for
//! desktop-only platform work (Steam Deck low-power detection,
//! desktop-only diagnostics).

use bevy::prelude::*;

use super::power::PowerProfile;

/// Pick a sensible default `PowerProfile` for desktop builds.
pub fn default_power_profile() -> PowerProfile {
    PowerProfile::Performance
}

/// Bevy plugin for desktop-only setup.
pub struct DesktopPlatformPlugin;

impl Plugin for DesktopPlatformPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(default_power_profile());
    }
}
