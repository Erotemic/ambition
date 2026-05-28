//! Smoothed camera scale + world-target state with tunable ease rates.
//!
//! Extracted from `lib.rs` during the themed-module reorg. Belongs to the
//! `time/` umbrella because the values are time-driven scalars (rate-per-
//! second eases), even though the consumer is presentation. Re-exported
//! at the crate root for backward-compat call sites.

use crate::engine_core as ae;
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

/// Live camera-shake amplitude in world pixels. The follow system
/// reads this each frame to add a randomized offset to the camera
/// transform, then [`tick_camera_shake`] decays it toward zero.
///
/// Producers call [`CameraShakeState::kick`] with the desired
/// amplitude. The strongest kick wins (no addition / no overflow):
/// landing from a tall drop should saturate the shake budget, not
/// stack it. A trickle from a small bounce can't reset a still-active
/// big shake.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct CameraShakeState {
    /// Current shake amplitude in world pixels. Zero means "no shake."
    pub amplitude_px: f32,
    /// Seed bumped each frame so the random offset is deterministic
    /// within a frame (camera follow can call into multiple samples)
    /// but uncorrelated across frames.
    pub seed: u32,
}

impl CameraShakeState {
    /// Bump the active shake to at least `amplitude_px` if the kick
    /// is bigger than what's already in flight. Caps clamp keep gut-
    /// punch landings from making the entire screen unreadable.
    pub fn kick(&mut self, amplitude_px: f32) {
        const MAX_AMPLITUDE_PX: f32 = 14.0;
        let target = amplitude_px.max(0.0).min(MAX_AMPLITUDE_PX);
        if target > self.amplitude_px {
            self.amplitude_px = target;
        }
    }
}

/// Per-second decay rate of `CameraShakeState::amplitude_px`. At 30 px/s,
/// a 6 px shake (mid-strength land) decays to 0 in ~0.2 s — long enough
/// to feel a thump, short enough not to interfere with the next move.
pub const CAMERA_SHAKE_DECAY_PX_PER_S: f32 = 30.0;

/// Decay system: subtracts `CAMERA_SHAKE_DECAY_PX_PER_S * dt` from
/// `amplitude_px` and clamps at zero. Runs every frame on `Update`
/// before `camera_follow` so the follow logic sees the post-decay
/// amplitude.
pub fn tick_camera_shake(
    time: bevy::prelude::Res<bevy::prelude::Time>,
    mut shake: bevy::prelude::ResMut<CameraShakeState>,
) {
    let dt = time.delta_secs();
    shake.amplitude_px = (shake.amplitude_px - CAMERA_SHAKE_DECAY_PX_PER_S * dt).max(0.0);
    shake.seed = shake.seed.wrapping_add(1);
}

/// Below this incoming downward velocity, a landing produces no
/// screen shake — tiny hops and footstep landings shouldn't rattle
/// the camera.
pub const HARD_FALL_SHAKE_FLOOR_VY: f32 = 360.0;

/// Pixels-of-shake per (vy − floor_vy). `(720 - 360) * 1/60 = 6 px`
/// — a tall drop gets a meaty thump; terminal velocity falls
/// saturate at the 14-px cap inside [`CameraShakeState::kick`].
pub const HARD_FALL_SHAKE_GAIN: f32 = 1.0 / 60.0;

/// Compute the shake amplitude for a player landing transition. Pure
/// function so the trigger logic in `player_simulation_phase` is
/// unit-testable independent of the surrounding bevy plumbing.
///
/// Returns 0.0 when the landing isn't a hard fall (no transition, or
/// vy is below the dead-zone). Otherwise returns the post-gain
/// amplitude that should be fed to `shake.kick(...)`.
pub fn hard_fall_shake_amplitude(was_grounded: bool, on_ground: bool, pre_sim_vy: f32) -> f32 {
    if was_grounded || !on_ground {
        return 0.0;
    }
    let excess = (pre_sim_vy - HARD_FALL_SHAKE_FLOOR_VY).max(0.0);
    excess * HARD_FALL_SHAKE_GAIN
}

