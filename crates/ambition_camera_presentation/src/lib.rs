//! The camera's live presentation state and the systems that drive it: the
//! eased scale, target and roll of each view (`CameraEaseState`), the shake
//! (`CameraShakeState`) and the finishing zoom (`FinishZoomState`).
//!
//! The requests, tunings and amplitude laws the simulation uses are the
//! foundation's (`ambition_platformer2d_shared_tangle::camera_ease`). Only the
//! presentation side reads or writes the state here; the simulation asks
//! through a request, which a rollback host releases once the frame that made
//! it is confirmed.

use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::camera_ease::{
    CameraShakeRequest, CameraShakeTuning, FinishZoomRequest, FinishZoomTuning,
};
use ambition_platformer2d_shared_tangle::lifecycle;
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
    pub active_composition: Option<usize>,
    pub transition_from: ae::Vec2,
    pub transition_elapsed: f32,
    pub transition_duration: f32,
    pub transition_active: bool,
    pub transition_id: Option<u32>,
    /// Last ground focus for the jump-height hold policy.
    pub ground_focus: Option<ae::Vec2>,
    pub ground_subject: Option<bevy::prelude::Entity>,
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
            active_composition: None,
            transition_from: ae::Vec2::ZERO,
            transition_elapsed: 0.0,
            transition_duration: 0.0,
            transition_active: false,
            transition_id: None,
            ground_focus: None,
            ground_subject: None,
            scroll_watermark_x: None,
            live_observer_roll: None,
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
#[derive(Resource, Clone, Debug, Default)]
pub struct CameraShakeState {
    /// Current shake amplitude in world pixels, for every view. Zero means
    /// "no shake." A kick that names no room lands here.
    pub amplitude_px: f32,
    /// The shake of each live room, for the views that frame it (ISL-1b): a
    /// hit in one room does not shake the view of another.
    pub rooms: std::collections::BTreeMap<lifecycle::LiveRoomInstance, f32>,
    /// Seed bumped each frame so the random offset is deterministic
    /// within a frame (camera follow can call into multiple samples)
    /// but uncorrelated across frames.
    pub seed: u32,
}

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

    /// [`Self::kick`] the views of `room`, or every view with `None`.
    pub fn kick_in(&mut self, room: Option<lifecycle::LiveRoomInstance>, amplitude_px: f32, tuning: CameraShakeTuning) {
        let Some(room) = room else {
            self.kick(amplitude_px, tuning);
            return;
        };
        let target = amplitude_px.max(0.0).min(tuning.max_amplitude_px.max(0.0));
        let held = self.rooms.entry(room).or_insert(0.0);
        if target > *held {
            *held = target;
        }
    }

    /// The shake a view of `room` shows: the louder of every view's and the
    /// room's own.
    pub fn amplitude_in(&self, room: Option<lifecycle::LiveRoomInstance>) -> f32 {
        let own = room.and_then(|room| self.rooms.get(&room).copied()).unwrap_or(0.0);
        self.amplitude_px.max(own)
    }

    /// The loudest shake any view shows, every room's included.
    pub fn loudest(&self) -> f32 {
        self.rooms.values().copied().fold(self.amplitude_px, f32::max)
    }

    /// Settle every shake by `px`, and forget a room's once it is still.
    pub fn decay(&mut self, px: f32) {
        self.amplitude_px = (self.amplitude_px - px).max(0.0);
        for amplitude in self.rooms.values_mut() {
            *amplitude = (*amplitude - px).max(0.0);
        }
        self.rooms.retain(|_, amplitude| *amplitude > 0.0);
    }
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
        shake.kick_in(request.room, request.amplitude_px, *tuning);
    }
}

/// Decay system: subtracts `CAMERA_SHAKE_DECAY_PX_PER_S * dt` from
/// `amplitude_px` and clamps at zero. Runs every frame on `Update`
/// before the camera stage so the follow logic sees the post-decay
/// amplitude.
pub fn tick_camera_shake(
    time: bevy::prelude::Res<bevy::prelude::Time>,
    tuning: bevy::prelude::Res<CameraShakeTuning>,
    mut shake: bevy::prelude::ResMut<CameraShakeState>,
) {
    let dt = time.delta_secs();
    shake.decay(tuning.decay_px_per_s * dt);
    shake.seed = shake.seed.wrapping_add(1);
}

