use super::*;

pub(super) fn log_periodic_state(director: &mut MusicDirectorState, cue: &MusicCueSpec, dt: f32) {
    director.debug_log_timer -= dt;
    if director.debug_log_timer > 0.0 {
        return;
    }
    director.debug_log_timer = DEBUG_LOG_PERIOD_SECONDS;
    debug!(
        target: MUSIC_LOG_TARGET,
        "music_director mode={:?} cue={:?} state={:?} section={:?} t_mode={:.3} t_loop={:.3} bar_beat={} active_bank={} gains_a={} gains_b={}",
        director.mode,
        director.active_cue_id,
        director.current_state_id,
        director.current_section_id,
        director.seconds_in_mode,
        director.seconds_in_loop,
        format_bar_beat(cue, director.seconds_in_loop),
        director.active_bank.label(),
        format_gains(director.current_gains[MusicBank::A.index()]),
        format_gains(director.current_gains[MusicBank::B.index()]),
    );
}

pub(super) fn format_bar_beat(cue: &MusicCueSpec, seconds: f32) -> String {
    let beat = seconds / cue.seconds_per_beat();
    let beats_per_bar = cue.beats_per_bar.max(1.0);
    let bar = (beat / beats_per_bar).floor() as i32 + 1;
    let beat_in_bar = beat.rem_euclid(beats_per_bar) + 1.0;
    format!("{}.{}", bar, beat_in_bar.floor() as i32)
}

pub(super) fn format_gains(gains: LayerGains) -> String {
    gains
        .iter()
        .map(|g| format!("{g:.2}"))
        .collect::<Vec<_>>()
        .join(",")
}
