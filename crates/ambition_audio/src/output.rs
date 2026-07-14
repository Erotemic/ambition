//! Runtime selection of the audio backend and final output side effect.
//!
//! Automated no-window hosts need the real authored-asset, ownership,
//! provider-resolution, and playback-evidence paths without opening a physical
//! output device. [`AudioOutputMode::Recording`] therefore installs Kira's
//! public asset types and loaders plus inert typed channels, but deliberately
//! omits Kira's device backend and command-drain systems. Windowed applications
//! use [`AudioOutputMode::Device`] by default.

use bevy::prelude::Resource;

#[cfg(feature = "kira")]
use bevy::{
    asset::AssetApp,
    prelude::{App, Plugin},
};
#[cfg(feature = "kira")]
use bevy_kira_audio::prelude::{
    AudioApp, AudioChannel, AudioInstance, AudioPlugin as KiraAudioPlugin, AudioSource,
    MainTrack, OggLoader, WavLoader,
};

/// Where accepted audio playback decisions are delivered.
#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AudioOutputMode {
    /// Deliver accepted playback to the real Kira device backend.
    #[default]
    Device,
    /// Record normal playback state/evidence without opening an audio device.
    Recording,
}

impl AudioOutputMode {
    /// Whether accepted playback should issue a literal backend `play` command.
    pub const fn emits_to_device(self) -> bool {
        matches!(self, Self::Device)
    }
}

/// Machine-readable proof of which backend was selected during App composition.
#[derive(Resource, Clone, Copy, Debug, Eq, PartialEq)]
pub struct AudioBackendState {
    pub mode: AudioOutputMode,
    pub device_backend_installed: bool,
}

/// Installs either the real Kira backend or a non-device recording foundation.
///
/// The output mode must be inserted before this plugin is added. If it is not,
/// normal device playback is selected for backwards-compatible visible hosts.
#[cfg(feature = "kira")]
pub struct AmbitionAudioBackendPlugin;

#[cfg(feature = "kira")]
impl Plugin for AmbitionAudioBackendPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioOutputMode>();
        let mode = *app.world().resource::<AudioOutputMode>();
        match mode {
            AudioOutputMode::Device => {
                app.add_plugins(KiraAudioPlugin);
            }
            AudioOutputMode::Recording => {
                // This is the asset half of bevy_kira_audio::AudioPlugin. The
                // omitted non-send AudioOutput and channel-drain systems are the
                // only pieces that can open and drive the physical device.
                app.init_asset::<AudioSource>()
                    .init_asset::<AudioInstance>()
                    .init_asset_loader::<OggLoader>()
                    .init_asset_loader::<WavLoader>()
                    .insert_resource(AudioChannel::<MainTrack>::default());
            }
        }
        app.insert_resource(AudioBackendState {
            mode,
            device_backend_installed: mode == AudioOutputMode::Device,
        });
    }
}

/// Add a typed audio channel appropriate for the selected output backend.
#[cfg(feature = "kira")]
pub trait AmbitionAudioAppExt {
    fn add_ambition_audio_channel<T: Resource>(&mut self) -> &mut Self;
}

#[cfg(feature = "kira")]
impl AmbitionAudioAppExt for App {
    fn add_ambition_audio_channel<T: Resource>(&mut self) -> &mut Self {
        let mode = self
            .world()
            .get_resource::<AudioBackendState>()
            .map(|state| state.mode)
            .or_else(|| self.world().get_resource::<AudioOutputMode>().copied())
            .unwrap_or_default();
        match mode {
            AudioOutputMode::Device => AudioApp::add_audio_channel::<T>(self),
            AudioOutputMode::Recording => {
                self.insert_resource(AudioChannel::<T>::default())
            }
        }
    }
}

/// Resolve an optional resource to the backwards-compatible device default.
pub fn emits_to_device(mode: Option<&AudioOutputMode>) -> bool {
    match mode {
        Some(mode) => mode.emits_to_device(),
        None => true,
    }
}
