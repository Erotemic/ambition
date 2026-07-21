//! **[the frame clock]** — presented poses sampled from tick read-models.
//!
//! The read-models in this crate are republished once per SIM TICK, so every
//! position they carry is a step function on a 60 Hz clock. Presentation draws
//! once per RENDERED FRAME. On a display that is not exactly 60 Hz those two
//! clocks disagree, and the disagreement is visible.
//!
//! # The failure this exists to prevent
//!
//! A subject's screen position is `subject_world - camera_world`. The camera
//! eases every rendered frame (`camera_snapshot::resolve_camera_observation`,
//! in `Update` — deliberately, see that module). If the subject's world
//! position only advances on tick boundaries, then between ticks the camera
//! keeps converging on a target that has stopped moving, and at the tick
//! boundary the subject jumps a whole tick of travel at once. The result is a
//! sawtooth in the subject's screen position at the tick rate, with amplitude
//! equal to one tick of travel — a horizontal shudder that grows with speed and
//! disappears in slow motion. Static room geometry is immune, because its world
//! position is constant and its screen position is therefore just `-camera`,
//! which is smooth by construction. That asymmetry is the tell: the world looks
//! rock-steady while the character alone appears to vibrate.
//!
//! # The rule
//!
//! **Everything anchored to a body reads the SAME presented pose**: the sprite,
//! the camera's focus, and every attached visual. A consumer that reads
//! `BodyPoseView::pos` directly while its neighbours read the presented pose
//! will visibly drift from them at speed. That coherence — not smoothness on
//! its own — is what removes the shake.
//!
//! # Extrapolation, not interpolation
//!
//! The presented pose leads the last published tick rather than lagging it:
//!
//! ```text
//! presented = current + phase * (current - previous)
//! ```
//!
//! Interpolating between the two most recent ticks would also be smooth, but it
//! draws the body up to a full tick (~16.7 ms) behind the simulation — real
//! added input latency in a precision platformer, and a visible gap against any
//! overlay drawn from authoritative sim state.
//!
//! Extrapolating from the ACTUAL per-tick displacement (`current - previous`)
//! rather than from raw velocity matters:
//!
//! * it is self-limiting on impact — the simulation's own collision resolution
//!   already clamped that displacement, so a body that gets stopped extrapolates
//!   by only the distance it truly moved;
//! * it inherits bullet time, hitstop, and pause for free, because a scaled sim
//!   dt shrinks the displacement while leaving `vel` in world units per second;
//! * it reflects whatever the movement model actually did, including modes that
//!   move a body without a conventional velocity.
//!
//! The residual cost is that a body which was free last tick and is blocked this
//! tick can be drawn up to one tick of travel into the geometry it is about to
//! hit, for one frame. If that ever reads worse than the shake it fixes, the
//! fallback is a two-tick interpolation buffer — the same machinery, sampling
//! backwards instead of forwards.

use ambition_engine_core::Vec2;
use ambition_time::SimTick;
use bevy::prelude::{
    Commands, Component, Entity, Fixed, IntoScheduleConfigs, Query, Res, ResMut, Resource,
    SystemSet, Time, Update,
};

use crate::pose_view::BodyPoseView;
use crate::view_index::FeatureViewIndex;

/// Plausibility bound for the discontinuity guard — NOT a clock. Used only to
/// ask "could a body have travelled this far under its own power in one tick?",
/// so a fixed nominal tick is fine even when the real one differs.
const NOMINAL_TICK_DT: f32 = 1.0 / 60.0;

/// Slack on that bound, absorbing collision response and within-tick velocity
/// change before a move is judged a teleport.
const TRAVEL_SLACK: f32 = 4.0;
const TRAVEL_FLOOR_PX: f32 = 32.0;

/// Where the current rendered frame sits inside the current sim tick, as a
/// fraction of one tick.
///
/// `0.0` means a tick just completed. Stays `0.0` on hosts where the question
/// is meaningless or unanswerable, which degrades exactly to today's behaviour:
///
/// * **frame-stepped host** — the sim advances once per rendered frame, so the
///   published pose is already current and there is nothing to extrapolate.
/// * **rollback (GGRS) host** — `bevy_ggrs` owns the accumulator that decides
///   when to advance and does not expose it. Reconstructing a parallel
///   accumulator would disagree with the real one during stalls, multi-advance
///   frames, and rollback, so this reports no phase until the driver can be
///   asked truthfully.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct PresentationPhase {
    phase: f32,
}

impl PresentationPhase {
    /// Fraction of a tick elapsed since the newest published pose, in `[0, 1)`.
    #[inline]
    pub fn get(self) -> f32 {
        self.phase
    }
}

/// Read the exact intra-tick remainder Bevy already computed.
///
/// `Time<Fixed>` accumulates real time and spends it in whole timesteps inside
/// `RunFixedMainLoop`, which runs before `Update`; what is left over IS the
/// phase. Deliberately not reimplemented as a hand-rolled accumulator — an
/// approximation of this quantity produces uneven per-frame steps and reduces
/// the shake instead of removing it.
pub fn sample_fixed_overstep_phase(fixed: Res<Time<Fixed>>, mut phase: ResMut<PresentationPhase>) {
    phase.phase = fixed.overstep_fraction().clamp(0.0, 1.0);
}

