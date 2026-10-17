//! Profiling-only probe: which span allocates, and which code changes archetypes.
//!
//! Exists only with the `perf_probe` cargo feature. Reports only when
//! `AMBITION_PROFILE_CENSUS` is set, on the census clock.
//!
//! * Allocations: [`CountingAlloc`] counts each allocation into the innermost
//!   entered `tracing` span on its thread. Each Bevy system is a span (Bevy
//!   `trace`), and so is each `info_span!` in our code.
//! * Churn: observers count each spawn, despawn, component add and component
//!   remove by the source location that asked for it (Bevy `track_location`).
//! * Changes: once a frame, each component value (resources included) that was
//!   changed and not added, by the source location of the last change. A `&mut`
//!   that writes nothing still counts: it wakes each `Changed<T>` reader.
//!
//! Each sample prints `[census] alloc`, `[census] churn` and `[census] changes`
//! for the last window. When `AMBITION_PERF_PROBE_DIR` is set, each sample also
//! writes `alloc_by_span.csv`, `time_by_span.csv`, `churn_by_site.csv` and
//! `changes_by_site.csv` there: totals from the first sample after `AMBITION_PERF_PROBE_SKIP_FRAMES`
//! frames, ranked.
//!
//! A binary needs two lines to feed the probe: a [`CountingAlloc`] global
//! allocator, and [`log_layer`] as the `custom_layer` of its `LogPlugin`.
//! See `docs/recipes/profiling.md`.
//!
//! Do not read frame times from a `perf_probe` build: the counters and the
//! observers add cost to every allocation and every archetype move.

pub mod alloc;
pub mod changes;
pub mod churn;
pub mod traces;

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use bevy::ecs::component::Components;
use bevy::prelude::*;

pub use alloc::{CountingAlloc, ProbeScope, SpanStackLayer, SpanTotals};
pub use changes::{ChangeCounts, ChangeKey};
pub use churn::{ChurnCounts, ChurnEvent, ChurnKey};

use crate::runtime_census::RuntimeCensus;

/// Directory for the two CSV files. Unset: stderr rows only.
pub const DIR_ENV: &str = "AMBITION_PERF_PROBE_DIR";
/// Frames to leave out of the CSV totals, for example loading. Default 0.
pub const SKIP_ENV: &str = "AMBITION_PERF_PROBE_SKIP_FRAMES";
/// Rows in each stderr top list.
const TOP: usize = 10;

/// The `LogPlugin::custom_layer` that feeds [`CountingAlloc`] the span stack.
///
/// It also reads [`traces::TRACE_ENV`]: the log plugin builds before the first
/// span, so each span label is checked against the patterns.
pub fn log_layer(_app: &mut App) -> Option<bevy::log::BoxedLayer> {
    if let Some((patterns, every)) = traces::configure_from_env() {
        eprintln!("[census] config t=0.000 perf_probe_trace={} every={every}", patterns.join("|"));
    }
    Some(Box::new(SpanStackLayer))
}

#[derive(Clone, Default)]
struct Mark {
    frames: u64,
    spans: Vec<SpanTotals>,
    churn: HashMap<ChurnKey, u64>,
    changes: HashMap<ChangeKey, u64>,
}

#[derive(Resource)]
struct ProbeState {
    dir: Option<PathBuf>,
    skip_frames: u64,
    frames: u64,
    last: Option<Mark>,
    baseline: Option<Mark>,
}

/// Count frames, write the reports, and arm the churn observers.
/// `RuntimeCensusPlugin` adds it when the census is on.
pub struct PerfProbePlugin;

impl Plugin for PerfProbePlugin {
    fn build(&self, app: &mut App) {
        let dir = std::env::var_os(DIR_ENV).map(PathBuf::from);
        if let Some(dir) = &dir {
            if let Err(error) = std::fs::create_dir_all(dir) {
                eprintln!("[census] perf_probe cannot create {}: {error}", dir.display());
            }
        }
        let skip_frames = std::env::var(SKIP_ENV)
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        eprintln!(
            "[census] config t=0.000 perf_probe=on dir={} skip_frames={skip_frames}",
            dir.as_ref()
                .map_or_else(|| format!("(unset; set {DIR_ENV} for CSV files)"), |d| d.display().to_string())
        );
        app.insert_resource(ProbeState {
            dir,
            skip_frames,
            frames: 0,
            last: None,
            baseline: None,
        });
        churn::install(app);
        changes::install(app);
        app.add_systems(First, count_frame);
        app.add_systems(Last, report_perf_probe);
    }
}

