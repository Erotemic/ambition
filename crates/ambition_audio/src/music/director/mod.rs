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
    catalog: Option<Res<MusicCueCatalog>>,
    assets: Option<Res<LoadedMusicCueAssets>>,
    director: Option<ResMut<MusicDirectorState>>,
    intent: Res<MusicIntent>,
    layer_channels: MusicLayerChannels,
    base_music_channel: Res<AudioChannel<MusicChannel>>,
    mut library: ResMut<AudioLibrary>,
    asset_server: Res<AssetServer>,
    mut music_state: ResMut<MusicPlaybackState>,
    settings: Res<MusicMix>,
) {
    let Some(catalog) = catalog else {
        return;
    };
    let Some(assets) = assets else {
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

    let candidates = intent.simple_track_candidates.as_slice();
    match intent.adaptive.clone() {
        Some(AdaptiveCueDirective::Play { cue_id, state_id }) => {
            if let (Some(cue), Some(target_state)) = (
                catalog.cue(&cue_id),
                catalog.cue(&cue_id).and_then(|cue| cue.state(&state_id)),
            ) {
                drive_adaptive_cue_state(
                    &mut director,
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
                    candidates,
                );
            } else {
                apply_simple_music_intent(
                    &mut director,
                    &mut library,
                    &asset_server,
                    &mut music_state,
                    &base_music_channel,
                    candidates,
                );
            }
        }
    }

    if let Some(cue_id) = director.active_cue_id.clone() {
        if let Some(cue) = catalog.cue(&cue_id) {
            update_gain_smoothing(&mut director, &layer_channels, dt);
            drive_outro_tail(
                &mut director,
                cue,
                &layer_channels,
                &mut library,
                &asset_server,
                &mut music_state,
                &base_music_channel,
                candidates,
            );
            log_periodic_state(&mut director, cue, dt);
        }
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