/// The presented position of one body, plus the two ticks it was derived from.
///
/// Attached automatically to every entity carrying a [`BodyPoseView`].
/// Presentation-only: the simulation never reads it, so a rollback resim
/// neither restores nor consults it.
#[derive(Component, Clone, Copy, Debug)]
pub struct PresentedPose {
    previous: Vec2,
    current: Vec2,
    presented: Vec2,
    tick: u64,
}

impl PresentedPose {
    fn new(pos: Vec2, tick: u64) -> Self {
        Self {
            previous: pos,
            current: pos,
            presented: pos,
            tick,
        }
    }

    /// **The position to draw this body and everything anchored to it at.**
    #[inline]
    pub fn presented(self) -> Vec2 {
        self.presented
    }

    /// The newest authoritative tick position — what an overlay that must stay
    /// truthful about simulation state (a collision-box gizmo) draws from.
    #[inline]
    pub fn authoritative(self) -> Vec2 {
        self.current
    }

    /// Displacement the simulation actually produced across the last tick.
    #[inline]
    pub fn tick_delta(self) -> Vec2 {
        self.current - self.previous
    }

    /// Accept a newly published tick pose. `continuous` false means the body did
    /// not TRAVEL here (portal, room change, respawn, possession swap): the
    /// history collapses so the jump is drawn as a jump and never extrapolated
    /// along.
    fn push(&mut self, pos: Vec2, tick: u64, continuous: bool) {
        self.previous = if continuous { self.current } else { pos };
        self.current = pos;
        self.tick = tick;
    }

    fn resample(&mut self, phase: f32) {
        self.presented = self.current + self.tick_delta() * phase;
    }
}

/// **The one call every body-anchored visual makes** instead of reading
/// `BodyPoseView::pos`.
///
/// Falls back to the tick pose when no history exists yet or the host reports
/// no phase, so a consumer is always correct — just not smoothed.
#[inline]
pub fn draw_pos(pose: &BodyPoseView, presented: Option<&PresentedPose>) -> Vec2 {
    presented.map_or(pose.pos, |presented| presented.presented())
}

/// Could a body carrying `vel` have travelled `from → to` in one tick under its
/// own power? A teleport answers no, and must not be extrapolated across.
fn travelled_under_own_power(from: Vec2, to: Vec2, vel: Vec2) -> bool {
    let expected = vel.length() * NOMINAL_TICK_DT;
    from.distance(to) <= expected * TRAVEL_SLACK + TRAVEL_FLOOR_PX
}

/// Roll every body's presented pose forward: extend the history on a new tick,
/// then resample for THIS frame's phase.
pub fn advance_presented_body_poses(
    mut commands: Commands,
    tick: Res<SimTick>,
    phase: Res<PresentationPhase>,
    mut bodies: Query<(Entity, &BodyPoseView, Option<&mut PresentedPose>)>,
) {
    let phase = phase.get();
    for (entity, pose, presented) in &mut bodies {
        let Some(mut presented) = presented else {
            // First sight: no history, so it presents exactly where it spawned
            // rather than extrapolating away from a default.
            commands
                .entity(entity)
                .insert(PresentedPose::new(pose.pos, tick.0));
            continue;
        };
        // A new pose arrives only on a new tick; `BodyPoseView::pos` is read
        // here alone, on the frame the sim rebuilt it.
        if presented.tick != tick.0 {
            let continuous = travelled_under_own_power(presented.current, pose.pos, pose.vel);
            presented.push(pose.pos, tick.0, continuous);
        }
        // Resample EVERY frame — that is the entire point.
        presented.resample(phase);
    }
}

/// Id-keyed presented poses for the feature/actor visuals (enemies, NPCs,
/// bosses, moving props).
///
/// A body's presented pose is a component because its sprite lives on the body
/// entity; feature visuals join to the sim by string id instead, so their
/// history lives in one index beside the read-model it mirrors.
#[derive(Resource, Default, Debug)]
pub struct PresentedFeaturePoses {
    poses: std::collections::HashMap<String, PresentedPose>,
}

impl PresentedFeaturePoses {
    /// The position to draw feature `id` at, falling back to `authoritative`
    /// for a row with no history yet.
    #[inline]
    pub fn presented(&self, id: &str, authoritative: Vec2) -> Vec2 {
        self.poses
            .get(id)
            .map_or(authoritative, |pose| pose.presented())
    }
}

