//! Time domain plumbing: clocks (ADR 0010/0011), time-control authority,
//! per-entity proper-time scale, and game-feel tuning. Camera ease/shake
//! vocabulary lives in `ambition_platformer_primitives::camera_ease`.

pub mod feel;
pub mod time_control;
pub mod world_time;

/// Approach `target` from `value` by at most `delta`. Used for time-scale
/// ramping and similar 1-D eases throughout the sandbox.
pub fn move_toward(value: f32, target: f32, delta: f32) -> f32 {
    if value < target {
        (value + delta).min(target)
    } else {
        (value - delta).max(target)
    }
}
