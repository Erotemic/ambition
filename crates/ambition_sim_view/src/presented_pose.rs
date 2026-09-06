//! Presentation-time body poses sampled from tick read-models.
//!
//! Simulation poses update on fixed ticks while rendering and camera easing run per
//! frame. [`PresentedPose`] extrapolates from the two most recent authoritative poses:
//!
//! `presented = current + phase * ((current - previous) / ticks_spanned)`
//!
//! Sprite placement, camera focus, and attached visuals must use the same presented
//! pose or they drift relative to one another. Extrapolating measured displacement
//! preserves collision clamping, hitstop, pause, and other movement that is not
//! represented by raw velocity. The discontinuity guard suppresses extrapolation
//! across teleports and other implausible jumps.

use ambition_platformer2d_core::Vec2;
use ambition_time::SimTick;
use bevy::prelude::{
    Commands, Component, Entity, Fixed, IntoScheduleConfigs, Query, Res, ResMut, Resource,
    SystemSet, Time, Update,
};

use crate::pose_view::BodyPoseView;
use crate::view_index::FeatureViewIndex;

/// Plausibility bound for the discontinuity guard — NOT a clock.
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
/// * frame-stepped host — the sim advances once per rendered frame, so the
///   published pose is already current and there is nothing to extrapolate.
/// * rollback (GGRS) host — answered by reading the driver's own
///   accumulator, which requires a patched build widening its visibility. See
///   [`sample_ggrs_accumulator_phase`].
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

    /// Publish this frame's phase. For a host whose clock this crate cannot see
    /// — the rollback driver banks its own accumulator — the owning crate
    /// samples it and sets it here, in [`PresentedPoseSet`], before
    /// [`advance_presented_body_poses`].
    ///
    /// Clamped: extrapolating past a whole tick is never wanted, and a
    /// catch-up frame can legitimately bank more than one step.
    #[inline]
    pub fn set(&mut self, phase: f32) {
        self.phase = phase.clamp(0.0, 1.0);
    }
}

/// Read the exact intra-tick remainder Bevy already computed.
///
/// Deliberately not reimplemented as a hand-rolled accumulator — an approximation of this
/// quantity produces uneven per-frame steps and reduces the shake instead of removing it.
pub fn sample_fixed_overstep_phase(fixed: Res<Time<Fixed>>, mut phase: ResMut<PresentationPhase>) {
    phase.phase = fixed.overstep_fraction().clamp(0.0, 1.0);
}

/// The presented position of one body, plus the two ticks it was derived from.
///
/// Attached automatically to every entity carrying [`BodyKinematics`] — every
/// simulated BODY, not only the player-bodied ones that publish a
/// [`BodyPoseView`]. Presentation-only: the simulation never reads it, so a
/// rollback resim neither restores nor consults it.
///
/// [`BodyKinematics`]: ambition_platformer2d_core::BodyKinematics
#[derive(Component, Clone, Copy, Debug)]
pub struct PresentedPose {
    previous: Vec2,
    current: Vec2,
    /// How many sim ticks the gap `previous → current` spans.
    ///
    /// Not always one, and that is the whole reason this field exists. A
    /// host banks unspent real time and then spends it in whole ticks, so a
    /// single rendered frame can advance the simulation twice — and this pose is
    /// pushed once per FRAME, not once per tick, because a rendered frame is the
    /// only moment presentation gets to look. `previous` is therefore the pose
    /// from `spanned` ticks ago, and dividing by it is what turns the gap back
    /// into the ONE-tick step [`Self::tick_delta`] promises.
    spanned: u32,
    presented: Vec2,
    tick: u64,
}

impl PresentedPose {
    fn new(pos: Vec2, tick: u64) -> Self {
        Self {
            previous: pos,
            current: pos,
            spanned: 1,
            presented: pos,
            tick,
        }
    }

    /// The position to draw this body and everything anchored to it at.
    #[inline]
    pub fn presented(self) -> Vec2 {
        self.presented
    }

