//! Typed observer-only facts for explaining why simulation state changed.
//!
//! Domains publish [`CausalFact`] values and [`CausalLog::explain`] composes the
//! facts for one subject/tick. Simulation code must never read these facts:
//! recording is bounded, lossy, and not rewound by rollback, so branching on it
//! would violate determinism.
//!
//! [`with_sink`] is thread-local and is suitable only for a call tree that runs
//! on the same thread. ECS systems that may run on workers should record through
//! an ECS-owned [`CausalLog`]; [`facts_lost_offthread`] exposes misuse instead of
//! dropping facts without a signal.
//!
//! Fact kinds remain open vocabulary (`&'static str`) so capabilities can add
//! causal detail without editing a central enum.

#[cfg(feature = "bevy")]
mod ecs;
mod fact;
mod log;
mod sink;
mod unclaimed;
mod velocity;

#[cfg(feature = "bevy")]
pub use ecs::CausalRecording;
pub use fact::{
    domains, CausalDomain, CausalFact, Execution, FactDetail, FactId, FactValue, SubjectKey,
};
pub use log::{CausalLog, ExecutionKey, Explanation, RecordingPolicy};
pub use sink::{
    facts_lost_offthread, record, recording, reset_lost_offthread, with_sink, CausalSink,
};
pub use unclaimed::{UnclaimedStep, UnclaimedStepDetector};
pub use velocity::velocity_authored;

#[cfg(test)]
mod tests;
