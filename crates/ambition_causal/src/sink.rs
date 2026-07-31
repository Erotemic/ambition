//! The scoped sink — how a pure function deep in a call graph publishes a fact
//! without every caller between it and the ECS growing a parameter.
//!
//! ## Why this and not a threaded parameter
//!
//! The fighter's decision is a pure function five hops below the ECS system
//! that drives it, and `tick_state_machine` has 44 call sites. Threading a
//! recorder through all of them to instrument one domain is the kind of broad
//! speculative churn the program explicitly warns against — and it would have to
//! happen again for every domain that ever wants a fact.
//!
//! ## Why it is safe here, stated precisely
//!
//! This is a hidden channel, and a hidden channel is normally exactly what this
//! repo refuses ("do not hide new authoritative state in UI resources,
//! presentation systems, or unregistered components"). The distinction is that
//! this one is **not authoritative and cannot become so**:
//!
//! * it is **write-only** from the domain side — [`record`] returns `()`, so a
//!   simulation cannot read back what it wrote and branch on it;
//! * there is **no getter** on the sink, by construction;
//! * the sink is opened by a tool or a test, never by gameplay composition;
//! * when nothing opened one, [`record`] is one thread-local load and a return,
//!   which is the same cost the `AMBITION_FIGHTER_TRACE` env check it replaces
//!   already paid on every decision.
//!
//! If a future change gives this module a read path, that safety argument is
//! gone and the parameter threading becomes the right answer after all.

use std::cell::RefCell;

use crate::fact::CausalFact;
use crate::log::CausalLog;

thread_local! {
    static SINK: RefCell<Option<CausalLog>> = const { RefCell::new(None) };
}

/// A sink open for the duration of a call.
///
/// Held by the caller of [`with_sink`]; domains never see one.
pub struct CausalSink;

/// Run `body` with a sink open, and return the log it collected alongside the
/// body's value.
///
/// Re-entrant calls are NOT nested: an inner `with_sink` replaces the outer's
/// log for its duration and restores it after. That is deliberate — a nested
/// scope means a tool inside a tool, and silently merging the two would make
/// the outer dump contain facts the outer scope never asked for.
pub fn with_sink<T>(mut log: CausalLog, body: impl FnOnce() -> T) -> (CausalLog, T) {
    let previous = SINK.with(|sink| sink.borrow_mut().replace(std::mem::take(&mut log)));
    let value = body();
    let collected = SINK.with(|sink| sink.borrow_mut().take());
    SINK.with(|sink| *sink.borrow_mut() = previous);
    (collected.unwrap_or_default(), value)
}

/// Whether anything is listening. Domains that would have to DO WORK to build a
/// fact should ask first; a domain whose fact is a few moves should just call
/// [`record`] and let the policy drop it.
pub fn recording() -> bool {
    SINK.with(|sink| {
        sink.borrow()
            .as_ref()
            .is_some_and(CausalLog::is_recording)
    })
}

/// Publish a fact.
///
/// Returns nothing, on purpose: **there is no read path**, so a simulation
/// cannot branch on what it published and the log can never become authority.
pub fn record(fact: CausalFact) {
    SINK.with(|sink| {
        if let Some(log) = sink.borrow_mut().as_mut() {
            log.record(fact);
        }
    });
}