impl CameraShakeState {
    /// Cheap deterministic 2D offset within the current amplitude budget.
    /// xorshift on `seed` gives a per-frame value in `[-amp, +amp]`;
    /// independent xorshifts for x / y avoid the diagonal-only shake a
    /// naive `(s, s)` pair would produce.
    pub fn offset(&self) -> ae::Vec2 {
        self.offset_in(None)
    }

    /// [`Self::offset`] for a view of `room` ([`Self::amplitude_in`]).
    pub fn offset_in(&self, room: Option<lifecycle::LiveRoomInstance>) -> ae::Vec2 {
        let amplitude = self.amplitude_in(room);
        if amplitude <= 0.05 {
            return ae::Vec2::ZERO;
        }
        let mut sx = self.seed.wrapping_mul(0x45d9f3b).wrapping_add(0x9e3779b9);
        sx ^= sx >> 17;
        sx = sx.wrapping_mul(0xed5ad4bb);
        let mut sy = self.seed.wrapping_mul(0x119de1f3).wrapping_add(0x85ebca6b);
        sy ^= sy >> 15;
        sy = sy.wrapping_mul(0xc2b2ae35);
        let to_unit = |s: u32| (s as f32 / u32::MAX as f32) * 2.0 - 1.0;
        ae::Vec2::new(to_unit(sx) * amplitude, to_unit(sy) * amplitude)
    }
}

/// Live finishing-zoom state: how far in the camera currently is.
///
/// Strongest-kick-wins and non-stacking, exactly like [`CameraShakeState`] and
/// for the same reason — several requests released together by the quarantine
/// settle on the strongest rather than multiplying into an unreadable close-up.
#[derive(Resource, Clone, Debug, Default)]
pub struct FinishZoomState {
    /// 0.0 = no zoom, 1.0 = fully in, for every view. Not a scale: see
    /// [`Self::scale_factor`].
    pub closeness: f32,
    /// Seconds of hold left before the release begins.
    pub hold_secs_left: f32,
    /// Each live room's zoom, for the views that frame it (ISL-1b): its
    /// closeness and hold left. A boss falling in one room does not pull in
    /// the view of another.
    pub rooms: std::collections::BTreeMap<lifecycle::LiveRoomInstance, (f32, f32)>,
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

    /// [`Self::kick`] the views of `room`, or every view with `None`.
    pub fn kick_in(&mut self, room: Option<lifecycle::LiveRoomInstance>, closeness: f32, tuning: FinishZoomTuning) {
        let Some(room) = room else {
            self.kick(closeness, tuning);
            return;
        };
        let target = closeness.clamp(0.0, 1.0);
        let held = self.rooms.entry(room).or_insert((0.0, 0.0));
        if target > held.0 {
            *held = (target, tuning.hold_secs.max(0.0));
        }
    }

    /// [`Self::scale_factor`] for a view of `room`: the closer of every
    /// view's zoom and the room's own.
    pub fn scale_factor_in(&self, room: Option<lifecycle::LiveRoomInstance>, tuning: FinishZoomTuning) -> f32 {
        let own = room.and_then(|room| self.rooms.get(&room)).map_or(0.0, |(closeness, _)| *closeness);
        let close = self.closeness.max(own).clamp(0.0, 1.0) * tuning.max_close_fraction.clamp(0.0, 0.9);
        1.0 - close
    }

    /// Spend `dt` of every zoom's hold, then release it; a room at rest is
    /// forgotten.
    pub fn settle(&mut self, dt: f32, tuning: FinishZoomTuning) {
        let settle = |(closeness, hold): (f32, f32)| {
            if hold > 0.0 {
                (closeness, (hold - dt).max(0.0))
            } else {
                ((closeness - tuning.release_per_s * dt).max(0.0), 0.0)
            }
        };
        (self.closeness, self.hold_secs_left) = settle((self.closeness, self.hold_secs_left));
        for held in self.rooms.values_mut() {
            *held = settle(*held);
        }
        self.rooms.retain(|_, (closeness, hold)| *closeness > 0.0 || *hold > 0.0);
    }

    /// Whether any zoom, shared or a room's, is held or releasing.
    pub fn is_active(&self) -> bool {
        self.hold_secs_left > 0.0 || self.closeness > 0.0 || !self.rooms.is_empty()
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
        zoom.kick_in(request.room, request.closeness, *tuning);
    }
}