/// The feature-side counterpart of [`advance_presented_body_poses`].
///
/// `FeatureView` carries no velocity, so continuity is judged against the row's
/// own size instead: nothing walks several body-lengths in a tick, but a portal
/// or a room change moves it arbitrarily far.
pub fn advance_presented_feature_poses(
    tick: Res<SimTick>,
    phase: Res<PresentationPhase>,
    views: Res<FeatureViewIndex>,
    mut presented: ResMut<PresentedFeaturePoses>,
) {
    let phase = phase.get();
    for (id, view) in views.iter() {
        match presented.poses.get_mut(id) {
            Some(pose) => {
                if pose.tick != tick.0 {
                    let leap = view.size.max_element().max(TRAVEL_FLOOR_PX) * 3.0;
                    let continuous = pose.current.distance(view.pos) <= leap;
                    pose.push(view.pos, tick.0, continuous);
                }
                pose.resample(phase);
            }
            None => {
                presented
                    .poses
                    .insert(id.to_string(), PresentedPose::new(view.pos, tick.0));
            }
        }
    }
    // Drop history for rows the read-model no longer publishes, so a long
    // session does not retain one entry per feature ever spawned.
    presented.poses.retain(|id, _| views.get(id).is_some());
}

/// Ordering handle: the presented poses are resampled before ANY consumer —
/// the camera resolve and the whole presentation visual sync alike.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PresentedPoseSet;

/// Installs the frame-clock sampling layer.
pub struct PresentedPosePlugin;

impl bevy::prelude::Plugin for PresentedPosePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use ambition_platformer_primitives::schedule::SimScheduleExt as _;

        app.init_resource::<PresentationPhase>();
        app.init_resource::<PresentedFeaturePoses>();

        // Only a host that banks unspent real time between sim steps HAS an
        // intra-tick phase. See `PresentationPhase` for the other two.
        if app.sim_is(bevy::prelude::FixedUpdate) {
            app.add_systems(
                Update,
                sample_fixed_overstep_phase
                    .in_set(PresentedPoseSet)
                    .before(advance_presented_body_poses),
            );
        }
        app.add_systems(
            Update,
            (
                advance_presented_body_poses,
                advance_presented_feature_poses,
            )
                .in_set(PresentedPoseSet),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pose_at(pos: Vec2) -> PresentedPose {
        PresentedPose::new(pos, 0)
    }

    #[test]
    fn presented_leads_the_tick_pose_by_the_phase() {
        let mut pose = pose_at(Vec2::ZERO);
        pose.push(Vec2::new(6.0, 0.0), 1, true);

        pose.resample(0.0);
        assert_eq!(
            pose.presented(),
            Vec2::new(6.0, 0.0),
            "phase 0 draws the tick pose exactly — no latency"
        );
        pose.resample(0.5);
        assert_eq!(pose.presented(), Vec2::new(9.0, 0.0));
    }

    #[test]
    fn a_resting_body_never_drifts() {
        let mut pose = pose_at(Vec2::new(10.0, 20.0));
        pose.push(Vec2::new(10.0, 20.0), 1, true);
        for phase in [0.0, 0.5, 0.99] {
            pose.resample(phase);
            assert_eq!(pose.presented(), Vec2::new(10.0, 20.0));
        }
    }

    #[test]
    fn a_blocked_body_extrapolates_only_as_far_as_it_actually_moved() {
        // Running at 400 px/s would cover ~6.7 px, but collision stopped it
        // after 1.0. Extrapolation rides the REAL displacement, so it cannot
        // predict deep into the wall.
        let mut pose = pose_at(Vec2::ZERO);
        pose.push(Vec2::new(1.0, 0.0), 1, true);
        pose.resample(1.0);
        assert_eq!(pose.presented(), Vec2::new(2.0, 0.0));
    }

    #[test]
    fn a_teleport_is_drawn_as_a_jump_not_flung_further() {
        let vel = Vec2::new(400.0, 0.0);
        assert!(travelled_under_own_power(
            Vec2::ZERO,
            Vec2::new(6.7, 0.0),
            vel
        ));
        assert!(!travelled_under_own_power(
            Vec2::ZERO,
            Vec2::new(900.0, 0.0),
            vel
        ));

        let mut pose = pose_at(Vec2::ZERO);
        pose.push(Vec2::new(900.0, 0.0), 1, false);
        pose.resample(1.0);
        assert_eq!(
            pose.presented(),
            Vec2::new(900.0, 0.0),
            "a discontinuous move parks at the destination for the whole tick"
        );
    }

    #[test]
    fn slow_motion_shrinks_the_extrapolation_with_the_displacement() {
        // Same speed, a tenth of the sim dt: the tick delta shrinks, so the
        // presented lead shrinks with it. Nothing reads `time_scale` to do it.
        let mut full = pose_at(Vec2::ZERO);
        full.push(Vec2::new(6.7, 0.0), 1, true);
        full.resample(0.5);

        let mut slow = pose_at(Vec2::ZERO);
        slow.push(Vec2::new(0.67, 0.0), 1, true);
        slow.resample(0.5);

        assert!(slow.presented().x < full.presented().x * 0.2);
    }

    #[test]
    fn authoritative_stays_the_tick_pose() {
        let mut pose = pose_at(Vec2::ZERO);
        pose.push(Vec2::new(6.0, 0.0), 1, true);
        pose.resample(1.0);
        assert_eq!(
            pose.authoritative(),
            Vec2::new(6.0, 0.0),
            "debug overlays that must not lie read this, not the presented pose"
        );
    }
}
