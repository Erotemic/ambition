//! Input prediction and retention for the hosted world.

use ambition_platformer2d_core::ControlFrame;
use std::collections::BTreeMap;

/// Use all recorded input for an exact tick. Use only the held levels of earlier input.
pub(super) fn predicted_input(history: &BTreeMap<u64, ControlFrame>, tick: u64) -> ControlFrame {
    match history.range(..=tick).next_back() {
        Some((&at, &frame)) if at == tick => frame,
        Some((_, frame)) => frame.levels_only(),
        None => ControlFrame::default(),
    }
}

/// Keep the last earlier input so prediction can cross the retention boundary.
pub(super) fn retain_input_from(history: &mut BTreeMap<u64, ControlFrame>, first_tick: u64) {
    let predecessor = history
        .range(..first_tick)
        .next_back()
        .map(|(&tick, _)| tick);
    history.retain(|&tick, _| tick >= first_tick || Some(tick) == predecessor);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gap_keeps_the_predecessors_levels_without_repeating_its_edge() {
        let earlier = ControlFrame {
            axis_x: -1.0,
            ..ControlFrame::default()
        };
        let pressed = ControlFrame {
            axis_x: 1.0,
            jump_pressed: true,
            ..ControlFrame::default()
        };
        let mut history = BTreeMap::from([(2, earlier), (5, pressed), (8, earlier)]);

        assert_eq!(predicted_input(&history, 1), ControlFrame::default());
        assert_eq!(predicted_input(&history, 5), pressed);
        retain_input_from(&mut history, 7);
        assert_eq!(history.keys().copied().collect::<Vec<_>>(), [5, 8]);
        assert_eq!(predicted_input(&history, 7), pressed.levels_only());
        assert!(!predicted_input(&history, 7).jump_pressed);
        assert_eq!(predicted_input(&history, 8), earlier);
        assert_eq!(
            predicted_input(&BTreeMap::new(), 0),
            ControlFrame::default()
        );
    }
}
