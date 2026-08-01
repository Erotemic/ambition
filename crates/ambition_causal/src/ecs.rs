//! The log as a Bevy resource.
//!
//! ⚠ **this is the SOUND way to record from an ECS**, and the thread-local sink
//! is not: Bevy runs systems across worker threads, so a system publishing
//! through the sink publishes into nothing (see
//! [`crate::facts_lost_offthread`]). A system takes `ResMut<CausalRecording>`.
//!
//! It lives here rather than in a host crate so that ANY domain can publish
//! without depending on a host — which is the property that lets an explanation
//! survive a composition with movement and no combat.

use bevy_ecs::prelude::Resource;

use crate::log::CausalLog;

/// The app's causal log.
#[derive(Resource, Default)]
pub struct CausalRecording(pub CausalLog);

impl std::ops::Deref for CausalRecording {
    type Target = CausalLog;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for CausalRecording {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl CausalRecording {
    /// **Lend this log to the calling thread for the duration of `body`**, so
    /// code that cannot take a resource can still publish soundly.
    ///
    /// The module note above says a system must take `ResMut<CausalRecording>`,
    /// and that stays true for anything that CAN. This is for the case it
    /// cannot: a pure library — a brain, a solver, a scoring pass — that a host
    /// calls from inside a system. Threading a recorder through such a call
    /// would put the log on the simulation's own signatures, and the repo has
    /// already refused that once: the movement observer runs AFTER the brain
    /// tick precisely so "a system that only reads cannot be the thing that
    /// broke the tick".
    ///
    /// So the HOST opens the sink on whichever worker thread it happens to be
    /// on, the library keeps its plain [`crate::record`] call, and the facts
    /// land in this resource instead of in nothing.
    ///
    /// ⚠ **it is not a license to publish from an ECS system through the
    /// sink.** A system that could have taken the resource and reached for this
    /// instead has bought a thread-local swap and lost the compiler's help.
    pub fn lend_to_thread<T>(&mut self, body: impl FnOnce() -> T) -> T {
        let lent = std::mem::take(&mut self.0);
        let (returned, value) = crate::with_sink(lent, body);
        self.0 = returned;
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CausalFact, FactDetail, RecordingPolicy, SubjectKey, domains};

    /// **A library that cannot take a resource still publishes into one.**
    ///
    /// The case this exists for: a brain, five hops below the ECS, calling
    /// [`crate::record`]. Without the lend, that write goes to a thread-local
    /// sink the host never opened — and Bevy runs systems across worker threads,
    /// so it goes nowhere and [`crate::facts_lost_offthread`] counts it.
    #[test]
    fn a_lent_log_collects_what_a_plain_record_call_publishes() {
        let mut recording = CausalRecording::default();
        recording.set_policy(RecordingPolicy::only([domains::BRAIN]));
        recording.set_tick(9);

        let returned = recording.lend_to_thread(|| {
            // Exactly what a library publisher does — no resource in sight.
            crate::record(
                CausalFact::new(domains::BRAIN, 0, FactDetail::new("decided", "chose Dash"))
                    .about(SubjectKey::Sim("fighter_left".into())),
            );
            "the body's own return value"
        });

        assert_eq!(returned, "the body's own return value");
        // ⚠ deliberately NOT asserted here: `facts_lost_offthread()` is a
        // process-global counter, and `a_fact_published_off_thread_is_counted_
        // rather_than_vanishing` in `tests.rs` already owns that claim — in
        // more detail, including that a miss with NO sink open anywhere is the
        // shipped path and must not inflate it. A second test racing it on the
        // same global made that one fail (2 where it expected 1), which is the
        // counter doing its job about the wrong subject.
        assert!(
            recording
                .explain(9, &SubjectKey::Sim("fighter_left".into()))
                .first("decided")
                .is_some(),
            "the fact landed in the RESOURCE, which is what survives the system"
        );
    }
}
