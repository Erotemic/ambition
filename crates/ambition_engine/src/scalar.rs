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
