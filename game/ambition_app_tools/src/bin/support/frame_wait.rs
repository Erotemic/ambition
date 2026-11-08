//! Windowed Smash profiling: measure the gap between main-schedule executions.
//!
//! The gap contains render-subapp work, OS event dispatch, frame pacing, and
//! presentation waits. These marks do not assign it to any one of them.
//! The CPU clock is for the main thread only; render and task-pool threads
//! have separate clocks. This probe is opt-in for native profiling.

use bevy::app::MainScheduleOrder;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::time::{Duration, Instant};

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct BeforeMain;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct AfterMain;

#[derive(Resource, Default)]
struct FrameWaitProbe {
    started_at: Option<Instant>,
    epoch: Option<Instant>,
    last_end: Option<(Instant, Option<Duration>, Option<bool>)>,
    last_focus: Option<bool>,
    focused_frames: u32,
    unfocused_frames: u32,
    unknown_focus_frames: u32,
    focus_transitions: u32,
    focused_gaps: u32,
    unfocused_gaps: u32,
    unknown_focus_gaps: u32,
    focused_gap_ms: f64,
    unfocused_gap_ms: f64,
    begin: Option<(Instant, Option<Duration>)>,
    main_total_ms: f64,
    main_cpu_ms: f64,
    gap_total_ms: f64,
    gap_cpu_ms: f64,
    main_count: u32,
    gaps_ms: Vec<f64>,
    gaps_over_10ms: u32,
}

/// Return CPU time for this thread, when the OS supports it.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn main_thread_cpu() -> Option<Duration> {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    // SAFETY: `ts` is initialized and is writable for this call.
    let result = unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut ts) };
    if result != 0 {
        return None;
    }
    Some(Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32))
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn main_thread_cpu() -> Option<Duration> {
    None
}

fn cpu_delta_ms(earlier: Option<Duration>, later: Option<Duration>) -> f64 {
    match (earlier, later) {
        (Some(earlier), Some(later)) => later.saturating_sub(earlier).as_secs_f64() * 1000.0,
        _ => 0.0,
    }
}

fn begin_frame(
    mut probe: ResMut<FrameWaitProbe>,
    driver: Option<Res<super::MatchDriver>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    if !driver.is_some_and(|driver| matches!(driver.stage, super::Stage::Live(_))) {
        probe.last_end = None;
        probe.begin = None;
        return;
    }
    let now = Instant::now();
    let cpu = main_thread_cpu();
    if probe.started_at.is_none() {
        probe.started_at = Some(now);
    }
    if probe.epoch.is_none() {
        probe.epoch = Some(now);
    }
    let focused = windows.iter().next().map(|window| window.focused);
    if let (Some(before), Some(after)) = (probe.last_focus, focused) {
        if before != after {
            probe.focus_transitions += 1;
            eprintln!(
                "[frame-focus] t={:.3} focused={after}",
                now.duration_since(probe.epoch.expect("epoch is set above")).as_secs_f64(),
            );
        }
    }
    probe.last_focus = focused;
    match focused {
        Some(true) => probe.focused_frames += 1,
        Some(false) => probe.unfocused_frames += 1,
        None => probe.unknown_focus_frames += 1,
    }
    if let Some((last_wall, last_cpu, previous_focus)) = probe.last_end.take() {
        let gap_ms = now.duration_since(last_wall).as_secs_f64() * 1000.0;
        probe.gap_total_ms += gap_ms;
        probe.gap_cpu_ms += cpu_delta_ms(last_cpu, cpu);
        probe.gaps_over_10ms += u32::from(gap_ms >= 10.0);
        probe.gaps_ms.push(gap_ms);
        // A gap that touches an unfocused sample is not a focused frame.
        // It can include event-loop throttling independent of simulation.
        match (previous_focus, focused) {
            (Some(true), Some(true)) => {
                probe.focused_gaps += 1;
                probe.focused_gap_ms += gap_ms;
            }
            (Some(false), _) | (_, Some(false)) => {
                probe.unfocused_gaps += 1;
                probe.unfocused_gap_ms += gap_ms;
            }
            _ => probe.unknown_focus_gaps += 1,
        }
    }
    probe.begin = Some((now, cpu));
}

