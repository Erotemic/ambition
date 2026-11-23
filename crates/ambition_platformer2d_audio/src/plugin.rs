//! The Bevy plugin for the platformer's audio.
//!
//! Owns the Kira backend install, the SFX-bank loader, the web-audio
//! unlock hook, the channel resources (music + SFX + adaptive layers),
//! and the per-frame audio systems (SFX queue drain, environment
//! detect/smooth/apply, music director). Gated by the `audio` cargo
//! feature; without it the simulation half's `SfxMessage` queue drains
//! harmlessly per the ADR 0012 seam.
//!
//! Every system it installs lives in this crate; it orders itself against
//! the shared schedule sets (`AudioInitSet` after `PresentationSetupSet`,
//! the music intent after the simulation's core phase).

use ambition_audio::AmbitionAudioAppExt as _;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_kira_audio::prelude::AudioControl;

use super::environment::{
    AudioEnvironment, apply_audio_environment, detect_audio_environment, smooth_audio_environment,
};
use ambition_audio::audio_play_sfx_messages;
use ambition_audio::library::SfxChannel;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, InitialGameplayReadiness, SessionGatedSimulation, simulation_authorized,
};

/// Public Bevy plugin: installs the Kira backend, channel resources,
/// per-frame sound playback, and music goal publication.
///
/// Visible builds register this via the sandbox's presentation install
/// chain in `app/plugins.rs::add_presentation_plugins`. Headless / RL
/// builds drop the `audio` cargo feature and the entire dep graph
/// (`bevy_kira_audio` and friends) goes away.
pub struct Platformer2dAudioPlugin;