fn count_frame(mut state: ResMut<ProbeState>) {
    state.frames += 1;
}

fn report_perf_probe(
    census: Res<RuntimeCensus>,
    mut state: ResMut<ProbeState>,
    churn: Res<ChurnCounts>,
    changes: Res<ChangeCounts>,
    components: &Components,
    mut exits: MessageReader<AppExit>,
) {
    let exiting = exits.read().next().is_some();
    let due = census.due();
    if due.is_none() && !exiting {
        return;
    }
    let _probe = ProbeScope::enter();
    let now = Mark {
        frames: state.frames,
        spans: alloc::snapshot(),
        churn: churn.counts.clone(),
        changes: changes.counts.clone(),
    };
    if let (Some(at), Some(last)) = (due, &state.last) {
        print_window(at, &now, last, &churn, components);
    }
    if state.baseline.is_none() && due.is_some() && state.frames >= state.skip_frames {
        state.baseline = Some(now.clone());
    }
    if let (Some(dir), Some(baseline)) = (&state.dir, &state.baseline) {
        write_csvs(dir, &now, baseline, &churn, components);
    }
    if due.is_some() {
        state.last = Some(now);
    }
}

fn span_delta(now: &[SpanTotals], then: &[SpanTotals]) -> Vec<SpanTotals> {
    let zero = SpanTotals::default();
    let mut rows: Vec<SpanTotals> = now
        .iter()
        .enumerate()
        .map(|(slot, row)| {
            let before = then.get(slot).unwrap_or(&zero);
            SpanTotals {
                label: row.label.clone(),
                calls: row.calls.saturating_sub(before.calls),
                allocs: row.allocs.saturating_sub(before.allocs),
                bytes: row.bytes.saturating_sub(before.bytes),
                nanos: row.nanos.saturating_sub(before.nanos),
            }
        })
        .filter(|row| row.allocs > 0)
        .collect();
    rows.sort_by(|a, b| b.allocs.cmp(&a.allocs).then_with(|| a.label.cmp(&b.label)));
    rows
}

/// The spans that ran in the window, the dearest first.
fn time_delta(now: &[SpanTotals], then: &[SpanTotals]) -> Vec<(String, u64, u64)> {
    let mut rows: Vec<(String, u64, u64)> = now
        .iter()
        .enumerate()
        .map(|(slot, row)| {
            let before = then.get(slot);
            (
                row.label.clone(),
                row.calls.saturating_sub(before.map_or(0, |b| b.calls)),
                row.nanos.saturating_sub(before.map_or(0, |b| b.nanos)),
            )
        })
        .filter(|(_, calls, _)| *calls > 0)
        .collect();
    rows.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
    rows
}

fn churn_delta(now: &HashMap<ChurnKey, u64>, then: &HashMap<ChurnKey, u64>) -> Vec<(ChurnKey, u64)> {
    let mut rows: Vec<(ChurnKey, u64)> = now
        .iter()
        .map(|(key, count)| (*key, count.saturating_sub(then.get(key).copied().unwrap_or(0))))
        .filter(|(_, count)| *count > 0)
        .collect();
    rows.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| a.0.event.cmp(&b.0.event))
            .then_with(|| churn::site_label(a.0.site).cmp(&churn::site_label(b.0.site)))
    });
    rows
}

fn changes_delta(now: &HashMap<ChangeKey, u64>, then: &HashMap<ChangeKey, u64>) -> Vec<(ChangeKey, u64)> {
    let mut rows: Vec<(ChangeKey, u64)> = now
        .iter()
        .map(|(key, count)| (*key, count.saturating_sub(then.get(key).copied().unwrap_or(0))))
        .filter(|(_, count)| *count > 0)
        .collect();
    rows.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| a.0 .0.index().cmp(&b.0 .0.index()))
            .then_with(|| churn::site_label(a.0 .1).cmp(&churn::site_label(b.0 .1)))
    });
    rows
}

