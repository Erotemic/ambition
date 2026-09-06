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
/// a COMPONENT on a local view, not a resource. A second local view eases
/// its own zoom toward its own target, so "the camera's ease state" is a
/// question that has to name a view to mean anything.
#[derive(Component, Clone, Copy, Debug)]
pub struct CameraEaseState {
    pub live_scale: f32,
    /// Smoothed world-space camera target. Presentation-only: avoids hard
    /// jumps when look-ahead flips with facing or when framing presets change.
    pub live_target_world: ae::Vec2,
    pub target_initialized: bool,
    /// `None` outside such a zone — which is what makes re-entering one a fresh scroll rather
    /// than a camera pinned to where it stopped an hour ago.
    pub scroll_watermark_x: Option<f32>,
    /// The observer roll actually being presented, eased toward the roll the
    /// view's reference frame asks for.
    ///
    /// without it the world SNAPS. `presented_roll_radians` is a pure function of the
    /// CURRENT `subject_down`, so in `SubjectFrame` mode any discontinuity in that axis —
    /// possessing a body standing on a different surface, a gravity flip, any future
    /// view-subject change — rotated the whole world by up to a half turn in one frame.
    ///
    /// `None` until the first resolve, which then ADOPTS the target instead of
    /// easing to it: a view must open already oriented, not spin up from zero.
    ///
    /// presentation-only, like every other field here, and never rollback
    /// state. `WorldFixed` views are unaffected because their target roll is
    /// identically zero — which is every camera Ambition currently ships.
    pub live_observer_roll: Option<f32>,
}

/// How fast the presented observer roll follows its reference frame.
///
/// the genre answered this, so it is a dial and not a decision. A gravity
/// flip rotates the view over a short interval in VVVVVV and Mario Galaxy rather
/// than cutting; π radians in 0.30s is inside that band and reads as "the world
/// turns under you" instead of a jump cut.
///
/// a RATE, not a duration, so a small correction is quick and a half turn
/// takes the full interval — the alternative normalises every change to the same
/// time and makes tiny ones feel mushy.
pub const OBSERVER_ROLL_EASE_RAD_PER_S: f32 = std::f32::consts::PI / 0.30;

/// Ease `current` toward `target` along the SHORTEST angular path.
///
/// the wrap is the whole subtlety. Rolls live on a circle: +π and -π are
/// the same orientation, and a naive `target - current` would send the view the
/// long way round — a full rotation to reach an angle it was already at.
pub fn ease_roll_radians(current: f32, target: f32, dt: f32) -> f32 {
    use std::f32::consts::PI;
    let mut delta = (target - current) % (2.0 * PI);
    if delta > PI {
        delta -= 2.0 * PI;
    } else if delta < -PI {
        delta += 2.0 * PI;
    }
    let step = OBSERVER_ROLL_EASE_RAD_PER_S * dt.max(0.0);
    if delta.abs() <= step {
        target
    } else {
        current + delta.signum() * step
    }
}

