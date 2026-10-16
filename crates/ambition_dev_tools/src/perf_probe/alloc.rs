//! Allocation counts per tracing span.
//!
//! [`CountingAlloc`] wraps the allocator of the binary. Each allocation adds
//! to the counters of the innermost span that is entered on the current
//! thread. [`SpanStackLayer`] keeps that per-thread span stack. Bevy's `trace`
//! feature puts each system in a span, so each system gets a row.
//!
//! The allocator path takes no lock and does not allocate: it reads a
//! constant-initialized thread local and does three relaxed atomic adds.

use std::alloc::{GlobalAlloc, Layout};
use std::cell::Cell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::Mutex;

use bevy::log::tracing::span::{Attributes, Id};
use bevy::log::tracing::{field, Subscriber};
use bevy::log::tracing_subscriber::layer::{Context, Layer};
use bevy::log::tracing_subscriber::registry::LookupSpan;

/// Slot for allocations made outside every span.
pub const NO_SPAN: u32 = 0;
/// Slot for the probe's own work (span labels, reports, churn counts).
pub const PROBE: u32 = 1;
/// Slot for spans after [`MAX_SLOTS`] labels are in use.
pub const OVERFLOW: u32 = 2;
const FIRST_LABEL: u32 = 3;
/// Distinct span labels the probe can keep apart.
/// The hub app makes more than 8192 (each system has two spans). The table is
/// zeroed static memory, so unused slots cost no pages.
pub const MAX_SLOTS: usize = 65536;
const STACK_DEPTH: usize = 32;

/// One cache line per slot, so two threads in two systems do not share a line.
#[repr(align(64))]
struct SlotCounters {
    calls: AtomicU64,
    allocs: AtomicU64,
    bytes: AtomicU64,
}

impl SlotCounters {
    const fn new() -> Self {
        Self {
            calls: AtomicU64::new(0),
            allocs: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
        }
    }
}

static COUNTERS: [SlotCounters; MAX_SLOTS] = [const { SlotCounters::new() }; MAX_SLOTS];

/// The per-thread span stack. All fields are `Cell<u32>` with a constant
/// initializer, so the thread local has no destructor and no lazy setup. The
/// allocator can read it at any time, also while a thread starts or stops.
struct ThreadState {
    depth: Cell<u32>,
    probe: Cell<u32>,
    stack: [Cell<u32>; STACK_DEPTH],
}

thread_local! {
    static STATE: ThreadState = const {
        ThreadState {
            depth: Cell::new(0),
            probe: Cell::new(0),
            stack: [const { Cell::new(NO_SPAN) }; STACK_DEPTH],
        }
    };
}

#[inline]
fn current_slot() -> u32 {
    STATE
        .try_with(|state| {
            if state.probe.get() > 0 {
                return PROBE;
            }
            match state.depth.get() as usize {
                0 => NO_SPAN,
                depth => state.stack[depth.min(STACK_DEPTH) - 1].get(),
            }
        })
        .unwrap_or(NO_SPAN)
}

#[inline]
fn record(bytes: usize) {
    let slot = current_slot();
    let counters = &COUNTERS[slot as usize];
    counters.allocs.fetch_add(1, Relaxed);
    counters.bytes.fetch_add(bytes as u64, Relaxed);
    super::traces::on_alloc(slot);
}

/// A global allocator that counts each allocation into the active span.
///
/// Declare it in the crate that owns the binary's `#[global_allocator]`, around
/// the allocator that the binary uses without the probe:
///
/// ```ignore
/// #[cfg(feature = "perf_probe")]
/// #[global_allocator]
/// static GLOBAL: CountingAlloc<std::alloc::System> = CountingAlloc::new(std::alloc::System);
/// ```
///
/// `realloc` counts as one allocation of the new size. `dealloc` is not counted.
pub struct CountingAlloc<A>(A);

impl<A> CountingAlloc<A> {
    pub const fn new(inner: A) -> Self {
        Self(inner)
    }
}

// SAFETY: every call goes to the inner allocator with the same arguments.
// `record` does not allocate and does not touch the returned memory.
unsafe impl<A: GlobalAlloc> GlobalAlloc for CountingAlloc<A> {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        unsafe { self.0.alloc(layout) }
    }

    #[inline]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        unsafe { self.0.alloc_zeroed(layout) }
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { self.0.dealloc(ptr, layout) }
    }

    #[inline]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record(new_size);
        unsafe { self.0.realloc(ptr, layout, new_size) }
    }
}

/// While this guard lives, allocations on this thread go to the [`PROBE`] slot.
/// Use it around probe work, so that the probe does not inflate the counts of
/// the span it runs in.
pub struct ProbeScope(());

impl ProbeScope {
    pub fn enter() -> Self {
        let _ = STATE.try_with(|state| state.probe.set(state.probe.get() + 1));
        Self(())
    }
}

impl Drop for ProbeScope {
    fn drop(&mut self) {
        let _ = STATE.try_with(|state| state.probe.set(state.probe.get().saturating_sub(1)));
    }
}

