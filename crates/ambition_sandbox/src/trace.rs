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
//! Dumps are written to
//! `debug_traces/ambition_trace_{secs}-{nanos}-{seq}_{Dd}d{HH}h{MM}m{SS}s.{json,md}`
//! relative to the sandbox working directory. The `{secs}-{nanos}-{seq}`
//! prefix makes filenames unique even when two dumps fire in the same
//! nanosecond (the `{seq}` segment is a process-wide atomic counter).
//! JSON is the machine-readable source of truth; the Markdown file is a
//! human summary built from the same snapshot. Path generation is
//! offline-safe (no system calls).
//!
//! See `docs/gameplay_trace_recorder.md` for the workflow and bug-reporting
//! checklist.

use std::collections::VecDeque;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Process-wide monotonically increasing sequence appended to dump
/// filenames so two dumps requested in the same nanosecond cannot
/// collide. Uses `Relaxed` ordering because the value's only purpose
/// is uniqueness within the running process — there are no
/// happens-before relationships to preserve.
static DUMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn next_dump_sequence() -> u64 {
    DUMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
}

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
    pub aabb: TraceAabb,
    /// Direction of travel along the platform's authored path, +1 or -1.
    /// Used by the trace to spot platform-related OOB / tunneling
    /// patterns (e.g. an OOB always coincides with the platform at the
    /// far end of its sweep).
    pub direction: f32,
    /// True if the player is currently riding this platform per
    /// `MovingPlatformState::is_riding`.
    pub player_riding: bool,
    /// Distance from player center to platform center in world units.
    pub player_distance: f32,
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
    /// Projectile lifecycle event (player fireball / Hadouken). The
    /// `event` string is one of `fired`, `blocked_by_resource`, `hit`,
    /// `expired` so trace consumers can grep by phase.
    Projectile {
        tick: u64,
        kind: String,
        event: String,
        damage: i32,
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
            | GameplayTraceEvent::Death { tick }
            | GameplayTraceEvent::Projectile { tick, .. } => *tick,
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
            GameplayTraceEvent::Projectile { .. } => "Projectile",
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

/// Snapshot of the gameplay state we diff against on each tick to
/// synthesize per-frame events without threading a Vec collector
/// through every `sandbox_update` phase. Stored on the buffer so the
/// recorder is the single owner of trace state.
///
/// `fly_enabled` and `fast_falling` are recorded for future event
/// detection (e.g. flight toggles, fast-fall edges) — they are
/// captured now so the snapshot shape stays stable as we add more
/// diffs.
#[derive(Clone, Debug)]
#[allow(dead_code)]
struct PreviousFrameSnapshot {
    pos: ae::Vec2,
    vel: ae::Vec2,
    on_ground: bool,
    fly_enabled: bool,
    blink_aiming: bool,
    blink_grace_timer: f32,
    fast_falling: bool,
    dash_charges_available: u8,
    air_jumps_available: u8,
    resets: u32,
    hp_current: i32,
    locomotion: ae::LocomotionState,
    body_mode: ae::BodyMode,
    active_area: String,
    controls: ControlFrame,
}

/// If the per-frame position delta exceeds the maximum movement we'd
/// expect from the player's velocity (plus a small slack), the
/// recorder treats it as a teleport / collision correction and emits
/// a `CollisionCorrection` event. This catches the active OOB bug
/// where the player teleports from a wall-cling position to an
/// out-of-world ledge with no input change.
const TELEPORT_DETECTION_SLACK_PX: f32 = 16.0;

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
    /// Frame-to-frame diff source for synthetic events.
    previous: Option<PreviousFrameSnapshot>,
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
            previous: None,
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
///
/// The world envelope / inside-solid check is delegated to
/// `ae::classify_player_safety` so the trace recorder and
/// `SandboxRuntime::remember_safe_player_position` use the same
/// definition. The recorder layers the trace-only "absurd velocity"
/// rule on top.
pub fn detect_oob(player: &ae::Player, world: &ae::World, margin: f32) -> Option<OobReason> {
    let speed = player.vel.length();
    if speed.is_finite() && speed > ABSURD_VELOCITY_MAGNITUDE {
        return Some(OobReason::AbsurdVelocity { magnitude: speed });
    }
    match ae::classify_player_safety(player, world, margin, |b| {
        matches!(b.kind, ae::BlockKind::Solid)
    }) {
        ae::PlayerSafetyVerdict::Safe => None,
        ae::PlayerSafetyVerdict::PositionNonFinite => Some(OobReason::PositionNonFinite),
        ae::PlayerSafetyVerdict::VelocityNonFinite => Some(OobReason::VelocityNonFinite),
        ae::PlayerSafetyVerdict::OutsideWorldEnvelope { axis } => {
            Some(OobReason::OutsideWorldEnvelope { axis })
        }
        ae::PlayerSafetyVerdict::InsideSolid => {
            // Find which block we're inside so the dump names it. The
            // shared classifier doesn't return the block reference (it
            // takes a predicate closure to stay engine-side); a small
            // second walk here is fine for the "we're already in trouble"
            // path.
            let aabb = player.aabb();
            let block_name = world
                .blocks
                .iter()
                .find(|b| matches!(b.kind, ae::BlockKind::Solid) && aabb.strict_intersects(b.aabb))
                .map(|b| b.name.clone())
                .unwrap_or_else(|| "<unknown>".into());
            Some(OobReason::InsideSolid { block_name })
        }
    }
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
#[allow(clippy::too_many_arguments)]
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
        moving_platforms: build_moving_platform_states(runtime),
    }
}

