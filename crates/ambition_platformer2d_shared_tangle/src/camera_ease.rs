//! The camera vocabulary lower layers share: the simulation's requests (a
//! shake, a finishing zoom), the amplitude laws that turn a landing or a hit
//! into a request, the tuning resources a route's presentation profile
//! publishes, and the per-body blink/snap state the simulation writes.
//!
//! The live state these drive (the eased view, the shake and the zoom) and the
//! systems that apply and settle them are presentation, in
//! `ambition_camera_presentation`: a change to how the camera moves rebuilds
//! that crate and the crates that draw, not every crate on this foundation.

use ambition_platformer2d_core as ae;
use bevy::prelude::{Component, Resource};

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

/// Follow and zoom settings for a route. Replaces the
/// hardcoded `CAMERA_ZOOM_{IN,OUT}_RATE` constants so the sandbox or
/// tests can override the rates without recompiling. The defaults
/// match the previous constants (`1.6` zoom-out, `0.9` zoom-in).
///
/// `target_scale > live_scale` (zooming out) uses `zoom_out_rate`;
/// the inverse direction uses `zoom_in_rate`. `snap_epsilon` is the
/// distance at which the ease finalizes onto the target value.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct CameraEaseTuning {
    /// Hold the last ground height during a rise. Follow falls below that height.
    pub hold_jump_height: bool,
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
            hold_jump_height: false,
            zoom_out_rate: DEFAULT_CAMERA_ZOOM_OUT_RATE,
            zoom_in_rate: DEFAULT_CAMERA_ZOOM_IN_RATE,
            snap_epsilon: DEFAULT_CAMERA_ZOOM_SNAP_EPSILON,
        }
    }
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

/// A simulation-produced request to kick the camera. (P0.1)
///
/// the simulation must not touch `ambition_camera_presentation::CameraShakeState` directly. A rollback host runs each
/// frame more than once, and the FIRST of those passes is already unconfirmed: GGRS predicts
/// the remote input, so a hit that a later correction erases has, by then, already kicked the
/// live camera.
///
/// So the shake became what the sound already was: an intent, written into a quarantined
/// channel, journalled per frame, and released once the host confirms the frame that produced
/// it. A re-simulation that no longer produces the hit replaces its frame's batch with an empty
/// one and the kick never happens; a confirmed hit is released exactly once.
///
/// strongest-wins survives the trip. `ambition_camera_presentation::CameraShakeState::kick` is a `max`,
/// so several requests released together settle on the loudest exactly as several
/// direct kicks did.
#[derive(bevy::ecs::message::Message, Clone, Copy, Debug)]
pub struct CameraShakeRequest {
    /// Desired amplitude in world pixels, before [`CameraShakeTuning`]'s cap.
    pub amplitude_px: f32,
    /// The live room whose views shake; `None` shakes every view.
    pub room: Option<crate::lifecycle::LiveRoomInstance>,
}

/// Per-second decay rate of `CameraShakeState::amplitude_px`. At 30 px/s,
/// a 6 px shake (mid-strength land) decays to 0 in ~0.2 s — long enough
/// to feel a thump, short enough not to interfere with the next move.
pub const CAMERA_SHAKE_DECAY_PX_PER_S: f32 = 30.0;

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

/// Camera easing and blink-in presentation state, per body. Authoritative ECS
/// component; written by `cleanup_timers_system`, `load_room`, and
/// the shared body feedback system. Read by the camera follow system and the
/// sprite animator.
///
/// It lives on this foundation, not with the live camera state in
/// `ambition_camera_presentation`, because the simulation writes it (a blink, a
/// placement) and rolls it back. Nothing about it is actor-domain — four `f32`s
/// and two [`ae::Vec2`]s — and its readers (`ambition_sim_view`'s pose/camera
/// snapshots, the runtime's reset, room transition and rollback paths) sit
/// ABOVE the actor crate and only ever needed the numbers.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PlayerBlinkCameraState {
    /// Counts down from `blink_in_duration` to 0 after a blink; the camera
    /// and animator use this to play the arrival ease-in.
    pub blink_in_timer: f32,
    pub blink_in_duration: f32,
    /// World-space camera position at the moment the blink fired; the camera
    /// eases from here toward the new player position.
    pub blink_camera_from: ae::Vec2,
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
    /// The live room whose views pull in; `None` pulls in every view.
    pub room: Option<crate::lifecycle::LiveRoomInstance>,
}

#[cfg(test)]
mod tests;
