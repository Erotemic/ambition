use super::*;

pub(super) fn apply_simple_music_intent(
    director: &mut MusicDirectorState,
    library: &mut AudioLibrary,
    asset_server: &AssetServer,
    music_state: &mut MusicPlaybackState,
    base_music_channel: &AudioChannel<MusicChannel>,
    simple_track_candidates: &[String],
) {
    let target = resolved_simple_track(library, simple_track_candidates);
    let needs_switch = director.last_simple_track.as_deref() != Some(target.as_str())
        || music_state.active_track != target;
    if needs_switch && library.track(&target).is_some() {
        info!(target: MUSIC_LOG_TARGET, "simple_music target={}", target);
        switch_to_music_track(
            library,
            asset_server,
            music_state,
            base_music_channel,
            &target,
        );
        director.last_simple_track = Some(target.clone());
        director.mode = MusicDirectorMode::SimpleTrack;
    }
}

/// Pick the first candidate id that exists in the library. The candidate list
/// is built content-side (priority: boss > encounter > radio > room > default)
/// in `super::super::intent`; here the director only owns the "does the library
/// have it?" decision and falls back to the last candidate (the content default
/// track) when none of the higher-priority ids are loaded.
fn resolved_simple_track(library: &AudioLibrary, candidates: &[String]) -> String {
    for candidate in candidates {
        if library.track(candidate).is_some() {
            return candidate.clone();
        }
    }
    candidates.last().cloned().unwrap_or_default()
}

/// Resume the base/simple track. `set_mode_to_simple_track` is false during
/// adaptive-outro overlap: the base channel fades back in, but the director stays
/// in `AdaptiveOutro` until the tail is complete.
pub(super) fn resume_simple_music(
    director: &mut MusicDirectorState,
    library: &mut AudioLibrary,
    asset_server: &AssetServer,
    music_state: &mut MusicPlaybackState,
    base_music_channel: &AudioChannel<MusicChannel>,
    simple_track_candidates: &[String],
    set_mode_to_simple_track: bool,
) {
    let target = resolved_simple_track(library, simple_track_candidates);
    if library.track(&target).is_some() {
        info!(
            target: MUSIC_LOG_TARGET,
            "resume_simple_music target={} set_mode={}",
            target,
            set_mode_to_simple_track,
        );
        switch_to_music_track(
            library,
            asset_server,
            music_state,
            base_music_channel,
            &target,
        );
        director.last_simple_track = Some(target.clone());
        if set_mode_to_simple_track {
            director.mode = MusicDirectorMode::SimpleTrack;
        }
    }
}
