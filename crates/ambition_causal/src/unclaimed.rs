//! **Velocity changes no operation claims — the detector, made portable.**
//!
//! A body's velocity moved by more than the integrator can produce, and nothing
//! in the tick named a writer. That question took the S51 thread six reading
//! cycles and eight eliminations before an instrument answered it in one run, so
//! it lives here rather than inside the one probe that needed it first.
//!
//! ## What it does NOT know
//!
//! Deliberately nothing about platformers, hosts, or ECS. It takes a subject
//! key, a velocity, and whether the tick carried any operation — so any
//! composition that publishes those can use it, which is the whole point: the
//! trace that motivated it was taken in the SANDBOX and the first detector was
//! wired into the smash LADDER, two compositions that never see each other's
//! bodies.
//!
//! ## Three traps, all paid for
//!
//! ⛔ **the threshold belongs to the CALLER**, because only the caller knows its
//! own kernel constants. The first version hardcoded `25.0` from a grep that
//! missed `pub const RUN_ACCEL = 5200.0` and was 5.8× too low; the second
//! carried a 1.5× "safety margin" for per-character tuning that does not exist
//! in the tree, putting the bar ABOVE the ramp it was built to find. Pass
//! `max_integrator_step` derived from the constants you actually integrate with.
//!
//! ⛔ **state resets when the tick does not advance.** A process that runs many
//! matches under one subject id will otherwise compare the last tick of one
//! against the first of the next and report the difference as a step. That
//! produced six spurious findings on the first run, character-identical to the
//! real ones — an artifact shaped exactly like the thing it hunts.
//!
//! ⚠ **an empty operation list is not proof that nothing explains the step.**
//! A velocity kill at tick N caused by a knockout at tick N−k shows no
//! explaining fact on tick N, and a tick-scoped explanation cannot join them.
//! Read a finding as *"no operation on this tick claims it"*, never as *"this is
//! unexplained"*.

use std::collections::HashMap;

/// One velocity change that no operation on its tick accounts for.
#[derive(Clone, Debug, PartialEq)]
pub struct UnclaimedStep {
    pub tick: u64,
    pub subject: String,
    /// Velocity before and after, so a reader can see the shape rather than only
    /// the magnitude — a constant per-tick delta is an acceleration, a single
    /// large one is an impulse, and the two point at different writers.
    pub before: f32,
    pub after: f32,
}

impl UnclaimedStep {
    pub fn delta(&self) -> f32 {
        self.after - self.before
    }
}

/// Carries the previous tick's velocity per subject and reports steps larger
/// than the integrator can make.
#[derive(Debug, Default)]
pub struct UnclaimedStepDetector {
    previous: HashMap<String, f32>,
    last_tick: Option<u64>,
}

impl UnclaimedStepDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Offer one subject's velocity for one tick.
    ///
    /// `had_operation` is whether the tick carried ANY kernel operation for this
    /// subject; a step is reported only when it did not. `max_integrator_step`
    /// is the largest per-tick change the caller's integrator can produce
    /// without announcing anything — see the module note about deriving it.
    ///
    /// ⚠ call this for every tick, not only the ones being printed. The S51
    /// ramp survived six cycles because the trace SAMPLED one tick in five and
    /// the ramp was three ticks long; the data was there the whole time.
    pub fn observe(
        &mut self,
        tick: u64,
        subject: &str,
        velocity: f32,
        had_operation: bool,
        max_integrator_step: f32,
    ) -> Option<UnclaimedStep> {
        // A tick that did not advance is a new run: everything carried across
        // that boundary describes a body that no longer exists.
        if self.last_tick.is_some_and(|last| tick <= last) {
            self.previous.clear();
        }
        self.last_tick = Some(tick);

        let found = self.previous.get(subject).and_then(|&before| {
            let step = velocity - before;
            (step.abs() > max_integrator_step && !had_operation).then(|| UnclaimedStep {
                tick,
                subject: subject.to_string(),
                before,
                after: velocity,
            })
        });
        self.previous.insert(subject.to_string(), velocity);
        found
    }

    /// Forget every carried velocity. For a caller that knows a run ended
    /// without the tick counter going backwards.
    pub fn reset(&mut self) {
        self.previous.clear();
        self.last_tick = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAX_STEP: f32 = 86.67;

    #[test]
    fn a_step_within_the_integrators_reach_is_not_reported() {
        let mut detector = UnclaimedStepDetector::new();
        assert!(detector.observe(1, "body", 0.0, false, MAX_STEP).is_none());
        // Exactly the integrator's own maximum: legitimate, and the boundary
        // case that decides whether the bar is `>` or `>=`.
        assert!(detector
            .observe(2, "body", 86.67, false, MAX_STEP)
            .is_none());
    }

    #[test]
    fn a_larger_step_with_no_operation_is_reported_with_both_endpoints() {
        let mut detector = UnclaimedStepDetector::new();
        detector.observe(1, "body", 760.0, false, MAX_STEP);
        let found = detector
            .observe(2, "body", 0.0, false, MAX_STEP)
            .expect("760 -> 0 in one tick is far past the integrator");
        assert_eq!(found.before, 760.0);
        assert_eq!(found.after, 0.0);
        assert_eq!(found.delta(), -760.0);
    }

    #[test]
    fn an_operation_on_the_tick_claims_the_step() {
        let mut detector = UnclaimedStepDetector::new();
        detector.observe(1, "body", 0.0, false, MAX_STEP);
        assert!(
            detector.observe(2, "body", 760.0, true, MAX_STEP).is_none(),
            "a dash announces itself; the detector is for writers that do not"
        );
    }

    /// ⛔ the artifact that six spurious findings came from, pinned.
    #[test]
    fn a_tick_that_does_not_advance_drops_the_carried_velocity() {
        let mut detector = UnclaimedStepDetector::new();
        detector.observe(400, "body", 760.0, false, MAX_STEP);
        assert!(
            detector.observe(2, "body", 0.0, false, MAX_STEP).is_none(),
            "tick 2 after tick 400 is a NEW run — comparing across that boundary \
             manufactures a 760 -> 0 step identical to the real ones"
        );
    }

    #[test]
    fn subjects_do_not_borrow_each_others_velocity() {
        let mut detector = UnclaimedStepDetector::new();
        detector.observe(1, "a", 760.0, false, MAX_STEP);
        assert!(
            detector.observe(1, "b", 0.0, false, MAX_STEP).is_none(),
            "b has no previous velocity of its own; a's must not stand in for it"
        );
    }
}
