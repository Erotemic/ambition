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
//! ## Open vocabulary
//!
//! [`FactDetail::kind`] is a `&'static str`, not an enum variant. A capability
//! publishes its own kinds without editing a central enum — the same rule the
//! content compiler follows for schemas, for the same reason.

mod fact;
mod log;
mod sink;

pub use fact::{
    CausalDomain, CausalFact, Execution, FactDetail, FactId, FactValue, SubjectKey, domains,
};
pub use log::{CausalLog, Explanation, RecordingPolicy};
pub use sink::{CausalSink, record, recording, with_sink};

#[cfg(test)]
mod tests;
