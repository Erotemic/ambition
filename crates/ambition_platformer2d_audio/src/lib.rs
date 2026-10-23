//! The platformer's audio runtime: the Kira install and channels, the
//! acoustic environment, and the music direction ([`music`]) that turns
//! encounter, boss, room, conversation and radio state into a music intent.
//! Carved out of `ambition_platformer2d_actor_monolith` (D33, 2026-10-23): it
//! named nothing else in that crate. A composition with audio installs
//! [`Platformer2dAudioPlugin`]; a headless one leaves the crate out.
//!
//! All Ambition-game audio playback is authored: pre-rendered OGG music tracks loaded through
//! the asset manager catalog, and SFX served from the packed `.sfxbank` (also catalog-routed). Kira
//! owns the backend, channels, fades, and looping.
//!
//! Realtime DSP/effects (underwater muffle, low-pass filtering,
//! reverb) live in [`environment`]. Today only a Kira-friendly
//! channel-attenuation fallback is wired up because `bevy_kira_audio`
//! 0.25 does not expose track-level effect insertion or the
//! underlying `kira::AudioManager`; search for
//! `TODO: kira_underwater_filter_backend` for the exact swap points.
//! Reverb / heavier coloration remains future work.

#[cfg(test)]
use ambition_platformer2d_core as ae;
// The audio runtime submodules import it directly.
#[cfg(test)]
use ambition_sfx as sfx;

pub mod environment;
pub mod music;
mod plugin;

#[cfg(test)]
mod tests;

pub use environment::{AudioEnvironment, AudioEnvironmentMode};
// SoundCue / SfxMessageCue / ORIGINAL_TRACK_ID live in `ambition_audio`
// (Kira-gated); headless paths use `SoundCueKey` from the data schema.

pub use environment::{
    apply_audio_environment, detect_audio_environment, smooth_audio_environment,
};
pub use plugin::Platformer2dAudioPlugin;
// ⛔⛔ THE `ambition_audio` RE-EXPORTS ARE GONE (D33, 2026-08-27). Twenty-odd
// names — the playback library, the render cache, the web-unlock plugin and the
// sfx bank — were forwarded here so *"historical `crate::audio::…` paths keep
// resolving"*. Historical paths resolving is what a facade is FOR, and it is
// also what makes a coupling census read this crate as the owner of an audio
// library it does not own. One consumer in the tree still used them.
//
// ⭐ What stays is what this crate declares: the environment model, the music
// direction and the plugin that installs them.
