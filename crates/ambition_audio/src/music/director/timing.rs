pub(super) fn seconds_until_next_bar(bar_seconds: f32, seconds_in_loop: f32) -> f32 {
    let bar = bar_seconds;
    let rem = seconds_in_loop.rem_euclid(bar);
    if rem <= 0.001 { 0.0 } else { bar - rem }
}

/// Seconds until the next phrase marker: a multiple of `bars_per_phrase` bars
/// from the start of the pass that plays, or the end of that pass when the
/// section loops (`pass_seconds`), whichever comes first. The grid starts
/// again with each pass of a loop, so the phrases of a loop with an odd
/// number of bars stay where its music has them.
pub(super) fn seconds_until_next_phrase_marker(
    bar_seconds: f32,
    seconds_in_loop: f32,
    bars_per_phrase: f32,
    pass_seconds: Option<f32>,
) -> f32 {
    let phrase = (bar_seconds * bars_per_phrase.max(1.0)).max(0.001);
    let pass = pass_seconds.filter(|seconds| *seconds > 0.001);
    let in_pass = pass.map_or(seconds_in_loop, |pass| seconds_in_loop.rem_euclid(pass));
    let rem = in_pass.rem_euclid(phrase);
    if rem <= 0.001 {
        return 0.0;
    }
    let to_marker = phrase - rem;
    pass.map_or(to_marker, |pass| to_marker.min(pass - in_pass))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A three-bar loop with two-bar phrases (GNU-ton's fight loop is 73
    /// bars): the marker of each pass is at its bar 2, and the end of a pass
    /// is a marker. A grid that did not start again put the markers of the
    /// second pass on its odd bars.
    #[test]
    fn the_phrase_grid_starts_again_with_each_pass_of_a_loop() {
        let pass = Some(12.0);
        // Pass 1, half a bar before its end: the end of the pass.
        assert_eq!(seconds_until_next_phrase_marker(4.0, 10.0, 2.0, pass), 2.0);
        // Pass 2, half a bar in: its bar 2, not bar 1.
        assert_eq!(seconds_until_next_phrase_marker(4.0, 14.0, 2.0, pass), 6.0);
        // A section that does not loop keeps one grid.
        assert_eq!(seconds_until_next_phrase_marker(4.0, 14.0, 2.0, None), 2.0);
    }
}
