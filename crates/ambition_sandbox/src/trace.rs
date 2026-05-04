//! Gameplay flight recorder / OOB debug logging.
//!
//! A rolling ring buffer of per-frame player snapshots and discrete gameplay
//! events. The buffer is filled inside `sandbox_update` (simulation-side) so
//! the recorder works in the headless binary as well as the visible game.
//!
//! Two ways to dump:
//!
//! * `F8` (visible binary, presentation-side hotkey) — `Manual` reason,
//!   captures whatever is currently in the ring.
//! * Out-of-bounds detection — `OobAuto` reason, fired automatically the
//!   first frame the player drifts outside the active-area envelope, has
//!   non-finite pos/vel, has absurd velocity, or sits inside a `Solid`
//!   after movement resolution.
//!
//! Dumps are written under `debug_traces/ambition_trace_YYYYMMDD_HHMMSS.{json,md}`
//! relative to the sandbox working directory. JSON is the machine-readable
//! source of truth; the Markdown file is a human summary built from the
//! same snapshot. Path generation is offline-safe (no system calls).
//!
//! See `docs/gameplay_trace_recorder.md` for the workflow and bug-reporting
//! checklist.

use std::collections::VecDeque;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ae::AabbExt;
use ambition_engine as ae;
use bevy::prelude::*;
use serde::Serialize;

use crate::input::ControlFrame;
use crate::{GameWorld, SandboxRuntime};

const DEFAULT_FRAME_CAPACITY: usize = 240;
const DEFAULT_EVENT_CAPACITY: usize = 240;
const MARKDOWN_FRAME_SUMMARY_TAIL: usize = 120;
const MARKDOWN_EVENT_TAIL: usize = 100;
const NEARBY_COLLISION_RADIUS: f32 = 220.0;
const MAX_NEARBY_COLLISION: usize = 32;
const ABSURD_VELOCITY_MAGNITUDE: f32 = 8000.0;
/// Margin (in active-area coords) beyond which a player is considered OOB.
/// This is intentionally generous so authored levels with intentional
/// camera-out-of-room moments do not auto-dump on every frame.
const OOB_MARGIN: f32 = 96.0;

/// Lightweight 2D point used in the serialized payload. Avoids leaking
/// `bevy_math::Vec2` into the JSON shape (which is not directly Serialize
/// without a feature flag).
#[derive(Serialize, Clone, Copy, Debug, Default)]
pub struct TracePoint {
    pub x: f32,
    pub y: f32,
}

impl From<ae::Vec2> for TracePoint {
    fn from(v: ae::Vec2) -> Self {
        Self { x: v.x, y: v.y }
    }
}

#[derive(Serialize, Clone, Copy, Debug, Default)]
pub struct TraceAabb {
    pub min: TracePoint,
    pub max: TracePoint,
}

impl From<ae::Aabb> for TraceAabb {
    fn from(a: ae::Aabb) -> Self {
        Self {
            min: a.min.into(),
            max: a.max.into(),
        }
    }
}

#[derive(Serialize, Clone, Copy, Debug, Default)]
pub struct ControlFrameTrace {
    pub axis_x: f32,
    pub axis_y: f32,
    pub jump_pressed: bool,
    pub jump_held: bool,
    pub jump_released: bool,
    pub dash_pressed: bool,
    pub up_pressed: bool,
    pub down_pressed: bool,
    pub fast_fall_pressed: bool,
    pub blink_pressed: bool,
    pub blink_held: bool,
    pub blink_released: bool,
    pub attack_pressed: bool,
    pub pogo_pressed: bool,
    pub fly_toggle_pressed: bool,
    pub interact_pressed: bool,
    pub reset_pressed: bool,
    pub start_pressed: bool,
}