/// Put the camera at rest when a gameplay session activates.
///
/// The shake and the finishing zoom are what one session asked of the screen.
/// They are applied in `Update` and they decay over time, so a session that
/// is replaced while its camera moves leaves that motion to the next one.
/// MEASURED 2026-10-04 on the shell host: a session that shook its camera by
/// 6 px and held a full finishing zoom was replaced, and the next session
/// started with a shake of 6 px and a full zoom.
///
/// The appliers can run before the activation in the update that replaces a
/// session, so emptying the two request channels is not sufficient.
pub fn rest_the_camera_on_activation(
    mut activated: bevy::ecs::message::MessageReader<lifecycle::SessionScopeActivated>,
    mut shake: bevy::prelude::ResMut<CameraShakeState>,
    mut zoom: bevy::prelude::ResMut<FinishZoomState>,
) {
    if activated.read().count() == 0 {
        return;
    }
    // The seed is a counter of frames, not a motion: it keeps its value.
    shake.amplitude_px = 0.0;
    shake.rooms.clear();
    *zoom = FinishZoomState::default();
}

/// Run condition of [`tick_finish_zoom`]: a zoom is held or releasing. At
/// rest the tick has nothing to do, and it wrote the state each frame.
pub fn finish_zoom_is_active(zoom: bevy::prelude::Res<FinishZoomState>) -> bool {
    zoom.is_active()
}

/// Hold, then release. Runs beside [`tick_camera_shake`] while
/// [`finish_zoom_is_active`].
pub fn tick_finish_zoom(
    time: bevy::prelude::Res<bevy::prelude::Time>,
    tuning: bevy::prelude::Res<FinishZoomTuning>,
    mut zoom: bevy::prelude::ResMut<FinishZoomState>,
) {
    zoom.settle(time.delta_secs(), *tuning);
}

/// The two stages of this crate's frame, for a consumer to order against
/// without naming its systems: released requests are applied, then the live
/// state decays and releases.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CameraPresentationSet {
    Apply,
    Settle,
}

/// The state and the request half: the live shake and zoom, their appliers, and
/// the rest at a session's activation.
///
/// Every composition that owns the state installs it, headless ones included:
/// a headless run still asks for shakes and still has the amplitude read off it
/// (`app_it::hit_shakes_the_camera` watches a duel through the sim harness).
pub struct CameraRequestsPlugin;

impl bevy::prelude::Plugin for CameraRequestsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use ambition_platformer2d_core::a_message_waits;
        use bevy::prelude::{IntoScheduleConfigs as _, Update};
        app.init_resource::<CameraShakeState>()
            .init_resource::<FinishZoomState>()
            .add_message::<CameraShakeRequest>()
            .add_message::<FinishZoomRequest>()
            .add_message::<lifecycle::SessionScopeActivated>()
            .configure_sets(Update, CameraPresentationSet::Apply.before(CameraPresentationSet::Settle))
            // Applied BEFORE the settle, so a request that arrives this frame
            // gets its full hold rather than being decayed by a tick ahead of it.
            .add_systems(
                Update,
                (
                    apply_camera_shake_requests.run_if(a_message_waits::<CameraShakeRequest>),
                    apply_finish_zoom_requests.run_if(a_message_waits::<FinishZoomRequest>),
                )
                    .in_set(CameraPresentationSet::Apply),
            )
            // A composition with no session lifecycle has no activation; the
            // reader then reads an empty channel.
            .add_systems(
                Update,
                rest_the_camera_on_activation
                    .run_if(a_message_waits::<lifecycle::SessionScopeActivated>)
                    .in_set(lifecycle::SessionScopeSet::Activate),
            );
    }
}

/// The settle half: the shake decays and the finishing zoom holds and releases
/// on the frame clock. Only a host with a camera to move installs it; it orders
/// its follow `.after(CameraPresentationSet::Settle)`, since the follow reads
/// both live values and a tick after it presents them a frame stale.
pub struct CameraSettlePlugin;

impl bevy::prelude::Plugin for CameraSettlePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::{IntoScheduleConfigs as _, Update};
        app.configure_sets(Update, CameraPresentationSet::Apply.before(CameraPresentationSet::Settle))
            .add_systems(
                Update,
                (tick_camera_shake, tick_finish_zoom.run_if(finish_zoom_is_active))
                    .in_set(CameraPresentationSet::Settle)
                    .run_if(lifecycle::session_world_exists),
            );
    }
}

#[cfg(test)]
mod tests;