fn end_frame(mut probe: ResMut<FrameWaitProbe>) {
    let Some((start, start_cpu)) = probe.begin.take() else {
        return;
    };
    let now = Instant::now();
    let cpu = main_thread_cpu();
    probe.main_total_ms += now.duration_since(start).as_secs_f64() * 1000.0;
    probe.main_cpu_ms += cpu_delta_ms(start_cpu, cpu);
    probe.main_count += 1;
    let focused = probe.last_focus;
    probe.last_end = Some((now, cpu, focused));
    let started_at = probe.started_at.expect("begin_frame sets the timestamp");
    if now.duration_since(started_at) < Duration::from_secs(1) {
        return;
    }
    let main_count = f64::from(probe.main_count);
    let gap_count = probe.gaps_ms.len();
    let gap_mean = if gap_count > 0 { probe.gap_total_ms / gap_count as f64 } else { 0.0 };
    let gap_cpu = if gap_count > 0 { probe.gap_cpu_ms / gap_count as f64 } else { 0.0 };
    probe.gaps_ms.sort_by(f64::total_cmp);
    let p95_index = (gap_count.saturating_mul(95).saturating_add(99) / 100)
        .saturating_sub(1);
    let p95 = probe.gaps_ms.get(p95_index).copied().unwrap_or(0.0);
    eprintln!(
        "[frame-wait] t={:.3} frames={} gaps={} main_wall_ms={:.4} main_cpu_ms={:.4} gap_wall_ms={:.4} gap_cpu_ms={:.4} gap_p95_ms={:.4} gap_over_10ms={} cpu_clock={} focused_frames={} unfocused_frames={} unknown_focus_frames={} focus_transitions={} focused_gaps={} unfocused_gaps={} unknown_focus_gaps={} focused_gap_ms={:.4} unfocused_gap_ms={:.4}",
        now.duration_since(probe.epoch.expect("begin_frame sets the epoch")).as_secs_f64(),
        probe.main_count,
        gap_count,
        probe.main_total_ms / main_count,
        probe.main_cpu_ms / main_count,
        gap_mean,
        gap_cpu,
        p95,
        probe.gaps_over_10ms,
        if cpu.is_some() { "thread" } else { "unavailable" },
        probe.focused_frames,
        probe.unfocused_frames,
        probe.unknown_focus_frames,
        probe.focus_transitions,
        probe.focused_gaps,
        probe.unfocused_gaps,
        probe.unknown_focus_gaps,
        if probe.focused_gaps > 0 { probe.focused_gap_ms / f64::from(probe.focused_gaps) } else { 0.0 },
        if probe.unfocused_gaps > 0 { probe.unfocused_gap_ms / f64::from(probe.unfocused_gaps) } else { 0.0 },
    );
    probe.started_at = Some(now);
    probe.main_total_ms = 0.0;
    probe.main_cpu_ms = 0.0;
    probe.gap_total_ms = 0.0;
    probe.gap_cpu_ms = 0.0;
    probe.main_count = 0;
    probe.gaps_over_10ms = 0;
    probe.gaps_ms.clear();
    probe.focused_frames = 0;
    probe.unfocused_frames = 0;
    probe.unknown_focus_frames = 0;
    probe.focus_transitions = 0;
    probe.focused_gaps = 0;
    probe.unfocused_gaps = 0;
    probe.unknown_focus_gaps = 0;
    probe.focused_gap_ms = 0.0;
    probe.unfocused_gap_ms = 0.0;
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<FrameWaitProbe>();
    app.add_systems(BeforeMain, begin_frame);
    app.add_systems(AfterMain, end_frame);
    // The first mark also samples the primary window focus state. A 60 Hz
    // background cap can otherwise look like expensive simulation work.
    // Schedule boundary marks have a known order. Systems inside First or Last
    // do not have this guarantee. Keep the existing schedule order intact.
    let mut order = app.world_mut().resource_mut::<MainScheduleOrder>();
    order.labels.insert(0, BeforeMain.intern());
    order.labels.push(AfterMain.intern());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_marks_bracket_all_main_schedules() {
        let mut app = App::new();
        app.init_resource::<MainScheduleOrder>();
        let prior = app.world().resource::<MainScheduleOrder>().labels.len();
        install(&mut app);
        let labels = &app.world().resource::<MainScheduleOrder>().labels;
        assert_eq!(labels.len(), prior + 2);
        assert_eq!(format!("{:?}", labels[0]), "BeforeMain");
        assert_eq!(format!("{:?}", labels[labels.len() - 1]), "AfterMain");
    }
}
