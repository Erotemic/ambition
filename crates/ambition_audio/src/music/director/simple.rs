use super::*;

use crate::selection::MusicAuthority;

/// Keep only the priority candidates the active provider is permitted to play.
///
/// This is the provider-relative-authority gate (Issue 1): a track id present in
/// the process-wide combined [`AudioLibrary`] but foreign to the active provider
/// is dropped here, BEFORE the director resolves anything against the library, so
/// it can never drive the base channel. Frontend music is started by the
/// frontend policy from its own explicit governed context, not by this gameplay
/// director.
pub(super) fn authorized_candidates(
    authority: &MusicAuthority,
    candidates: &[String],
) -> Vec<String> {
    candidates
        .iter()
        .filter(|id| authority.allows(id))
        .cloned()
        .collect()
}

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

#[cfg(test)]
mod authority_tests {
    use super::*;
    use crate::selection::MusicAuthority;

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    /// Poison (Issue 1): Ambition ran, its resident room/radio/encounter request
    /// state still names Ambition tracks. Quit to Home, then Sanic activates.
    /// This frame's candidates carry the stale Ambition ids AND Sanic's default.
    /// Only Sanic's own track survives the authority gate — the Ambition track
    /// cannot play even though it exists in the combined library.
    #[test]
    fn a_foreign_providers_stale_track_is_filtered_out() {
        let authority = MusicAuthority::governed(ids(&["you_are_too_slow"]));
        let stale = ids(&[
            "ambition_boss_theme", // stale encounter request (higher priority!)
            "ambition_radio_lofi", // stale radio request
            "you_are_too_slow",    // Sanic's own default (lowest priority)
        ]);
        assert_eq!(
            authorized_candidates(&authority, &stale),
            ids(&["you_are_too_slow"]),
            "a Sanic session may only play Sanic-authored tracks"
        );
    }

    /// Under the active provider's own authority every one of its tracks passes,
    /// so Ambition's real priority order (boss > radio > room > default) is
    /// preserved unchanged — the gate is a no-op for the authoring provider.
    #[test]
    fn the_authoring_provider_keeps_its_full_priority_list() {
        let authority = MusicAuthority::governed(ids(&[
            "ambition_boss_theme",
            "ambition_radio_lofi",
            "ambition_room_calm",
        ]));
        let candidates = ids(&["ambition_boss_theme", "ambition_room_calm"]);
        assert_eq!(
            authorized_candidates(&authority, &candidates),
            candidates,
            "the provider that authored these tracks plays them in priority order"
        );
    }

    /// No active context authorizes no gameplay candidate. Frontend playback is
    /// driven by its explicit shell-owned profile instead.
    #[test]
    fn denied_authority_filters_everything() {
        let candidates = ids(&["a_possible_morning"]);
        assert!(authorized_candidates(&MusicAuthority::Denied, &candidates).is_empty());
    }

    /// A deliberately-silent provider (empty authorized set) drops everything;
    /// the director's separate silence gate then stops the base channel.
    #[test]
    fn a_silent_provider_authorizes_nothing() {
        let authority = MusicAuthority::governed(Vec::<String>::new());
        assert!(authority.is_deliberate_silence());
        assert!(
            authorized_candidates(&authority, &ids(&["anything", "at_all"])).is_empty(),
            "Mary-O authored no music: no candidate is authorized"
        );
    }
}