impl Plugin for Platformer2dAudioPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            PostUpdate,
            (
                ambition_sfx::SfxPresentationSet::Listeners,
                ambition_sfx::SfxPresentationSet::Playback,
                ambition_sfx::SfxPresentationSet::Attenuation,
            )
                .chain()
                .before(bevy_kira_audio::AudioSystemSet::PlayTypedChannels),
        );
        app.add_systems(
            PostUpdate,
            ambition_audio::spatial::update_sfx_attenuation
                .in_set(ambition_sfx::SfxPresentationSet::Attenuation)
                .run_if(
                    |shots: Res<ambition_audio::spatial::SfxOneShots>,
                     loops: Res<ambition_audio::SfxLoopPlayback>| {
                        !shots.0.is_empty() || loops.has_voices()
                    },
                ),
        );
        app.init_resource::<ambition_audio::AudioOutputMode>()
            .add_plugins(ambition_audio::AmbitionAudioBackendPlugin)
            .add_message::<ambition_audio::selection::AudioContextChanged>()
            .add_message::<ambition_encounter::EncounterRelease>()
            .add_message::<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeActivated>()
            .add_plugins(ambition_audio::SfxBankAssetPlugin)
            // Browser AudioContext unlock telemetry.
            .add_plugins(ambition_audio::web_unlock::WebAudioUnlockPlugin)
            .init_resource::<ambition_audio::library::RadioStationState>()
            .init_resource::<ambition_audio::render::ProviderSfxHandleCache>()
            .init_resource::<AudioEnvironment>()
            .add_ambition_audio_channel::<SfxChannel>()
            // ⭐ NO PROFILING MARKS HERE. They bracketed this one system from
            // inside the kernel, which made a SIMULATION crate depend on a
            // developer tool to describe itself. Instrumentation can live
            // anywhere, so it lives with the rest of it: the host brackets
            // `AudioInitSet` in `app/plugins.rs`, where every other
            // `phase_mark` already is.
            .add_systems(
                Startup,
                ambition_audio::music::load_music_cues
                    .in_set(ambition_platformer2d_shared_tangle::schedule::AudioInitSet)
                    .after(ambition_platformer2d_shared_tangle::schedule::PresentationSetupSet),
            )
            .add_systems(
                PostUpdate,
                audio_play_sfx_messages
                    .run_if(ambition_audio::a_cue_waits_or_was_played)
                    .in_set(ambition_sfx::SfxPresentationSet::Playback),
            )
            // Sustained cues (a gate's open hum): held by presentation each
            // frame they are wanted, accepted like the one-shots above.
            .add_message::<ambition_sfx::OwnedSfxLoopMessage>()
            .init_resource::<ambition_audio::SfxLoopPlayback>()
            .add_systems(
                PostUpdate,
                ambition_audio::audio_play_sfx_loops
                    .run_if(ambition_audio::a_loop_request_waits_or_a_loop_is_held)
                    .in_set(ambition_sfx::SfxPresentationSet::Playback)
                    .after(audio_play_sfx_messages),
            )
            // Observe the player's WaterContact and request the matching
            // audio environment; the smoother ramps `wetness`, then
            // `apply_audio_environment` publishes the combined user and
            // environment mix. Order: detect → smooth →
            // apply so a single frame fully propagates a state change.
            .add_systems(
                Update,
                (
                    detect_audio_environment,
                    smooth_audio_environment,
                    apply_audio_environment,
                )
                    .chain()
                    .after(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::CoreSimulation),
            )
            // Neutral music intent: the content layer resolves Ambition
            // encounter/boss/room/radio gameplay into a content-agnostic
            // `MusicIntent`, then the director consumes only that resource.
            .init_resource::<ambition_audio::music::MusicIntent>()
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
            // Publish the gameplay goal after all simulation source facts.
            .add_systems(
                Update,
                (
                    crate::music::release_narrative_music_on_room_change,
                    crate::music::compute_music_intent,
                )
                    .chain()
                    .run_if(simulation_authorized)
                    .in_set(ambition_audio::music::MusicGoalPublication)
                    .after(ambition_platformer2d_shared_tangle::schedule::GameplaySimulationRoot),
            )
            // Clear old requests before the new session can author requests.
            .add_systems(
                Update,
                reset_audio_request_state_on_context_change
                    .in_set(ambition_platformer2d_shared_tangle::lifecycle::SessionScopeSet::Activate)
                    .before(ambition_platformer2d_shared_tangle::schedule::GameplaySimulationRoot)
                    .before(audio_play_sfx_messages)
                    .before(crate::music::compute_music_intent)
                    .before(apply_frontend_music_policy),
            )
            // Context authority and frontend goals also run outside gameplay.
            .add_systems(
                Update,
                apply_frontend_music_policy
                    .before(crate::music::compute_music_intent)
                    .in_set(ambition_audio::music::MusicGoalPublication)
                    .after(ambition_platformer2d_shared_tangle::schedule::GameplaySimulationRoot),
            );
        app.add_systems(
            Update,
            ambition_audio::music::drive_music_director
                .after(ambition_audio::music::MusicGoalPublication)
                .after(apply_frontend_music_policy),
        );
        // After the backend plugin, which owns the channel store on a device.
        ambition_audio::music::add_music_playback(app);
        app.init_resource::<ambition_audio::music::diagnostics::MusicPlaybackDiagnostics>()
            .add_systems(Update, publish_audio_readiness)
            .add_systems(
                PostUpdate,
                ambition_audio::music::diagnostics::observe_music_playback
                    .after(bevy_kira_audio::AudioSystemSet::PlayTypedChannels)
                    .after(bevy_kira_audio::AudioSystemSet::PlayDynamicChannels)
                    .after(ambition_audio::music::playback::advance_music_playback),
            );
    }
}

fn publish_audio_readiness(
    readiness: Option<Res<InitialGameplayReadiness>>,
    gate: Option<Res<SessionGatedSimulation>>,
    scope: Option<Res<ActiveSessionScope>>,
    roots: Query<&ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>,
    mut conditions: ResMut<ambition_audio::output::AudioPlaybackConditions>,
) {
    conditions.initial_presentation_ready =
        readiness.as_deref().is_none_or(|ready| ready.is_ready());
    conditions.music_context_required = simulation_authorized(gate, readiness, scope, roots);
}

