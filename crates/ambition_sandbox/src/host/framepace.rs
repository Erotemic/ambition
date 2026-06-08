//! Frame pacing (battery saver) — wires [`bevy_framepace`] to the Video setting
//! [`crate::persistence::settings::video::VideoSettings::frame_pacing`].
//!
//! With pacing ON ([`Limiter::Auto`]), the renderer sleeps to match the display
//! refresh rate instead of free-running as fast as the hardware allows — a large
//! battery/heat win on mobile. With it OFF ([`Limiter::Off`]) the app renders
//! unthrottled (useful for benchmarking).
//!
//! This whole module is gated behind the `frame_pacing` feature because the
//! limiter lives in the render sub-app; headless builds have no render app and
//! don't compile it. The *setting* itself lives in `UserSettings` and persists on
//! every platform — only the effect is visible-build-only.

use bevy::prelude::*;
use bevy_framepace::{FramepacePlugin, FramepaceSettings, Limiter};

use crate::persistence::settings::UserSettings;

/// Installs `bevy_framepace` and keeps its limiter mirrored to the Video setting.
pub struct FramePacePlugin;

impl Plugin for FramePacePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FramepacePlugin)
            .add_systems(Update, sync_framepace_from_settings);
    }
}

/// Mirror `UserSettings::video::frame_pacing` into `bevy_framepace`'s limiter
/// whenever settings change. `Auto` caps to the display refresh (battery saver);
/// `Off` renders unthrottled.
fn sync_framepace_from_settings(
    settings: Res<UserSettings>,
    mut framepace: ResMut<FramepaceSettings>,
) {
    // Only react when settings actually change (toggle from the Video menu, or
    // the initial load) — pacing isn't a per-frame decision.
    if !settings.is_changed() {
        return;
    }
    framepace.limiter = if settings.video.frame_pacing {
        Limiter::Auto
    } else {
        Limiter::Off
    };
}
