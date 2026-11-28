//! Measure a fixed interval of a capture simulation.

use std::path::PathBuf;
use std::time::Instant;

use ambition_platformer2d::time::SimTick;
use bevy::prelude::{App, AppExit, Last, MessageWriter, Res, ResMut, Resource};

#[derive(Resource)]
struct FrameTiming {
    start: u64,
    end: u64,
    output: PathBuf,
    previous: Option<(u64, Instant)>,
    samples_ms: Vec<f64>,
    finished: bool,
}

pub(super) fn install(app: &mut App, start: u64, end: u64, output: PathBuf) {
    app.insert_resource(FrameTiming {
        start,
        end,
        output,
        previous: None,
        samples_ms: Vec::new(),
        finished: false,
    })
    .add_systems(Last, record_frame);
}

fn record_frame(
    tick: Option<Res<SimTick>>,
    mut timing: ResMut<FrameTiming>,
    mut exits: MessageWriter<AppExit>,
) {
    let Some(tick) = tick.map(|tick| tick.get()) else {
        return;
    };
    if timing.finished || tick < timing.start {
        return;
    }
    let now = Instant::now();
    let expected = match timing.previous.replace((tick, now)) {
        Some((previous, then)) => {
            timing
                .samples_ms
                .push(now.duration_since(then).as_secs_f64() * 1000.0);
            previous + 1
        }
        None => timing.start,
    };
    if tick != expected {
        eprintln!("capture_scene timing expected tick {expected}, got {tick}");
        timing.finished = true;
        exits.write(AppExit::from_code(2));
        return;
    }
    if tick == timing.end {
        timing.finished = true;
        let result = serde_json::json!({
            "start_tick": timing.start,
            "end_tick": timing.end,
            "samples_ms": timing.samples_ms,
        });
        if let Err(error) = std::fs::write(&timing.output, result.to_string()) {
            eprintln!(
                "capture_scene could not write timing to {}: {error}",
                timing.output.display()
            );
            exits.write(AppExit::from_code(2));
        }
    }
}
