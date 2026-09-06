//! Thread-local causal-fact sink for instrumenting pure call trees without
//! threading a recorder through every intermediate function.
//!
//! The sink is observer-only: domain code can write through [`record`] but has
//! no read path, and gameplay composition does not open it. This prevents the
//! hidden channel from becoming authoritative simulation state.
//!
//! It is valid only when publisher and collector run on the same thread. ECS
//! recording that may cross worker threads should use an ECS-owned log instead;
//! [`facts_lost_offthread`] reports violations of that restriction.

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

/// Number of facts published on a thread without a local sink while another
/// thread had a sink open. ECS collectors should require this to remain zero.
pub fn facts_lost_offthread() -> u64 {
    LOST_OFFTHREAD.load(Ordering::Relaxed)
}

/// Serialize tests that touch process-global sink counters. Poisoning is
/// ignored so one failed test does not cascade the same failure into siblings.
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
/// Returns nothing, on purpose: there is no read path, so a simulation
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
