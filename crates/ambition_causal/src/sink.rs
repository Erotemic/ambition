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
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::fact::CausalFact;
use crate::log::CausalLog;

thread_local! {
    static SINK: RefCell<Option<CausalLog>> = const { RefCell::new(None) };
}

/// How many sinks are open across all threads. Only read to decide whether a
/// `record` on a thread with no sink is a MISS or simply instrumentation being
/// off.
static OPEN_SINKS: AtomicUsize = AtomicUsize::new(0);

/// Facts published on a thread with no sink while a sink was open elsewhere.
static LOST_OFFTHREAD: AtomicU64 = AtomicU64::new(0);

/// ⛔ **Facts this process published on the wrong thread.**
///
/// A sink is THREAD-LOCAL, and that is sound for the case it was built for: a
/// pure call tree driven from one thread (`ladder_probe`, a headless test, a
/// rollout). It is NOT sound for Bevy's multithreaded scheduler — a system
/// running on a worker thread publishes into a sink the main thread never sees,
/// and the facts vanish.
///
/// Vanishing silently is the part that would be unacceptable, so it does not:
/// this counter is non-zero exactly when it has happened. **A caller collecting
/// from an ECS should assert it is zero**, and if it is not, the answer is a
/// `ResMut<CausalLog>` recorder for that domain rather than the sink.
pub fn facts_lost_offthread() -> u64 {
    LOST_OFFTHREAD.load(Ordering::Relaxed)
}

/// **Serialise tests that touch the global sink state.**
///
/// ⛔ `OPEN_SINKS` and `LOST_OFFTHREAD` are PROCESS globals, and Rust runs tests
/// in parallel. A test that opens a sink makes every other test's "is anybody
/// listening" answer true for as long as it holds one, which is exactly what
/// `a_fact_published_off_thread_is_counted_rather_than_vanishing` asserts on —
/// so it went red (`left: 2, right: 1`) the day a second test opened a sink,
/// having been quietly vulnerable to it since it was written.
///
/// Any test that opens a sink or reads either counter takes this first.
/// Poisoning is ignored: a test that panicked while holding it has already
/// reported its own failure, and cascading that into every sibling reports the
/// same bug N times.
#[cfg(test)]
pub(crate) fn global_sink_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Reset the off-thread counter. For a test that deliberately provokes one.
pub fn reset_lost_offthread() {
    LOST_OFFTHREAD.store(0, Ordering::Relaxed);
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
    if previous.is_none() {
        OPEN_SINKS.fetch_add(1, Ordering::Relaxed);
    }
    let value = body();
    let collected = SINK.with(|sink| sink.borrow_mut().take());
    SINK.with(|sink| *sink.borrow_mut() = previous.clone());
    if previous.is_none() {
        OPEN_SINKS.fetch_sub(1, Ordering::Relaxed);
    }
    (collected.unwrap_or_default(), value)
}

/// Whether anything is listening. Domains that would have to DO WORK to build a
/// fact should ask first; a domain whose fact is a few moves should just call
/// [`record`] and let the policy drop it.
pub fn recording() -> bool {
    SINK.with(|sink| sink.borrow().as_ref().is_some_and(CausalLog::is_recording))
}

/// Publish a fact.
///
/// Returns nothing, on purpose: **there is no read path**, so a simulation
/// cannot branch on what it published and the log can never become authority.
pub fn record(fact: CausalFact) {
    let recorded = SINK.with(|sink| match sink.borrow_mut().as_mut() {
        Some(log) => {
            log.record(fact);
            true
        }
        None => false,
    });
    // Only a MISS when somebody is listening. With instrumentation off (the
    // shipped default) this is the ordinary path and costs one relaxed load.
    if !recorded && OPEN_SINKS.load(Ordering::Relaxed) > 0 {
        LOST_OFFTHREAD.fetch_add(1, Ordering::Relaxed);
    }
}
