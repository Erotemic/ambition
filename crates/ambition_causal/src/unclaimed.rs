//! Detect velocity steps larger than the caller's integrator can produce when
//! no operation on that tick claims the change.
//!
//! The detector is independent of ECS and game composition. The caller supplies
//! `max_integrator_step` from its actual integration constants. State resets when
//! ticks do not advance so separate runs are never compared. A finding means
//! only that no operation on the same tick claims the step; delayed causes from
//! earlier ticks remain possible.

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
    /// `had_operation` reports whether the tick carried any kernel operation for
    /// this subject. `max_integrator_step` is the largest per-tick change the
    /// caller's integrator can produce without an operation claim. Call this for
    /// every tick so short steps are not skipped by display sampling.
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

    ///  the artifact that six spurious findings came from, pinned.
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
