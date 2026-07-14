use super::*;

mod adaptive;
mod gains;
mod loader;
mod logging;
mod simple;
mod timing;

pub use loader::load_music_cues;

use adaptive::{drive_adaptive_cue_state, drive_outro_tail, shutdown_adaptive_cue};
use gains::update_gain_smoothing;
use logging::log_periodic_state;
use simple::apply_simple_music_intent;

#[cfg(test)]
pub(super) use adaptive::should_restart_adaptive;

/// Gate an adaptive directive by provider authority: an unauthorized `Play`
/// (a cue the active provider did not author) is downgraded to `None`, so any
/// running adaptive layer is shut down and the base channel resumes rather than
/// a foreign cue starting. `StopNow` and `None` pass through unchanged. This is
/// the adaptive analogue of [`simple::authorized_candidates`].
fn authorized_adaptive(
    authority: &crate::selection::MusicAuthority,
    adaptive: Option<AdaptiveCueDirective>,
) -> Option<AdaptiveCueDirective> {
    match adaptive {
        Some(AdaptiveCueDirective::Play { cue_id, .. }) if !authority.allows_cue(&cue_id) => None,
        other => other,
    }
}

/// Content-agnostic music director.
///
/// Handles both simple track selection and adaptive cue state transitions. It
/// reads only the neutral [`MusicIntent`] (the game's content layer resolves
/// "which cue / track for which game event" into that resource — see
/// [`super::intent`]) plus its own catalog/assets/audio backend. It names no
/// encounter, boss, room, or track, so the machinery is reusable across games.
///
/// The simple track backend reuses the existing `AudioLibrary` / `MusicChannel`
/// sources; adaptive cues use the generic layer-bank scheduler in this module.
pub fn drive_music_director(
    time: Res<Time>,
    catalogs: Res<AdaptiveMusicCatalogRegistry>,
    assets: Option<ResMut<LoadedMusicCueAssets>>,
    director: Option<ResMut<MusicDirectorState>>,
    intent: Res<MusicIntent>,
    layer_channels: MusicLayerChannels,
    base_music_channel: Res<AudioChannel<MusicChannel>>,
    mut library: ResMut<AudioLibrary>,
    asset_server: Res<AssetServer>,
    mut music_state: ResMut<MusicPlaybackState>,
    settings: Res<MusicMix>,
) {
    let output = layer_channels.output_mode();
    let provider_id = intent.provider_id.as_deref();
    let catalog = provider_id.and_then(|provider| catalogs.catalog_for(provider));
    let Some(mut assets) = assets else {
        return;
    };
    let Some(mut director) = director else {
        return;
    };

    let dt = time.delta_secs();
    director.seconds_in_mode += dt;
    if director.mode == MusicDirectorMode::AdaptiveLoop {
        director.seconds_in_loop += dt;
    }

    // Provider-relative authority (Issues 1 & 2). A governed-but-empty authority
    // is a DELIBERATE stop: the active provider authored no music, so nothing —
    // neither a simple track nor an adaptive cue — may play. Silence exactly once
    // and leave the backend idle (guarded so we do not re-stop every frame).
    if intent.authority.is_deliberate_silence() {
        let already_silent =
            director.mode == MusicDirectorMode::Idle && music_state.active_track.is_empty();
        if !already_silent {
            super::silence_music_backend(
                &base_music_channel,
                &layer_channels,
                &mut director,
                &mut music_state,
            );
        }
        return;
    }

    // Only tracks the active provider authored may drive the base channel. A
    // stale candidate carried over from another provider's resident request
    // state is filtered out here, so it can never be resolved against the
    // combined library.
    let authorized =
        simple::authorized_candidates(&intent.authority, &intent.simple_track_candidates);
    let candidates = authorized.as_slice();
    // An adaptive cue the active provider did not author must not start, even
    // when the cue exists in the process-wide catalog and a (stale) directive
    // requests it. Downgrade an unauthorized `Play` to `None` so any running
    // adaptive layer is shut down and the base channel resumes — the adaptive
    // analogue of `simple::authorized_candidates`.
    let adaptive = authorized_adaptive(&intent.authority, intent.adaptive.clone());
    match adaptive {
        Some(AdaptiveCueDirective::Play { cue_id, state_id }) => {
            if let (Some(cue), Some(target_state)) = (
                catalog.and_then(|catalog| catalog.cue(&cue_id)),
                catalog
                    .and_then(|catalog| catalog.cue(&cue_id))
                    .and_then(|cue| cue.state(&state_id)),
            ) {
                // Lazily pull this provider's cue sources on first play. Cue ids
                // are provider-local, so the loaded-source key includes provider.
                let provider_id = provider_id.expect("selected adaptive catalog has a provider");
                assets.ensure_cue_loaded(provider_id, cue, &asset_server);
                drive_adaptive_cue_state(
                    &mut director,
                    provider_id,
                    cue,
                    target_state,
                    &assets,
                    &layer_channels,
                    &base_music_channel,
                    &settings,
                    dt,
                );
            } else {
                warn!(
                    target: MUSIC_LOG_TARGET,
                    "adaptive directive references missing cue/state cue={} state={}",
                    cue_id,
                    state_id,
                );
            }
        }
        Some(AdaptiveCueDirective::StopNow) => {
            if director.active_cue_id.is_some() {
                shutdown_adaptive_cue(
                    &mut director,
                    &layer_channels,
                    &mut library,
                    &asset_server,
                    &mut music_state,
                    &base_music_channel,
                    output,
                    candidates,
                );
            }
        }
        None => {
            if director.active_cue_id.is_some()
                && director.mode != MusicDirectorMode::AdaptiveFinished
                && director.mode != MusicDirectorMode::Idle
            {
                // Leaving the room or losing the cue owner without a clear should
                // not leave the adaptive channels running.
                shutdown_adaptive_cue(
                    &mut director,
                    &layer_channels,
                    &mut library,
                    &asset_server,
                    &mut music_state,
                    &base_music_channel,
                    output,
                    candidates,
                );
            } else {
                apply_simple_music_intent(
                    &mut director,
                    &mut library,
                    &asset_server,
                    &mut music_state,
                    &base_music_channel,
                    output,
                    candidates,
                );
            }
        }
    }

    if let Some(cue_id) = director.active_cue_id.clone() {
        if let Some(cue) = catalog.and_then(|catalog| catalog.cue(&cue_id)) {
            update_gain_smoothing(&mut director, &layer_channels, dt);
            drive_outro_tail(
                &mut director,
                cue,
                &layer_channels,
                &mut library,
                &asset_server,
                &mut music_state,
                &base_music_channel,
                output,
                candidates,
            );
            log_periodic_state(&mut director, cue, dt);
        }
    }
}