fn component_label(components: &Components, id: bevy::ecs::component::ComponentId) -> String {
    churn::component_set_label(components, &[id], 1)
}

fn per_frame(count: u64, frames: u64) -> f64 {
    count as f64 / frames.max(1) as f64
}

fn print_window(at: f64, now: &Mark, last: &Mark, churn: &ChurnCounts, components: &Components) {
    let frames = now.frames.saturating_sub(last.frames);
    let spans = span_delta(&now.spans, &last.spans);
    let allocs: u64 = spans.iter().map(|row| row.allocs).sum();
    let bytes: u64 = spans.iter().map(|row| row.bytes).sum();
    let mut top = String::new();
    for row in spans.iter().take(TOP) {
        let _ = write!(
            top,
            "{}{} {:.1}/f {:.0}B/f",
            if top.is_empty() { "" } else { " | " },
            row.label.replace('"', "'"),
            per_frame(row.allocs, frames),
            per_frame(row.bytes, frames),
        );
    }
    eprintln!(
        "[census] alloc t={at:.3} frames={frames} allocs_per_frame={:.1} bytes_per_frame={:.0} \
         labels={} counted={} top=\"{top}\"",
        per_frame(allocs, frames),
        per_frame(bytes, frames),
        now.spans.len(),
        // Zero allocations means the binary has no `CountingAlloc`.
        if allocs > 0 { "yes" } else { "no" },
    );

    let rows = churn_delta(&now.churn, &last.churn);
    let mut by_event = [0u64; 4];
    for (key, count) in &rows {
        by_event[key.event as usize] += count;
    }
    let mut top = String::new();
    for (key, count) in rows.iter().take(TOP) {
        let _ = write!(
            top,
            "{}{} {} @ {} {:.2}/f",
            if top.is_empty() { "" } else { " | " },
            key.event.as_str(),
            churn::component_set_label(components, churn.components(key.components), 3),
            churn::site_label(key.site),
            per_frame(*count, frames),
        );
    }
    eprintln!(
        "[census] churn t={at:.3} frames={frames} spawn_per_frame={:.2} insert_per_frame={:.2} \
         remove_per_frame={:.2} despawn_per_frame={:.2} top=\"{}\"",
        per_frame(by_event[0], frames),
        per_frame(by_event[1], frames),
        per_frame(by_event[2], frames),
        per_frame(by_event[3], frames),
        top.replace('"', "'"),
    );

    let rows = changes_delta(&now.changes, &last.changes);
    let total: u64 = rows.iter().map(|(_, count)| count).sum();
    let mut top = String::new();
    for ((id, site), count) in rows.iter().take(TOP) {
        let _ = write!(
            top,
            "{}{} @ {} {:.2}/f",
            if top.is_empty() { "" } else { " | " },
            component_label(components, *id),
            churn::site_label(*site),
            per_frame(*count, frames),
        );
    }
    eprintln!(
        "[census] changes t={at:.3} frames={frames} changed_per_frame={:.1} top=\"{}\"",
        per_frame(total, frames),
        top.replace('"', "'"),
    );
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn write_atomically(path: PathBuf, text: &str) {
    let partial = path.with_extension("csv.partial");
    let result = std::fs::write(&partial, text).and_then(|()| std::fs::rename(&partial, &path));
    if let Err(error) = result {
        eprintln!("[census] perf_probe cannot write {}: {error}", path.display());
    }
}

fn write_csvs(
    dir: &std::path::Path,
    now: &Mark,
    baseline: &Mark,
    churn: &ChurnCounts,
    components: &Components,
) {
    let frames = now.frames.saturating_sub(baseline.frames);
    let mut text = String::from(
        "span,calls,allocs,bytes,allocs_per_frame,bytes_per_frame,allocs_per_call,frames\n",
    );
    for row in span_delta(&now.spans, &baseline.spans) {
        let _ = writeln!(
            text,
            "{},{},{},{},{:.2},{:.0},{:.2},{frames}",
            csv_field(&row.label),
            row.calls,
            row.allocs,
            row.bytes,
            per_frame(row.allocs, frames),
            per_frame(row.bytes, frames),
            // Per run of the span. Use it for a fixed-step system: a slow frame
            // runs more fixed steps, so its per-frame count grows.
            per_frame(row.allocs, row.calls),
        );
    }
    write_atomically(dir.join("alloc_by_span.csv"), &text);

    // Wall time, nested spans included, so a schedule's row holds its systems'
    // rows. The probe's counters slow each allocation: read the ranks and the
    // ratios, not the microseconds.
    let mut text = String::from("span,calls,us_per_frame,us_per_call,frames\n");
    for (label, calls, nanos) in time_delta(&now.spans, &baseline.spans) {
        let _ = writeln!(
            text,
            "{},{calls},{:.2},{:.3},{frames}",
            csv_field(&label),
            per_frame(nanos, frames) / 1000.0,
            per_frame(nanos, calls) / 1000.0,
        );
    }
    write_atomically(dir.join("time_by_span.csv"), &text);

    let mut text = String::from("event,components,site,count,per_frame,frames\n");
    for (key, count) in churn_delta(&now.churn, &baseline.churn) {
        let _ = writeln!(
            text,
            "{},{},{},{count},{:.3},{frames}",
            key.event.as_str(),
            csv_field(&churn::component_set_label(components, churn.components(key.components), 6)),
            csv_field(&churn::site_label(key.site)),
            per_frame(count, frames),
        );
    }
    write_atomically(dir.join("churn_by_site.csv"), &text);

    let mut text = String::from("component,site,count,per_frame,frames\n");
    for ((id, site), count) in changes_delta(&now.changes, &baseline.changes) {
        let _ = writeln!(
            text,
            "{},{},{count},{:.3},{frames}",
            csv_field(&component_label(components, id)),
            csv_field(&churn::site_label(site)),
            per_frame(count, frames),
        );
    }
    write_atomically(dir.join("changes_by_site.csv"), &text);

    // Whole run, not from the baseline: a capture is rare, and it stands for
    // `every` allocations.
    let (chains, every) = traces::snapshot();
    if !chains.is_empty() {
        let frames = now.frames.max(1);
        let mut text = String::from("captures,est_allocs_per_frame,chain\n");
        for (chain, count) in chains {
            let _ = writeln!(
                text,
                "{count},{:.1},{}",
                (count * every) as f64 / frames as f64,
                csv_field(&chain)
            );
        }
        write_atomically(dir.join("alloc_traces.csv"), &text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::log::tracing;
    use bevy::log::tracing_subscriber::layer::SubscriberExt;
    use bevy::log::tracing_subscriber::Registry;

    #[global_allocator]
    static GLOBAL: CountingAlloc<std::alloc::System> = CountingAlloc::new(std::alloc::System);

    #[derive(Component)]
    struct Body;

    #[derive(Component)]
    struct Marker;

    #[test]
    fn an_allocation_counts_to_its_span_and_an_insert_to_its_caller() {
        let subscriber = Registry::default().with(SpanStackLayer);
        tracing::subscriber::with_default(subscriber, || {
            let _span = tracing::info_span!("perf_probe_test_span").entered();
            std::hint::black_box(Vec::<u8>::with_capacity(4096));
        });
        let row = alloc::snapshot()
            .into_iter()
            .find(|row| row.label == "ambition_dev_tools: perf_probe_test_span")
            .expect("the span has a row");
        assert_eq!(row.calls, 1);
        assert!(row.allocs >= 1 && row.bytes >= 4096, "{row:?}");

        let mut app = App::new();
        churn::install(&mut app);
        app.world_mut().register_component::<Body>();
        app.world_mut().register_component::<Marker>();
        app.update();
        let entity = app.world_mut().spawn(Body).id();
        let spawn_line = line!() - 1;
        app.world_mut().entity_mut(entity).insert(Marker);
        let insert_line = line!() - 1;

        let counts = app.world().resource::<ChurnCounts>();
        let at = |event: ChurnEvent, line: u32| {
            counts
                .counts
                .iter()
                .filter(|(key, _)| {
                    key.event == event && key.site.is_some_and(|s| s.file() == file!() && s.line() == line)
                })
                .map(|(_, count)| *count)
                .sum::<u64>()
        };
        assert_eq!(at(ChurnEvent::Spawn, spawn_line), 1);
        assert_eq!(at(ChurnEvent::Insert, insert_line), 1);
    }
}
