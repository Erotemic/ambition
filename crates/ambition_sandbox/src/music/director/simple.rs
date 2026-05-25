use super::*;

pub(super) fn apply_simple_music_intent(
    director: &mut MusicDirectorState,
    library: &mut AudioLibrary,
    asset_server: &AssetServer,
    music_state: &mut MusicPlaybackState,
    base_music_channel: &AudioChannel<MusicChannel>,
    room_music: &RoomMusicRequest,
    radio: Option<&RadioStationState>,
    sandbox_data: &SandboxDataSpec,
    encounter_music: &mut EncounterMusicRequest,
    boss_music: &mut BossEncounterMusicRequest,
) {
    let target = resolved_simple_track(
        library,
        room_music,
        radio,
        sandbox_data,
        encounter_music,
        boss_music,
    );
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
    encounter_music.last_applied = Some(target.clone());
    boss_music.last_applied = Some(target);
}

fn resolved_simple_track(
    library: &AudioLibrary,
    room_music: &RoomMusicRequest,
    radio: Option<&RadioStationState>,
    sandbox_data: &SandboxDataSpec,
    encounter_music: &EncounterMusicRequest,
    boss_music: &BossEncounterMusicRequest,
) -> String {
    // Priority: boss-encounter music wins over the regular encounter
    // music wins over radio wins over room default. Boss is split
    // into its own resource so the per-frame regular-encounter
    // writeback (`encounter/systems.rs::update_encounters_from_world`)
    // — which writes `desired_track = None` when no in-flight
    // regular encounter exists — can't clobber the boss's
    // `MusicRequested` events.
    if let Some(track) = &boss_music.desired_track {
        if library.track(track).is_some() {
            return track.clone();
        }
    }
    if let Some(track) = &encounter_music.desired_track {
        if library.track(track).is_some() {
            return track.clone();
        }
    }
    if let Some(track) = radio.and_then(|radio| radio.selected_track()) {
        if library.track(track).is_some() {
            return track.to_string();
        }
    }
    room_music
        .desired_track
        .as_ref()
        .filter(|track| library.track(track).is_some())
        .cloned()
        .unwrap_or_else(|| sandbox_data.audio.default_music_track.clone())
}

/// Decide whether `drive_adaptive_cue_state` should stop the base
/// (simple-track) channel and (re)start the adaptive cue from its
/// intro.
///
/// Three conditions trigger a restart, all preserving the invariant
/// that **simple base track and adaptive layers cannot remain
/// audible at the same time**:
///
/// 1. A different cue is taking over (the obvious case).
/// 2. The director's mode says a simple base track is playing
///    (`SimpleTrack`, `Idle`, `AdaptiveFinished`). Defensive: the
///    primary `resume_simple_music(set_mode = false)` fix prevents
///    this state from coexisting with `Some(active_cue_id)`, but
///    if anything leaves the director in that shape we still need
///    to stop the base channel before the adaptive layers ramp up.
/// 3. The cue is in `AdaptiveOutro` and the new directive points
///    back to a non-outro state — i.e. the encounter restarted
///    during the outro tail. `drive_outro_tail` had already started
///    the base lofi channel for the overlap; we must stop it
///    before the adaptive layers come back.
///
/// Captured as a free function so the decision can be unit-tested
/// without spinning up Bevy resources (audio channels, asset
/// server, etc.).
pub(super) fn resume_simple_music(
    director: &mut MusicDirectorState,
    library: &mut AudioLibrary,
    asset_server: &AssetServer,
    music_state: &mut MusicPlaybackState,
    base_music_channel: &AudioChannel<MusicChannel>,
    room_music: &RoomMusicRequest,
    radio: Option<&RadioStationState>,
    sandbox_data: &SandboxDataSpec,
    encounter_music: &mut EncounterMusicRequest,
    boss_music: &mut BossEncounterMusicRequest,
    set_mode_to_simple_track: bool,
) {
    let target = resolved_simple_track(
        library,
        room_music,
        radio,
        sandbox_data,
        encounter_music,
        boss_music,
    );
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
        encounter_music.last_applied = Some(target.clone());
        boss_music.last_applied = Some(target);
        if set_mode_to_simple_track {
            director.mode = MusicDirectorMode::SimpleTrack;
        }
    }
}