#[cfg(test)]
mod adaptive_authority_tests {
    use super::authorized_adaptive;
    use crate::music::AdaptiveCueDirective;
    use crate::selection::MusicAuthority;

    fn play(cue: &str) -> Option<AdaptiveCueDirective> {
        Some(AdaptiveCueDirective::Play {
            cue_id: cue.to_string(),
            state_id: "intro".to_string(),
        })
    }

    #[test]
    fn a_foreign_adaptive_cue_is_downgraded_to_stop() {
        // Sanic's session (authorizes no cues) must not start Ambition's
        // goblin cue even if a stale directive requests it.
        let mut sanic = MusicAuthority::governed(vec!["you_are_too_slow".to_string()]);
        sanic.authorize_cues(Vec::<String>::new());
        assert_eq!(
            authorized_adaptive(&sanic, play("first_goblin_tune_v2")),
            None,
            "an unauthorized adaptive cue is downgraded so the layer shuts down"
        );
    }

    #[test]
    fn the_authoring_provider_keeps_its_cue() {
        let mut ambition = MusicAuthority::governed(vec!["a_possible_morning".to_string()]);
        ambition.authorize_cues(vec!["first_goblin_tune_v2".to_string()]);
        assert_eq!(
            authorized_adaptive(&ambition, play("first_goblin_tune_v2")),
            play("first_goblin_tune_v2"),
            "the provider that authored the cue keeps it"
        );
    }

