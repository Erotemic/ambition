//! Serializable trace data shapes: the per-frame `GameplayTraceFrame` (player +
//! platform + control state) and the discrete `GameplayTraceEvent` / `DumpReason`
//! / `OobReason` enums, plus serde-friendly geometry mirrors (`TracePoint`,
//! `TraceAabb`) that avoid leaking `bevy_math`/engine types into the JSON shape.

use ambition_engine_core as ae;
use ambition_input::ControlFrame;
use serde::Serialize;

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
    pub left_pressed: bool,
    pub right_pressed: bool,
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
            left_pressed: c.left_pressed,
            right_pressed: c.right_pressed,
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
    /// Post-blink grace i-frames active (the semantic fact; the raw timer is
    /// policy-private, ADR 0024).
    pub blink_grace: bool,
    pub locomotion: String,
    pub body_mode: String,
    pub last_safe_pos: TracePoint,
    pub time_alive: f32,
    pub resets: u32,
    /// X-component of the wall normal the player is currently in
    /// contact with (`-1` = wall on player's right, `+1` = wall on
    /// player's left, `0` = no wall contact). Captured so the trace
    /// can attribute a `CollisionCorrection` snap to the wall side
    /// the snap aligned with — the canonical case is "body.left ==
    /// wall.right" (wall_normal_x = +1) or "body.right == wall.left"
    /// (wall_normal_x = -1).
    pub wall_normal_x: f32,
    /// True iff the player has an active `LedgeGrabState`. The
    /// ledge-grab path writes `player.pos = contact.anchor` which is
    /// the most-likely source of a teleporting position snap; having
    /// the boolean per-frame lets the trace post-hoc check whether a
    /// `CollisionCorrection` tick coincides with a `false → true`
    /// transition of this flag.
    pub ledge_grabbing: bool,
    /// True iff a melee swing is currently active (`BodyMelee`
    /// is `Some`). Cross-referenced with the `attack_pressed` input edge
    /// this is the canonical "did the swing actually START" signal: a
    /// frame with `attack_pressed = true` that never flips `attacking`
    /// to `true` means an attack was REQUESTED but GATED — the next
    /// three fields say why.
    pub attacking: bool,
    /// Hitstun timer (`PlayerCombatState::hitstun_timer`). The longer,
    /// softer partial-movement window after a hit. Note the attack-start
    /// gate is the briefer `recoil_lock_timer`, NOT this — a positive
    /// `hitstun_timer` no longer blocks a swing once recoil has cleared.
    pub hitstun_timer: f32,
    /// Damage i-frame timer (`PlayerCombatState::damage_invuln_timer`).
    /// A positive value means a recent hit; useful for confirming
    /// whether contact damage fired (the thing that SETS hitstun).
    pub damage_invuln_timer: f32,
    /// Whether the `attack` ability is enabled in the body's `AbilitySet`.
    /// A disabled attack ability is another way the swing can silently not fire
    /// (the moveset trigger only starts an `"attack"` move for a capable body).
    pub attack_ability_enabled: bool,
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
    /// The rollback-session generation this row belongs to, when there is one.
    /// Frame numbers restart at zero for every session, so `sim_frame` is only a
    /// stable identity together with this generation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sim_session: Option<u64>,
    /// The host's simulation frame this row describes, when there is one.
    ///
    /// `seq`/`tick` are the buffer's own append counters: they only ever go
    /// up, so they cannot identify a frame that gets simulated twice. Together
    /// with `sim_session`, this is the rewindable identity that lets a corrected
    /// pass REPLACE the row a mispredicted pass wrote rather than appending a
    /// second, contradictory row for the same instant.
    ///
    /// `None` on every host that does not speculate, where a row is written
    /// once and the distinction cannot arise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sim_frame: Option<i32>,
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