struct Labels {
    slot_of: HashMap<String, u32>,
    /// Slots of spans with no `name` field, by callsite. A span that is made
    /// each frame then costs no label format.
    slot_of_callsite: HashMap<usize, u32>,
    names: Vec<String>,
}

static LABELS: Mutex<Option<Labels>> = Mutex::new(None);

fn with_labels<R>(f: impl FnOnce(&mut Labels) -> R) -> R {
    let mut guard = LABELS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let labels = guard.get_or_insert_with(|| Labels {
        slot_of: HashMap::new(),
        slot_of_callsite: HashMap::new(),
        names: ["(no span)", "(perf_probe)", "(label overflow)"]
            .map(String::from)
            .to_vec(),
    });
    f(labels)
}

/// The slot for `label`. The same label always gives the same slot.
pub fn intern(label: &str) -> u32 {
    with_labels(|labels| labels.intern(label))
}

impl Labels {
    fn intern(&mut self, label: &str) -> u32 {
        if let Some(slot) = self.slot_of.get(label) {
            return *slot;
        }
        let slot = self.names.len() as u32;
        if slot as usize >= MAX_SLOTS {
            return OVERFLOW;
        }
        debug_assert!(slot >= FIRST_LABEL);
        self.names.push(label.to_owned());
        self.slot_of.insert(label.to_owned(), slot);
        super::traces::note_label(slot, label);
        slot
    }
}

/// Totals for one span label since the process started.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpanTotals {
    pub label: String,
    pub calls: u64,
    pub allocs: u64,
    pub bytes: u64,
}

/// Read the counters of every label in use, indexed by slot.
pub fn snapshot() -> Vec<SpanTotals> {
    with_labels(|labels| {
        labels
            .names
            .iter()
            .zip(COUNTERS.iter())
            .map(|(label, counters)| SpanTotals {
                label: label.clone(),
                calls: counters.calls.load(Relaxed),
                allocs: counters.allocs.load(Relaxed),
                bytes: counters.bytes.load(Relaxed),
            })
            .collect()
    })
}

/// The row name of a span.
///
/// A Bevy system span is `"system"` with a `name` field: its row is the system
/// name. Every other span is `"<crate>: <span name>"`, plus the `name` field
/// when it has one (for example `bevy_ecs: schedule Update`).
fn span_label(target: &str, name: &str, name_field: Option<&str>) -> String {
    match (name, name_field) {
        ("system", Some(system)) => system.to_owned(),
        ("system_commands", Some(system)) => format!("{system} (commands)"),
        (_, field) => {
            let krate = target.split("::").next().unwrap_or(target);
            match field {
                Some(field) => format!("{krate}: {name} {field}"),
                None => format!("{krate}: {name}"),
            }
        }
    }
}

#[derive(Default)]
struct NameField(Option<String>);

impl field::Visit for NameField {
    fn record_str(&mut self, field: &field::Field, value: &str) {
        if field.name() == "name" {
            self.0 = Some(value.to_owned());
        }
    }

    fn record_debug(&mut self, field: &field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "name" {
            self.0 = Some(format!("{value:?}"));
        }
    }
}

#[derive(Clone, Copy)]
struct SpanSlot(u32);

/// The `tracing` layer that keeps the per-thread span stack for
/// [`CountingAlloc`]. Add it with [`super::log_layer`] as a `LogPlugin`
/// `custom_layer`. It composes with the fmt and Tracy layers of the plugin.
pub struct SpanStackLayer;

impl<S> Layer<S> for SpanStackLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let _probe = ProbeScope::enter();
        let meta = attrs.metadata();
        let mut name_field = NameField::default();
        attrs.record(&mut name_field);
        let slot = match name_field.0.as_deref() {
            Some(field) => intern(&span_label(meta.target(), meta.name(), Some(field))),
            None => with_labels(|labels| {
                let callsite = std::ptr::from_ref(meta) as usize;
                if let Some(slot) = labels.slot_of_callsite.get(&callsite) {
                    return *slot;
                }
                let slot = labels.intern(&span_label(meta.target(), meta.name(), None));
                labels.slot_of_callsite.insert(callsite, slot);
                slot
            }),
        };
        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(SpanSlot(slot));
        }
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        let slot = ctx
            .span(id)
            .and_then(|span| span.extensions().get::<SpanSlot>().copied())
            .map_or(NO_SPAN, |slot| slot.0);
        COUNTERS[slot as usize].calls.fetch_add(1, Relaxed);
        let _ = STATE.try_with(|state| {
            let depth = state.depth.get() as usize;
            if depth < STACK_DEPTH {
                state.stack[depth].set(slot);
            }
            state.depth.set(depth as u32 + 1);
        });
    }

    fn on_exit(&self, _id: &Id, _ctx: Context<'_, S>) {
        let _ = STATE.try_with(|state| state.depth.set(state.depth.get().saturating_sub(1)));
    }
}

