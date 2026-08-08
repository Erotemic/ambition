//! Smoothed camera scale + world-target state with tunable ease rates, plus the
//! per-body [`PlayerBlinkCameraState`] the arrival ease reads.
//!
//! Moved below `ambition_platformer2d_actor_monolith` during F1.5 so render/host can share camera
//! presentation timing state without depending on the actor-domain crate.

use ambition_platformer2d_core as ae;
use bevy::prelude::{Component, Resource};

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
    /// **M2's no-backtrack watermark.** The furthest `+x` the camera has reached
    /// during the current visit to a `ForwardOnlyX` zone. `None` outside such a
    /// zone — which is what makes re-entering one a fresh scroll rather than a
    /// camera pinned to where it stopped an hour ago.
    pub scroll_watermark_x: Option<f32>,
}

impl Default for CameraEaseState {
    fn default() -> Self {
        Self {
            live_scale: 1.0,
            live_target_world: ae::Vec2::ZERO,
            target_initialized: false,
            scroll_watermark_x: None,
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

/// **How hard this game is allowed to shake, and how fast it settles.** (D14)
///
/// The cap was a `const` inside `kick` and the decay a module constant, so two
/// games in one host shook identically whether or not that suited either of
/// them. They are a resource now, published from the ACTIVE route's
/// `GameplayPresentationProfile` by the same selection system that publishes
/// viewport and framing — so a second game gets its own feel by declaring a
/// profile, not by editing the engine.
///
/// Defaults are exactly the old constants, which is what makes this safe to put
/// in front of every existing caller.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct CameraShakeTuning {
    /// Ceiling for a single kick, in world pixels. A gut-punch landing should
    /// saturate the budget rather than make the screen unreadable.
    pub max_amplitude_px: f32,
    /// Per-second decay toward zero.
    pub decay_px_per_s: f32,
}

impl Default for CameraShakeTuning {
    fn default() -> Self {
        Self {
            max_amplitude_px: DEFAULT_CAMERA_SHAKE_MAX_PX,
            decay_px_per_s: CAMERA_SHAKE_DECAY_PX_PER_S,
        }
    }
}

/// The historical cap, kept as the default rather than deleted: it is the number
/// every existing game was tuned against.
pub const DEFAULT_CAMERA_SHAKE_MAX_PX: f32 = 14.0;

impl CameraShakeState {
    /// Bump the active shake to at least `amplitude_px` if the kick is bigger
    /// than what is already in flight, clamped by `tuning`.
    ///
    /// The cap is a PARAMETER rather than a constant so the ceiling can differ
    /// per game. Strongest-kick-wins is unchanged: a trickle from a small bounce
    /// cannot reset a still-active big shake, and kicks do not stack.
    pub fn kick(&mut self, amplitude_px: f32, tuning: CameraShakeTuning) {
        let target = amplitude_px.max(0.0).min(tuning.max_amplitude_px.max(0.0));
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
    tuning: bevy::prelude::Res<CameraShakeTuning>,
    mut shake: bevy::prelude::ResMut<CameraShakeState>,
) {
    let dt = time.delta_secs();
    shake.amplitude_px = (shake.amplitude_px - tuning.decay_px_per_s * dt).max(0.0);
    shake.seed = shake.seed.wrapping_add(1);
}

/// Below this incoming downward velocity, a landing produces no
/// screen shake — tiny hops, normal-jump landings, and short drops
/// shouldn't rattle the camera. A jump-in-place from flat ground
/// lands at ~`JUMP_SPEED` (≈630 px/s), so the floor sits above that
/// to keep the hard-fall reaction reserved for genuinely tall drops
/// (8+ tiles, where `sqrt(2 * GRAVITY * h)` clears 700 px/s).
pub const HARD_FALL_SHAKE_FLOOR_VY: f32 = 700.0;

/// Pixels-of-shake per (vy − floor_vy). At terminal `MAX_FALL_SPEED`
/// (~950 px/s) the raw amplitude is `(950 - 700) / 60 ≈ 4 px` — a
/// visible but not screen-eating thump; fast-fall terminal (~1380)
/// reaches `(1380 - 700) / 60 ≈ 11 px` and the 14-px `kick()` cap
/// saturates beyond that.
pub const HARD_FALL_SHAKE_GAIN: f32 = 1.0 / 60.0;

/// Compute the shake amplitude for a semantic landing transition. Pure
/// function so the trigger logic in `player_simulation_phase` is
/// unit-testable independent of the surrounding bevy plumbing.
///
/// Returns 0.0 when there was no landing, or when the impact speed is below
/// the dead-zone. Otherwise returns the post-gain
/// amplitude that should be fed to `shake.kick(...)`.
pub fn hard_fall_shake_amplitude(impact_speed: Option<f32>) -> f32 {
    let Some(impact_speed) = impact_speed else {
        return 0.0;
    };
    let excess = (impact_speed - HARD_FALL_SHAKE_FLOOR_VY).max(0.0);
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

/// Camera easing and blink-in presentation state, per body. Authoritative ECS
/// component; written by `cleanup_timers_system`, `load_room`, and
/// `handle_player_events` (blink path). Read by the camera follow system and the
/// sprite animator.
///
/// It lives here with the rest of the camera ease vocabulary rather than in the
/// actor crate for the same reason [`CameraEaseState`] does: nothing about it is
/// actor-domain — four `f32`s and two [`ae::Vec2`]s — and its readers
/// (`ambition_sim_view`'s pose/camera snapshots, the runtime's reset, room
/// transition and rollback paths) sit ABOVE the actor crate and only ever needed
/// the numbers.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PlayerBlinkCameraState {
    /// Counts down from `blink_in_duration` to 0 after a blink; the camera
    /// and animator use this to play the arrival ease-in.
    pub blink_in_timer: f32,
    /// Set to `BLINK_IN_ANIM_TIME` when a blink fires; used to normalise
    /// `blink_in_timer` into a 0..1 progress value.
    pub blink_in_duration: f32,
    /// World-space camera position at the moment the blink fired; the camera
    /// eases from here toward the new player position.
    pub blink_camera_from: ae::Vec2,
    /// Blink destination in world space (set alongside `blink_camera_from`
    /// for future use; not yet consumed by the camera easing path).
    pub blink_camera_to: ae::Vec2,
    /// Positive while the camera should snap (not ease) to the player position.
    /// Set on door transitions; zero on edge exits to allow scroll effects.
    pub camera_snap_timer: f32,
}

impl Default for PlayerBlinkCameraState {
    fn default() -> Self {
        Self {
            blink_in_timer: 0.0,
            blink_in_duration: 0.0,
            blink_camera_from: ae::Vec2::ZERO,
            blink_camera_to: ae::Vec2::ZERO,
            camera_snap_timer: 0.0,
        }
    }
}

impl PlayerBlinkCameraState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests;
