//! Replay a recorded `GameplayTraceBuffer` JSON dump through a fresh
//! `SandboxSim` and compare the resulting trajectory to the recorded
//! one.
//!
//! Use cases:
//!
//! - **Bug repro from production** — drop a `ambition_gameplay_trace_*.json`
//!   from a player's machine into the repo, run `cargo run --bin
//!   trace_replay -- path.json`, and watch where the live sim
//!   diverges from the recorded state. The first non-trivial
//!   position delta is usually adjacent to the bug.
//! - **Determinism validation** — after a refactor that should be
//!   behavior-preserving, replay an old trace; if every frame matches,
//!   the change is verified determinism-preserving.
//! - **Foundation for CI guardrails** — an in-tree fixture trace can
//!   become a regression test ("this 600-frame replay must match
//!   exactly" pins all the gameplay invariants in one shot).
//!
//! The replay drives the same fixed-60Hz timestep the determinism
//! test in `crates/ambition_gameplay_core/src/rl_sim.rs` uses, so live-sim
//! divergence on a deterministic-mode trace localizes a behavior
//! change. Wall-clock-recorded traces will diverge by construction
//! (the original wasn't deterministic) — the binary still prints the
//! divergence point so the tail of the trace can be visually
//! compared.
//!
//! Usage:
//!
//! ```bash
//! cargo run -p ambition_gameplay_core --bin trace_replay -- path/to/trace.json
//! cargo run -p ambition_gameplay_core --bin trace_replay -- path/to/trace.json --tolerance 0.5
//! ```
//!
//! The binary reads only the `frames[*].controls` array from the JSON
//! dump (plus `frames[*].player.pos` for divergence reporting). The
//! rest of the recorded state is informational; we don't try to
//! restore world/encounter snapshots, since the SandboxSim starts at
//! the canonical embedded LDtk world spawn.

use std::fs;
use std::path::PathBuf;

use ambition_app::rl_sim::TimestepMode;
use ambition_app::{AgentAction, SandboxSim};

#[derive(Debug, Default, Clone, Copy)]
struct RecordedControls {
    axis_x: f32,
    axis_y: f32,
    jump_pressed: bool,
    jump_held: bool,
    jump_released: bool,
    dash_pressed: bool,
    blink_pressed: bool,
    blink_held: bool,
    blink_released: bool,
    attack_pressed: bool,
    pogo_pressed: bool,
    fly_toggle_pressed: bool,
    interact_pressed: bool,
    reset_pressed: bool,
    start_pressed: bool,
}

#[derive(Debug, Clone, Copy)]
struct RecordedPos {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Copy)]
struct RecordedFrame {
    tick: u64,
    controls: RecordedControls,
    player_pos: RecordedPos,
}

impl From<RecordedControls> for AgentAction {
    fn from(c: RecordedControls) -> Self {
        AgentAction {
            move_x: c.axis_x,
            // Trace stores axis_y in sim convention (+Y = down) already.
            move_y: c.axis_y,
            up_pressed: false,
            down_pressed: false,
            jump: c.jump_pressed,
            jump_held: c.jump_held,
            jump_released: c.jump_released,
            dash: c.dash_pressed,
            attack: c.attack_pressed,
            blink: c.blink_pressed,
            blink_held: c.blink_held,
            blink_released: c.blink_released,
            pogo: c.pogo_pressed,
            interact: c.interact_pressed,
            projectile: false,
            projectile_held: false,
            projectile_released: false,
            fly_toggle: c.fly_toggle_pressed,
            reset: c.reset_pressed,
            start: c.start_pressed,
            aim_x: 0.0,
            aim_y: 0.0,
        }
    }
}

fn parse_trace_json(text: &str) -> Result<Vec<RecordedFrame>, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("invalid JSON: {e}"))?;
    let frames = value
        .get("frames")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "missing or non-array `frames`".to_string())?;
    let mut out = Vec::with_capacity(frames.len());
    for (i, frame) in frames.iter().enumerate() {
        let tick = frame
            .get("tick")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| format!("frame {i}: missing or non-integer `tick`"))?;
        let controls = frame
            .get("controls")
            .ok_or_else(|| format!("frame {i}: missing `controls`"))?;
        let pos_obj = frame
            .get("player")
            .and_then(|v| v.get("pos"))
            .ok_or_else(|| format!("frame {i}: missing `player.pos`"))?;
        out.push(RecordedFrame {
            tick,
            controls: RecordedControls {
                axis_x: f32_field(controls, "axis_x"),
                axis_y: f32_field(controls, "axis_y"),
                jump_pressed: bool_field(controls, "jump_pressed"),
                jump_held: bool_field(controls, "jump_held"),
                jump_released: bool_field(controls, "jump_released"),
                dash_pressed: bool_field(controls, "dash_pressed"),
                blink_pressed: bool_field(controls, "blink_pressed"),
                blink_held: bool_field(controls, "blink_held"),
                blink_released: bool_field(controls, "blink_released"),
                attack_pressed: bool_field(controls, "attack_pressed"),
                pogo_pressed: bool_field(controls, "pogo_pressed"),
                fly_toggle_pressed: bool_field(controls, "fly_toggle_pressed"),
                interact_pressed: bool_field(controls, "interact_pressed"),
                reset_pressed: bool_field(controls, "reset_pressed"),
                start_pressed: bool_field(controls, "start_pressed"),
            },
            player_pos: RecordedPos {
                x: f32_field(pos_obj, "x"),
                y: f32_field(pos_obj, "y"),
            },
        });
    }
    Ok(out)
}

