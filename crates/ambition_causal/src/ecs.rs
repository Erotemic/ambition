//! The causal log as a Bevy resource.
//!
//! ECS systems should record through `ResMut<CausalRecording>` because the generic
//! sink is thread-local and Bevy may schedule systems on worker threads.

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
    /// Lend this resource's log to the calling thread while `body` runs.
    ///
    /// Use this for pure library code invoked by an ECS system that cannot accept a
    /// Bevy resource directly. ECS systems themselves should take `ResMut<CausalRecording>`.
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

    /// A plain library `record` call lands in a temporarily lent resource log.
    #[test]
    fn a_lent_log_collects_what_a_plain_record_call_publishes() {
        // Sink state affects process-global diagnostics, so serialize sink tests.
        let _serialised = crate::sink::global_sink_test_lock();
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
        // The process-global off-thread counter is tested separately.
        assert!(
            recording
                .explain(9, &SubjectKey::Sim("fighter_left".into()))
                .first("decided")
                .is_some(),
            "the fact landed in the RESOURCE, which is what survives the system"
        );
    }
}