    /// The newest tick position, unsmoothed.
    ///
    /// No shipped overlay draws from this. The debug collision box deliberately
    /// does NOT: gizmos are drawn through a camera that advances on the render
    /// clock, so a box placed here is a step function sampled by a smoothly
    /// moving observer and visibly shakes — while the simulation behind it is
    /// perfectly regular. Kept for a deliberate second overlay that wants to
    /// SHOW the extrapolation lead, which is the only thing this reveals that
    /// [`Self::presented`] does not.
    #[inline]
    pub fn authoritative(self) -> Vec2 {
        self.current
    }

    /// Displacement the simulation actually produced across ONE tick — the
    /// per-tick step, never the whole gap the last push happened to span.
    #[inline]
    pub fn tick_delta(self) -> Vec2 {
        (self.current - self.previous) / self.spanned.max(1) as f32
    }

    /// The one translation everything rigidly attached to this body takes.
    ///
    /// `presented − authoritative`: how far this frame's drawn body has been
    /// carried away from the tick position every authoritative geometry row was
    /// resolved against. A consumer holding tick-clock geometry for this body —
    /// its collision envelope, its hurtboxes, a strike anchored to it — moves
    /// the WHOLE rigid group by this and by nothing else.
    ///
    /// Shape, size and relative placement are never touched — this is a rigid
    /// translation of already-resolved geometry, never a recomputation.
    #[inline]
    pub fn delta(self) -> Vec2 {
        self.presented - self.current
    }

    /// Accept a newly published tick pose. `continuous` false means the body did
    /// not TRAVEL here (portal, room change, respawn, possession swap): the
    /// history collapses so the jump is drawn as a jump and never extrapolated
    /// along.
    fn push(&mut self, pos: Vec2, tick: u64, continuous: bool) {
        // Clamped to at least one: a host that republished the same tick would
        // divide by zero, and a rollback that somehow moved the counter
        // backwards would underflow. Neither is reachable through
        // `advance_presented_body_poses` (it only pushes on a CHANGED tick, and
        // GGRS resimulates forward to the frame it already reached), but the
        // arithmetic must not depend on that being true.
        self.spanned = tick.saturating_sub(self.tick).clamp(1, u32::MAX as u64) as u32;
        self.previous = if continuous { self.current } else { pos };
        self.current = pos;
        self.tick = tick;
    }

    fn resample(&mut self, phase: f32) {
        self.presented = self.current + self.tick_delta() * phase;
    }
}

/// The one call every body-anchored visual makes instead of reading
/// `BodyPoseView::pos`.
///
/// Falls back to the tick pose when no history exists yet or the host reports
/// no phase, so a consumer is always correct — just not smoothed.
#[inline]
pub fn draw_pos(pose: &BodyPoseView, presented: Option<&PresentedPose>) -> Vec2 {
    presented.map_or(pose.pos, |presented| presented.presented())
}

/// Could a body carrying `vel` have travelled `from → to` in `ticks` ticks under
/// its own power? A teleport answers no, and must not be extrapolated across.
///
/// Shared with the camera's cast framing, which must not CHASE a body that did
/// not travel any more than this must extrapolate one. Both are asking "did
/// this body get here by moving", and one predicate is how they cannot drift.
pub(crate) fn travelled_under_own_power(from: Vec2, to: Vec2, vel: Vec2, ticks: u32) -> bool {
    let expected = vel.length() * NOMINAL_TICK_DT * ticks.max(1) as f32;
    from.distance(to) <= expected * TRAVEL_SLACK + TRAVEL_FLOOR_PX
}

