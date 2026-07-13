//! Audio runtime for the Ambition sandbox.
//!
//! All audio playback in the sandbox is **authored**: pre-rendered OGG
//! music tracks loaded through the asset manager catalog, and SFX served
//! from the packed `.sfxbank` (also catalog-routed). Kira owns the
//! backend, channels, fades, and looping. The old runtime fundsp
//! procedural music generator + SFX synthesizer was retired; see
//! `docs/archive/retired/fundsp-audio.md` for the historical note.
//!
//! Realtime DSP/effects (underwater muffle, low-pass filtering,
//! reverb) live in [`environment`]. Today only a Kira-friendly
//! channel-attenuation fallback is wired up because `bevy_kira_audio`
//! 0.25 does not expose track-level effect insertion or the
//! underlying `kira::AudioManager`; search for
//! `TODO: kira_underwater_filter_backend` for the exact swap points.
//! Reverb / heavier coloration remains future work.

#[cfg(all(test, feature = "audio"))]
use ambition_engine_core as ae;
// `SfxMessage` lives in the reusable `ambition_sfx` crate (moved down so
// mechanics request sound without naming a sandbox module). Fable review §D1:
// this module no longer re-exports it — every caller names `ambition_sfx::
// SfxMessage` at its true home. The audio runtime submodules import it directly.
#[cfg(all(test, feature = "audio"))]
use ambition_sfx as sfx;

pub mod environment;
#[cfg(feature = "audio")]
mod plugin;

#[cfg(all(test, feature = "audio"))]
mod tests;

pub use environment::{AudioEnvironment, AudioEnvironmentMode};
// SoundCue / SfxMessageCue / ORIGINAL_TRACK_ID live in `ambition_audio`
// (Kira-gated); headless paths use `SoundCueKey` from the data schema.

#[cfg(feature = "audio")]
pub use environment::{
    apply_audio_environment, detect_audio_environment, smooth_audio_environment,
};
#[cfg(feature = "audio")]
pub use plugin::SandboxAudioPlugin;
// The playback library + render cache + web unlock moved to the
// `ambition_audio` crate (Stage 20 / B1); re-exported so historical
// `crate::audio::…` paths keep resolving.
#[cfg(feature = "audio")]
pub use ambition_audio::library::{
    amplitude_to_decibels, set_radio_track, start_default_music_when_ready, switch_to_music_track,
    AudioLibrary, DefaultMusicStarted, MusicChannel, MusicPlaybackState, MusicTrackRuntime,
    RadioStationState, SfxChannel, SfxMessageCue, SoundCue, ORIGINAL_TRACK_ID,
};
#[cfg(feature = "audio")]
pub use ambition_audio::render::ProviderSfxHandleCache;
#[cfg(feature = "audio")]
pub use ambition_audio::web_unlock::{AudioUnlockState, WebAudioUnlockPlugin};
#[cfg(feature = "audio")]
pub use ambition_audio::{
    audio_play_sfx_messages, SfxBankAsset, SfxBankAssetPath, SfxBankAssetPlugin, SfxBankResource,
};
