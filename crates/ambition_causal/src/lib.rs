//! **Why did this actor change on this tick?**
//!
//! ```text
//! participant/input → semantic actions → control intent → AI or rules decision
//!   → chosen move → movement contributions → contacts and support
//!   → hit and damage resolution → reactions → lifecycle → rollback identity
//! ```
//!
//! Domains publish typed [`CausalFact`]s. [`CausalLog::explain`] composes them
//! into one chain for one subject on one tick. **Nothing here parses a text
//! log**, and nothing here is read by the simulation.
//!
//! ## What this replaces
//!
//! `AMBITION_FIGHTER_TRACE=1` printed one `eprintln!` per decision. It was the
//! right instinct and the wrong artifact: unqueryable, uncorrelatable with any
//! other domain, and — by its own docstring — *"not rollback-safe and does not
//! pretend to be"*, because a resimulated frame prints again with nothing to
//! say it was a repeat. A [`CausalFact`] carries [`Execution`], so the repeat is
//! LABELLED rather than indistinguishable.
//!
//! ## Observer-only, and it must stay that way
//!
//! ⛔ **the simulation must never read a fact.** Facts are recorded after the
//! decision they describe, they are bounded and therefore lossy, and under a
//! rollback host the log is not rewound. Anything that branched on one would
//! desync the moment history was replayed. The log is write-only from the
//! domain side and read-only from the tool side, and that split is the whole
//! safety argument.
//!
//! ## ⛔ The sink is THREAD-LOCAL, and that is a contract, not an accident
//!
//! [`with_sink`] collects facts published on the SAME thread. That is sound for
//! what it was built for — a pure call tree driven from one thread — and it is
//! NOT sound for Bevy's multithreaded scheduler, where a system on a worker
//! thread would publish into a sink the collector never sees.
//!
//! Silently losing them would be the unacceptable part, so it does not:
//! [`facts_lost_offthread`] counts exactly that. A collector running over an
//! ECS should assert it is zero; if it is not, that domain needs a
//! `ResMut<CausalLog>` recorder rather than the sink.
//!
//! ## Open vocabulary
//!
//! [`FactDetail::kind`] is a `&'static str`, not an enum variant. A capability
//! publishes its own kinds without editing a central enum — the same rule the
//! content compiler follows for schemas, for the same reason.

#[cfg(feature = "bevy")]
mod ecs;
mod fact;
mod log;
mod sink;

pub use fact::{
    CausalDomain, CausalFact, Execution, FactDetail, FactId, FactValue, SubjectKey, domains,
};
#[cfg(feature = "bevy")]
pub use ecs::CausalRecording;
pub use log::{CausalLog, Explanation, RecordingPolicy};
pub use sink::{
    CausalSink, facts_lost_offthread, record, recording, reset_lost_offthread, with_sink,
};

#[cfg(test)]
mod tests;