/// Roll every body's presented pose forward: extend the history on a new tick,
/// then resample for THIS frame's phase.
///
/// The two facts this needs are `pos` and `vel`, and `rebuild_body_pose_views` copies both from
/// `BodyKinematics` verbatim, so the player population's numbers are unchanged to the bit while
/// every other body gains the smoothing it was always meant to have.
pub fn advance_presented_body_poses(
    mut commands: Commands,
    tick: Res<SimTick>,
    phase: Res<PresentationPhase>,
    mut bodies: Query<(
        Entity,
        &ambition_platformer2d_core::BodyKinematics,
        Option<&mut PresentedPose>,
    )>,
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
            let spanned = tick
                .0
                .saturating_sub(presented.tick)
                .clamp(1, u32::MAX as u64) as u32;
            let continuous =
                travelled_under_own_power(presented.current, pose.pos, pose.vel, spanned);
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
                    // Per TICK, times the ticks this frame actually spanned —
                    // the body-side guard scales the same way and for the same
                    // reason.
                    let spanned = tick.0.saturating_sub(pose.tick).clamp(1, u32::MAX as u64) as f32;
                    let leap = view.size.max_element().max(TRAVEL_FLOOR_PX) * 3.0 * spanned;
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

/// Ordering handle for presented-pose resampling.
/// Consumers of [`PresentedPose`] must run after this set or they may read the
/// previous frame's pose while other presentation reads the current one.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PresentedPoseSet;

/// Ordered stages for presented-pose resampling.
///
/// Every phase sampler joins [`Self::SamplePhase`]; every pose resampler joins
/// [`Self::Resample`]. The set order prevents consumers from resampling against a
/// stale intra-tick phase without requiring per-system ordering edges.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PresentedPoseStage {
    /// Publish this frame's intra-tick phase into [`PresentationPhase`]. One
    /// member per host; only the host's own sampler is installed.
    SamplePhase,
    /// Roll every presented pose forward and resample it for that phase.
    Resample,
}

/// Installs the frame-clock sampling layer.
pub struct PresentedPosePlugin;

