//! Content-free audio data/runtime layer.
//!
//! [`spec`] defines authored cue schemas; with the `kira` feature, [`library`],
//! [`render`], [`music`], and [`web_unlock`] provide SFX/music loading, playback,
//! adaptive direction, and browser unlock support. Hosts decide which track to
//! request, where bank bytes come from, and how settings map into [`mix::MusicMix`].

pub mod mix;
pub mod spec;

#[cfg(feature = "kira")]
pub mod bank_asset;
pub mod catalog;
#[cfg(feature = "kira")]
pub mod library;
#[cfg(feature = "kira")]
pub mod music;
#[cfg(feature = "kira")]
pub mod render;
#[cfg(feature = "kira")]
pub mod web_unlock;

pub use mix::MusicMix;

#[cfg(feature = "kira")]
pub use bank_asset::{
    audio_play_sfx_messages, SfxBankAsset, SfxBankAssetPath, SfxBankAssetPlugin, SfxBankResource,
};
