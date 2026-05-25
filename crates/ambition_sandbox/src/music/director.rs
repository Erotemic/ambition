use super::*;

mod adaptive;
mod gains;
mod loader;
mod logging;
mod resolver;
mod simple;
mod timing;

pub use loader::load_music_cues;

use adaptive::{drive_adaptive_cue_state, drive_outro_tail, shutdown_adaptive_cue};
use gains::update_gain_smoothing;
use logging::log_periodic_state;
pub(super) use resolver::resolve_adaptive_directive;
use simple::apply_simple_music_intent;

#[cfg(test)]
pub(super) use adaptive::should_restart_adaptive;
#[cfg(test)]
pub(super) use resolver::resolve_directive_for_binding;

/// Unified music director.
///
/// Handles both simple track selection and adaptive cue state transitions. The
/// simple track backend still reuses the existing `AudioLibrary` / `MusicChannel`
/// sources; adaptive cues use the generic layer-bank scheduler in this module.
pub fn drive_music_director(
    time: Res<Time>,
    catalog: Option<Res<MusicCueCatalog>>,
    assets: Option<Res<LoadedMusicCueAssets>>,
    director: Option<ResMut<MusicDirectorState>>,
    encounters: Res<EncounterRegistry>,
    mut encounter_music: ResMut<EncounterMusicRequest>,
    mut boss_music: ResMut<BossEncounterMusicRequest>,
    room_music: Res<RoomMusicRequest>,
    layer_channels: MusicLayerChannels,
    base_music_channel: Res<AudioChannel<MusicChannel>>,
    mut library: ResMut<AudioLibrary>,
    asset_server: Res<AssetServer>,
    mut music_state: ResMut<MusicPlaybackState>,
    radio: Option<Res<RadioStationState>>,
    sandbox_data: Res<SandboxDataSpec>,
    settings: Res<UserSettings>,
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

    let adaptive = resolve_adaptive_directive(&catalog, &encounters, &director);
    match adaptive {
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
                    &room_music,
                    radio.as_deref(),
                    &sandbox_data,
                    &mut encounter_music,
                    &mut boss_music,
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
                    &room_music,
                    radio.as_deref(),
                    &sandbox_data,
                    &mut encounter_music,
                    &mut boss_music,
                );
            } else {
                apply_simple_music_intent(
                    &mut director,
                    &mut library,
                    &asset_server,
                    &mut music_state,
                    &base_music_channel,
                    &room_music,
                    radio.as_deref(),
                    &sandbox_data,
                    &mut encounter_music,
                    &mut boss_music,
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
                &room_music,
                radio.as_deref(),
                &sandbox_data,
                &mut encounter_music,
                &mut boss_music,
            );
            log_periodic_state(&mut director, cue, dt);
        }
    }
}
