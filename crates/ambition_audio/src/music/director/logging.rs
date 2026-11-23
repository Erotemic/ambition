use super::*;

pub(super) fn log_periodic_state(director: &mut MusicDirectorState, cue: &MusicCueSpec, dt: f32) {
    director.debug_log_timer -= dt;
    if director.debug_log_timer > 0.0 {
        return;
    }
    director.debug_log_timer = DEBUG_LOG_PERIOD_SECONDS;
    debug!(target: MUSIC_LOG_TARGET,
        "music_director mode={:?} cue={} state={:?} section={:?} elapsed={:.3}",
        director.mode, cue.id, director.current_state_id,
        director.current_section_id, director.seconds_in_mode);
}
