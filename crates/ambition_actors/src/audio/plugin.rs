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

use ambition_audio::AmbitionAudioAppExt as _;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_kira_audio::prelude::AudioControl;

use super::environment::{
    apply_audio_environment, detect_audio_environment, smooth_audio_environment, AudioEnvironment,
};
use ambition_audio::audio_play_sfx_messages;
use ambition_audio::library::{
    start_default_music_when_ready, DefaultMusicStarted, MusicChannel, SfxChannel,
};
use ambition_platformer_primitives::lifecycle::{
    simulation_authorized, ActiveSessionScope, InitialGameplayReadiness,
    SessionGatedSimulation,
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
        app.init_resource::<ambition_audio::AudioOutputMode>()
            .add_plugins(ambition_audio::AmbitionAudioBackendPlugin)
            .add_message::<ambition_audio::selection::AudioContextChanged>()
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
            .init_resource::<ambition_audio::render::ProviderSfxHandleCache>()
            .init_resource::<AudioEnvironment>()
            .init_resource::<DefaultMusicStarted>()
            .add_ambition_audio_channel::<MusicChannel>()
            .add_ambition_audio_channel::<SfxChannel>()
            .add_ambition_audio_channel::<crate::music::MusicLayer0AChannel>()
            .add_ambition_audio_channel::<crate::music::MusicLayer1AChannel>()
            .add_ambition_audio_channel::<crate::music::MusicLayer2AChannel>()
            .add_ambition_audio_channel::<crate::music::MusicLayer3AChannel>()
            .add_ambition_audio_channel::<crate::music::MusicLayer4AChannel>()
            .add_ambition_audio_channel::<crate::music::MusicLayer5AChannel>()
            .add_ambition_audio_channel::<crate::music::MusicLayer0BChannel>()
            .add_ambition_audio_channel::<crate::music::MusicLayer1BChannel>()
            .add_ambition_audio_channel::<crate::music::MusicLayer2BChannel>()
            .add_ambition_audio_channel::<crate::music::MusicLayer3BChannel>()
            .add_ambition_audio_channel::<crate::music::MusicLayer4BChannel>()
            .add_ambition_audio_channel::<crate::music::MusicLayer5BChannel>()
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
            // `ActiveAudioSelection`), and frontend routes own their explicit
            // title/menu audio profile, so this must not fire there.
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
            .init_resource::<ambition_sfx::SfxEmissionContext>()
            // Provider-contributed SFX bank ids, feeding each session's SFX
            // authority. Defaults empty (a host may ship no bank); the resident
            // bank's owner registers its ids once the bank loads.
            .init_resource::<ambition_audio::catalog::SfxBankRegistry>()
            // Complete provider-contributed adaptive catalogs. Cached
            // definitions do not confer authority: the active audio context
            // selects the one provider the director may resolve.
            .init_resource::<ambition_audio::music::AdaptiveMusicCatalogRegistry>()
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
                    // authorized. Direct visible startup may additionally hold
                    // the initial-presentation readiness gate closed; session-
                    // routed hosts sleep at frontend/title routes. In either
                    // case a stale room/encounter candidate cannot switch the
                    // base channel before gameplay authority exists.
                    .run_if(simulation_authorized)
                    .after(crate::schedule::SandboxSet::CoreSimulation),
            )
            // Reset all activation-local audio request/director state on both
            // gameplay and frontend transitions. This runs outside the gameplay
            // gate so returning home clears ownership immediately.
            .add_systems(
                Update,
                reset_audio_request_state_on_context_change
                    .after(crate::schedule::SandboxSet::CoreSimulation)
                    .before(audio_play_sfx_messages)
                    .before(crate::music::compute_music_intent)
                    .before(apply_frontend_music_policy),
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

#[derive(SystemParam)]
struct AudioRequestState<'w, 's> {
    encounter: Option<
        ambition_platformer_primitives::lifecycle::SessionWorldMut<
            'w,
            's,
            crate::encounter::EncounterMusicRequest,
        >,
    >,
    room: Option<
        ambition_platformer_primitives::lifecycle::SessionWorldMut<
            'w,
            's,
            crate::rooms::RoomMusicRequest,
        >,
    >,
    radio: Option<ResMut<'w, super::RadioStationState>>,
    intent: Option<ResMut<'w, crate::music::MusicIntent>>,
    director: Option<ResMut<'w, crate::music::MusicDirectorState>>,
    music_playback: Option<ResMut<'w, ambition_audio::library::MusicPlaybackState>>,
    sfx_playback: Option<ResMut<'w, ambition_audio::render::SfxPlaybackState>>,
    default_started: Option<ResMut<'w, DefaultMusicStarted>>,
}

/// Reset activation-local audio requests exactly when the shell audio owner
/// changes. The bundle keeps the mutable request components behind one Bevy parameter, well below
/// the system argument limit, while making same-provider relaunch as fresh as a
/// cross-provider switch.
fn reset_audio_request_state_on_context_change(
    mut changes: MessageReader<ambition_audio::selection::AudioContextChanged>,
    selection: Res<ambition_audio::selection::ActiveAudioSelection>,
    base_music_channel: Res<bevy_kira_audio::prelude::AudioChannel<MusicChannel>>,
    sfx_channel: Res<bevy_kira_audio::prelude::AudioChannel<SfxChannel>>,
    layer_channels: crate::music::MusicLayerChannels,
    mut state: AudioRequestState,
) {
    let current = selection.owner();
    let reset = changes
        .read()
        .any(|change| change.current == current && change.previous != change.current);
    if !reset {
        return;
    }
    if let Some(encounter) = state.encounter.as_deref_mut() {
        **encounter = Default::default();
    }
    if let Some(room) = state.room.as_deref_mut() {
        **room = Default::default();
    }
    if let Some(radio) = state.radio.as_deref_mut() {
        *radio = Default::default();
    }
    if let Some(intent) = state.intent.as_deref_mut() {
        *intent = Default::default();
    }
    match (&mut state.director, &mut state.music_playback) {
        (Some(director), Some(playback)) => {
            ambition_audio::music::silence_music_backend(
                &base_music_channel,
                &layer_channels,
                &mut **director,
                &mut **playback,
            );
        }
        (director, playback) => {
            if let Some(director) = director.as_deref_mut() {
                *director = Default::default();
            }
            if let Some(playback) = playback.as_deref_mut() {
                playback.active_track.clear();
            }
        }
    }
    // SFX channels are activation-owned too. Stop any in-flight loop or long
    // one-shot from the previous context before the new context's owned queue
    // is drained later in this frame.
    sfx_channel.stop();
    if let Some(playback) = state.sfx_playback.as_deref_mut() {
        playback.last_played = None;
    }
    if let Some(started) = state.default_started.as_deref_mut() {
        started.0 = false;
    }
}

/// Run condition: auto-start the resident default music track only when this App
/// is NOT a session-routed host (no [`SessionGatedSimulation`] marker). A
/// session-routed host starts music per activation through the director +
/// `ActiveAudioSelection`; frontend routes use their explicit host profile.
fn music_auto_start_when_ungated(
    gate: Option<Res<SessionGatedSimulation>>,
    readiness: Option<Res<InitialGameplayReadiness>>,
) -> bool {
    direct_music_auto_start_allowed(
        gate.is_some(),
        readiness.as_deref().map(|readiness| readiness.is_ready()),
    )
}

fn direct_music_auto_start_allowed(session_gated: bool, initial_ready: Option<bool>) -> bool {
    !session_gated && initial_ready.unwrap_or(true)
}

/// Apply the current frontend shell activation's audio profile.
///
/// Frontend audio is not an exception to authority: the shell bridge selects an
/// exact [`ambition_sfx::AudioContextOwner::Frontend`] context, whose preferred
/// title track and menu-SFX allowlist come from `FrontendAudioProfile`. On every
/// frontend activation change this system stops gameplay channels, resets the
/// director, and starts that context's title track (or deliberate silence).
/// Direct-entry apps remain unaffected because they do not install the session-routing marker;
/// their separate initial-reveal gate only delays the normal direct music start.
#[allow(clippy::too_many_arguments)]
fn apply_frontend_music_policy(
    gate: Option<Res<SessionGatedSimulation>>,
    readiness: Option<Res<InitialGameplayReadiness>>,
    scope: Option<Res<ActiveSessionScope>>,
    roots: Query<&ambition_platformer_primitives::lifecycle::SessionRoot>,
    base_music_channel: Res<bevy_kira_audio::prelude::AudioChannel<MusicChannel>>,
    layer_channels: crate::music::MusicLayerChannels,
    library: Option<ResMut<ambition_audio::library::AudioLibrary>>,
    asset_server: Res<AssetServer>,
    selection: Res<ambition_audio::selection::ActiveAudioSelection>,
    emission: Res<ambition_sfx::SfxEmissionContext>,
    director: Option<ResMut<crate::music::MusicDirectorState>>,
    music_state: Option<ResMut<ambition_audio::library::MusicPlaybackState>>,
    mut intent: ResMut<crate::music::MusicIntent>,
    mut started: ResMut<DefaultMusicStarted>,
    mut applied_owner: Local<Option<ambition_sfx::AudioContextOwner>>,
) {
    // A closed direct-start reveal gate is not a frontend route. Frontend
    // music policy exists only in session-routed hosts; direct entry merely
    // waits to start its normal track until the first coherent frame reveals.
    let Some(gate) = gate else {
        *applied_owner = None;
        return;
    };
    if simulation_authorized(Some(gate), readiness, scope, roots) {
        *applied_owner = None;
        return;
    }
    let owner = emission.owner();
    if *applied_owner == owner {
        return;
    }
    let (Some(mut director), Some(mut music_state), Some(mut library)) =
        (director, music_state, library)
    else {
        *applied_owner = owner;
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
    if matches!(owner, Some(ambition_sfx::AudioContextOwner::Frontend(_))) {
        if let Some(track_id) = selection.preferred_track() {
            if selection.music_authority().allows(track_id) {
                if let Some(handle) = library.resolve_track_handle(track_id, &asset_server) {
                    if layer_channels.output_mode().emits_to_device() {
                        base_music_channel.play(handle).looped();
                    }
                    music_state.active_track = track_id.to_owned();
                }
            }
        }
    }
    *applied_owner = owner;
}

#[cfg(test)]
mod startup_readiness_tests {
    use super::direct_music_auto_start_allowed;

    #[test]
    fn direct_music_waits_for_initial_readiness_without_becoming_frontend() {
        assert!(!direct_music_auto_start_allowed(false, Some(false)));
        assert!(direct_music_auto_start_allowed(false, Some(true)));
        assert!(direct_music_auto_start_allowed(false, None));
        assert!(!direct_music_auto_start_allowed(true, Some(true)));
    }
}
