//! Apply the V-sync preference to the primary window.

use bevy::prelude::*;
use bevy::window::{PresentMode, PrimaryWindow};

use ambition_persistence::settings::video::VsyncMode;
use ambition_persistence::settings::UserSettings;

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
    settings: Option<Res<UserSettings>>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Some(settings) = settings else {
        return;
    };
    let Ok(mut window) = window.single_mut() else {
        return;
    };
    let wanted = if ambition_persistence::settings::video::profile_uncapped() {
        PresentMode::Immediate
    } else {
        present_mode_for(settings.video.vsync)
    };
    if window.present_mode != wanted {
        window.present_mode = wanted;
    }
}