impl GameplayTraceFrame {
    /// Stable rollback identity for replacement/correction bookkeeping.
    pub const fn simulation_identity(&self) -> Option<(u64, i32)> {
        match (self.sim_session, self.sim_frame) {
            (Some(session), Some(frame)) => Some((session, frame)),
            _ => None,
        }
    }
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
        /// Blocks the player's AABB overlaps (or touches within 1 px)
        /// at the post-teleport position. Captures the snap target
        /// directly so the next OOB trace can attribute the snap to
        /// a specific wall edge instead of forcing a re-derivation
        /// from world geometry. Empty when the player is in clear
        /// space after the snap (unusual — typically a snap means
        /// "edge-aligned with something").
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        nearby_after: Vec<CollisionTraceShape>,
        /// State flags that flipped between the previous frame and
        /// this one. Each entry is `"<field>: <prev>→<curr>"`. The
        /// big two: `ledge_grabbing: false→true` is the smoking gun
        /// for a `try_start_ledge_grab` snap; `fly_enabled:
        /// false→true` (or `body_mode:` flip) is the smoking gun
        /// for a body-mode resize snap.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        state_flips: Vec<String>,
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
    pub fn tick(&self) -> u64 {
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

    pub fn label(&self) -> &'static str {
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
    OobAuto {
        reason: String,
    },
    /// Auto-dump triggered the frame an unexplained position correction
    /// (teleport-class `CollisionCorrection`) fires. Unlike `OobAuto`,
    /// this catches teleports that land *inside* the OOB margin — e.g.
    /// the lock-wall wall-cling snap to `y = -23`, which is within
    /// `OOB_MARGIN` and so never tripped the OOB detector. Captures the
    /// ring buffer at the moment of the snap so the pre-teleport frames
    /// are still present (a manual dump seconds later only catches the
    /// stuck aftermath — see `dev/journals/code_smells.md`).
    TeleportAuto {
        reason: String,
    },
    Programmatic {
        label: String,
    },
}

impl DumpReason {
    /// Whether the recorder decided this on its own, rather than being asked.
    ///
    /// Automatic dumps are the ones [`crate::TraceDumpPolicy`] gates. Matching
    /// exhaustively rather than testing for `Manual` means a new trigger has to
    /// state which side it is on instead of defaulting into the ungated one.
    pub const fn is_automatic(&self) -> bool {
        match self {
            Self::OobAuto { .. } | Self::TeleportAuto { .. } => true,
            // `Programmatic` is a caller asking in so many words — a test
            // harness or tool naming its own label — so it is not gated.
            Self::Manual | Self::Programmatic { .. } => false,
        }
    }
}

impl DumpReason {
    pub fn label(&self) -> String {
        match self {
            DumpReason::Manual => "Manual (F8)".into(),
            DumpReason::OobAuto { reason } => format!("OOB auto: {reason}"),
            DumpReason::TeleportAuto { reason } => format!("Teleport auto: {reason}"),
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
/// through every player-tick phase. Stored on the buffer so the
/// recorder is the single owner of trace state.
///
/// `fly_enabled` and `fast_falling` are recorded for future event
/// detection (e.g. flight toggles, fast-fall edges) — they are
/// captured now so the snapshot shape stays stable as we add more
/// diffs.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct PreviousFrameSnapshot {
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    pub on_ground: bool,
    pub fly_enabled: bool,
    pub blink_aiming: bool,
    pub blink_grace: bool,
    pub fast_falling: bool,
    pub dash_charges_available: u8,
    pub air_jumps_available: u8,
    pub resets: u32,
    pub hp_current: i32,
    pub locomotion: ae::LocomotionState,
    pub body_mode: ae::BodyMode,
    pub active_area: String,
    pub controls: ControlFrame,
    /// Snapshot of `player.ledge_grab.is_some()` and `wall_normal_x`
    /// so `synthesize_events_from_diff` can attribute a teleporting
    /// position snap to a ledge-grab entry or a wall-side flip.
    pub ledge_grabbing: bool,
    pub wall_normal_x: f32,
    pub on_wall: bool,
}
