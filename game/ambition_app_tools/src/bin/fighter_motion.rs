//! How does a Smash fighter MOVE, set against Ultimate's cast?
//!
//! `cargo run -p ambition_app_tools --bin fighter_motion -- [ID ...] [anchor=Mario] [tsv]`
//!
//! Seats each fighter on the real Smash stage, takes both brains off, and
//! drives the production input road through scripted presses: a run from rest,
//! the stop out of it, a full hop, a short hop, an air jump, a fall from height
//! with and without the fast fall, air drift, a forward roll, a spot dodge and
//! the neutral and sideways air dodges. Every number is read off the body,
//! never off the tuning, so it is what a player gets.
//!
//! Each measurement is printed beside Ultimate's: one anchor fighter and the
//! roster's p10 / median / p90. Those come from
//! `docs/related-work/data/smash_ultimate_fighter_stats.csv`, read at run time
//! (see `docs/related-work/smash-ultimate-fighter-reference.md`), and are put on
//! our scale through the stage: `k = platform width / 160` px per unit, 60
//! frames a second. Nothing here holds a target of its own.
//!
//! `tsv` prints one machine-readable row per fighter instead of the report.
//!
//! Observational: no threshold, no pass/fail.

#[path = "../probe_stage.rs"]
mod probe_stage;

use std::collections::BTreeMap;

use ambition_platformer2d::engine_core::{
    BodyGroundState, BodyKinematics, BodyMotionFacts, ControlFrame,
};
use bevy::prelude::*;

const DT: f32 = 1.0 / 60.0;

/// Ultimate's stages: Battlefield and Final Destination are 160 units wide.
const ULTIMATE_STAGE_UNITS: f32 = 160.0;

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let anchor = argv
        .iter()
        .find_map(|a| a.strip_prefix("anchor="))
        .unwrap_or("Mario")
        .to_string();
    let tsv = argv.iter().any(|a| a == "tsv");
    let mut ids: Vec<String> = argv
        .iter()
        .filter(|a| !a.contains('=') && a.as_str() != "tsv")
        .cloned()
        .collect();
    if ids.is_empty() {
        ids.push("smash_duelist_a".into());
    }

    let reference = Reference::load();
    let stage = ambition_demo_smash::smash_stage().world;
    let platform = stage.blocks[0].aabb;
    let k = (platform.max.x - platform.min.x) / ULTIMATE_STAGE_UNITS;

    if tsv {
        println!("fighter\t{}", METRICS.iter().map(|m| m.key).collect::<Vec<_>>().join("\t"));
    } else {
        println!(
            "# stage platform {:.0} px wide -> k = {k:.2} px per Ultimate unit; anchor {anchor}",
            platform.max.x - platform.min.x
        );
    }
    for id in &ids {
        let measured = measure(id, platform.min.x, platform.max.x, platform.min.y);
        if tsv {
            let cells: Vec<String> = METRICS
                .iter()
                .map(|m| measured.get(m.key).map_or("-".into(), |v| format!("{v:.3}")))
                .collect();
            println!("{id}\t{}", cells.join("\t"));
        } else {
            report(id, &measured, &reference, &anchor, k);
        }
    }
}

// ---------------------------------------------------------------------------
// What is measured, and how each maps onto Ultimate's columns.

/// How a measurement converts to Ultimate's units.
#[derive(Clone, Copy)]
enum Unit {
    /// px/s -> units/frame.
    Speed,
    /// px/s² -> units/frame².
    Accel,
    /// px -> units.
    Distance,
    /// s -> frames.
    Time,
}

struct Metric {
    key: &'static str,
    label: &'static str,
    unit: Unit,
    /// The CSV column it is compared with, if Ultimate measures it.
    column: Option<&'static str>,
}

