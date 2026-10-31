//! Apply the host's frame cap to a rendered native window.
//! Requires a rendered Winit window and app-owned `UserSettings`.
//! The simulation clock is separate. User settings have app lifetime.

use bevy::prelude::*;
use bevy_framepace::{FramepacePlugin, FramepaceSettings, Limiter};

use ambition_persistence::settings::video::FramePaceCap;
use ambition_persistence::settings::UserSettings;

/// Set the native display's frame cap from the Video setting.
pub struct FramePacePlugin;

impl Plugin for FramePacePlugin {
    fn build(&self, app: &mut App) {
        // A windowless or no-GPU host has no display clock to follow.
        if !app.is_plugin_added::<bevy::winit::WinitPlugin>()
            || app.get_sub_app_mut(bevy::render::RenderApp).is_none()
        {
            return;
        }
        app.add_plugins(FramepacePlugin)
            .add_systems(Update, sync_framepace_from_settings);
    }
}

/// Apply the Video frame cap when settings change.
fn sync_framepace_from_settings(
    settings: Res<UserSettings>,
    mut framepace: ResMut<FramepaceSettings>,
) {
    if !settings.is_changed() {
        return;
    }
    framepace.limiter = match settings.video.frame_cap {
        FramePaceCap::Auto => Limiter::Auto,
        FramePaceCap::Off => Limiter::Off,
        cap => match cap.target_fps() {
            Some(fps) => Limiter::from_framerate(fps),
            None => Limiter::Auto,
        },
    };
}
