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
//! presented = current + phase * one_tick_of_travel
//! ```
//!
//! `one_tick_of_travel` is `(current - previous) / ticks_spanned`, and the
//! divisor is not pedantry: this layer samples once per rendered FRAME, and a
//! frame can advance the simulation more than once. Treating a two-tick gap as
//! one tick's travel drew a body nearly two ticks ahead on the next frame that
//! advanced no tick at all — see [`PresentedPose::tick_delta`].
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

use ambition_platformer2d_core::Vec2;
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
/// * **rollback (GGRS) host** — answered by reading the driver's own
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
    /// How many sim ticks the gap `previous → current` spans.
    ///
    /// **Not always one, and that is the whole reason this field exists.** A
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

    /// **The position to draw this body and everything anchored to it at.**
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
    ///
    /// The distinction is the difference between smooth and worse-than-nothing.
    /// [`Self::resample`] multiplies this by a phase in `[0, 1)` meaning
    /// "fraction of ONE tick elapsed", so the two have to agree on what a tick
    /// is. When they did not, a frame that advanced the sim twice left a
    /// double-width delta behind, and the next frame that advanced it zero times
    /// — the pairing a jittery frame clock produces constantly — multiplied that
    /// double width by a phase grown to ~0.95 and drew the body nearly TWO ticks
    /// ahead, then snapped it back when the next tick landed. Measured at
    /// 400 px/s: a 13 px lurch every beat, against a 6.7 px true step.
    #[inline]
    pub fn tick_delta(self) -> Vec2 {
        (self.current - self.previous) / self.spanned.max(1) as f32
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

/// **The one call every body-anchored visual makes** instead of reading
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
/// `ticks` is not decoration: the caller observes once per rendered FRAME, and a
/// frame that spent a banked backlog can legitimately carry a body several ticks
/// of travel. Budgeting one tick for that gap calls honest running a teleport and
/// silently drops the smoothing for a frame — the failure would look like an
/// occasional stutter, i.e. exactly what this module exists to remove.
fn travelled_under_own_power(from: Vec2, to: Vec2, vel: Vec2, ticks: u32) -> bool {
    let expected = vel.length() * NOMINAL_TICK_DT * ticks.max(1) as f32;
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

/// Ordering handle: the presented poses are resampled before ANY consumer —
/// the camera resolve and the whole presentation visual sync alike.
///
/// **A consumer that reads a presented pose must order `.after` this set.** Not
/// ordering is not "runs late enough in practice": the resample WRITES
/// [`PresentedPose`], so an unordered reader merely conflicts with it, and Bevy
/// resolves a conflict by picking an order — one that is stable for a given
/// schedule build and therefore silently, consistently WRONG. A reader placed
/// before the resample sees last frame's presented pose while the camera sees
/// this frame's, and the two disagree by one frame of motion every frame: a
/// sawtooth at the tick rate, which is the exact artifact this module exists to
/// remove. The debug collision-box overlay had no such edge and shook for that
/// reason (Jon, 2026-07-29).
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PresentedPoseSet;

/// The two halves of [`PresentedPoseSet`], ordered — **ask the phase first, then
/// resample every pose against it.**
///
/// This exists because stating that order per-system did not survive contact with
/// a second resampler. Both phase samplers (`Time<Fixed>`'s and the rollback
/// driver's) were written `.before(advance_presented_body_poses)`, naming ONE of
/// the two systems that consume the phase. [`advance_presented_feature_poses`] —
/// which positions every actor, enemy, NPC and duel fighter — was left to race
/// the sampler, so every id-keyed subject was resampled against whichever phase
/// the executor happened to hand it. On the frame a tick lands the phase drops by
/// nearly a whole tick, so reading the stale one draws the subject a full tick
/// ahead and snaps it back: a per-frame stutter on exactly the bodies whose
/// sprites are joined by id, while the primary player — the one system that HAD
/// the edge — looked fine.
///
/// A per-system `.before` is a claim each new author must remember to repeat.
/// These two sets are a claim the schedule enforces once: join
/// [`Self::SamplePhase`] to publish a phase, [`Self::Resample`] to consume one,
/// and the edge cannot be forgotten because no one has to write it again.
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

        // THE edge, declared once: ask the phase, then resample against it.
        // Every sampler and every resampler joins one of these, so no member has
        // to restate the relationship — and none can omit it.
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
        // When the frame-stepped host runs simulation in `Update`, Bevy needs
        // an explicit same-schedule edge from the sim's read-model tail. Without
        // it this set may sample yesterday's BodyPoseView/FeatureViewIndex while
        // the portal continuity pass has already observed today's authoritative
        // body — exactly one frame of whole-portal camera lag. Fixed-tick and
        // GGRS hosts finish their separate sim schedules before `Update`, so the
        // schedule boundary already supplies this ordering there.
        if app.sim_is(bevy::prelude::Update) {
            app.configure_sets(
                Update,
                PresentedPoseSet
                    .after(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::FeatureViewSync),
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

    /// **Replay a display clock against the sim's banked accumulator.**
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

    /// ~60 fps with deterministic sub-millisecond jitter — the frame clock an
    /// unlocked or barely-keeping-up host actually produces. `xorshift` rather
    /// than `rand` so the sequence is fixed and the crate gains no dependency.
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

    /// **The defect this module was already supposed to prevent, reached by the
    /// other door.** Reported by Jon (2026-07-29) as `player_robot_v2` stuttering
    /// under the rollback host, seen earlier in Mary-O.
    ///
    /// A jittery frame clock produces a frame that advances the sim TWICE
    /// followed by one that advances it not at all. `previous → current` then
    /// spanned two ticks while `resample` read it as one, so the zero-tick frame
    /// — whose phase has grown to ~0.95 — drew the body nearly two full ticks
    /// ahead and snapped it back on the next tick. Measured at 400 px/s: a 13 px
    /// lurch against a 6.7 px true step, once per beat between the two clocks.
    ///
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

    /// A steady clock at any rate — locked, faster than the sim, slower than it —
    /// was already exact, and must stay exact. This is the case that made the
    /// defect look absent: it only appears when frame durations VARY.
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
        use ambition_platformer2d_shared_tangle::schedule::{Platformer2dSimulationPhase, SimScheduleExt as _};
        use bevy::ecs::schedule::{NodeId, Schedules};
        use bevy::prelude::{App, IntoScheduleConfigs as _, Update};

        let mut app = App::new();
        app.set_sim_schedule(Update);
        app.add_plugins(PresentedPosePlugin);
        // Touch the producer set explicitly; the presented-pose systems already
        // register their consumer set through the plugin.
        app.add_systems(Update, (|| {}).in_set(Platformer2dSimulationPhase::FeatureViewSync));

        let schedules = app.world().resource::<Schedules>();
        let schedule = schedules
            .get(Update)
            .expect("Update schedule must exist after PresentedPosePlugin");
        let graph = schedule.graph();
        let producer = graph
            .system_sets
            .get_key(Platformer2dSimulationPhase::FeatureViewSync.intern())
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

    /// **Every resampler is ordered after the phase it resamples against.**
    ///
    /// Stated on the SETS, so it holds for the rollback host's sampler too — that
    /// one lives in `ambition_platformer2d_runtime` (this crate must not learn about netcode)
    /// and cannot be reached from here. What can be checked from here is the edge
    /// it relies on, which is the thing that was missing: both samplers named one
    /// resampler with a `.before` and the id-keyed poses raced the phase.
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
}
