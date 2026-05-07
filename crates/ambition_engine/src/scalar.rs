//! Small scalar utilities used by the deterministic simulation.
//!
//! Vector math comes from `bevy_math::Vec2`, matching Bevy's math stack. This module
//! only keeps Ambition-specific scalar helpers that are not worth pulling from a
//! larger crate yet.

/// Move `value` toward `target` by at most `delta`.
///
/// This is the workhorse for acceleration, friction, and dummy knockback decay.
/// Keeping it here avoids each module inventing slightly different easing code.
pub fn approach(value: f32, target: f32, delta: f32) -> f32 {
    if value < target {
        (value + delta).min(target)
    } else {
        (value - delta).max(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approach_below_target_steps_up_clamped() {
        // Halfway short of target: full delta is applied.
        assert!((approach(0.0, 10.0, 3.0) - 3.0).abs() < 1e-6);
        // Larger delta than gap: lands exactly on target.
        assert_eq!(approach(0.0, 10.0, 50.0), 10.0);
    }

    #[test]
    fn approach_above_target_steps_down_clamped() {
        assert!((approach(10.0, 0.0, 3.0) - 7.0).abs() < 1e-6);
        // Overshoot from above also clamps to target.
        assert_eq!(approach(10.0, 0.0, 50.0), 0.0);
    }

    #[test]
    fn approach_at_target_is_no_op() {
        assert_eq!(approach(5.0, 5.0, 100.0), 5.0);
    }

    #[test]
    fn approach_zero_delta_is_no_op() {
        assert_eq!(approach(2.0, 7.0, 0.0), 2.0);
        assert_eq!(approach(9.0, 1.0, 0.0), 9.0);
    }

    #[test]
    fn approach_handles_negatives() {
        assert!((approach(-3.0, 0.0, 1.0) - (-2.0)).abs() < 1e-6);
        assert!((approach(0.0, -3.0, 1.0) - (-1.0)).abs() < 1e-6);
    }
}