impl Default for CameraEaseState {
    fn default() -> Self {
        Self {
            live_scale: 1.0,
            live_target_world: ae::Vec2::ZERO,
            target_initialized: false,
            scroll_watermark_x: None,
            live_observer_roll: None,
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

/// How hard this game is allowed to shake, and how fast it settles.
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

/// A simulation-produced request to kick the camera. (P0.1)
///
/// the simulation must not touch [`CameraShakeState`] directly. A rollback host runs each
/// frame more than once, and the FIRST of those passes is already unconfirmed: GGRS predicts
/// the remote input, so a hit that a later correction erases has, by then, already kicked the
/// live camera.
///
/// So the shake became what the sound already was: an intent, written into a quarantined
/// channel, journalled per frame, and released once the host confirms the frame that produced
/// it. A re-simulation that no longer produces the hit replaces its frame's batch with an empty
/// one and the kick never happens; a confirmed hit is released exactly once.
///
/// strongest-wins survives the trip. [`CameraShakeState::kick`] is a `max`,
/// so several requests released together settle on the loudest exactly as several
/// direct kicks did.
#[derive(bevy::ecs::message::Message, Clone, Copy, Debug)]
pub struct CameraShakeRequest {
    /// Desired amplitude in world pixels, before [`CameraShakeTuning`]'s cap.
    pub amplitude_px: f32,
}

/// Apply released [`CameraShakeRequest`]s to the live shake state.
///
/// The presentation half of the seam above: the only writer of
/// [`CameraShakeState`] on behalf of the simulation. Runs in `Update` beside
/// [`tick_camera_shake`], downstream of the quarantine's `PreUpdate` release, so
/// what it reads is a confirmed frame's intent under a rollback host and this
/// frame's under every other one.
pub fn apply_camera_shake_requests(
    mut requests: bevy::ecs::message::MessageReader<CameraShakeRequest>,
    tuning: bevy::prelude::Res<CameraShakeTuning>,
    mut shake: bevy::prelude::ResMut<CameraShakeState>,
) {
    for request in requests.read() {
        shake.kick(request.amplitude_px, *tuning);
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

/// Pixels of shake per second of hitlag ABOVE the WEAKEST connect.
///
/// The engine's reference hitlag is 0.070 s, `hitlag_duration` floors at
/// `MIN_HITLAG_SCALE` (0.035 s) and rides a 4× ceiling (0.280 s), so the
/// hardest possible connect buys `(0.280 - 0.035) × 48 ≈ 11.8 px` — a heavy
/// jolt that still sits under the 14-px `kick()` cap a hard fall can reach.
/// Chosen so the two things that shake this screen sit on one scale rather than
/// each having its own idea of "hard".
pub const HIT_SHAKE_GAIN_PX_PER_S: f32 = 48.0;

/// the reference is a PARAMETER, not a constant here. `hitlag_time` is the
/// route's `Platformer2dFeelTuningMonolith` value; restating 0.070 in this crate
/// would be a second literal agreeing with the first by coincidence, and a route
/// that retunes its hitlag would silently retune its camera in the wrong
/// direction.
///
/// Every hit in Ambition's own combat lands UNDER the old dead zone, so the camera could never move
/// in the shipped game, and only a Smash-style growth knockback (the smash demo authors real
/// `knockback_growth`; every prefab swing authors `0.0`) could ever have cleared it. Anchoring on
/// [`ae::hit_response::MIN_HITLAG_SCALE`] instead uses the mechanic's whole dynamic range: the
/// softest possible connect still shakes NOTHING — which is the property the reference-anchored
/// version was actually reaching for — while the duel's ordinary trade buys ~1.2 px and the hardest
/// smash ~11.8 px, a tenfold spread rather than a cliff.
pub fn hit_shake_amplitude(hitstop_seconds: f32, reference_hitlag_seconds: f32) -> f32 {
    let weakest_connect = reference_hitlag_seconds.max(0.0) * ae::hit_response::MIN_HITLAG_SCALE;
    let excess = (hitstop_seconds - weakest_connect).max(0.0);
    excess * HIT_SHAKE_GAIN_PX_PER_S
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
    pub blink_in_duration: f32,
    /// World-space camera position at the moment the blink fired; the camera
    /// eases from here toward the new player position.
    pub blink_camera_from: ae::Vec2,
    /// Blink destination in world space (set alongside `blink_camera_from`
    /// for future use; not yet consumed by the camera easing path).
    pub blink_camera_to: ae::Vec2,
    /// Positive while the camera should snap (not ease) to the player position.
    ///
    /// ⭐⭐ ARMED BY WHOEVER MOVED THE BODY, and it has to be. A camera cannot
    /// tell a respawn from a portal transit — both are a position that jumped
    /// with no velocity to explain it — and the two want OPPOSITE answers: a
    /// respawn must snap, and a portal transit must NOT, because Ambition's
    /// default `PortalCameraTransitMode::Continuous` is a seam the camera walks
    /// through with you. So a snap is REQUESTED by the placer rather than
    /// inferred from the jump.
    ///
    /// Door transitions arm it; edge exits deliberately leave it zero so the
    /// scroll reads. A reset-to-spawn arms it too — see
    /// [`Self::snap_after_placement`].
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

    /// THIS BODY WAS PUT BACK AT A SPAWN — clear the blink, and keep the snap.
    ///
    /// ⭐⭐ ONE VERB BECAUSE THE TWO-STEP WAS THE BUG. Every placer wrote
    /// `reset()` and then had to remember [`Self::snap_after_placement`], and
    /// `reset()` CLEARS the snap — so forgetting the second line silently
    /// produced the defect Jon reported, and both call sites had forgotten it.
    /// A pair that must be called in one order, where one half undoes what the
    /// other needs, is a pair that should be one call.
    ///
    /// ⛔ `snap_after_placement` stays public for the placer that moves a body
    /// WITHOUT resetting it (a door transition already arms the timer its own
    /// way). This is the reset-shaped answer, not the only one.
    pub fn reset_to_spawn(&mut self, snap_seconds: f32) {
        self.reset();
        self.snap_after_placement(snap_seconds);
    }

    /// THIS BODY WAS PUT SOMEWHERE — do not chase it there.
    ///
    /// ⛔⛔ THE PLACER ASKS, and `reset()` above is exactly why this exists as a
    /// second call rather than a flag inside it. A reset-to-spawn clears the
    /// blink easing (right — the old blink is over) and in doing so it ZEROED
    /// the snap, so the one moment the camera most needed to jump was the one
    /// moment it was told to ease. Measured by Jon: a same-room teleport panned
    /// the camera 440px over about 40 ticks.
    ///
    /// `seconds` is a WINDOW rather than an instant because the camera resolves
    /// on the frame clock: a single-frame flag can be missed between two sim
    /// ticks, and a body placed on the tick a frame is not drawn would ease
    /// after all.
    pub fn snap_after_placement(&mut self, seconds: f32) {
        self.camera_snap_timer = self.camera_snap_timer.max(seconds);
    }
}

/// How close the finishing blow pulls the camera, and how long it stays there.
///
/// ⛔⛔ THIS IS PRESENTATION-ONLY AND MUST STAY THAT WAY, which is a stronger
/// claim than it sounds. The camera's SCALE is a zoom-OUT quantity by design:
/// `CameraZoneSpec::effective_zoom` is `.max(1.0)` and `camera_snapshot` floors
/// `target_scale` at `1.0` again, independently — *"the view is a FLOOR, so
/// authored zoom still wins whenever it is already wider"*, a readability
/// guarantee that the player never gets LESS than the design view.
///
/// ⇒ A finishing zoom wants to go the other way, and the tempting fix — let a
/// transient push `zoom_multiplier` under 1.0 — would spend that guarantee for
/// every consumer and put a second authority on a number `CameraZoneSpec` owns.
/// So this rides where the SHAKE rides instead: applied by `camera_follow`, on
/// the presented projection, downstream of the resolved snapshot. Nothing
/// downstream of the snapshot feeds back into it, so a rollback host cannot
/// desynchronise on it and gameplay framing is untouched.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct FinishZoomTuning {
    /// How far in, as a fraction of the presented view, at full strength.
    /// `0.30` shows 70% of the width — noticeably closer, still legible.
    pub max_close_fraction: f32,
    /// Seconds at full strength before releasing. A zoom that begins releasing
    /// on the frame it arrives is a flicker rather than a beat.
    pub hold_secs: f32,
    /// Per-second release rate once the hold expires.
    pub release_per_s: f32,
}

impl Default for FinishZoomTuning {
    fn default() -> Self {
        Self { max_close_fraction: 0.30, hold_secs: 0.60, release_per_s: 0.60 }
    }
}

/// Live finishing-zoom state: how far in the camera currently is.
///
/// Strongest-kick-wins and non-stacking, exactly like [`CameraShakeState`] and
/// for the same reason — several requests released together by the quarantine
/// settle on the strongest rather than multiplying into an unreadable close-up.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct FinishZoomState {
    /// 0.0 = no zoom, 1.0 = fully in. Not a scale: see [`Self::scale_factor`].
    pub closeness: f32,
    /// Seconds of hold left before the release begins.
    pub hold_secs_left: f32,
}

impl FinishZoomState {
    /// Pull the camera in to at least `closeness`, clamped to `0.0..=1.0`.
    pub fn kick(&mut self, closeness: f32, tuning: FinishZoomTuning) {
        let target = closeness.clamp(0.0, 1.0);
        if target > self.closeness {
            self.closeness = target;
            self.hold_secs_left = tuning.hold_secs.max(0.0);
        }
    }

    /// The multiplier to apply to the presented orthographic scale.
    ///
    /// ⭐ Returns exactly `1.0` when idle, so a host that never kicks it is
    /// byte-identical to one without the feature — which is what makes it safe
    /// to multiply unconditionally at the call site.
    pub fn scale_factor(&self, tuning: FinishZoomTuning) -> f32 {
        let close = self.closeness.clamp(0.0, 1.0) * tuning.max_close_fraction.clamp(0.0, 0.9);
        1.0 - close
    }
}

/// A simulation-produced request for the finishing zoom.
///
/// An INTENT, for the reason [`CameraShakeRequest`] spells out at length: a
/// rollback host runs a frame more than once and its first pass is unconfirmed,
/// so a match-decided that a later correction erases must not have already
/// moved the camera. Journalled per frame, released once the host confirms.
#[derive(bevy::ecs::message::Message, Clone, Copy, Debug)]
pub struct FinishZoomRequest {
    /// 0.0..=1.0. `1.0` is the full pull described by [`FinishZoomTuning`].
    pub closeness: f32,
}

/// Apply released [`FinishZoomRequest`]s to the live state.
///
/// The presentation half of the seam, and the only writer of
/// [`FinishZoomState`] on behalf of the simulation.
pub fn apply_finish_zoom_requests(
    mut requests: bevy::ecs::message::MessageReader<FinishZoomRequest>,
    tuning: bevy::prelude::Res<FinishZoomTuning>,
    mut zoom: bevy::prelude::ResMut<FinishZoomState>,
) {
    for request in requests.read() {
        zoom.kick(request.closeness, *tuning);
    }
}

/// Hold, then release. Runs every frame beside [`tick_camera_shake`].
pub fn tick_finish_zoom(
    time: bevy::prelude::Res<bevy::prelude::Time>,
    tuning: bevy::prelude::Res<FinishZoomTuning>,
    mut zoom: bevy::prelude::ResMut<FinishZoomState>,
) {
    let dt = time.delta_secs();
    if zoom.hold_secs_left > 0.0 {
        zoom.hold_secs_left = (zoom.hold_secs_left - dt).max(0.0);
        return;
    }
    zoom.closeness = (zoom.closeness - tuning.release_per_s * dt).max(0.0);
}

#[cfg(test)]
mod tests;
