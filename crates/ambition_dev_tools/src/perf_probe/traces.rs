//! Sampled backtraces of the allocations in chosen spans.
//!
//! `alloc_by_span.csv` says how much a span allocates; this says from where.
//! Set [`TRACE_ENV`] to comma-separated parts of span labels, for example
//! `schedule GgrsSchedule,schedule Update`. One allocation in
//! [`TRACE_EVERY_ENV`] (default 500) inside a span whose label holds one of
//! them takes a backtrace. The probe writes the call chains, ranked, to
//! `alloc_traces.csv`. A chain is the first frames that are not the
//! allocator, the probe or `core`/`alloc`.
//!
//! A schedule span with no system span inside it holds the executor's own work:
//! command application, observers, run conditions. Its row in
//! `alloc_by_span.csv` has no owner; its chains here name one.
//!
//! A capture symbolizes the stack, which is slow: use this in a probe run only,
//! and read counts, not times.

use std::backtrace::Backtrace;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::{Mutex, OnceLock};

use super::alloc::{ProbeScope, MAX_SLOTS};

/// Comma-separated parts of span labels whose allocations take backtraces.
pub const TRACE_ENV: &str = "AMBITION_PERF_PROBE_TRACE";
/// One traced allocation in this many takes a backtrace. Default 500.
pub const TRACE_EVERY_ENV: &str = "AMBITION_PERF_PROBE_TRACE_EVERY";
/// Frames kept in a chain.
const CHAIN_FRAMES: usize = 16;

static TRACED: [AtomicBool; MAX_SLOTS] = [const { AtomicBool::new(false) }; MAX_SLOTS];
/// 0: tracing is off.
static EVERY: AtomicU64 = AtomicU64::new(0);
static TICK: AtomicU64 = AtomicU64::new(0);
static PATTERNS: OnceLock<Vec<String>> = OnceLock::new();
static CHAINS: Mutex<Option<HashMap<String, u64>>> = Mutex::new(None);

/// Read [`TRACE_ENV`] and [`TRACE_EVERY_ENV`]. Call it before the first span
/// is made, so each label is checked when it is interned. Returns the
/// patterns and the sampling interval when tracing is on.
pub fn configure_from_env() -> Option<(Vec<String>, u64)> {
    let patterns: Vec<String> = std::env::var(TRACE_ENV)
        .ok()?
        .split(',')
        .map(|part| part.trim().to_owned())
        .filter(|part| !part.is_empty())
        .collect();
    if patterns.is_empty() {
        return None;
    }
    let every = std::env::var(TRACE_EVERY_ENV)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|every| *every > 0)
        .unwrap_or(500);
    let _ = PATTERNS.set(patterns.clone());
    EVERY.store(every, Relaxed);
    Some((patterns, every))
}

/// Mark `slot` as traced when `label` holds a configured pattern. The label
/// table calls this when it gives a label its slot.
pub(crate) fn note_label(slot: u32, label: &str) {
    if let Some(patterns) = PATTERNS.get() {
        if patterns.iter().any(|pattern| label.contains(pattern.as_str())) {
            TRACED[slot as usize].store(true, Relaxed);
        }
    }
}

/// Called by the counting allocator for each allocation in `slot`.
#[inline]
pub(crate) fn on_alloc(slot: u32) {
    let every = EVERY.load(Relaxed);
    if every == 0 || !TRACED[slot as usize].load(Relaxed) {
        return;
    }
    if TICK.fetch_add(1, Relaxed) % every != 0 {
        return;
    }
    // The capture allocates. Inside the scope those allocations count to the
    // probe's slot, which is never traced, so this does not recurse.
    let _probe = ProbeScope::enter();
    let chain = chain_of(&Backtrace::force_capture().to_string());
    let mut chains = CHAINS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    *chains.get_or_insert_with(HashMap::new).entry(chain).or_insert(0) += 1;
}

/// The captured chains and their counts, most first, and the interval: one
/// capture stands for that many allocations.
pub fn snapshot() -> (Vec<(String, u64)>, u64) {
    let chains = CHAINS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut rows: Vec<(String, u64)> = chains
        .as_ref()
        .map(|chains| chains.iter().map(|(chain, count)| (chain.clone(), *count)).collect())
        .unwrap_or_default();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    (rows, EVERY.load(Relaxed))
}

/// The function names of a printed backtrace, innermost first, joined by
/// ` <- `. The chain starts at the first frame with a path (an inlined frame
/// prints its bare name) that is not the allocator, the probe or the standard
/// library; after it, every frame but those is kept.
fn chain_of(printed: &str) -> String {
    let skipped = [
        "std::backtrace",
        "ambition_dev_tools::perf_probe",
        "<ambition_dev_tools::perf_probe",
        "__rustc",
        "__rust_",
        "alloc::",
        "<alloc::",
        "core::",
        "<core::",
        "std::",
        "<std::",
        "hashbrown::raw",
    ];
    let mut frames = Vec::with_capacity(CHAIN_FRAMES);
    for line in printed.lines() {
        let line = line.trim_start();
        // `N: function` lines; `at file:line` lines follow them.
        let Some((index, function)) = line.split_once(": ") else {
            continue;
        };
        if index.is_empty() || !index.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        if skipped.iter().any(|prefix| function.starts_with(prefix)) {
            continue;
        }
        if frames.is_empty() && !without_generics(function).contains("::") {
            continue;
        }
        frames.push(strip_hash(function));
        if frames.len() == CHAIN_FRAMES {
            break;
        }
    }
    frames.join(" <- ")
}

/// `name<T, U>` as `name`, at every depth of `<…>`.
fn without_generics(function: &str) -> String {
    let mut depth = 0usize;
    function
        .chars()
        .filter(|c| match c {
            '<' => {
                depth += 1;
                false
            }
            '>' => {
                depth = depth.saturating_sub(1);
                false
            }
            _ => depth == 0,
        })
        .collect()
}

/// `path::name::h0123456789abcdef` without its hash.
fn strip_hash(function: &str) -> &str {
    match function.rsplit_once("::h") {
        Some((name, hash)) if hash.len() == 16 && hash.bytes().all(|b| b.is_ascii_hexdigit()) => name,
        _ => function,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chain_keeps_the_callers_and_drops_the_allocator() {
        let printed = "   0: std::backtrace::Backtrace::force_capture::h0123456789abcdef\n\
                       \x20            at /rust/library/std/src/backtrace.rs:1\n\
                       \x20  1: ambition_dev_tools::perf_probe::traces::on_alloc::h0123456789abcdef\n\
                       \x20  2: alloc::raw_vec::RawVec::grow::h0123456789abcdef\n\
                       \x20  2: finish_grow<alloc::alloc::Global>\n\
                       \x20  3: bevy_ecs::world::command_queue::RawCommandQueue::apply::h0123456789abcdef\n\
                       \x20  4: my_game::systems::spawn_things::h0123456789abcdef\n";
        assert_eq!(
            chain_of(printed),
            "bevy_ecs::world::command_queue::RawCommandQueue::apply <- my_game::systems::spawn_things"
        );
    }
}
