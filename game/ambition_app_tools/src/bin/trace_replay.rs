//! Replay a `GameplayTraceBuffer` JSON dump through a fresh
//! `Platformer2dSimHarness` and report trajectory divergence.
//!
//! The replay consumes recorded controls and player positions; other trace state
//! remains informational because the harness starts from the canonical embedded
//! world. Wall-clock traces may diverge, while deterministic traces can serve as
//! behavior-preservation checks.

use std::fs;
use std::path::PathBuf;

use ambition_app::rl_sim::TimestepMode;
use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};

#[derive(Debug, Default, Clone, Copy)]
struct RecordedControls {
    axis_x: f32,
    axis_y: f32,
    jump_pressed: bool,
    jump_held: bool,
    jump_released: bool,
    burst_pressed: bool,
    left_pressed: bool,
    right_pressed: bool,
    up_pressed: bool,
    down_pressed: bool,
    blink_pressed: bool,
    blink_held: bool,
    blink_released: bool,
    attack_pressed: bool,
    attack_held: bool,
    attack_released: bool,
    attack_strength: ambition_platformer2d::sim::AttackStrengthHint,
    attack_from_aim_stick: bool,
    attack_aim_x: f32,
    attack_aim_y: f32,
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
            left_pressed: c.left_pressed,
            right_pressed: c.right_pressed,
            up_pressed: c.up_pressed,
            down_pressed: c.down_pressed,
            jump: c.jump_pressed,
            jump_held: c.jump_held,
            jump_released: c.jump_released,
            dash: c.burst_pressed,
            attack: c.attack_pressed,
            attack_held: c.attack_held,
            attack_released: c.attack_released,
            attack_strength: c.attack_strength,
            attack_from_aim_stick: c.attack_from_aim_stick,
            attack_aim: (c.attack_aim_x, c.attack_aim_y),
            // Recorded traces predate the dedicated Special slot, so a replay
            // carries no special edge and no hold.
            special: false,
            special_held: false,
            blink: c.blink_pressed,
            blink_held: c.blink_held,
            blink_released: c.blink_released,
            pogo: c.pogo_pressed,
            interact: c.interact_pressed,
            // The trace format records only the interact edge; replay it as a
            // single-frame held (possession holds aren't captured in fixtures).
            interact_held: c.interact_pressed,
            projectile: false,
            projectile_held: false,
            projectile_released: false,
            fly_toggle: c.fly_toggle_pressed,
            reset: c.reset_pressed,
            start: c.start_pressed,
            // The trace format predates the modifier slot; an old recording holds
            // nothing on it.
            modifier: false,
            modifier_held: false,
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
                // The recorded key stays `dash_pressed`: renaming the Rust
                // channel to burst did not rewrite traces on disk.
                burst_pressed: bool_field(controls, "dash_pressed"),
                left_pressed: bool_field(controls, "left_pressed"),
                right_pressed: bool_field(controls, "right_pressed"),
                up_pressed: bool_field(controls, "up_pressed"),
                down_pressed: bool_field(controls, "down_pressed"),
                blink_pressed: bool_field(controls, "blink_pressed"),
                blink_held: bool_field(controls, "blink_held"),
                blink_released: bool_field(controls, "blink_released"),
                attack_pressed: bool_field(controls, "attack_pressed"),
                attack_held: bool_field(controls, "attack_held"),
                attack_released: bool_field(controls, "attack_released"),
                attack_strength: strength_hint_field(controls),
                attack_from_aim_stick: bool_field(controls, "attack_from_aim_stick"),
                attack_aim_x: f32_field(controls, "attack_aim_x"),
                attack_aim_y: f32_field(controls, "attack_aim_y"),
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

/// What strength did this recorded press ask for, across both trace generations?
///
/// Two keys, one question. Older traces carry `attack_strong_hint: bool`;
/// newer ones carry the three-valued `attack_strength_hint`
/// (`AttackStrengthHint`), which can force a tilt at full deflection
/// (`RightStickMode::TiltAttack`). Reading only the new key would replay every
/// archived trace with no smashes and report no divergence. `Tilt` must stay
/// `Tilt`: `Auto` at full deflection resolves to `Smash`, a different move.
fn strength_hint_field(
    controls: &serde_json::Value,
) -> ambition_platformer2d::sim::AttackStrengthHint {
    use ambition_platformer2d::sim::AttackStrengthHint as Hint;
    if let Some(hint) = controls
        .get("attack_strength_hint")
        .and_then(|v| v.as_str())
    {
        return match hint {
            "Smash" => Hint::Smash,
            "Tilt" => Hint::Tilt,
            _ => Hint::Auto,
        };
    }
    // The archived shape: one bool meaning "smash, or you decide".
    if bool_field(controls, "attack_strong_hint") {
        Hint::Smash
    } else {
        Hint::Auto
    }
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

    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .map_err(|e| format!("Platformer2dSimHarness::new failed: {e}"))?;
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

    // Apply frames[i].controls and expect the post-step `live.player_pos` to
    // match `frames[i].player_pos`.
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

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::sim::AttackStrengthHint as Hint;

    /// A recorded tilt must replay as a tilt. `Auto` at full stick deflection
    /// resolves to `Smash`, so a replay would play a different move and still
    /// report a clean run. `RightStickMode::TiltAttack` produces the hint.
    #[test]
    fn a_recorded_tilt_replays_as_a_tilt() {
        let tilt = serde_json::json!({ "attack_strength_hint": "Tilt" });
        assert_eq!(strength_hint_field(&tilt), Hint::Tilt);
        let smash = serde_json::json!({ "attack_strength_hint": "Smash" });
        assert_eq!(strength_hint_field(&smash), Hint::Smash);
        let auto = serde_json::json!({ "attack_strength_hint": "Auto" });
        assert_eq!(strength_hint_field(&auto), Hint::Auto);
    }

    /// …and an archived trace, whose only vocabulary was one bool, still
    /// reads. Reading only the new key would replay every archived smash as
    /// `Auto` and report no divergence.
    #[test]
    fn an_archived_bool_trace_still_says_smash() {
        let old_smash = serde_json::json!({ "attack_strong_hint": true });
        assert_eq!(strength_hint_field(&old_smash), Hint::Smash);
        let old_plain = serde_json::json!({ "attack_strong_hint": false });
        assert_eq!(strength_hint_field(&old_plain), Hint::Auto);
        // An old trace never recorded a tilt, so `Auto` is correct.
        assert_eq!(strength_hint_field(&serde_json::json!({})), Hint::Auto);
    }

    /// The C-stick direction survives the trace, so a replayed right-stick
    /// attack points where the stick went rather than where the body was running.
    #[test]
    fn a_recorded_c_stick_press_replays_with_its_own_direction() {
        let controls = serde_json::json!({
            "attack_pressed": true,
            "attack_strength_hint": "Tilt",
            "attack_from_aim_stick": true,
            "attack_aim_x": 1.0,
            "attack_aim_y": 0.0,
            "axis_x": -1.0,
        });
        let recorded = RecordedControls {
            attack_pressed: bool_field(&controls, "attack_pressed"),
            attack_strength: strength_hint_field(&controls),
            attack_from_aim_stick: bool_field(&controls, "attack_from_aim_stick"),
            attack_aim_x: f32_field(&controls, "attack_aim_x"),
            attack_aim_y: f32_field(&controls, "attack_aim_y"),
            axis_x: f32_field(&controls, "axis_x"),
            ..RecordedControls::default()
        };
        let action = AgentAction::from(recorded);
        assert_eq!(action.attack_strength, Hint::Tilt);
        assert!(action.attack_from_aim_stick);
        assert_eq!(action.attack_aim, (1.0, 0.0));
        // The movement axis points the other way, which the direction has to
        // beat: without it the replayed attack comes out left.
        assert!(action.move_x < 0.0);
    }
}
