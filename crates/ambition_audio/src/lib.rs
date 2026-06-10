//! Ambition's authored-audio stack (Stage 20 / B1 extraction).
//!
//! - [`spec`] — the RON data schema for SFX cues + pre-rendered music
//!   tracks (Kira-free; the host's data manifest embeds these types).
//! - Behind the `kira` feature: [`library`] (typed SFX table, lazily
//!   loaded music tracks, channel markers, track switch/radio/default-
//!   start), [`render`] (bank bytes -> Kira sources + handle cache),
//!   [`music`] (the adaptive cue catalog + layered director), and
//!   [`web_unlock`] (browser AudioContext gesture gate).
//!
//! The HOST keeps the game-side adapters: which track the game
//! requests, where the bank bytes come from, and how its settings map
//! into [`mix::MusicMix`]. This crate never reads game state.

pub mod mix;
pub mod spec;

#[cfg(feature = "kira")]
pub mod library;
#[cfg(feature = "kira")]
pub mod music;
#[cfg(feature = "kira")]
pub mod render;
#[cfg(feature = "kira")]
pub mod web_unlock;

pub use mix::MusicMix;