const METRICS: &[Metric] = &[
    Metric { key: "initial_dash", label: "initial dash speed", unit: Unit::Speed, column: Some("initial_dash") },
    Metric { key: "dash_time", label: "initial dash length", unit: Unit::Time, column: Some("dash_frames") },
    Metric { key: "run", label: "run speed", unit: Unit::Speed, column: Some("run_speed") },
    Metric { key: "run_90", label: "time to 90% of run", unit: Unit::Time, column: None },
    Metric { key: "walk", label: "half-stick speed", unit: Unit::Speed, column: Some("walk_speed") },
    Metric { key: "stop_decel", label: "stop deceleration", unit: Unit::Accel, column: Some("traction") },
    Metric { key: "stop_dist", label: "stop slide", unit: Unit::Distance, column: None },
    Metric { key: "squat", label: "press to takeoff", unit: Unit::Time, column: None },
    Metric { key: "full_hop", label: "full hop height", unit: Unit::Distance, column: Some("full_hop") },
    Metric { key: "fh_air", label: "full hop airtime", unit: Unit::Time, column: Some("fh_frames") },
    Metric { key: "short_hop", label: "short hop height", unit: Unit::Distance, column: Some("short_hop") },
    Metric { key: "sh_air", label: "short hop airtime", unit: Unit::Time, column: Some("sh_frames") },
    Metric { key: "air_jump", label: "air jump height", unit: Unit::Distance, column: Some("air_jump") },
    Metric { key: "gravity", label: "gravity (early fall)", unit: Unit::Accel, column: Some("gravity") },
    Metric { key: "fall", label: "fall speed", unit: Unit::Speed, column: Some("fall_speed") },
    Metric { key: "fall_95", label: "time to 95% of fall", unit: Unit::Time, column: None },
    Metric { key: "fast_fall", label: "fast-fall speed", unit: Unit::Speed, column: Some("fast_fall_speed") },
    Metric { key: "air_speed", label: "air speed", unit: Unit::Speed, column: Some("air_speed") },
    Metric { key: "air_accel", label: "air acceleration", unit: Unit::Accel, column: Some("air_accel_max") },
    Metric { key: "air_brake", label: "air friction (stick let go)", unit: Unit::Accel, column: Some("air_friction") },
    Metric { key: "roll_dist", label: "forward roll distance", unit: Unit::Distance, column: Some("froll_distance") },
    Metric { key: "roll_safe", label: "forward roll protected", unit: Unit::Time, column: None },
    Metric { key: "roll_total", label: "forward roll total", unit: Unit::Time, column: Some("froll_total") },
    Metric { key: "spot_safe", label: "spot dodge protected", unit: Unit::Time, column: None },
    Metric { key: "spot_total", label: "spot dodge total", unit: Unit::Time, column: Some("spot_dodge_total") },
    Metric { key: "nair_safe", label: "neutral air dodge protected", unit: Unit::Time, column: None },
    Metric { key: "nair_total", label: "neutral air dodge total", unit: Unit::Time, column: Some("nairdodge_total") },
    Metric { key: "sair_dist", label: "side air dodge travel", unit: Unit::Distance, column: None },
    Metric { key: "sair_total", label: "side air dodge total", unit: Unit::Time, column: None },
];

impl Unit {
    /// Ours (px, s) in Ultimate's units at `k` px per unit.
    fn to_ultimate(self, v: f32, k: f32) -> f32 {
        match self {
            Unit::Speed => v / (k * 60.0),
            Unit::Accel => v / (k * 3600.0),
            Unit::Distance => v / k,
            Unit::Time => v * 60.0,
        }
    }
    fn ours(self, v: f32) -> String {
        match self {
            Unit::Speed => format!("{v:.0} px/s"),
            Unit::Accel => format!("{v:.0} px/s²"),
            Unit::Distance => format!("{v:.1} px"),
            Unit::Time => format!("{v:.3} s"),
        }
    }
    fn ultimate(self, v: f32) -> String {
        match self {
            Unit::Speed => format!("{v:.3}"),
            Unit::Accel => format!("{v:.4}"),
            Unit::Distance => format!("{v:.1}"),
            Unit::Time => format!("{v:.0}f"),
        }
    }
}

// ---------------------------------------------------------------------------
// The reference table.

struct Reference {
    rows: BTreeMap<String, BTreeMap<String, String>>,
}

