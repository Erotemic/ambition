//! V-sync — mirrors the Video setting
//! [`ambition_platformer2d::persistence::settings::video::VideoSettings::vsync`]
//! into the primary window's [`PresentMode`].
//!
//! Bevy reconfigures the swapchain itself when `Window::present_mode` changes,
//! so this is the whole mechanism: one system, settings → window. The window
//! is created with Bevy's default (`Fifo`, i.e. `VsyncMode::On`), and the
//! first `UserSettings` load counts as a change, so a persisted `Off` applies
//! on the first frame.

use bevy::prelude::*;
use bevy::window::{PresentMode, PrimaryWindow};

use ambition_platformer2d::persistence::settings::video::VsyncMode;
use ambition_platformer2d::persistence::settings::UserSettings;

pub struct VsyncPlugin;

impl Plugin for VsyncPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync_present_mode_from_settings);
    }
}

pub fn present_mode_for(mode: VsyncMode) -> PresentMode {
    match mode {
        VsyncMode::On => PresentMode::Fifo,
        VsyncMode::Off => PresentMode::Immediate,
    }
}

fn sync_present_mode_from_settings(
    settings: Res<UserSettings>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !settings.is_changed() {
        return;
    }
    let Ok(mut window) = window.single_mut() else {
        return;
    };
    let wanted = present_mode_for(settings.video.vsync);
    // Read-compare-write: a `DerefMut` on `Window` marks it changed and makes
    // winit re-examine the window, which every other settings edit would pay.
    if window.present_mode != wanted {
        window.present_mode = wanted;
    }
}
