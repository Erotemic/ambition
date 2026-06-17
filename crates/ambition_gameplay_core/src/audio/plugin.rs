//! Module-local Bevy plugin for the sandbox audio subsystem.
//!
//! Owns the Kira backend install, the SFX-bank loader, the web-audio
//! unlock hook, the channel resources (music + SFX + adaptive layers),
//! and the per-frame audio systems (SFX queue drain, environment
//! detect/smooth/apply, music director). Gated by the `audio` cargo
//! feature; without it the simulation half's `SfxMessage` queue drains
//! harmlessly per the ADR 0012 seam.
//!
//! Carved out of `app/plugins.rs::add_audio_plugins` per OVERNIGHT-TODO
//! #6 — every system referenced here lives in `crate::audio` or
//! `crate::music`, the two domain modules that own audio playback.
//! The one cross-module ordering hook — `.after(setup_presentation_system)`
//! on the `load_music_cues` startup chain — pulls
//! `crate::schedule::setup_presentation_system` through the `pub(crate)`
//! re-export added in `app.rs`.

use bevy::prelude::*;
use bevy_kira_audio::prelude::{AudioApp, AudioPlugin as KiraAudioPlugin};

use super::environment::{
    apply_audio_environment, detect_audio_environment, smooth_audio_environment, AudioEnvironment,
};
use super::runtime::audio_play_sfx_messages;
use ambition_audio::library::{
    start_default_music_when_ready, DefaultMusicStarted, MusicChannel, SfxChannel,
};

/// Public Bevy plugin: installs the Kira backend, channel resources,
/// per-frame audio systems, and the deferred music-start poller.
///
/// Visible builds register this via the sandbox's presentation install
/// chain in `app/plugins.rs::add_presentation_plugins`. Headless / RL
/// builds drop the `audio` cargo feature and the entire dep graph
/// (`bevy_kira_audio` and friends) goes away.
pub struct SandboxAudioPlugin;

impl Plugin for SandboxAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(KiraAudioPlugin)
            // Async SFX-bank loader for profiles whose bank is not picked
            // up by the sync fast path in `setup::try_load_sfx_bank_via_catalog`
            // (web HTTP fetch, plain desktop loose FS without env override).
            // Idempotent against the sync path; both insert the same
            // `SfxBankResource` and the second writer no-ops.
            .add_plugins(super::SfxBankAssetPlugin)
            // Browser AudioContext unlock telemetry. No-op on desktop
            // except the one-shot "audio unlocked" log; on wasm it also
            // emits the startup "audio locked until first gesture" line
            // so anyone watching devtools knows why audio is silent
            // before they click.
            .add_plugins(super::WebAudioUnlockPlugin)
            .init_resource::<super::RadioStationState>()
            .init_resource::<super::SfxBankHandleCache>()
            .init_resource::<AudioEnvironment>()
            .init_resource::<DefaultMusicStarted>()
            .add_audio_channel::<MusicChannel>()
            .add_audio_channel::<SfxChannel>()
            .add_audio_channel::<crate::music::MusicLayer0AChannel>()
            .add_audio_channel::<crate::music::MusicLayer1AChannel>()
            .add_audio_channel::<crate::music::MusicLayer2AChannel>()
            .add_audio_channel::<crate::music::MusicLayer3AChannel>()
            .add_audio_channel::<crate::music::MusicLayer4AChannel>()
            .add_audio_channel::<crate::music::MusicLayer5AChannel>()
            .add_audio_channel::<crate::music::MusicLayer0BChannel>()
            .add_audio_channel::<crate::music::MusicLayer1BChannel>()
            .add_audio_channel::<crate::music::MusicLayer2BChannel>()
            .add_audio_channel::<crate::music::MusicLayer3BChannel>()
            .add_audio_channel::<crate::music::MusicLayer4BChannel>()
            .add_audio_channel::<crate::music::MusicLayer5BChannel>()
            .add_systems(
                Startup,
                (
                    crate::dev::profiling::phase_mark("before_audio_init"),
                    crate::music::load_music_cues,
                    crate::dev::profiling::phase_mark("after_audio_init"),
                )
                    .chain()
                    .after(crate::schedule::PresentationSetupSet),
            )
            // Deferred music start: polls each Update for (a) user
            // gesture observed (AudioUnlockState) and (b) the default
            // music track's asset handle finished loading. On wasm the
            // gesture gate is what prevents `play()` from no-op'ing
            // against a suspended AudioContext; on desktop the gate
            // flips during Startup so behavior matches the old direct-
            // startup system.
            .add_systems(Update, start_default_music_when_ready)
            .add_systems(
                Update,
                audio_play_sfx_messages.after(crate::schedule::SandboxSet::CoreSimulation),
            )
            // Observe the player's WaterContact and request the matching
            // audio environment; the smoother ramps `wetness`, then
            // `apply_audio_environment` writes the combined user-mixer
            // × environment volume to Kira. Order: detect → smooth →
            // apply so a single frame fully propagates a state change.
            .add_systems(
                Update,
                (
                    detect_audio_environment,
                    smooth_audio_environment,
                    apply_audio_environment,
                )
                    .chain()
                    .after(crate::schedule::SandboxSet::CoreSimulation),
            )
            // Neutral music intent: the content layer resolves Ambition
            // encounter/boss/room/radio gameplay into a content-agnostic
            // `MusicIntent`, then the director consumes only that resource.
            .init_resource::<crate::music::MusicIntent>()
            // The HOST owns its settings model and the authored cue
            // catalog; the reusable music core consumes the synced
            // `MusicMix` + the inserted catalog (Stage 20 / B1 seam).
            .init_resource::<ambition_audio::MusicMix>()
            // Unified director: resolves room/encounter simple tracks and
            // adaptive cue states behind one music intent layer. Runs after
            // `compute_music_intent` so the intent is fresh this frame; the
            // mix sync runs first so the director sees this frame's volume.
            .add_systems(
                Update,
                (
                    crate::music::sync_music_mix,
                    crate::music::compute_music_intent,
                    crate::music::drive_music_director,
                )
                    .chain()
                    .after(crate::schedule::SandboxSet::CoreSimulation),
            );
    }
}
