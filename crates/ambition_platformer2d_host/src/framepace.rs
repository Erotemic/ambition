//! Apply the host's frame cap to a rendered native window.
//! Requires a rendered Winit window and app-owned `UserSettings`.
//! The simulation clock is separate. User settings have app lifetime.

use bevy::prelude::*;
use bevy_framepace::{FramepacePlugin, FramepaceSettings, Limiter};

use ambition_persistence::settings::UserSettings;
use ambition_persistence::settings::video::{FramePaceCap, VsyncMode};

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
    if ambition_persistence::settings::video::profile_uncapped() {
        framepace.limiter = Limiter::Off;
        return;
    }
    framepace.limiter = match (settings.video.frame_cap, settings.video.vsync) {
        // V-Sync already limits frames to the refresh rate. The automatic
        // software cap uses a lower rate and can cause missed refreshes.
        (FramePaceCap::Auto, VsyncMode::On) | (FramePaceCap::Off, _) => Limiter::Off,
        (FramePaceCap::Auto, VsyncMode::Off) => Limiter::Auto,
        (cap, _) => match cap.target_fps() {
            Some(fps) => Limiter::from_framerate(fps),
            None => Limiter::Auto,
        },
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_uses_one_refresh_limiter_and_keeps_explicit_caps() {
        let mut app = App::new();
        app.init_resource::<UserSettings>()
            .init_resource::<FramepaceSettings>()
            .add_systems(Update, sync_framepace_from_settings);

        app.update();
        assert!(matches!(
            app.world().resource::<FramepaceSettings>().limiter,
            Limiter::Off
        ));

        app.world_mut().resource_mut::<UserSettings>().video.vsync = VsyncMode::Off;
        app.update();
        assert!(matches!(
            app.world().resource::<FramepaceSettings>().limiter,
            Limiter::Auto
        ));

        let mut settings = app.world_mut().resource_mut::<UserSettings>();
        settings.video.vsync = VsyncMode::On;
        settings.video.frame_cap = FramePaceCap::Hz30;
        app.update();
        assert!(matches!(
            app.world().resource::<FramepaceSettings>().limiter,
            Limiter::Manual(period) if period == std::time::Duration::from_secs_f64(1.0 / 30.0)
        ));
    }
}
