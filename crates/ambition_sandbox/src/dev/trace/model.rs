use super::*;

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
    pub(super) fn tick(&self) -> u64 {
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

    pub(super) fn label(&self) -> &'static str {
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
pub(super) struct PreviousFrameSnapshot {
    pub(super) pos: ae::Vec2,
    pub(super) vel: ae::Vec2,
    pub(super) on_ground: bool,
    pub(super) fly_enabled: bool,
    pub(super) blink_aiming: bool,
    pub(super) blink_grace_timer: f32,
    pub(super) fast_falling: bool,
    pub(super) dash_charges_available: u8,
    pub(super) air_jumps_available: u8,
    pub(super) resets: u32,
    pub(super) hp_current: i32,
    pub(super) locomotion: ae::LocomotionState,
    pub(super) body_mode: ae::BodyMode,
    pub(super) active_area: String,
    pub(super) controls: ControlFrame,
}