fn f32_field(value: &serde_json::Value, key: &str) -> f32 {
    value.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32
}

fn bool_field(value: &serde_json::Value, key: &str) -> bool {
    value.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

fn replay(path: &PathBuf, tolerance: f32) -> Result<(), String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read {path:?}: {e}"))?;
    let frames = parse_trace_json(&text)?;
    println!(
        "trace_replay: loaded {} frames from {}",
        frames.len(),
        path.display()
    );
    if frames.is_empty() {
        return Err("trace contains zero frames".into());
    }

    let mut sim = SandboxSim::new_with_timestep(TimestepMode::fixed_60hz())
        .map_err(|e| format!("SandboxSim::new failed: {e}"))?;
    let pre_step_pos = sim.observation().player_pos;
    println!(
        "replay start: live pre-step pos=({:.1},{:.1})",
        pre_step_pos.0, pre_step_pos.1
    );
    println!(
        "first recorded frame: pos=({:.1},{:.1}) (this is state AFTER step 1 in the recorded run)",
        frames[0].player_pos.x, frames[0].player_pos.y
    );

    let mut max_dx: f32 = 0.0;
    let mut max_dy: f32 = 0.0;
    let mut first_divergence: Option<(usize, f32, f32)> = None;
    let mut diverged_frames = 0usize;

    // Trace dump convention: frames[i] is the state AFTER step i+1
    // (the dump records each frame after stepping, not before). So
    // the replay applies frames[i].controls and expects the
    // post-step `live.player_pos` to match `frames[i].player_pos`.
    // The off-by-one in the original implementation (skip(1)) was
    // applying the wrong controls to each step. Fix: loop from i=0
    // and align controls + position with the recorded shape.
    for (i, frame) in frames.iter().enumerate() {
        let action = AgentAction::from(frame.controls);
        let live = sim.step(action);
        let recorded = frame.player_pos;
        let dx = (live.player_pos.0 - recorded.x).abs();
        let dy = (live.player_pos.1 - recorded.y).abs();
        if dx > max_dx {
            max_dx = dx;
        }
        if dy > max_dy {
            max_dy = dy;
        }
        if dx + dy > tolerance {
            diverged_frames += 1;
            if first_divergence.is_none() {
                first_divergence = Some((i, dx, dy));
                println!(
                    "  [frame {:>5} tick={}] diverged: live=({:.2},{:.2}) recorded=({:.2},{:.2}) delta=({:.2},{:.2})",
                    i, frame.tick, live.player_pos.0, live.player_pos.1, recorded.x, recorded.y, dx, dy
                );
            }
        }
    }

    println!("--- replay complete ---");
    println!("frames replayed     : {}", frames.len() - 1);
    println!(
        "diverged frames     : {} (tolerance={tolerance})",
        diverged_frames
    );
    println!("max dx              : {:.3}", max_dx);
    println!("max dy              : {:.3}", max_dy);
    match first_divergence {
        Some((idx, dx, dy)) => {
            println!("first divergence    : frame {idx} (delta=({dx:.2},{dy:.2}))");
            println!(
                "exit status         : 1 (replay diverged) — investigate around frame {idx} in source trace"
            );
            std::process::exit(1);
        }
        None => {
            println!("first divergence    : none (replay matches within tolerance)");
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: trace_replay <path.json> [--tolerance VAL]");
        std::process::exit(2);
    }
    let path = PathBuf::from(&args[1]);
    let mut tolerance: f32 = 0.001;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--tolerance" => {
                if let Some(raw) = args.get(i + 1) {
                    if let Ok(v) = raw.parse() {
                        tolerance = v;
                    }
                }
                i += 2;
            }
            other if other.starts_with("--tolerance=") => {
                if let Ok(v) = other.trim_start_matches("--tolerance=").parse() {
                    tolerance = v;
                }
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
    if let Err(e) = replay(&path, tolerance) {
        eprintln!("trace_replay: {e}");
        std::process::exit(1);
    }
}