#[derive(SystemParam)]
struct AudioRequestState<'w> {
    radio: Option<ResMut<'w, ambition_audio::library::RadioStationState>>,
    narrative: Option<ResMut<'w, ambition_conversation::NarrativeMusicRequest>>,
    sfx_playback: Option<ResMut<'w, ambition_audio::render::SfxPlaybackState>>,
}

/// Clear old requests before the new session publishes its requests.
fn reset_audio_request_state_on_context_change(
    mut changes: MessageReader<ambition_audio::selection::AudioContextChanged>,
    mut activated: MessageReader<
        ambition_platformer2d_shared_tangle::lifecycle::SessionScopeActivated,
    >,
    selection: Res<ambition_audio::selection::ActiveAudioSelection>,
    sfx_channel: Res<bevy_kira_audio::prelude::AudioChannel<SfxChannel>>,
    mut state: AudioRequestState,
) {
    let current = selection.owner();
    let reset = changes
        .read()
        .filter(|change| change.current == current && change.previous != change.current)
        .count()
        > 0;
    let session_reset = activated.read().count() > 0;
    if !reset && !session_reset {
        return;
    }

    if let Some(radio) = state.radio.as_deref_mut() {
        *radio = Default::default();
    }
    if let Some(narrative) = state.narrative.as_deref_mut() {
        narrative.clear();
    }
    if let Some(sfx) = state.sfx_playback.as_deref_mut() {
        sfx.last_played = None;
    }
    sfx_channel.stop();
}

/// Frontend policy publishes through the same goal as gameplay.
fn apply_frontend_music_policy(
    selection: Res<ambition_audio::selection::ActiveAudioSelection>,
    library: Option<Res<ambition_audio::library::AudioLibrary>>,
    catalogs: Res<ambition_audio::music::AdaptiveMusicCatalogRegistry>,
    mut intent: ResMut<ambition_audio::music::MusicIntent>,
) {
    use ambition_audio::music::*;
    let owner = selection.owner();
    intent.context = owner;
    intent.outgoing = None;
    if selection.is_changed()
        || catalogs.is_changed()
        || intent.provider_id.as_deref() != selection.provider_id()
    {
        intent.provider_id = selection.provider_id().map(str::to_owned);
        let mut authority = selection.music_authority();
        if let Some(catalog) = selection
            .provider_id()
            .and_then(|provider| catalogs.catalog_for(provider))
        {
            authority.authorize_cues(catalog.cue_ids().map(str::to_owned));
        }
        intent.authority = authority;
    }
    if matches!(
        owner,
        Some(
            ambition_sfx::AudioContextOwner::Gameplay(_) | ambition_sfx::AudioContextOwner::Direct
        )
    ) {
        return;
    }
    let Some(id) = selection.preferred_track() else {
        intent.publish(MusicGoal::Silence {
            context: owner,
            reason: MusicSilenceReason::FrontendHasNoMusic,
        });
        return;
    };
    let (Some(library), Some(context), Some(provider)) =
        (library.as_deref(), owner, selection.provider_id())
    else {
        return;
    };
    let Some(track) = library.track(id) else {
        return;
    };
    intent.publish(MusicGoal::Play(MusicSelection {
        provider: provider.into(),
        generation: track.generation,
        claim: MusicClaim {
            context,
            session: None,
            scope: None,
            source: "frontend".into(),
            attempt: 0,
        },
        release_owner: None,
        choice: MusicChoice::Track(id.into()),
        continuity: MusicContinuity::SharedProgram,
        entry_policy: MusicEntryPolicy::ContinueCompatible,
        phase_policy: MusicPhasePolicy::default(),
        priority: 0,
        crossfade_s: 0.22,
        completed_state: None,
    }));
}

#[cfg(test)]
mod tests;