impl Reference {
    fn load() -> Self {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/related-work/data/smash_ultimate_fighter_stats.csv"
        );
        let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
        let mut lines = text.lines();
        let heads: Vec<String> = split_csv(lines.next().expect("a header"));
        let rows = lines
            .map(|line| {
                let cells = split_csv(line);
                let row: BTreeMap<String, String> = heads.iter().cloned().zip(cells).collect();
                (row["character"].clone(), row)
            })
            .collect();
        Self { rows }
    }

    /// A cell as a number. Spot dodge totals read "cancel/full"; the full one
    /// is the body's own end, so it is the one taken.
    fn value(&self, fighter: &str, column: &str) -> Option<f32> {
        let raw = self.rows.get(fighter)?.get(column)?;
        let raw = raw.rsplit('/').next()?.trim();
        raw.parse().ok()
    }

    /// p10, median, p90 of a column across the roster.
    fn spread(&self, column: &str) -> Option<(f32, f32, f32)> {
        let mut v: Vec<f32> = self.rows.keys().filter_map(|f| self.value(f, column)).collect();
        if v.is_empty() {
            return None;
        }
        v.sort_by(|a, b| a.total_cmp(b));
        let at = |q: f32| v[((v.len() - 1) as f32 * q).round() as usize];
        Some((at(0.1), at(0.5), at(0.9)))
    }
}

fn split_csv(line: &str) -> Vec<String> {
    let (mut out, mut cell, mut quoted) = (Vec::new(), String::new(), false);
    for c in line.chars() {
        match c {
            '"' => quoted = !quoted,
            ',' if !quoted => out.push(std::mem::take(&mut cell)),
            _ => cell.push(c),
        }
    }
    out.push(cell);
    out
}

fn report(id: &str, measured: &BTreeMap<&'static str, f32>, reference: &Reference, anchor: &str, k: f32) {
    println!("\n== {id}");
    println!(
        "  {:<28} {:>12} {:>9}   {:>9} {:>26}",
        "", "ours", "in u/f", anchor, "roster p10 / median / p90"
    );
    for m in METRICS {
        let Some(&v) = measured.get(m.key) else {
            println!("  {:<28} {:>12}", m.label, "-");
            continue;
        };
        let u = m.unit.to_ultimate(v, k);
        let (anchor_cell, spread_cell, flag) = match m.column {
            Some(col) => {
                let a = reference
                    .value(anchor, col)
                    .map_or("-".into(), |a| m.unit.ultimate(a));
                match reference.spread(col) {
                    Some((lo, mid, hi)) => (
                        a,
                        format!(
                            "{} / {} / {}",
                            m.unit.ultimate(lo),
                            m.unit.ultimate(mid),
                            m.unit.ultimate(hi)
                        ),
                        if u < lo {
                            format!("LOW  x{:.2} of median", u / mid)
                        } else if u > hi {
                            format!("HIGH x{:.2} of median", u / mid)
                        } else {
                            String::new()
                        },
                    ),
                    None => (a, String::new(), String::new()),
                }
            }
            None => (String::new(), String::new(), String::new()),
        };
        println!(
            "  {:<28} {:>12} {:>9}   {:>9} {:>26}  {flag}",
            m.label,
            m.unit.ours(v),
            m.unit.ultimate(u),
            anchor_cell,
            spread_cell
        );
    }
}

// ---------------------------------------------------------------------------
// The measurements.

struct Rig {
    app: App,
    body: Entity,
    home: Vec2,
    stage_left: f32,
    stage_right: f32,
}

#[derive(Clone, Copy, Default)]
struct Sample {
    pos: Vec2,
    vel: Vec2,
    on_ground: bool,
    facts: Facts,
}

#[derive(Clone, Copy, Default)]
struct Facts {
    initial_dashing: bool,
    evading: bool,
    committed: bool,
    rolling: bool,
    roll_endlag: bool,
    air_dodging: bool,
    air_dodge_endlag: bool,
}