impl From<ControlFrame> for ControlFrameTrace {
    fn from(c: ControlFrame) -> Self {
        Self {
            axis_x: c.axis_x,
            axis_y: c.axis_y,
            jump_pressed: c.jump_pressed,
            jump_held: c.jump_held,
            jump_released: c.jump_released,
            dash_pressed: c.dash_pressed,
            up_pressed: c.up_pressed,
            down_pressed: c.down_pressed,
            fast_fall_pressed: c.fast_fall_pressed,
            blink_pressed: c.blink_pressed,
            blink_held: c.blink_held,
            blink_released: c.blink_released,
            attack_pressed: c.attack_pressed,
            pogo_pressed: c.pogo_pressed,
            fly_toggle_pressed: c.fly_toggle_pressed,
            interact_pressed: c.interact_pressed,
            reset_pressed: c.reset_pressed,
            start_pressed: c.start_pressed,
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct PlayerTraceState {
    pub pos: TracePoint,
    pub vel: TracePoint,
    pub size: TracePoint,
    pub aabb: TraceAabb,
    pub facing: f32,
    pub on_ground: bool,
    pub on_wall: bool,
    pub wall_clinging: bool,
    pub wall_climbing: bool,
    pub fast_falling: bool,
    pub fly_enabled: bool,
    pub dash_charges_available: u8,
    pub air_jumps_available: u8,
    pub blink_aiming: bool,
    pub blink_grace_timer: f32,
    pub locomotion: String,
    pub body_mode: String,
    pub last_safe_pos: TracePoint,
    pub time_alive: f32,
    pub resets: u32,
}

#[derive(Serialize, Clone, Debug)]
pub struct CollisionTraceShape {
    pub kind: String,
    pub name: String,
    pub aabb: TraceAabb,
    pub distance: f32,
}

#[derive(Serialize, Clone, Debug)]
pub struct MovingPlatformTraceState {
    pub pos: TracePoint,
    pub size: TracePoint,
    pub phase: f32,
}

/// One per-frame snapshot. Heavy fields (collision shapes, moving
/// platforms) are kept short on purpose so a 240-frame ring buffer
/// stays cheap in memory.
#[derive(Serialize, Clone, Debug)]
pub struct GameplayTraceFrame {
    pub seq: u64,
    pub tick: u64,
    pub real_dt: f32,
    pub sim_dt: f32,
    pub time_scale: f32,
    pub game_mode: String,
    pub active_area: String,
    pub world_size: TracePoint,
    pub world_spawn: TracePoint,
    pub player: PlayerTraceState,
    pub controls: ControlFrameTrace,
    pub nearby_collision: Vec<CollisionTraceShape>,
    pub moving_platforms: Vec<MovingPlatformTraceState>,
}

#[derive(Serialize, Clone, Debug)]
pub enum GameplayTraceEvent {
    InputEdge {
        tick: u64,
        action: String,
    },
    PlayerModeChanged {
        tick: u64,
        from: String,
        to: String,
    },
    Jump {
        tick: u64,
    },
    DoubleJump {
        tick: u64,
    },
    Dash {
        tick: u64,
    },
    Blink {
        tick: u64,
        from: TracePoint,
        to: TracePoint,
        precision: bool,
    },
    Attack {
        tick: u64,
        kind: String,
    },
    Damage {
        tick: u64,
        source: String,
        amount: i32,
    },
    RoomTransition {
        tick: u64,
        from: String,
        to: String,
    },
    OobDetected {
        tick: u64,
        reason: String,
        pos: TracePoint,
    },
    CollisionCorrection {
        tick: u64,
        before: TracePoint,
        after: TracePoint,
        reason: String,
    },
    Sfx {
        tick: u64,
        label: String,
    },
    Vfx {
        tick: u64,
        label: String,
    },
    Reset {
        tick: u64,
    },
    Death {
        tick: u64,
    },
}

impl GameplayTraceEvent {
    fn tick(&self) -> u64 {
        match self {
            GameplayTraceEvent::InputEdge { tick, .. }
            | GameplayTraceEvent::PlayerModeChanged { tick, .. }
            | GameplayTraceEvent::Jump { tick }
            | GameplayTraceEvent::DoubleJump { tick }
            | GameplayTraceEvent::Dash { tick }
            | GameplayTraceEvent::Blink { tick, .. }
            | GameplayTraceEvent::Attack { tick, .. }
            | GameplayTraceEvent::Damage { tick, .. }
            | GameplayTraceEvent::RoomTransition { tick, .. }
            | GameplayTraceEvent::OobDetected { tick, .. }
            | GameplayTraceEvent::CollisionCorrection { tick, .. }
            | GameplayTraceEvent::Sfx { tick, .. }
            | GameplayTraceEvent::Vfx { tick, .. }
            | GameplayTraceEvent::Reset { tick }
            | GameplayTraceEvent::Death { tick } => *tick,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            GameplayTraceEvent::InputEdge { .. } => "InputEdge",
            GameplayTraceEvent::PlayerModeChanged { .. } => "PlayerModeChanged",
            GameplayTraceEvent::Jump { .. } => "Jump",
            GameplayTraceEvent::DoubleJump { .. } => "DoubleJump",
            GameplayTraceEvent::Dash { .. } => "Dash",
            GameplayTraceEvent::Blink { .. } => "Blink",
            GameplayTraceEvent::Attack { .. } => "Attack",
            GameplayTraceEvent::Damage { .. } => "Damage",
            GameplayTraceEvent::RoomTransition { .. } => "RoomTransition",
            GameplayTraceEvent::OobDetected { .. } => "OobDetected",
            GameplayTraceEvent::CollisionCorrection { .. } => "CollisionCorrection",
            GameplayTraceEvent::Sfx { .. } => "Sfx",
            GameplayTraceEvent::Vfx { .. } => "Vfx",
            GameplayTraceEvent::Reset { .. } => "Reset",
            GameplayTraceEvent::Death { .. } => "Death",
        }
    }
}

/// Why a dump is being requested. Drives the file's `dump_reason` and the
/// Markdown summary header.
#[derive(Serialize, Clone, Debug)]
pub enum DumpReason {
    Manual,
    OobAuto { reason: String },
    Programmatic { label: String },
}

impl DumpReason {
    pub fn label(&self) -> String {
        match self {
            DumpReason::Manual => "Manual (F8)".into(),
            DumpReason::OobAuto { reason } => format!("OOB auto: {reason}"),
            DumpReason::Programmatic { label } => format!("Programmatic: {label}"),
        }
    }
}

/// Why an OOB was detected on a particular frame.
#[derive(Serialize, Clone, Debug)]
pub enum OobReason {
    PositionNonFinite,
    VelocityNonFinite,
    OutsideWorldEnvelope { axis: char },
    InsideSolid { block_name: String },
    AbsurdVelocity { magnitude: f32 },
}

impl OobReason {
    pub fn short_label(&self) -> String {
        match self {
            OobReason::PositionNonFinite => "position non-finite".into(),
            OobReason::VelocityNonFinite => "velocity non-finite".into(),
            OobReason::OutsideWorldEnvelope { axis } => {
                format!("outside world envelope ({axis})")
            }
            OobReason::InsideSolid { block_name } => format!("inside solid ({block_name})"),
            OobReason::AbsurdVelocity { magnitude } => format!("absurd velocity ({magnitude:.0})"),
        }
    }
}

/// Top-level rolling buffer.
#[derive(Resource, Debug)]
pub struct GameplayTraceBuffer {
    capacity_frames: usize,
    capacity_events: usize,
    frames: VecDeque<GameplayTraceFrame>,
    events: VecDeque<GameplayTraceEvent>,
    sequence: u64,
    tick: u64,
    pub last_dump_path: Option<String>,
    pub last_dump_status: Option<String>,
    pub dump_request: Option<DumpReason>,
    /// Once an OOB has auto-dumped we suppress further auto-dumps until
    /// the player is no longer OOB; otherwise a single broken frame would
    /// produce 60 dump files per second.
    auto_dump_armed: bool,
    /// True after the very first frame has been recorded; lets us produce
    /// useful "first OOB frame" output without indexing into an empty
    /// buffer.
    has_recorded_any: bool,
}

impl Default for GameplayTraceBuffer {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_FRAME_CAPACITY, DEFAULT_EVENT_CAPACITY)
    }
}

impl GameplayTraceBuffer {
    pub fn with_capacity(frames: usize, events: usize) -> Self {
        Self {
            capacity_frames: frames.max(1),
            capacity_events: events.max(1),
            frames: VecDeque::with_capacity(frames.max(1)),
            events: VecDeque::with_capacity(events.max(1)),
            sequence: 0,
            tick: 0,
            last_dump_path: None,
            last_dump_status: None,
            dump_request: None,
            auto_dump_armed: true,
            has_recorded_any: false,
        }
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn current_tick(&self) -> u64 {
        self.tick
    }

    pub fn request_dump(&mut self, reason: DumpReason) {
        if self.dump_request.is_none() {
            self.dump_request = Some(reason);
        }
    }

    fn push_frame(&mut self, frame: GameplayTraceFrame) {
        if self.frames.len() == self.capacity_frames {
            self.frames.pop_front();
        }
        self.frames.push_back(frame);
        self.sequence = self.sequence.saturating_add(1);
        self.tick = self.tick.saturating_add(1);
        self.has_recorded_any = true;
    }

    pub fn push_event(&mut self, event: GameplayTraceEvent) {
        if self.events.len() == self.capacity_events {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    /// Drain `events` into the buffer in order.
    pub fn extend_events<I: IntoIterator<Item = GameplayTraceEvent>>(&mut self, events: I) {
        for ev in events {
            self.push_event(ev);
        }
    }

    pub fn frames(&self) -> impl Iterator<Item = &GameplayTraceFrame> {
        self.frames.iter()
    }

    pub fn events(&self) -> impl Iterator<Item = &GameplayTraceEvent> {
        self.events.iter()
    }
}

/// Inspect the current player state against the active world and produce
/// the *first* OOB reason found, if any. Order matters: NaN/inf should
/// be reported before "outside envelope" because both can be true.
pub fn detect_oob(player: &ae::Player, world: &ae::World, margin: f32) -> Option<OobReason> {
    if !player.pos.x.is_finite() || !player.pos.y.is_finite() {
        return Some(OobReason::PositionNonFinite);
    }
    if !player.vel.x.is_finite() || !player.vel.y.is_finite() {
        return Some(OobReason::VelocityNonFinite);
    }
    let speed = player.vel.length();
    if speed > ABSURD_VELOCITY_MAGNITUDE {
        return Some(OobReason::AbsurdVelocity { magnitude: speed });
    }
    let aabb = player.aabb();
    if aabb.left() < -margin || aabb.right() > world.size.x + margin {
        return Some(OobReason::OutsideWorldEnvelope { axis: 'x' });
    }
    if aabb.top() < -margin || aabb.bottom() > world.size.y + margin {
        return Some(OobReason::OutsideWorldEnvelope { axis: 'y' });
    }
    for block in &world.blocks {
        if matches!(block.kind, ae::BlockKind::Solid) && aabb.strict_intersects(block.aabb) {
            return Some(OobReason::InsideSolid {
                block_name: block.name.clone(),
            });
        }
    }
    None
}

fn nearby_collision(world: &ae::World, player: &ae::Player) -> Vec<CollisionTraceShape> {
    let center = player.pos;
    let mut hits: Vec<CollisionTraceShape> = world
        .blocks
        .iter()
        .map(|block| {
            let bcenter = block.aabb.center();
            let distance = (bcenter - center).length();
            CollisionTraceShape {
                kind: format!("{:?}", block.kind),
                name: block.name.clone(),
                aabb: block.aabb.into(),
                distance,
            }
        })
        .filter(|shape| shape.distance < NEARBY_COLLISION_RADIUS)
        .collect();
    hits.sort_by(|a, b| {
        a.distance
            .partial_cmp(&b.distance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hits.truncate(MAX_NEARBY_COLLISION);
    hits
}

/// Build a `GameplayTraceFrame` from current sim resources. This lives
/// next to `record_frame_in_simulation` so the sandbox phase pipeline
/// can call it once per `sandbox_update` tick.
pub fn build_frame(
    runtime: &SandboxRuntime,
    world: &ae::World,
    controls: ControlFrame,
    real_dt: f32,
    sim_dt: f32,
    game_mode: &str,
    active_area: &str,
    seq: u64,
    tick: u64,
    locomotion: &str,
    body_mode: &str,
) -> GameplayTraceFrame {
    let player = &runtime.player;
    GameplayTraceFrame {
        seq,
        tick,
        real_dt,
        sim_dt,
        time_scale: runtime.time_scale,
        game_mode: game_mode.into(),
        active_area: active_area.into(),
        world_size: world.size.into(),
        world_spawn: world.spawn.into(),
        player: PlayerTraceState {
            pos: player.pos.into(),
            vel: player.vel.into(),
            size: player.size.into(),
            aabb: player.aabb().into(),
            facing: player.facing,
            on_ground: player.on_ground,
            on_wall: player.on_wall,
            wall_clinging: player.wall_clinging,
            wall_climbing: player.wall_climbing,
            fast_falling: player.fast_falling,
            fly_enabled: player.fly_enabled,
            dash_charges_available: player.dash_charges_available,
            air_jumps_available: player.air_jumps_available,
            blink_aiming: player.blink_aiming,
            blink_grace_timer: player.blink_grace_timer,
            locomotion: locomotion.into(),
            body_mode: body_mode.into(),
            last_safe_pos: runtime.last_safe_player_pos.into(),
            time_alive: player.time_alive,
            resets: player.resets,
        },
        controls: controls.into(),
        nearby_collision: nearby_collision(world, player),
        moving_platforms: Vec::new(),
    }
}

/// Push the constructed frame into the buffer and (if not already armed)
/// auto-request an OOB dump.
pub fn record_frame(
    buffer: &mut GameplayTraceBuffer,
    frame: GameplayTraceFrame,
    oob: Option<&OobReason>,
) {
    if let Some(reason) = oob {
        let label = reason.short_label();
        buffer.push_event(GameplayTraceEvent::OobDetected {
            tick: buffer.tick,
            reason: label.clone(),
            pos: frame.player.pos,
        });
        if buffer.auto_dump_armed && buffer.dump_request.is_none() {
            buffer.dump_request = Some(DumpReason::OobAuto { reason: label });
            buffer.auto_dump_armed = false;
        }
    } else if !buffer.auto_dump_armed {
        // Player returned to a healthy state; rearm so a future OOB
        // re-fires.
        buffer.auto_dump_armed = true;
    }
    buffer.push_frame(frame);
}

#[derive(Serialize, Debug)]
struct DumpPayload<'a> {
    schema_version: u32,
    timestamp_unix: u64,
    timestamp_label: String,
    dump_reason: String,
    capacity_frames: usize,
    capacity_events: usize,
    frame_count: usize,
    event_count: usize,
    sequence: u64,
    current_tick: u64,
    frames: &'a [GameplayTraceFrame],
    events: &'a [GameplayTraceEvent],
}

/// Pure path-formatting helper. Does not touch the filesystem so tests
/// stay fast and the function is safe to call when the dump directory
/// doesn't yet exist.
pub fn dump_paths(dir: &Path, timestamp_label: &str) -> (PathBuf, PathBuf) {
    let stem = format!("ambition_trace_{timestamp_label}");
    let json = dir.join(format!("{stem}.json"));
    let md = dir.join(format!("{stem}.md"));
    (json, md)
}

fn timestamp_label(ts: SystemTime) -> String {
    // Avoid pulling chrono just for a label. UNIX seconds + naive tick is
    // sufficient: the interesting field is "different invocations get
    // different filenames", not human readability of seconds.
    let secs = ts
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let total_minutes = secs / 60;
    let seconds = secs % 60;
    let total_hours = total_minutes / 60;
    let minutes = total_minutes % 60;
    let total_days = total_hours / 24;
    let hours = total_hours % 24;
    // Approximate calendar conversion is intentional; the label only
    // needs to be unique-ish per dump. Prefix with raw seconds so even
    // sub-second collisions sort correctly when several dumps happen
    // back-to-back during a debug session.
    format!(
        "{secs:010}_{}d{:02}h{:02}m{:02}s",
        total_days, hours, minutes, seconds
    )
}

/// Convert the buffer into a `DumpPayload` and write JSON + Markdown to
/// `dir`. Returns the JSON path on success.
pub fn write_dump(
    buffer: &GameplayTraceBuffer,
    reason: &DumpReason,
    dir: &Path,
) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let now = SystemTime::now();
    let label = timestamp_label(now);
    let (json_path, md_path) = dump_paths(dir, &label);

    let frames_slice: Vec<GameplayTraceFrame> = buffer.frames.iter().cloned().collect();
    let events_slice: Vec<GameplayTraceEvent> = buffer.events.iter().cloned().collect();

    let payload = DumpPayload {
        schema_version: 1,
        timestamp_unix: now
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        timestamp_label: label.clone(),
        dump_reason: reason.label(),
        capacity_frames: buffer.capacity_frames,
        capacity_events: buffer.capacity_events,
        frame_count: frames_slice.len(),
        event_count: events_slice.len(),
        sequence: buffer.sequence,
        current_tick: buffer.tick,
        frames: &frames_slice,
        events: &events_slice,
    };
    let json_body = serde_json::to_string_pretty(&payload)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    fs::write(&json_path, json_body)?;

    let md_body = render_markdown(&payload);
    fs::write(&md_path, md_body)?;

    Ok(json_path)
}

fn render_markdown(payload: &DumpPayload<'_>) -> String {
    let mut out = String::new();
    out.push_str("# Ambition gameplay trace\n\n");
    out.push_str(&format!("- **Reason**: {}\n", payload.dump_reason));
    out.push_str(&format!(
        "- **Timestamp**: {} (unix {})\n",
        payload.timestamp_label, payload.timestamp_unix
    ));
    out.push_str(&format!(
        "- **Frames captured**: {} / {} (cap)\n",
        payload.frame_count, payload.capacity_frames
    ));
    out.push_str(&format!(
        "- **Events captured**: {} / {} (cap)\n",
        payload.event_count, payload.capacity_events
    ));
    out.push_str(&format!("- **Current tick**: {}\n\n", payload.current_tick));

    if let Some(latest) = payload.frames.last() {
        out.push_str("## Latest frame\n\n");
        out.push_str(&format!("- Active area: `{}`\n", latest.active_area));
        out.push_str(&format!(
            "- Player pos: ({:.2}, {:.2})\n",
            latest.player.pos.x, latest.player.pos.y
        ));
        out.push_str(&format!(
            "- Player vel: ({:.2}, {:.2})\n",
            latest.player.vel.x, latest.player.vel.y
        ));
        out.push_str(&format!(
            "- Player AABB: ({:.1}, {:.1}) → ({:.1}, {:.1})\n",
            latest.player.aabb.min.x,
            latest.player.aabb.min.y,
            latest.player.aabb.max.x,
            latest.player.aabb.max.y
        ));
        out.push_str(&format!(
            "- Last safe pos: ({:.2}, {:.2})\n",
            latest.player.last_safe_pos.x, latest.player.last_safe_pos.y
        ));
        out.push_str(&format!(
            "- Locomotion: `{}`  Body: `{}`\n",
            latest.player.locomotion, latest.player.body_mode
        ));
        out.push_str(&format!("- on_ground: {}\n", latest.player.on_ground));
        out.push_str(&format!("- on_wall: {}\n", latest.player.on_wall));
        out.push_str(&format!(
            "- World size: ({:.0}, {:.0})\n",
            latest.world_size.x, latest.world_size.y
        ));
        out.push('\n');

        if !latest.nearby_collision.is_empty() {
            out.push_str("## Nearby collision (around latest pos)\n\n");
            for shape in latest.nearby_collision.iter().take(16) {
                out.push_str(&format!(
                    "- `{}` `{}` ({:.2}, {:.2}) → ({:.2}, {:.2}) — d={:.1}\n",
                    shape.kind,
                    shape.name,
                    shape.aabb.min.x,
                    shape.aabb.min.y,
                    shape.aabb.max.x,
                    shape.aabb.max.y,
                    shape.distance,
                ));
            }
            out.push('\n');
        }
    }

    let oob_first = payload
        .events
        .iter()
        .find(|e| matches!(e, GameplayTraceEvent::OobDetected { .. }));
    if let Some(GameplayTraceEvent::OobDetected { tick, reason, pos }) = oob_first {
        out.push_str("## First OOB event in window\n\n");
        out.push_str(&format!(
            "- tick {tick}: `{reason}` at ({:.2}, {:.2})\n\n",
            pos.x, pos.y
        ));
    }

    out.push_str(&format!(
        "## Frames (last {} of {})\n\n",
        payload.frames.len().min(MARKDOWN_FRAME_SUMMARY_TAIL),
        payload.frames.len()
    ));
    let frames_tail_start = payload
        .frames
        .len()
        .saturating_sub(MARKDOWN_FRAME_SUMMARY_TAIL);
    for f in &payload.frames[frames_tail_start..] {
        out.push_str(&format!(
            "- t={:>5} pos=({:>7.1},{:>7.1}) vel=({:>7.1},{:>7.1}) gnd={} loco={} body={} dt={:.4} ts={:.2}\n",
            f.tick,
            f.player.pos.x,
            f.player.pos.y,
            f.player.vel.x,
            f.player.vel.y,
            f.player.on_ground,
            f.player.locomotion,
            f.player.body_mode,
            f.real_dt,
            f.time_scale,
        ));
    }
    out.push('\n');

    out.push_str(&format!(
        "## Events (last {} of {})\n\n",
        payload.events.len().min(MARKDOWN_EVENT_TAIL),
        payload.events.len()
    ));
    let events_tail_start = payload.events.len().saturating_sub(MARKDOWN_EVENT_TAIL);
    for ev in &payload.events[events_tail_start..] {
        out.push_str(&format!("- t={:>5} `{}` :: ", ev.tick(), ev.label()));
        match ev {
            GameplayTraceEvent::InputEdge { action, .. } => out.push_str(action),
            GameplayTraceEvent::PlayerModeChanged { from, to, .. } => {
                out.push_str(&format!("{from} → {to}"));
            }
            GameplayTraceEvent::Jump { .. } => out.push_str("jump"),
            GameplayTraceEvent::DoubleJump { .. } => out.push_str("double jump"),
            GameplayTraceEvent::Dash { .. } => out.push_str("dash"),
            GameplayTraceEvent::Blink {
                from,
                to,
                precision,
                ..
            } => out.push_str(&format!(
                "({:.1},{:.1}) → ({:.1},{:.1}) precision={}",
                from.x, from.y, to.x, to.y, precision
            )),
            GameplayTraceEvent::Attack { kind, .. } => out.push_str(kind),
            GameplayTraceEvent::Damage { source, amount, .. } => {
                out.push_str(&format!("{source} {amount}"));
            }
            GameplayTraceEvent::RoomTransition { from, to, .. } => {
                out.push_str(&format!("{from} → {to}"));
            }
            GameplayTraceEvent::OobDetected { reason, pos, .. } => {
                out.push_str(&format!("{reason} @ ({:.1},{:.1})", pos.x, pos.y));
            }
            GameplayTraceEvent::CollisionCorrection {
                before,
                after,
                reason,
                ..
            } => out.push_str(&format!(
                "({:.1},{:.1}) → ({:.1},{:.1}) [{reason}]",
                before.x, before.y, after.x, after.y
            )),
            GameplayTraceEvent::Sfx { label, .. } | GameplayTraceEvent::Vfx { label, .. } => {
                out.push_str(label)
            }
            GameplayTraceEvent::Reset { .. } => out.push_str("reset"),
            GameplayTraceEvent::Death { .. } => out.push_str("death"),
        }
        out.push('\n');
    }
    out.push('\n');

    out.push_str("## Hints\n\n");
    out.push_str("- Compare `last_safe_pos` to the OOB position; the difference\n");
    out.push_str("  hints at whether the player tunneled, blinked, or fell.\n");
    out.push_str("- Look for the latest `Blink` / `Dash` / `RoomTransition` event\n");
    out.push_str("  before the OOB. Tunneling under a one-way platform is the\n");
    out.push_str("  most common cause of the active OOB bug.\n");
    out.push_str("- This trace is *not* a deterministic replay yet: timing\n");
    out.push_str("  jitter and audio/VFX subscribers can vary across runs.\n");
    out
}

/// Default dump directory relative to the current working directory.
pub fn default_dump_dir() -> PathBuf {
    PathBuf::from("debug_traces")
}

/// SystemParam-friendly bundle: gives `sandbox_update` everything it
/// needs to record one frame and (if requested) write a dump.
#[allow(clippy::too_many_arguments)]
pub fn record_simulation_frame(
    buffer: &mut GameplayTraceBuffer,
    runtime: &SandboxRuntime,
    world: &ae::World,
    controls: ControlFrame,
    real_dt: f32,
    sim_dt: f32,
    game_mode: &str,
    active_area: &str,
    locomotion: &str,
    body_mode: &str,
) {
    let oob = detect_oob(&runtime.player, world, OOB_MARGIN);
    let frame = build_frame(
        runtime,
        world,
        controls,
        real_dt,
        sim_dt,
        game_mode,
        active_area,
        buffer.sequence,
        buffer.tick,
        locomotion,
        body_mode,
    );
    record_frame(buffer, frame, oob.as_ref());
}

/// Bevy system: drains pending dump requests, writes JSON+MD if any.
/// Ordered after `sandbox_update` so manual F8 presses recorded earlier
/// in the frame still see the latest snapshot.
pub fn flush_pending_dump(mut buffer: ResMut<GameplayTraceBuffer>) {
    let Some(reason) = buffer.dump_request.take() else {
        return;
    };
    let dir = default_dump_dir();
    match write_dump(&buffer, &reason, &dir) {
        Ok(path) => {
            let path_str = path.to_string_lossy().to_string();
            buffer.last_dump_path = Some(path_str.clone());
            buffer.last_dump_status = Some(format!("OK: {path_str}"));
            eprintln!("ambition trace dumped: {path_str}");
        }
        Err(err) => {
            buffer.last_dump_status = Some(format!("error: {err}"));
            eprintln!("ambition trace dump failed: {err}");
        }
    }
}

/// Presentation-side hotkey reader: F8 sets a manual dump request.
/// Lives in `trace.rs` rather than `app.rs` so the lookup is grep-able
/// near the rest of the recorder code.
pub fn handle_trace_hotkey(
    keys: Res<ButtonInput<KeyCode>>,
    mut buffer: ResMut<GameplayTraceBuffer>,
) {
    if keys.just_pressed(KeyCode::F8) {
        buffer.request_dump(DumpReason::Manual);
    }
}

/// Bevy system: when in scope, writes one trace frame per Update tick by
/// reading the resources `sandbox_update` already consumes. We keep this
/// outside the phase pipeline so the recorder stays out of `sandbox_update`'s
/// 16-system-param budget.
pub fn record_frame_system(
    mut buffer: ResMut<GameplayTraceBuffer>,
    runtime: Res<SandboxRuntime>,
    world: Res<GameWorld>,
    control_frame: Res<ControlFrame>,
    time: Res<Time>,
    rooms: Option<Res<crate::rooms::RoomSet>>,
    mode: Res<State<crate::game_mode::GameMode>>,
) {
    let real_dt = time.delta_secs();
    let sim_dt = real_dt * runtime.time_scale;
    let active_area = rooms
        .as_ref()
        .map(|r| r.active_spec().id.clone())
        .unwrap_or_else(|| "<unknown>".into());
    let mode_label = format!("{:?}", mode.get());
    let locomotion = ae::LocomotionState::from_player(&runtime.player)
        .label()
        .to_string();
    let body_mode = ae::BodyMode::from_player(&runtime.player)
        .label()
        .to_string();
    record_simulation_frame(
        &mut buffer,
        &runtime,
        &world.0,
        *control_frame,
        real_dt,
        sim_dt,
        &mode_label,
        &active_area,
        &locomotion,
        &body_mode,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ae::{Block, World};

    fn dummy_world() -> World {
        let blocks = vec![Block::solid(
            "floor",
            ae::Vec2::new(0.0, 100.0),
            ae::Vec2::new(200.0, 20.0),
        )];
        World::new(
            "test",
            ae::Vec2::new(200.0, 200.0),
            ae::Vec2::new(50.0, 50.0),
            blocks,
        )
    }

    fn dummy_player(at: ae::Vec2) -> ae::Player {
        ae::Player::new(at)
    }

    #[test]
    fn ring_buffer_caps_at_capacity() {
        let mut buf = GameplayTraceBuffer::with_capacity(4, 4);
        for i in 0..10 {
            buf.push_frame(GameplayTraceFrame {
                seq: i,
                tick: i,
                real_dt: 0.016,
                sim_dt: 0.016,
                time_scale: 1.0,
                game_mode: "Playing".into(),
                active_area: "test".into(),
                world_size: TracePoint::default(),
                world_spawn: TracePoint::default(),
                player: PlayerTraceState {
                    pos: TracePoint::default(),
                    vel: TracePoint::default(),
                    size: TracePoint::default(),
                    aabb: TraceAabb::default(),
                    facing: 1.0,
                    on_ground: false,
                    on_wall: false,
                    wall_clinging: false,
                    wall_climbing: false,
                    fast_falling: false,
                    fly_enabled: false,
                    dash_charges_available: 0,
                    air_jumps_available: 0,
                    blink_aiming: false,
                    blink_grace_timer: 0.0,
                    locomotion: "Airborne".into(),
                    body_mode: "Standing".into(),
                    last_safe_pos: TracePoint::default(),
                    time_alive: 0.0,
                    resets: 0,
                },
                controls: ControlFrameTrace::default(),
                nearby_collision: Vec::new(),
                moving_platforms: Vec::new(),
            });
        }
        assert_eq!(buf.frame_count(), 4, "wraparound should cap at capacity");
        // Earliest preserved frame should be the 6th pushed (index 6).
        let first = buf.frames.front().expect("non-empty");
        assert_eq!(first.tick, 6);
        let last = buf.frames.back().expect("non-empty");
        assert_eq!(last.tick, 9);
    }

    #[test]
    fn detect_oob_inside_world_returns_none() {
        let world = dummy_world();
        let player = dummy_player(ae::Vec2::new(50.0, 50.0));
        assert!(detect_oob(&player, &world, OOB_MARGIN).is_none());
    }

    #[test]
    fn detect_oob_outside_envelope_x() {
        let world = dummy_world();
        // Place player far to the right of world envelope + margin.
        let player = dummy_player(ae::Vec2::new(2000.0, 50.0));
        match detect_oob(&player, &world, OOB_MARGIN) {
            Some(OobReason::OutsideWorldEnvelope { axis }) => assert_eq!(axis, 'x'),
            other => panic!("expected OutsideWorldEnvelope x, got {other:?}"),
        }
    }

    #[test]
    fn detect_oob_outside_envelope_y() {
        let world = dummy_world();
        let player = dummy_player(ae::Vec2::new(50.0, -2000.0));
        match detect_oob(&player, &world, OOB_MARGIN) {
            Some(OobReason::OutsideWorldEnvelope { axis }) => assert_eq!(axis, 'y'),
            other => panic!("expected OutsideWorldEnvelope y, got {other:?}"),
        }
    }

    #[test]
    fn detect_oob_inside_solid() {
        let world = dummy_world();
        // Floor is at (0,100)-(200,120). Place player center in floor.
        let player = dummy_player(ae::Vec2::new(100.0, 110.0));
        match detect_oob(&player, &world, OOB_MARGIN) {
            Some(OobReason::InsideSolid { block_name }) => assert_eq!(block_name, "floor"),
            other => panic!("expected InsideSolid, got {other:?}"),
        }
    }

    #[test]
    fn detect_oob_position_non_finite() {
        let world = dummy_world();
        let mut player = dummy_player(ae::Vec2::new(50.0, 50.0));
        player.pos = ae::Vec2::new(f32::NAN, 0.0);
        assert!(matches!(
            detect_oob(&player, &world, OOB_MARGIN),
            Some(OobReason::PositionNonFinite)
        ));
    }

    #[test]
    fn detect_oob_absurd_velocity() {
        let world = dummy_world();
        let mut player = dummy_player(ae::Vec2::new(50.0, 50.0));
        player.vel = ae::Vec2::new(1.0e6, 0.0);
        assert!(matches!(
            detect_oob(&player, &world, OOB_MARGIN),
            Some(OobReason::AbsurdVelocity { .. })
        ));
    }

    #[test]
    fn dump_paths_does_not_panic_and_is_unique_per_label() {
        let dir = Path::new("/tmp/nope");
        let (a_json, a_md) = dump_paths(dir, "label_a");
        let (b_json, b_md) = dump_paths(dir, "label_b");
        assert!(a_json.to_string_lossy().ends_with("label_a.json"));
        assert!(a_md.to_string_lossy().ends_with("label_a.md"));
        assert_ne!(a_json, b_json);
        assert_ne!(a_md, b_md);
    }

    #[test]
    fn timestamp_label_changes_with_time() {
        // Construct two SystemTimes one second apart.
        let a = UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
        let b = UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_001);
        assert_ne!(timestamp_label(a), timestamp_label(b));
    }

    #[test]
    fn record_frame_with_oob_pushes_event_and_requests_dump() {
        let mut buf = GameplayTraceBuffer::with_capacity(8, 8);
        let world = dummy_world();
        let mut player = dummy_player(ae::Vec2::new(50.0, 50.0));
        player.pos = ae::Vec2::new(2000.0, 50.0); // outside envelope.x
        let frame = build_frame(
            &SandboxRuntime {
                player: player.clone(),
                player_health: ae::Health::new(5),
                debug: false,
                slowmo: false,
                presets: crate::input::KeyboardPreset::presets().to_vec(),
                preset_index: 0,
                preset_flash: 0.0,
                flash_timer: 0.0,
                hitstop_timer: 0.0,
                damage_invuln_timer: 0.0,
                hitstun_timer: 0.0,
                last_safe_player_pos: ae::Vec2::ZERO,
                time_scale: 1.0,
                down_tap_timer: 0.0,
                up_tap_timer: 0.0,
                interact_buffer_timer: 0.0,
                moving_platform: crate::platforms::MovingPlatformState::time_reference(&world),
                features: crate::features::FeatureRuntime::from_world(&world),
                dialogue: crate::dialog::DialogState::default(),
                physics_settings: crate::physics::PhysicsSandboxSettings::default(),
                room_transition_cooldown: 0.0,
                slash_anim_timer: 0.0,
            },
            &world,
            ControlFrame::default(),
            0.016,
            0.016,
            "Playing",
            "test",
            0,
            0,
            "Airborne",
            "Standing",
        );
        let oob = detect_oob(&player, &world, OOB_MARGIN);
        record_frame(&mut buf, frame, oob.as_ref());
        assert_eq!(buf.frame_count(), 1);
        assert_eq!(buf.event_count(), 1, "OOB event should be pushed");
        assert!(matches!(buf.dump_request, Some(DumpReason::OobAuto { .. })));
    }

    #[test]
    fn write_dump_writes_two_files() {
        let mut buf = GameplayTraceBuffer::with_capacity(4, 4);
        let world = dummy_world();
        let player = dummy_player(ae::Vec2::new(50.0, 50.0));
        let frame = build_frame(
            &SandboxRuntime {
                player: player.clone(),
                player_health: ae::Health::new(5),
                debug: false,
                slowmo: false,
                presets: crate::input::KeyboardPreset::presets().to_vec(),
                preset_index: 0,
                preset_flash: 0.0,
                flash_timer: 0.0,
                hitstop_timer: 0.0,
                damage_invuln_timer: 0.0,
                hitstun_timer: 0.0,
                last_safe_player_pos: ae::Vec2::ZERO,
                time_scale: 1.0,
                down_tap_timer: 0.0,
                up_tap_timer: 0.0,
                interact_buffer_timer: 0.0,
                moving_platform: crate::platforms::MovingPlatformState::time_reference(&world),
                features: crate::features::FeatureRuntime::from_world(&world),
                dialogue: crate::dialog::DialogState::default(),
                physics_settings: crate::physics::PhysicsSandboxSettings::default(),
                room_transition_cooldown: 0.0,
                slash_anim_timer: 0.0,
            },
            &world,
            ControlFrame::default(),
            0.016,
            0.016,
            "Playing",
            "test",
            0,
            0,
            "Airborne",
            "Standing",
        );
        record_frame(&mut buf, frame, None);
        let dir = std::env::temp_dir().join("ambition_trace_test_dump");
        let _ = std::fs::remove_dir_all(&dir);
        let json_path = write_dump(&buf, &DumpReason::Manual, &dir).expect("write dump");
        assert!(json_path.exists());
        let md_path = json_path.with_extension("md");
        assert!(md_path.exists());
        let json_body = std::fs::read_to_string(&json_path).unwrap();
        assert!(json_body.contains("\"schema_version\": 1"));
        assert!(json_body.contains("\"dump_reason\""));
    }
}
