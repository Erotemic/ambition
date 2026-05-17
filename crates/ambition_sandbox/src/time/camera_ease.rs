//! Smoothed camera scale + world-target state with tunable ease rates.
//!
//! Extracted from `lib.rs` during the themed-module reorg. Belongs to the
//! `time/` umbrella because the values are time-driven scalars (rate-per-
//! second eases), even though the consumer is presentation. Re-exported
//! at the crate root for backward-compat call sites.

use ambition_engine as ae;
use bevy::prelude::Resource;

/// Live camera scale + ease state. The camera reads target scale from
/// the encounter registry (or developer overview override) every
/// frame; this resource holds the smoothed value so transitions feel
/// like a breath instead of a snap.
#[derive(Resource, Clone, Copy, Debug)]
pub struct CameraEaseState {
    pub live_scale: f32,
    /// Smoothed world-space camera target. Presentation-only: avoids hard
    /// jumps when look-ahead flips with facing or when framing presets change.
    pub live_target_world: ae::Vec2,
    pub target_initialized: bool,
}

impl Default for CameraEaseState {
    fn default() -> Self {
        Self {
            live_scale: 1.0,
            live_target_world: ae::Vec2::ZERO,
            target_initialized: false,
        }
    }
}

/// Scale-units per second when easing camera *into* an encounter
/// (zoom-out). Faster than the recovery rate so the player feels the
/// arena widen quickly when the lock-wall slams.
pub const DEFAULT_CAMERA_ZOOM_OUT_RATE: f32 = 1.6;

/// Scale-units per second when easing camera *out of* an encounter
/// (zoom-in). Slower than zoom-out; the post-fight breathing room is
/// the moment to savor.
pub const DEFAULT_CAMERA_ZOOM_IN_RATE: f32 = 0.9;

/// Below this absolute delta the camera-ease snap completes — prevents
/// floating-point drift from accumulating into never-converges
/// territory at the tail of the ease.
pub const DEFAULT_CAMERA_ZOOM_SNAP_EPSILON: f32 = 0.0025;

/// Tunable knobs for the camera-ease behavior. Replaces the
/// hardcoded `CAMERA_ZOOM_{IN,OUT}_RATE` constants so the sandbox or
/// tests can override the rates without recompiling. The defaults
/// match the previous constants (`1.6` zoom-out, `0.9` zoom-in).
///
/// `target_scale > live_scale` (zooming out) uses `zoom_out_rate`;
/// the inverse direction uses `zoom_in_rate`. `snap_epsilon` is the
/// distance at which the ease finalizes onto the target value.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct CameraEaseTuning {
    /// Scale-units per second when easing into a wider view
    /// (encounter starts; lock-wall slam moment).
    pub zoom_out_rate: f32,
    /// Scale-units per second when easing back to the close view
    /// (post-encounter breathing room).
    pub zoom_in_rate: f32,
    /// Snap-to-target threshold to terminate the ease.
    pub snap_epsilon: f32,
}

impl Default for CameraEaseTuning {
    fn default() -> Self {
        Self {
            zoom_out_rate: DEFAULT_CAMERA_ZOOM_OUT_RATE,
            zoom_in_rate: DEFAULT_CAMERA_ZOOM_IN_RATE,
            snap_epsilon: DEFAULT_CAMERA_ZOOM_SNAP_EPSILON,
        }
    }
}
