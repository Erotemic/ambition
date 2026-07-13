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
use bevy_kira_audio::prelude::{AudioApp, AudioControl, AudioPlugin as KiraAudioPlugin};

use super::environment::{
    apply_audio_environment, detect_audio_environment, smooth_audio_environment, AudioEnvironment,
};
use ambition_audio::audio_play_sfx_messages;
use ambition_audio::library::{
    start_default_music_when_ready, DefaultMusicStarted, MusicChannel, SfxChannel,
};
use ambition_platformer_primitives::lifecycle::{
    simulation_authorized, ActiveSessionScope, SessionGatedSimulation,
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
            .add_plugins(ambition_audio::SfxBankAssetPlugin)
            // Browser AudioContext unlock telemetry. No-op on desktop
            // except the one-shot "audio unlocked" log; on wasm it also
            // emits the startup "audio locked until first gesture" line
            // so anyone watching devtools knows why audio is silent
            // before they click.
            .add_plugins(super::WebAudioUnlockPlugin)
            .init_resource::<super::RadioStationState>()
            .init_resource::<ambition_audio::render::SfxBankHandleCache>()
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
                    ambition_dev_tools::profiling::phase_mark("before_audio_init"),
                    crate::music::load_music_cues,
                    ambition_dev_tools::profiling::phase_mark("after_audio_init"),
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
            // Deferred first-play is a DIRECT-ENTRY convenience: it auto-starts
            // the resident default track. Under a session-routed host the
            // director owns music per activation (driven by
            // `ActiveAudioSelection`), and the title route is deliberately
            // silent, so this must not fire there.
            .add_systems(
                Update,
                start_default_music_when_ready.run_if(music_auto_start_when_ungated),
            )
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
            // The active session audio authority. Empty (silence) until a
            // host selects it: session-routed hosts select per activation via
            // the shell bridge; direct-entry hosts select their provider
            // statically at composition.
            .init_resource::<ambition_audio::selection::ActiveAudioSelection>()
            // Provider-contributed SFX bank ids, feeding each session's SFX
            // authority. Defaults empty (a host may ship no bank); the resident
            // bank's owner registers its ids once the bank loads.
            .init_resource::<ambition_audio::catalog::SfxBankRegistry>()
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
                    // Gameplay music only drives while the simulation is
                    // authorized. In direct-entry apps there is no session gate,
                    // so `simulation_authorized` is always true (unchanged). In a
                    // session-routed host the chain sleeps at frontend/title
                    // routes, so a stale room/encounter candidate cannot re-switch
                    // the base channel frame after frame.
                    .run_if(simulation_authorized)
                    .after(crate::schedule::SandboxSet::CoreSimulation),
            )
            // Apply the frontend audio policy: the frame the simulation
            // deauthorizes (Quit to Home), stop every gameplay music channel
            // once, reset the director so no previous session's music lingers,
            // then start the host's title theme if one is configured (else
            // deliberate silence). Ungated so it can observe the transition;
            // latched so it acts exactly once per frontend entry (no thrash).
            .add_systems(
                Update,
                apply_frontend_music_policy.after(crate::schedule::SandboxSet::CoreSimulation),
            );
    }
}

/// Run condition: auto-start the resident default music track only when this App
/// is NOT a session-routed host (no [`SessionGatedSimulation`] marker). A
/// session-routed host starts music per activation through the director +
/// `ActiveAudioSelection` instead, and keeps the title route silent.
fn music_auto_start_when_ungated(gate: Option<Res<SessionGatedSimulation>>) -> bool {
    gate.is_none()
}

/// Apply the frontend audio policy the frame the simulation deauthorizes: stop
/// all gameplay music, reset the director so a session-routed host's title route
/// owns no gameplay playback, then start the host's title theme
/// ([`FrontendMusicPolicy`]) if one is configured — otherwise deliberate
/// silence. Acts once per frontend entry via the `entered_frontend` latch;
/// no-op in direct-entry apps (there the simulation is always authorized).
#[allow(clippy::too_many_arguments)]
fn apply_frontend_music_policy(
    gate: Option<Res<SessionGatedSimulation>>,
    scope: Option<Res<ActiveSessionScope>>,
    base_music_channel: Res<bevy_kira_audio::prelude::AudioChannel<MusicChannel>>,
    layer_channels: crate::music::MusicLayerChannels,
    library: Option<ResMut<ambition_audio::library::AudioLibrary>>,
    asset_server: Res<AssetServer>,
    policy: Option<Res<ambition_audio::selection::FrontendMusicPolicy>>,
    director: Option<ResMut<crate::music::MusicDirectorState>>,
    music_state: Option<ResMut<ambition_audio::library::MusicPlaybackState>>,
    mut intent: ResMut<crate::music::MusicIntent>,
    mut started: ResMut<DefaultMusicStarted>,
    mut entered_frontend: Local<bool>,
) {
    if simulation_authorized(gate, scope) {
        *entered_frontend = false;
        return;
    }
    if *entered_frontend {
        return;
    }
    let (Some(mut director), Some(mut music_state), Some(mut library)) =
        (director, music_state, library)
    else {
        *entered_frontend = true;
        return;
    };
    ambition_audio::music::silence_music_backend(
        &base_music_channel,
        &layer_channels,
        &mut director,
        &mut music_state,
    );
    intent.simple_track_candidates.clear();
    intent.adaptive = None;
    // Keep the deferred first-play latched shut at the frontend; a fresh
    // session re-selects and the director restarts music from the selection.
    started.0 = true;

    // Host title theme, if any: loop it on the base music channel. Absent policy
    // (or an unresolvable track) leaves the frontend deliberately silent.
    if let Some(track_id) = policy.and_then(|policy| policy.title_track.clone()) {
        if let Some(handle) = library.resolve_track_handle(&track_id, &asset_server) {
            base_music_channel.play(handle).looped();
            music_state.active_track = track_id;
        }
    }
    *entered_frontend = true;
}