    #[test]
    fn denied_context_blocks_play_while_stop_passes_through() {
        assert_eq!(
            authorized_adaptive(&MusicAuthority::Denied, play("anything")),
            None
        );
        assert_eq!(
            authorized_adaptive(
                &MusicAuthority::governed(Vec::<String>::new()),
                Some(AdaptiveCueDirective::StopNow)
            ),
            Some(AdaptiveCueDirective::StopNow)
        );
    }
}

#[cfg(test)]
mod restart_tests {
    use super::*;
    use crate::music::MusicDirectorMode;

    #[test]
    fn should_restart_adaptive_when_cue_id_changes() {
        // The classic case: a different cue is taking over.
        assert!(should_restart_adaptive(
            Some("first_goblin_tune_v2"),
            MusicDirectorMode::AdaptiveLoop,
            "boss_intro_v1",
            false,
        ));
    }

    #[test]
    fn should_restart_adaptive_when_no_cue_was_active() {
        // No prior adaptive cue — definitely need to set one up.
        assert!(should_restart_adaptive(
            None,
            MusicDirectorMode::SimpleTrack,
            "first_goblin_tune_v2",
            false,
        ));
    }

    #[test]
    fn should_not_restart_adaptive_on_same_cue_in_loop() {
        // Steady-state: cue is already running its loop. Moving between
        // wave states does NOT reset the adaptive cue from its intro.
        assert!(!should_restart_adaptive(
            Some("first_goblin_tune_v2"),
            MusicDirectorMode::AdaptiveLoop,
            "first_goblin_tune_v2",
            false,
        ));
    }

    #[test]
    fn should_restart_adaptive_when_mode_says_simple_track_playing() {
        // Defensive: if anything leaves the director in mode=SimpleTrack
        // while still claiming an active adaptive cue, the new directive
        // must stop the base channel before the adaptive layers ramp up.
        // (The primary fix prevents this state from being created, but
        // the predicate is robust against other code paths.)
        assert!(should_restart_adaptive(
            Some("first_goblin_tune_v2"),
            MusicDirectorMode::SimpleTrack,
            "first_goblin_tune_v2",
            false,
        ));
        // Same for the post-outro Idle/Finished states.
        assert!(should_restart_adaptive(
            Some("first_goblin_tune_v2"),
            MusicDirectorMode::Idle,
            "first_goblin_tune_v2",
            false,
        ));
        assert!(should_restart_adaptive(
            Some("first_goblin_tune_v2"),
            MusicDirectorMode::AdaptiveFinished,
            "first_goblin_tune_v2",
            false,
        ));
    }

    #[test]
    fn should_restart_adaptive_when_outro_returns_to_active_state() {
        // The Jon-2026-05-09 race: encounter cleared → outro tail
        // playing AND base lofi playing (overlap), then encounter
        // restarts → directive points to a non-outro state. Without
        // this guard, the same-cue match would skip the
        // stop-base-channel path and lofi + adaptive layers play
        // simultaneously.
        assert!(should_restart_adaptive(
            Some("first_goblin_tune_v2"),
            MusicDirectorMode::AdaptiveOutro,
            "first_goblin_tune_v2",
            false, // target_state is e.g. wave1, NOT the outro
        ));
    }

    #[test]
    fn should_not_restart_adaptive_when_outro_continues_to_outro() {
        // The encounter is still in its cleared/outro phase; the same
        // outro keeps tailing. No restart.
        assert!(!should_restart_adaptive(
            Some("first_goblin_tune_v2"),
            MusicDirectorMode::AdaptiveOutro,
            "first_goblin_tune_v2",
            true, // target_state IS the outro
        ));
    }
}