/// Snapshot the active moving platforms into trace shapes. Today the
/// sandbox owns exactly one moving platform on `runtime.moving_platform`;
/// the function returns a `Vec` so future patches that add more
/// platforms (or move them onto entity components) can append entries
/// without changing the trace schema.
fn build_moving_platform_states(runtime: &SandboxRuntime) -> Vec<MovingPlatformTraceState> {
    let p = &runtime.moving_platform;
    let aabb = p.aabb();
    let player_distance = (runtime.player.pos - p.pos).length();
    vec![MovingPlatformTraceState {
        pos: p.pos.into(),
        size: p.size.into(),
        aabb: aabb.into(),
        direction: p.direction(),
        player_riding: p.is_riding(&runtime.player),
        player_distance,
    }]
}

/// Diff the current player+control state against the previous snapshot
/// and synthesize gameplay events. The buffer is the single owner of
/// trace state so this stays alongside `record_frame`.
///
/// Events emitted, in order:
///
/// 1. `RoomTransition` (if `active_area` changed),
/// 2. `Reset` (if `player.resets` increased),
/// 3. `CollisionCorrection` for unexplained position deltas — i.e.
///    deltas larger than what the recent velocity could produce. This
///    catches teleports that aren't covered by `Reset` /
///    `RoomTransition`.
/// 4. `LocomotionChanged`,
/// 5. `Dash`, `DoubleJump`, `Jump` (heuristics from charge / vel deltas),
/// 6. `Blink` start / fail,
/// 7. `Damage` (HP delta),
/// 8. `InputEdge` for newly-pressed buttons.
///
/// The recorder is intentionally a passive observer. Sandbox phases
/// can still push richer events directly via `buffer.push_event` if
/// they have non-state-derivable info (e.g. "pogo missed because
/// target was a non-pogo block"), but the diff gives us a useful
/// timeline without touching every phase helper.
fn synthesize_events_from_diff(
    buffer: &mut GameplayTraceBuffer,
    runtime: &SandboxRuntime,
    controls: ControlFrame,
    real_dt: f32,
    active_area: &str,
    locomotion: ae::LocomotionState,
    body_mode: ae::BodyMode,
) {
    let Some(prev) = buffer.previous.clone() else {
        return;
    };
    let tick = buffer.tick;
    let player = &runtime.player;
    let cur_pos = player.pos;
    let cur_vel = player.vel;

    let mut suppressed_teleport = false;

    if prev.active_area != active_area {
        buffer.push_event(GameplayTraceEvent::RoomTransition {
            tick,
            from: prev.active_area.clone(),
            to: active_area.into(),
        });
        suppressed_teleport = true;
    }

    if player.resets > prev.resets {
        buffer.push_event(GameplayTraceEvent::Reset { tick });
        suppressed_teleport = true;
    }

    // Position-delta vs velocity-budget check. Catches teleports that
    // aren't covered by Reset / RoomTransition. This is the OOB-debug
    // smoking-gun event: a 1500-px jump in one tick will surface here.
    let dpos = cur_pos - prev.pos;
    let dlen = dpos.length();
    let max_speed = prev.vel.length().max(cur_vel.length());
    let budget = max_speed * real_dt.max(0.0) + TELEPORT_DETECTION_SLACK_PX;
    if !suppressed_teleport && dlen > budget && dlen > TELEPORT_DETECTION_SLACK_PX {
        buffer.push_event(GameplayTraceEvent::CollisionCorrection {
            tick,
            before: prev.pos.into(),
            after: cur_pos.into(),
            reason: format!(
                "unexplained delta {:.1}px (vel-budget {:.1}px)",
                dlen, budget
            ),
        });
    }

    if prev.locomotion != locomotion {
        buffer.push_event(GameplayTraceEvent::PlayerModeChanged {
            tick,
            from: prev.locomotion.label().into(),
            to: locomotion.label().into(),
        });
    }
    if prev.body_mode != body_mode {
        buffer.push_event(GameplayTraceEvent::PlayerModeChanged {
            tick,
            from: format!("body:{}", prev.body_mode.label()),
            to: format!("body:{}", body_mode.label()),
        });
    }

    if player.dash_charges_available < prev.dash_charges_available {
        buffer.push_event(GameplayTraceEvent::Dash { tick });
    }
    if player.air_jumps_available < prev.air_jumps_available {
        buffer.push_event(GameplayTraceEvent::DoubleJump { tick });
    } else if !prev.on_ground && cur_vel.y < prev.vel.y - 50.0 && controls.jump_pressed {
        // Jump-edge heuristic: y velocity went meaningfully more
        // negative (Ambition's screen-space +y is down so upward
        // jumps make vel.y decrease) on a frame where the player
        // pressed jump, while the player was airborne.
        buffer.push_event(GameplayTraceEvent::Jump { tick });
    } else if prev.on_ground && !player.on_ground && controls.jump_pressed && cur_vel.y < 0.0 {
        buffer.push_event(GameplayTraceEvent::Jump { tick });
    }

    if !prev.blink_aiming && player.blink_aiming {
        buffer.push_event(GameplayTraceEvent::Blink {
            tick,
            from: prev.pos.into(),
            to: cur_pos.into(),
            precision: false,
        });
    }
    // Blink-fired heuristic: blink_grace_timer just became positive,
    // which the engine sets after a successful blink commit.
    if prev.blink_grace_timer <= 0.0 && player.blink_grace_timer > 0.0 {
        buffer.push_event(GameplayTraceEvent::Blink {
            tick,
            from: prev.pos.into(),
            to: cur_pos.into(),
            precision: true,
        });
    }

    if runtime.player_health.current < prev.hp_current {
        let amount = (prev.hp_current - runtime.player_health.current).max(0);
        buffer.push_event(GameplayTraceEvent::Damage {
            tick,
            source: "feature".into(),
            amount,
        });
        if runtime.player_health.current <= 0 {
            buffer.push_event(GameplayTraceEvent::Death { tick });
        }
    }

    if controls.attack_pressed && !prev.controls.attack_pressed {
        buffer.push_event(GameplayTraceEvent::Attack {
            tick,
            kind: "slash".into(),
        });
    }
    if controls.pogo_pressed && !prev.controls.pogo_pressed {
        buffer.push_event(GameplayTraceEvent::Attack {
            tick,
            kind: "pogo".into(),
        });
    }

    // Input edges for the bool fields the player can newly press this
    // frame. We compare against the previous frame's `controls` so a
    // genuine press → release → press in one tick still records the
    // press (the previous frame's value was already false).
    let pairs: &[(&str, bool, bool)] = &[
        ("Jump", controls.jump_pressed, prev.controls.jump_pressed),
        ("Dash", controls.dash_pressed, prev.controls.dash_pressed),
        ("Blink", controls.blink_pressed, prev.controls.blink_pressed),
        ("Up", controls.up_pressed, prev.controls.up_pressed),
        ("Down", controls.down_pressed, prev.controls.down_pressed),
        (
            "Attack",
            controls.attack_pressed,
            prev.controls.attack_pressed,
        ),
        ("Pogo", controls.pogo_pressed, prev.controls.pogo_pressed),
        (
            "Interact",
            controls.interact_pressed,
            prev.controls.interact_pressed,
        ),
        ("Reset", controls.reset_pressed, prev.controls.reset_pressed),
        ("Start", controls.start_pressed, prev.controls.start_pressed),
        (
            "FlyToggle",
            controls.fly_toggle_pressed,
            prev.controls.fly_toggle_pressed,
        ),
        (
            "FastFall",
            controls.fast_fall_pressed,
            prev.controls.fast_fall_pressed,
        ),
    ];
    for (label, cur, prev_v) in pairs {
        if *cur && !*prev_v {
            buffer.push_event(GameplayTraceEvent::InputEdge {
                tick,
                action: (*label).into(),
            });
        }
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

/// Format a unique, lexically-sortable label for a dump filename.
///
/// Format: `{secs:010}-{nanos:09}-{seq:06}_{Dd}d{HH}h{MM}m{SS}s`.
/// The `seq` segment is a process-wide atomic counter, so two dumps
/// taken in the same nanosecond still get distinct paths. Lexical
/// order matches chronological order so `ls -1` lists dumps in the
/// order they were taken.
fn timestamp_label_with_seq(ts: SystemTime, seq: u64) -> String {
    let dur = ts.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs();
    let nanos = dur.subsec_nanos();
    let total_minutes = secs / 60;
    let seconds = secs % 60;
    let total_hours = total_minutes / 60;
    let minutes = total_minutes % 60;
    let total_days = total_hours / 24;
    let hours = total_hours % 24;
    format!(
        "{secs:010}-{nanos:09}-{seq:06}_{}d{:02}h{:02}m{:02}s",
        total_days, hours, minutes, seconds
    )
}

/// Convenience wrapper used by `write_dump`: pulls a fresh sequence
/// counter and formats `ts` against it. Tests can call
/// `timestamp_label_with_seq` directly with explicit sequences to
/// pin behavior.
fn timestamp_label(ts: SystemTime) -> String {
    timestamp_label_with_seq(ts, next_dump_sequence())
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
            GameplayTraceEvent::Projectile {
                kind,
                event,
                damage,
                ..
            } => {
                out.push_str(&format!("{kind} {event} dmg={damage}"));
            }
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

/// Replace the diff snapshot with the just-recorded frame's state.
/// Caller drives this after `record_simulation_frame` so the next
/// tick's `synthesize_events_from_diff` sees an up-to-date baseline.
pub(crate) fn update_previous_snapshot(
    buffer: &mut GameplayTraceBuffer,
    runtime: &SandboxRuntime,
    controls: ControlFrame,
    active_area: &str,
    locomotion: ae::LocomotionState,
    body_mode: ae::BodyMode,
) {
    let player = &runtime.player;
    buffer.previous = Some(PreviousFrameSnapshot {
        pos: player.pos,
        vel: player.vel,
        on_ground: player.on_ground,
        fly_enabled: player.fly_enabled,
        blink_aiming: player.blink_aiming,
        blink_grace_timer: player.blink_grace_timer,
        fast_falling: player.fast_falling,
        dash_charges_available: player.dash_charges_available,
        air_jumps_available: player.air_jumps_available,
        resets: player.resets,
        hp_current: runtime.player_health.current,
        locomotion,
        body_mode,
        active_area: active_area.into(),
        controls,
    });
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
/// 16-system-param budget. Synthesizes per-frame events by diffing
/// against the previous tick's snapshot (input edges, locomotion
/// changes, dash/jump/blink heuristics, room transitions, resets,
/// damage, and unexplained position deltas).
///
/// The trace's collision view (`nearby_collision`, `detect_oob`'s
/// inside-solid check) uses the same `world_with_sandbox_solids` view
/// that `sandbox_update` feeds to the engine. Without that, the trace
/// would miss feature-runtime solids the player can collide with —
/// which is exactly what happened in the May 2026 wall-cling teleport
/// trace, where `nearby_collision` was empty even though the player
/// was clinging to a wall.
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
    let locomotion_state = ae::LocomotionState::from_player(&runtime.player);
    let body_mode_state = ae::BodyMode::from_player(&runtime.player);
    let locomotion = locomotion_state.label().to_string();
    let body_mode = body_mode_state.label().to_string();

    let augmented_world = crate::features::world_with_sandbox_solids(
        &world.0,
        &runtime.moving_platform,
        &runtime.features,
    );

    // Synthesize events from the diff before pushing the frame so the
    // event tick aligns with the frame the user will see in the dump.
    synthesize_events_from_diff(
        &mut buffer,
        &runtime,
        *control_frame,
        real_dt,
        &active_area,
        locomotion_state,
        body_mode_state,
    );

    record_simulation_frame(
        &mut buffer,
        &runtime,
        &augmented_world,
        *control_frame,
        real_dt,
        sim_dt,
        &mode_label,
        &active_area,
        &locomotion,
        &body_mode,
    );

    // Update the diff snapshot AFTER recording so the next tick's
    // `synthesize_events_from_diff` can compare against this frame's
    // state. Setting it after `record_simulation_frame` also means a
    // panic / early return upstream leaves the previous snapshot in
    // place rather than corrupting the timeline.
    update_previous_snapshot(
        &mut buffer,
        &runtime,
        *control_frame,
        &active_area,
        locomotion_state,
        body_mode_state,
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
                ledge_grab: None,
                double_tap_down_pending: false,
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
                ledge_grab: None,
                double_tap_down_pending: false,
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

    /// P1 — `timestamp_label` calls in quick succession (same nanosecond
    /// or not) must produce distinct strings, because the atomic
    /// sequence counter is appended.
    #[test]
    fn timestamp_label_unique_in_tight_loop() {
        let now = SystemTime::now();
        // Use a fixed `ts` so the seconds/nanoseconds segments do not
        // change between calls; the only differentiator left is the
        // atomic sequence.
        let labels: Vec<String> = (0..32).map(|_| timestamp_label(now)).collect();
        let unique: std::collections::HashSet<&String> = labels.iter().collect();
        assert_eq!(
            unique.len(),
            labels.len(),
            "all dump labels in a tight loop must be unique; got {labels:?}"
        );
    }

    /// P1 — `timestamp_label_with_seq` lets tests pin a sequence value
    /// for stable expectations. Two distinct sequences must produce
    /// different strings even when `ts` is identical.
    #[test]
    fn timestamp_label_with_seq_is_stable_per_seq() {
        let now = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_777_902_031);
        let a = timestamp_label_with_seq(now, 0);
        let b = timestamp_label_with_seq(now, 1);
        assert_ne!(a, b);
        // Same inputs produce same output.
        assert_eq!(a, timestamp_label_with_seq(now, 0));
    }

    fn make_runtime(world: &ae::World, player: ae::Player) -> SandboxRuntime {
        SandboxRuntime {
            player,
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
            moving_platform: crate::platforms::MovingPlatformState::time_reference(world),
            features: crate::features::FeatureRuntime::from_world(world),
            dialogue: crate::dialog::DialogState::default(),
            physics_settings: crate::physics::PhysicsSandboxSettings::default(),
            room_transition_cooldown: 0.0,
            slash_anim_timer: 0.0,
            ledge_grab: None,
            double_tap_down_pending: false,
        }
    }

    /// P2 — pressing a button that wasn't pressed last frame should
    /// emit an `InputEdge` event. We seed the buffer with an initial
    /// snapshot, then call `synthesize_events_from_diff` directly so
    /// the test doesn't need a full Bevy App.
    #[test]
    fn synthesizes_input_edge_event_on_button_press() {
        let mut buf = GameplayTraceBuffer::with_capacity(16, 16);
        let world = dummy_world();
        let runtime = make_runtime(&world, dummy_player(ae::Vec2::new(50.0, 50.0)));
        // Seed previous snapshot with no buttons pressed.
        update_previous_snapshot(
            &mut buf,
            &runtime,
            ControlFrame::default(),
            "test",
            ae::LocomotionState::Grounded,
            ae::BodyMode::Standing,
        );
        // Player starts pressing Jump this frame.
        let mut controls = ControlFrame::default();
        controls.jump_pressed = true;
        synthesize_events_from_diff(
            &mut buf,
            &runtime,
            controls,
            0.016,
            "test",
            ae::LocomotionState::Grounded,
            ae::BodyMode::Standing,
        );
        let edges: Vec<_> = buf
            .events()
            .filter_map(|e| match e {
                GameplayTraceEvent::InputEdge { action, .. } => Some(action.clone()),
                _ => None,
            })
            .collect();
        assert!(
            edges.iter().any(|a| a == "Jump"),
            "expected Jump InputEdge; got {edges:?}"
        );
    }

    /// P2 — an unexplained position delta (much larger than the velocity
    /// budget) should produce a `CollisionCorrection` event so the
    /// trace surfaces teleports of the kind that landed in
    /// `debug_traces/ambition_trace_1777902031_*.json`.
    #[test]
    fn synthesizes_collision_correction_on_unexplained_teleport() {
        let mut buf = GameplayTraceBuffer::with_capacity(16, 16);
        let world = dummy_world();
        let runtime_prev = make_runtime(&world, dummy_player(ae::Vec2::new(62.0, 1564.0)));
        update_previous_snapshot(
            &mut buf,
            &runtime_prev,
            ControlFrame::default(),
            "square_arena",
            ae::LocomotionState::WallCling,
            ae::BodyMode::Standing,
        );
        // Now jump to a wildly different position with no plausible
        // velocity to explain it. Same active area + same `resets` so
        // the teleport detector isn't suppressed by Reset/RoomTransition.
        let mut player2 = dummy_player(ae::Vec2::new(62.0, -23.0));
        player2.vel = ae::Vec2::ZERO;
        let runtime_cur = make_runtime(&world, player2);
        synthesize_events_from_diff(
            &mut buf,
            &runtime_cur,
            ControlFrame::default(),
            0.0069,
            "square_arena",
            ae::LocomotionState::Grounded,
            ae::BodyMode::Standing,
        );
        let teleports: Vec<_> = buf
            .events()
            .filter(|e| matches!(e, GameplayTraceEvent::CollisionCorrection { .. }))
            .collect();
        assert_eq!(
            teleports.len(),
            1,
            "expected one CollisionCorrection event for the teleport; got {teleports:?}"
        );
    }

    /// P2 — incrementing `player.resets` should emit a `Reset` event
    /// AND suppress the teleport detector (the player position can
    /// legitimately jump to spawn on reset).
    #[test]
    fn reset_emits_event_and_suppresses_teleport_event() {
        let mut buf = GameplayTraceBuffer::with_capacity(16, 16);
        let world = dummy_world();
        let runtime_prev = make_runtime(&world, dummy_player(ae::Vec2::new(50.0, 50.0)));
        update_previous_snapshot(
            &mut buf,
            &runtime_prev,
            ControlFrame::default(),
            "test",
            ae::LocomotionState::Grounded,
            ae::BodyMode::Standing,
        );
        let mut player2 = dummy_player(ae::Vec2::new(150.0, 150.0));
        player2.resets = runtime_prev.player.resets + 1;
        let runtime_cur = make_runtime(&world, player2);
        synthesize_events_from_diff(
            &mut buf,
            &runtime_cur,
            ControlFrame::default(),
            0.016,
            "test",
            ae::LocomotionState::Grounded,
            ae::BodyMode::Standing,
        );
        let resets: Vec<_> = buf
            .events()
            .filter(|e| matches!(e, GameplayTraceEvent::Reset { .. }))
            .collect();
        assert_eq!(resets.len(), 1, "expected one Reset event");
        let teleports: Vec<_> = buf
            .events()
            .filter(|e| matches!(e, GameplayTraceEvent::CollisionCorrection { .. }))
            .collect();
        assert!(
            teleports.is_empty(),
            "Reset should suppress the teleport detector"
        );
    }

    /// P3 — frame snapshots include a populated `moving_platforms` slot
    /// with the active sandbox platform.
    #[test]
    fn frame_includes_moving_platform_state() {
        let world = dummy_world();
        let player = dummy_player(ae::Vec2::new(50.0, 50.0));
        let runtime = make_runtime(&world, player);
        let frame = build_frame(
            &runtime,
            &world,
            ControlFrame::default(),
            0.016,
            0.016,
            "Playing",
            "test",
            0,
            0,
            "Grounded",
            "Standing",
        );
        assert_eq!(
            frame.moving_platforms.len(),
            1,
            "expected one moving-platform entry per frame"
        );
        let platform = &frame.moving_platforms[0];
        assert!(platform.size.x > 0.0);
        assert!(platform.size.y > 0.0);
        assert!(platform.player_distance > 0.0);
    }

    /// P4 — `BodyMode::from_player` reads `player.body_mode` (the
    /// authoritative field). Default is `Standing`; setting the field
    /// changes what the recorder/HUD see.
    #[test]
    fn body_mode_reads_authoritative_field() {
        let mut player = dummy_player(ae::Vec2::ZERO);
        assert_eq!(ae::BodyMode::from_player(&player), ae::BodyMode::Standing);
        player.body_mode = ae::BodyMode::MorphBall;
        assert_eq!(ae::BodyMode::from_player(&player), ae::BodyMode::MorphBall);
    }
}