impl Rig {
    fn sample(&self) -> Sample {
        let w = self.app.world();
        let kin = w.get::<BodyKinematics>(self.body).expect("the fighter has a body");
        let facts = w.get::<BodyMotionFacts>(self.body).map_or(Facts::default(), |f| Facts {
            initial_dashing: f.initial_dashing,
            evading: f.evading(),
            committed: f.evade_committed,
            rolling: f.dodge_rolling,
            roll_endlag: f.dodge_roll_endlag,
            air_dodging: f.air_dodging,
            air_dodge_endlag: f.air_dodge_endlag,
        });
        Sample {
            pos: Vec2::new(kin.pos.x, kin.pos.y),
            vel: Vec2::new(kin.vel.x, kin.vel.y),
            on_ground: w.get::<BodyGroundState>(self.body).is_some_and(|g| g.on_ground),
            facts,
        }
    }

    /// One tick under `frame`, then the body after it.
    fn step(&mut self, frame: ControlFrame) -> Sample {
        ambition_platformer2d::sim::drive_control_frame(self.app.world_mut(), frame);
        self.app.update();
        self.sample()
    }

    fn idle(&mut self, ticks: usize) {
        for _ in 0..ticks {
            self.step(ControlFrame::default());
        }
    }

    /// Back on the floor at `x`, at rest, with every timer run out.
    fn reset_at(&mut self, x: f32) {
        probe_stage::place(&mut self.app, self.body, Vec2::new(x, self.home.y));
        self.idle(50);
    }

    /// In the air, `height` px above the floor at `x`, at rest.
    fn hang_at(&mut self, x: f32, height: f32) {
        self.reset_at(x);
        probe_stage::place(&mut self.app, self.body, Vec2::new(x, self.home.y - height));
    }
}

fn hold(x: f32, y: f32) -> ControlFrame {
    ControlFrame {
        axis_x: x,
        axis_y: y,
        right_pressed: x > 0.5,
        left_pressed: x < -0.5,
        down_pressed: y > 0.5,
        ..ControlFrame::default()
    }
}

fn measure(id: &str, stage_left: f32, stage_right: f32, stage_top: f32) -> BTreeMap<&'static str, f32> {
    let probe_stage::Staged { mut app, seat0, seat1 } = probe_stage::stage(probe_stage::StageRequest {
        cast: [id, id],
        demo_host: false,
        rendered: false,
    });
    // The partner stands still, off to the side, so it is never in the way.
    app.world_mut()
        .entity_mut(seat1)
        .remove::<ambition_platformer2d::characters::brain::Brain>();
    let (here, _) = probe_stage::kin(&app, seat0);
    probe_stage::place(&mut app, seat1, Vec2::new(stage_left + 12.0, here.y));
    let mut rig = Rig {
        app,
        body: seat0,
        home: here,
        stage_left,
        stage_right,
    };
    rig.idle(60);
    rig.home = rig.sample().pos;
    let _ = stage_top;

    let mut out = BTreeMap::new();
    ground(&mut rig, &mut out);
    jumps(&mut rig, &mut out);
    falls(&mut rig, &mut out);
    evades(&mut rig, &mut out);
    out
}

fn ground(rig: &mut Rig, out: &mut BTreeMap<&'static str, f32>) {
    let start = rig.stage_left + 60.0;
    let room = rig.stage_right - 30.0;

    // Run from rest.
    rig.reset_at(start);
    let mut speeds = Vec::new();
    let mut dash_ticks = 0usize;
    for _ in 0..90 {
        let s = rig.step(hold(1.0, 0.0));
        speeds.push(s.vel.x);
        if s.facts.initial_dashing {
            dash_ticks += 1;
        }
        if s.pos.x > room - 60.0 {
            break;
        }
    }
    let run = tail_mean(&speeds, 8);
    let dash_window = dash_ticks.max(1).min(speeds.len());
    let initial_dash = speeds[..dash_window].iter().cloned().fold(0.0, f32::max);
    out.insert("initial_dash", initial_dash);
    out.insert("dash_time", dash_ticks as f32 * DT);
    out.insert("run", run);
    if let Some(i) = speeds.iter().position(|v| *v >= 0.9 * run) {
        out.insert("run_90", (i + 1) as f32 * DT);
    }

    // Stop out of that run.
    let from = rig.sample();
    let mut ticks = 0usize;
    let mut s = from;
    while s.vel.x.abs() > 2.0 && ticks < 120 {
        s = rig.step(ControlFrame::default());
        ticks += 1;
    }
    if ticks > 0 && from.vel.x.abs() > 2.0 {
        let t = ticks as f32 * DT;
        out.insert("stop_dist", (s.pos.x - from.pos.x).abs());
        out.insert("stop_decel", from.vel.x.abs() / t);
    }

    // Half stick.
    rig.reset_at(start);
    let mut half = Vec::new();
    for _ in 0..60 {
        half.push(rig.step(hold(0.5, 0.0)).vel.x);
    }
    out.insert("walk", tail_mean(&half, 8));
}

