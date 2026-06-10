use super::*;

pub(super) fn seconds_until_next_bar(cue: &MusicCueSpec, seconds_in_loop: f32) -> f32 {
    let bar = cue.seconds_per_bar().max(0.001);
    let rem = seconds_in_loop.rem_euclid(bar);
    if rem <= 0.001 {
        0.0
    } else {
        bar - rem
    }
}

pub(super) fn seconds_until_next_phrase_marker(
    cue: &MusicCueSpec,
    seconds_in_loop: f32,
    bars_per_phrase: f32,
) -> f32 {
    let phrase = (cue.seconds_per_bar() * bars_per_phrase.max(1.0)).max(0.001);
    let rem = seconds_in_loop.rem_euclid(phrase);
    if rem <= 0.001 {
        0.0
    } else {
        phrase - rem
    }
}