impl bevy::prelude::Plugin for PresentedPosePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt as _;

        app.init_resource::<PresentationPhase>();
        app.init_resource::<PresentedFeaturePoses>();

        // Publish the phase before any presented pose is resampled from it.
        app.configure_sets(
            Update,
            (
                PresentedPoseStage::SamplePhase,
                PresentedPoseStage::Resample,
            )
                .chain()
                .in_set(PresentedPoseSet),
        );

        // Only a host that banks unspent real time between sim steps HAS an
        // intra-tick phase — and the two that do bank it in different places.
        // A frame-stepped host has none: it publishes a pose every rendered
        // frame, so there is nothing to sample between.
        if app.sim_is(bevy::prelude::FixedUpdate) {
            app.add_systems(
                Update,
                sample_fixed_overstep_phase.in_set(PresentedPoseStage::SamplePhase),
            );
        }
        // When the frame-stepped host runs simulation in `Update`, Bevy needs an explicit
        // same-schedule edge from the sim's read-model tail. Without it this set may sample
        // yesterday's BodyPoseView/FeatureViewIndex while the portal continuity pass has
        // already observed today's authoritative body — exactly one frame of whole-portal
        // camera lag.
        if app.sim_is(bevy::prelude::Update) {
            app.configure_sets(
                Update,
                PresentedPoseSet
                    .after(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::FeatureViewSync),
            );
        }
        // The rollback host's phase lives in the GGRS driver's own accumulator,
        // so its sampler belongs to the crate that owns GGRS — this one must
        // not learn about netcode. It publishes through
        // [`PresentationPhase::set`] and joins
        // [`PresentedPoseStage::SamplePhase`], which is all it has to know.
        app.add_systems(
            Update,
            (
                advance_presented_body_poses,
                advance_presented_feature_poses,
            )
                .in_set(PresentedPoseStage::Resample),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pose_at(pos: Vec2) -> PresentedPose {
        PresentedPose::new(pos, 0)
    }

    /// The integer-nanosecond tick period every banking host uses — `bevy_ggrs`
    /// computes it exactly this way, and `Time<Fixed>` at 60 Hz agrees.
    const TICK_NS: u64 = 1_000_000_000 / 60;

    /// Replay a display clock against the sim's banked accumulator.
    ///
    /// This is the arithmetic a real host performs: bank the frame's real delta,
    /// spend it in whole ticks, keep the remainder as the phase. A body moving at
    /// a constant `vel` is the one case where "where it should be drawn" has a
    /// closed form, so any deviation is the presentation layer's own error rather
    /// than the simulation's.
    ///
    /// Returns, per rendered frame, `(drawn_x, true_x)`.
    fn replay_display_clock(frame_dts_ns: &[u64], vel: f32) -> Vec<(f32, f32)> {
        let per_tick = vel / 60.0;
        let mut accumulator = 0u64;
        let mut tick = 0u64;
        let mut banked = 0u64;
        let mut pose = pose_at(Vec2::ZERO);
        let mut rows = Vec::with_capacity(frame_dts_ns.len());
        for &dt in frame_dts_ns {
            accumulator += dt;
            banked += dt;
            while accumulator >= TICK_NS {
                accumulator -= TICK_NS;
                tick += 1;
            }
            // Exactly what `advance_presented_body_poses` does, once per FRAME.
            if pose.tick != tick {
                pose.push(Vec2::new(per_tick * tick as f32, 0.0), tick, true);
            }
            pose.resample(accumulator as f32 / TICK_NS as f32);
            // Truth on the sim's own terms: `per_tick` of travel for every
            // `TICK_NS` of real time that has been banked, fraction included.
            rows.push((
                pose.presented().x,
                per_tick * (banked as f64 / TICK_NS as f64) as f32,
            ));
        }
        rows
    }

    /// ~60 fps with deterministic sub-millisecond jitter — the frame clock an unlocked or
    /// barely-keeping-up host actually produces.
    fn jittery_60fps(frames: usize, jitter_ns: i64) -> Vec<u64> {
        let mut seed = 0x2545_F491_4F6C_DD1Du64;
        (0..frames)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                let offset = (seed % (2 * jitter_ns as u64)) as i64 - jitter_ns;
                (TICK_NS as i64 + offset) as u64
            })
            .collect()
    }

    /// The metric is the DRAWN position against closed-form truth, because the
    /// smoothness the eye judges is a property of the drawn sequence and nothing
    /// else. Absent the fix the worst deviation here is a full tick of travel.
    #[test]
    fn a_jittery_frame_clock_draws_a_moving_body_exactly_where_it_is() {
        let rows = replay_display_clock(&jittery_60fps(600, 1_500_000), 400.0);
        let worst = rows
            .iter()
            .skip(20) // the first frames have no history to extrapolate from
            .map(|(drawn, truth)| (drawn - truth).abs())
            .fold(0.0f32, f32::max);
        assert!(
            worst < 0.05,
            "a constant-velocity body must be drawn where it truly is on every \
             frame; worst deviation {worst:.3} px. A value near one tick of \
             travel (6.7 px here) means the extrapolation is riding a multi-tick \
             gap as if it were one tick — see PresentedPose::tick_delta"
        );
    }

    /// The mechanism of the above, stated directly: a frame that banked two ticks
    /// must not double the lead the next frame extrapolates.
    #[test]
    fn a_frame_that_advanced_two_ticks_extrapolates_one_tick_of_travel() {
        let mut pose = pose_at(Vec2::ZERO);
        // ONE frame, TWO ticks: 6 px of travel each.
        pose.push(Vec2::new(12.0, 0.0), 2, true);
        assert_eq!(
            pose.tick_delta(),
            Vec2::new(6.0, 0.0),
            "tick_delta is the per-TICK step, never the whole gap"
        );
        // The next frame advances NO tick and banks 95% of one. The lead is 95%
        // of ONE tick (5.7 px), not of the two-tick gap (11.4 px).
        pose.resample(0.95);
        assert!(
            (pose.presented().x - 17.7).abs() < 1e-3,
            "drew at {} px, expected 17.7",
            pose.presented().x
        );
    }

    /// A steady clock at any rate — locked, faster than the sim, slower than it — was already
    /// exact, and must stay exact.
    #[test]
    fn a_steady_frame_clock_of_any_rate_is_exact() {
        for hz in [60.0, 59.94, 144.0, 120.0, 50.0, 30.0] {
            let dt = (1e9 / hz) as u64;
            let rows = replay_display_clock(&vec![dt; 400], 400.0);
            let worst = rows
                .iter()
                .skip(20)
                .map(|(drawn, truth)| (drawn - truth).abs())
                .fold(0.0f32, f32::max);
            assert!(worst < 0.05, "{hz} Hz display: worst {worst:.3} px");
        }
    }

    #[test]
    fn frame_stepped_presented_pose_runs_after_feature_view_sync() {
        use ambition_platformer2d_shared_tangle::schedule::{
            Platformer2dSimulationPhaseMonolith, SimScheduleExt as _,
        };
        use bevy::ecs::schedule::{NodeId, Schedules};
        use bevy::prelude::{App, IntoScheduleConfigs as _, Update};

        let mut app = App::new();
        app.set_sim_schedule(Update);
        app.add_plugins(PresentedPosePlugin);
        // Touch the producer set explicitly; the presented-pose systems already
        // register their consumer set through the plugin.
        app.add_systems(
            Update,
            (|| {}).in_set(Platformer2dSimulationPhaseMonolith::FeatureViewSync),
        );

        let schedules = app.world().resource::<Schedules>();
        let schedule = schedules
            .get(Update)
            .expect("Update schedule must exist after PresentedPosePlugin");
        let graph = schedule.graph();
        let producer = graph
            .system_sets
            .get_key(Platformer2dSimulationPhaseMonolith::FeatureViewSync.intern())
            .expect("FeatureViewSync must be registered");
        let consumer = graph
            .system_sets
            .get_key(PresentedPoseSet.intern())
            .expect("PresentedPoseSet must be registered");
        assert!(
            graph
                .dependency()
                .graph()
                .contains_edge(NodeId::Set(producer), NodeId::Set(consumer)),
            "frame-stepped hosts require FeatureViewSync -> PresentedPoseSet; otherwise the camera can sample a stale pre-portal pose"
        );
    }

    /// Every resampler is ordered after the phase it resamples against.
    ///
    /// Stated on the SETS, so it holds for the rollback host's sampler too — that one lives in
    /// `ambition_platformer2d_runtime` (this crate must not learn about netcode) and cannot be
    /// reached from here.
    #[test]
    fn the_phase_is_sampled_before_every_pose_is_resampled() {
        use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt as _;
        use bevy::ecs::schedule::{NodeId, Schedules};
        use bevy::prelude::{App, FixedUpdate};

        let mut app = App::new();
        app.set_sim_schedule(FixedUpdate);
        app.add_plugins(PresentedPosePlugin);

        let schedules = app.world().resource::<Schedules>();
        let graph = schedules
            .get(Update)
            .expect("Update exists after PresentedPosePlugin")
            .graph();
        let sample = graph
            .system_sets
            .get_key(PresentedPoseStage::SamplePhase.intern())
            .expect("SamplePhase must be registered");
        let resample = graph
            .system_sets
            .get_key(PresentedPoseStage::Resample.intern())
            .expect("Resample must be registered");
        assert!(
            graph
                .dependency()
                .graph()
                .contains_edge(NodeId::Set(sample), NodeId::Set(resample)),
            "a pose resampled against a phase nobody has published yet is drawn \
             a whole tick off on the frame a tick lands, then snapped back — a \
             per-frame stutter"
        );
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
            vel,
            1
        ));
        assert!(!travelled_under_own_power(
            Vec2::ZERO,
            Vec2::new(900.0, 0.0),
            vel,
            1
        ));
        // ...and the same 900 px IS honest travel when the frame banked a
        // backlog: 40 ticks at 400 px/s covers it. A one-tick budget would call
        // this a teleport and drop the smoothing for that frame.
        assert!(travelled_under_own_power(
            Vec2::ZERO,
            Vec2::new(900.0, 0.0),
            vel,
            40
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

    ///  THE POPULATION IS EVERY BODY, and it was not.
    ///
    /// A boss or an actor carries `BodyKinematics` and no `BodyPoseView` — that
    /// read model is rebuilt only `With<PlayerVisual>`. While this system joined
    /// on the view, every such body published no presented pose at all, and each
    /// consumer that looked one up took its "no history" fallback forever. The
    /// visible cost was a combat overlay drawing a player's strike on the frame
    /// clock beside a boss's on the tick clock.
    #[test]
    fn a_body_with_no_pose_view_still_gets_a_presented_pose() {
        use ambition_platformer2d_core::BodyKinematics;
        use bevy::prelude::{App, Update};

        let mut app = App::new();
        app.init_resource::<SimTick>();
        app.init_resource::<PresentationPhase>();
        app.add_systems(Update, advance_presented_body_poses);

        let mut kin = BodyKinematics::default();
        kin.pos = Vec2::new(120.0, 40.0);
        // A feature body: kinematics, no player pose view.
        let feature_body = app.world_mut().spawn(kin.clone()).id();
        // A player-bodied one, which already worked.
        let player_body = app
            .world_mut()
            .spawn((kin.clone(), BodyPoseView::default()))
            .id();

        app.update();

        for (label, entity) in [("feature", feature_body), ("player", player_body)] {
            let presented = app
                .world()
                .entity(entity)
                .get::<PresentedPose>()
                .unwrap_or_else(|| panic!("the {label} body must publish a presented pose"));
            assert_eq!(
                presented.presented(),
                kin.pos,
                "first sight presents exactly where the body is",
            );
            assert_eq!(
                presented.delta(),
                Vec2::ZERO,
                "and a body with no history is drawn where it was resolved",
            );
        }
    }

    /// The delta is what a rigidly attached row is translated by, and it is
    /// the same number for every row of one body.
    #[test]
    fn the_delta_is_the_presented_lead_over_the_authoritative_pose() {
        let mut pose = pose_at(Vec2::ZERO);
        pose.push(Vec2::new(10.0, 0.0), 1, true);
        pose.resample(0.5);
        assert_eq!(pose.authoritative(), Vec2::new(10.0, 0.0));
        assert_eq!(pose.presented(), Vec2::new(15.0, 0.0));
        assert_eq!(
            pose.delta(),
            pose.presented() - pose.authoritative(),
            "the delta IS that difference; a consumer must never re-derive it \
             from a second source that can disagree",
        );
    }

    ///  WHY THE CAMERA CARRIES ITS FRAMING POSE BY THE DELTA.
    ///
    /// That is correct only when the pose being framed IS the followed body's own — and it silently
    /// is not when the camera frames a CAST, where the pose is the pair's CENTRE and the presented
    /// sample comes from one anchor seat. Assigning then throws the centre away and points the
    /// camera at that seat. It was unreachable only while fighters published no presented pose.
    #[test]
    fn carrying_by_the_delta_equals_replacing_only_for_the_bodys_own_pose() {
        let mut pose = pose_at(Vec2::new(100.0, 0.0));
        pose.push(Vec2::new(112.0, 0.0), 1, true);
        pose.resample(0.5);

        let own = pose.authoritative();
        assert_eq!(
            own + pose.delta(),
            pose.presented(),
            "for the followed body's own pose the two rules agree exactly",
        );

        // A framing centre between two bodies is NOT the anchor's pose.
        let centre = Vec2::new(60.0, 0.0);
        assert_eq!(centre + pose.delta(), Vec2::new(66.0, 0.0));
        assert_ne!(
            centre + pose.delta(),
            pose.presented(),
            "replacing would have snapped the camera from the centre onto the \
             anchor seat — 46px here, and half the cast's span in general",
        );
    }
}