/// Height (px, up positive) and airtime of one jump held for `held` ticks
/// (`usize::MAX`: to the landing), from the press. Also returns the ticks from
/// the press to the takeoff, and the rise an air jump at the apex added.
fn hop(rig: &mut Rig, held: usize, air_jump_at_apex: bool) -> (f32, f32, usize, f32) {
    let x = 0.5 * (rig.stage_left + rig.stage_right);
    rig.reset_at(x);
    let floor = rig.sample().pos.y;
    let mut top = floor;
    let mut squat = 0usize;
    let mut left_ground: Option<usize> = None;
    let mut landed: Option<usize> = None;
    let mut first_apex: Option<f32> = None;
    let mut air_jumped = false;
    for tick in 0..400 {
        let pressed = tick == 0 || (air_jump_at_apex && first_apex.is_some() && !air_jumped);
        if pressed && tick > 0 {
            air_jumped = true;
        }
        let holding = tick < held || pressed;
        let s = rig.step(ControlFrame {
            jump_pressed: pressed,
            jump_held: holding,
            jump_released: tick == held,
            ..ControlFrame::default()
        });
        if left_ground.is_none() && !s.on_ground && s.pos.y < floor - 0.5 {
            left_ground = Some(tick);
            // Ticks before the one the body rose on: Ultimate's 3-frame squat
            // reads 3 here.
            squat = tick;
        }
        top = top.min(s.pos.y);
        if first_apex.is_none() && left_ground.is_some() && s.vel.y >= 0.0 {
            first_apex = Some(s.pos.y);
        }
        if left_ground.is_some() && s.on_ground {
            landed = Some(tick);
            break;
        }
    }
    let airtime = match (left_ground, landed) {
        (Some(a), Some(b)) => (b - a) as f32 * DT,
        _ => f32::NAN,
    };
    let extra = first_apex.map_or(f32::NAN, |a| a - top);
    (floor - top, airtime, squat, extra)
}

fn jumps(rig: &mut Rig, out: &mut BTreeMap<&'static str, f32>) {
    let (h, t, squat, _) = hop(rig, usize::MAX, false);
    out.insert("full_hop", h);
    out.insert("fh_air", t);
    out.insert("squat", squat as f32 * DT);
    let (h, t, _, _) = hop(rig, 1, false);
    out.insert("short_hop", h);
    out.insert("sh_air", t);
    let (_, _, _, extra) = hop(rig, usize::MAX, true);
    out.insert("air_jump", extra);
}