impl CameraShakeState {
    /// Cheap deterministic 2D offset within the current amplitude budget.
    /// xorshift on `seed` gives a per-frame value in `[-amp, +amp]`;
    /// independent xorshifts for x / y avoid the diagonal-only shake a
    /// naive `(s, s)` pair would produce.
    pub fn offset(&self) -> ae::Vec2 {
        if self.amplitude_px <= 0.05 {
            return ae::Vec2::ZERO;
        }
        let mut sx = self.seed.wrapping_mul(0x45d9f3b).wrapping_add(0x9e3779b9);
        sx ^= sx >> 17;
        sx = sx.wrapping_mul(0xed5ad4bb);
        let mut sy = self.seed.wrapping_mul(0x119de1f3).wrapping_add(0x85ebca6b);
        sy ^= sy >> 15;
        sy = sy.wrapping_mul(0xc2b2ae35);
        let to_unit = |s: u32| (s as f32 / u32::MAX as f32) * 2.0 - 1.0;
        ae::Vec2::new(
            to_unit(sx) * self.amplitude_px,
            to_unit(sy) * self.amplitude_px,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shake_starts_at_zero_amplitude() {
        let shake = CameraShakeState::default();
        assert_eq!(shake.amplitude_px, 0.0);
        // Zero amplitude → exactly Vec2::ZERO offset (no jitter at rest).
        assert_eq!(shake.offset(), ae::Vec2::ZERO);
    }

    #[test]
    fn kick_max_wins_no_stacking() {
        let mut shake = CameraShakeState::default();
        shake.kick(4.0);
        assert_eq!(shake.amplitude_px, 4.0);
        // Smaller kick after a big one should NOT reduce the active shake.
        shake.kick(1.0);
        assert_eq!(shake.amplitude_px, 4.0);
        // Bigger kick raises it.
        shake.kick(8.0);
        assert_eq!(shake.amplitude_px, 8.0);
    }

    #[test]
    fn kick_clamps_at_max_amplitude() {
        let mut shake = CameraShakeState::default();
        // 1000 px shake would white out the screen; cap holds.
        shake.kick(1000.0);
        assert!(shake.amplitude_px <= 14.0);
        assert!(shake.amplitude_px > 0.0);
    }

    #[test]
    fn kick_clamps_negative_to_zero() {
        let mut shake = CameraShakeState::default();
        shake.kick(-5.0);
        assert_eq!(shake.amplitude_px, 0.0);
    }

    #[test]
    fn offset_bounded_by_amplitude_and_independent_axes() {
        let mut shake = CameraShakeState::default();
        shake.kick(8.0);
        // Sample several seeds; both axes must stay inside ±amplitude.
        let mut max_x: f32 = 0.0;
        let mut max_y: f32 = 0.0;
        for s in 0..32u32 {
            shake.seed = s;
            let o = shake.offset();
            assert!(o.x.abs() <= shake.amplitude_px + 0.001, "x={} exceeded amp", o.x);
            assert!(o.y.abs() <= shake.amplitude_px + 0.001, "y={} exceeded amp", o.y);
            max_x = max_x.max(o.x.abs());
            max_y = max_y.max(o.y.abs());
        }
        // Both axes should produce non-trivial offsets across 32 seeds
        // (otherwise the xorshift is degenerate / x and y would be
        // correlated into a diagonal shake).
        assert!(max_x > 1.0, "x range too small: {max_x}");
        assert!(max_y > 1.0, "y range too small: {max_y}");
    }

    #[test]
    fn offset_below_dead_zone_is_zero() {
        let mut shake = CameraShakeState::default();
        shake.amplitude_px = 0.04; // below 0.05 dead-zone
        assert_eq!(shake.offset(), ae::Vec2::ZERO);
    }

    #[test]
    fn hard_fall_no_shake_when_already_grounded() {
        // Player was already grounded last frame → no landing → no shake.
        assert_eq!(
            hard_fall_shake_amplitude(true, true, 800.0),
            0.0,
            "no transition → no shake"
        );
    }

    #[test]
    fn hard_fall_no_shake_when_still_airborne() {
        // Was airborne and still airborne → no landing → no shake.
        assert_eq!(
            hard_fall_shake_amplitude(false, false, 800.0),
            0.0,
            "no landing → no shake"
        );
    }

    #[test]
    fn hard_fall_no_shake_below_floor_vy() {
        // A soft hop (vy < HARD_FALL_SHAKE_FLOOR_VY) shouldn't shake the camera.
        assert_eq!(
            hard_fall_shake_amplitude(false, true, 200.0),
            0.0,
            "soft landing → no shake"
        );
        // Right at the floor: still no shake (clamp at zero).
        assert_eq!(hard_fall_shake_amplitude(false, true, HARD_FALL_SHAKE_FLOOR_VY), 0.0);
    }

    #[test]
    fn hard_fall_amplitude_scales_with_excess_vy() {
        let amp_a = hard_fall_shake_amplitude(false, true, HARD_FALL_SHAKE_FLOOR_VY + 60.0);
        let amp_b = hard_fall_shake_amplitude(false, true, HARD_FALL_SHAKE_FLOOR_VY + 360.0);
        assert!(amp_a > 0.0, "kick fires above floor_vy");
        assert!(amp_b > amp_a, "bigger fall → bigger amplitude");
        // Amplitude scales linearly with excess: 6× the excess → 6× the kick.
        let ratio = amp_b / amp_a;
        assert!(
            (ratio - 6.0).abs() < 0.01,
            "ratio should be ~6.0, got {ratio}"
        );
    }

    #[test]
    fn hard_fall_saturates_through_kick_cap() {
        // Terminal-velocity fall produces a huge raw amplitude;
        // the `kick()` clamp is what enforces the 14-px cap.
        let raw = hard_fall_shake_amplitude(false, true, 5000.0);
        assert!(raw > 14.0, "raw amplitude exceeds cap, kick will clamp");
        let mut shake = CameraShakeState::default();
        shake.kick(raw);
        assert!(shake.amplitude_px <= 14.0);
    }
}