fn falls(rig: &mut Rig, out: &mut BTreeMap<&'static str, f32>) {
    let x = 0.5 * (rig.stage_left + rig.stage_right);
    let height = 400.0;

    rig.hang_at(x, height);
    let mut vy = Vec::new();
    for _ in 0..90 {
        let s = rig.step(ControlFrame::default());
        if s.on_ground {
            break;
        }
        vy.push(s.vel.y);
    }
    if vy.len() > 4 {
        // Gravity from the first ticks, before any cap.
        out.insert("gravity", (vy[3] - vy[0]) / (3.0 * DT));
        let cap = vy.iter().cloned().fold(0.0, f32::max);
        out.insert("fall", cap);
        if let Some(i) = vy.iter().position(|v| *v >= 0.95 * cap) {
            out.insert("fall_95", (i + 1) as f32 * DT);
        }
    }

    rig.hang_at(x, height);
    let mut fast = 0.0f32;
    for tick in 0..90 {
        let mut frame = hold(0.0, 1.0);
        frame.fast_fall_pressed = tick == 2;
        let s = rig.step(frame);
        if s.on_ground {
            break;
        }
        fast = fast.max(s.vel.y);
    }
    out.insert("fast_fall", fast);

    rig.hang_at(rig.stage_left + 60.0, height);
    let mut vx = Vec::new();
    for _ in 0..60 {
        let s = rig.step(hold(1.0, 0.0));
        if s.on_ground {
            break;
        }
        vx.push(s.vel.x);
    }
    if vx.len() > 4 {
        out.insert("air_accel", (vx[3] - vx[0]) / (3.0 * DT));
        out.insert("air_speed", vx.iter().cloned().fold(0.0, f32::max));
    }

    // Let go at speed, still in the air.
    rig.hang_at(rig.stage_left + 60.0, height);
    for _ in 0..30 {
        rig.step(hold(1.0, 0.0));
    }
    let before = rig.sample();
    let mut after = before;
    for _ in 0..6 {
        after = rig.step(ControlFrame::default());
    }
    if !after.on_ground && before.vel.x > 1.0 {
        out.insert("air_brake", (before.vel.x - after.vel.x) / (6.0 * DT));
    }
}

/// Shield, then the evade: `dir` is the stick.
fn evade(rig: &mut Rig, dir: Vec2, airborne: bool) -> (f32, f32, f32) {
    let x = 0.5 * (rig.stage_left + rig.stage_right) - 60.0;
    // Wait out the last evade's staling (Smash forgives one per 1.2 s), so
    // every evade is measured fresh.
    rig.idle(150);
    if airborne {
        rig.hang_at(x, 300.0);
        rig.step(ControlFrame::default());
    } else {
        rig.reset_at(x);
        for _ in 0..12 {
            rig.step(ControlFrame { shield_held: true, ..ControlFrame::default() });
        }
    }
    let start = rig.sample();
    let press = ControlFrame {
        shield_held: true,
        burst_pressed: true,
        ..hold(dir.x, dir.y)
    };
    let (mut safe, mut busy, mut began) = (0usize, 0usize, false);
    let mut end = start;
    for tick in 0..240 {
        let frame = if tick < 4 { press } else { ControlFrame::default() };
        let s = rig.step(frame);
        let f = s.facts;
        let in_it = f.evading || f.committed || f.rolling || f.roll_endlag || f.air_dodging || f.air_dodge_endlag;
        if f.evading {
            safe += 1;
        }
        if in_it {
            began = true;
            busy = tick + 1;
            end = s;
        } else if began {
            break;
        }
        if airborne && s.on_ground {
            break;
        }
    }
    (
        (end.pos.x - start.pos.x).abs(),
        safe as f32 * DT,
        busy as f32 * DT,
    )
}

fn evades(rig: &mut Rig, out: &mut BTreeMap<&'static str, f32>) {
    let (d, safe, total) = evade(rig, Vec2::new(1.0, 0.0), false);
    out.insert("roll_dist", d);
    out.insert("roll_safe", safe);
    out.insert("roll_total", total);
    let (_, safe, total) = evade(rig, Vec2::new(0.0, 1.0), false);
    out.insert("spot_safe", safe);
    out.insert("spot_total", total);
    let (_, safe, total) = evade(rig, Vec2::ZERO, true);
    out.insert("nair_safe", safe);
    out.insert("nair_total", total);
    let (d, _, total) = evade(rig, Vec2::new(1.0, 0.0), true);
    out.insert("sair_dist", d);
    out.insert("sair_total", total);
}

fn tail_mean(v: &[f32], n: usize) -> f32 {
    let tail = &v[v.len().saturating_sub(n)..];
    tail.iter().sum::<f32>() / tail.len().max(1) as f32
}
