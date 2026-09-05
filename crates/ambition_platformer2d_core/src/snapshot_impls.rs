//! `SnapshotState` for this crate's own types — the rollback wire format.
//!
//! These impls live HERE, beside the types they encode, because
//! `ambition_platformer2d::engine_core::snapshot` owns the trait and the orphan rule binds an impl to the
//! crate owning the trait OR the type. The orphan rule is what proves this file is in the right
//! crate: if a type moves, this stops compiling rather than drifting.
//!
//! A field added to an encoded type is a WIRE FORMAT change. Encode and
//! decode must stay in the same order, and `snapshot_unit_enum!` codes are
//! authored per variant so inserting one never renumbers the rest.

use crate::snapshot::{put_bool, put_f32, put_u32, put_vec2, Reader, SnapshotState};
use crate::{snapshot_pod, snapshot_unit_enum};

impl SnapshotState for crate::AbilitySet {
    fn encode(&self, out: &mut Vec<u8>) {
        put_bool(out, self.move_horizontal);
        put_bool(out, self.jump);
        put_bool(out, self.variable_jump);
        put_bool(out, self.double_jump);
        put_bool(out, self.fast_fall);
        put_bool(out, self.wall_jump);
        put_bool(out, self.wall_cling);
        put_bool(out, self.wall_climb);
        put_bool(out, self.dash);
        put_bool(out, self.double_dash);
        put_bool(out, self.fly);
        put_bool(out, self.fly_toggle);
        put_bool(out, self.blink);
        put_bool(out, self.precision_blink);
        put_bool(out, self.blink_through_soft_walls);
        put_bool(out, self.blink_through_hard_walls);
        put_bool(out, self.attack);
        put_bool(out, self.pogo);
        put_bool(out, self.directional_primary);
        put_bool(out, self.directional_special);
        put_bool(out, self.rebound);
        put_bool(out, self.reset);
        put_bool(out, self.ledge_grab);
        put_bool(out, self.swim);
        put_bool(out, self.glide);
        put_bool(out, self.dodge);
        put_bool(out, self.shield);
        put_bool(out, self.grab);
        put_bool(out, self.interact);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            move_horizontal: r.bool()?,
            jump: r.bool()?,
            variable_jump: r.bool()?,
            double_jump: r.bool()?,
            fast_fall: r.bool()?,
            wall_jump: r.bool()?,
            wall_cling: r.bool()?,
            wall_climb: r.bool()?,
            dash: r.bool()?,
            double_dash: r.bool()?,
            fly: r.bool()?,
            fly_toggle: r.bool()?,
            blink: r.bool()?,
            precision_blink: r.bool()?,
            blink_through_soft_walls: r.bool()?,
            blink_through_hard_walls: r.bool()?,
            attack: r.bool()?,
            pogo: r.bool()?,
            directional_primary: r.bool()?,
            directional_special: r.bool()?,
            rebound: r.bool()?,
            reset: r.bool()?,
            ledge_grab: r.bool()?,
            swim: r.bool()?,
            glide: r.bool()?,
            dodge: r.bool()?,
            shield: r.bool()?,
            grab: r.bool()?,
            interact: r.bool()?,
        })
    }
}

/// The host-code playable kit is derived from this component. Registering the
/// identity without its derivation input made a restore depend on whatever
/// abilities happened to be live at restore time; both now rewind together.
impl SnapshotState for crate::body_clusters::BodyAbilities {
    fn encode(&self, out: &mut Vec<u8>) {
        self.abilities.encode(out);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::body_clusters::BodyAbilities::new(
            <crate::AbilitySet as SnapshotState>::decode(r)?,
        ))
    }
}

snapshot_unit_enum!(crate::player_state::BodyMode {
    Standing = 0,
    Crouching = 1,
    Crawling = 2,
    Sliding = 3,
    MorphBall = 4,
    Climbing = 5,
    // ⛔ A NEW DISCRIMINANT AT THE END, never a renumbering. These bytes are the
    // rollback wire and a peer on an older build decodes them positionally: move
    // `Climbing` off 5 and two machines disagree about what a body is doing
    // while both report a clean decode.
    Submerged = 6,
});

impl SnapshotState for crate::body_clusters::BodyModeState {
    fn encode(&self, out: &mut Vec<u8>) {
        self.body_mode.encode(out);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::body_clusters::BodyModeState {
            body_mode: crate::player_state::BodyMode::decode(r)?,
        })
    }
}

snapshot_pod!(crate::body_clusters::BodyGroundState {
    on_ground: bool,
    // Rewound with `on_ground` beside it and for the same reason: it is a
    // CONTACT fact the next step's floor game reads before re-sampling, so a
    // restore without it decides a ceiling tech from the wrong tick.
    head_contact: bool,
    contact_initialized: bool,
});

snapshot_pod!(crate::body_clusters::BodyWallState {
    on_wall: bool,
    wall_normal_x: f32,
});

snapshot_pod!(crate::body_clusters::ActorSurfaceState {
    surface_normal: vec2,
    gravity_scale: f32,
});

snapshot_pod!(crate::body_clusters::BodyJumpState {
    air_jumps_available: u8,
    recovery_charges: u8,
    post_recovery_helpless: bool,
    footstool_claimed: bool,
    ladder_jump_boost: f32,
    ladder_drop_through_timer: f32,
    ladder_drop_through_hold_lock: bool,
});

snapshot_pod!(crate::body_clusters::BodyDashState {
    charges_available: u8,
    cooldown: f32,
});

snapshot_pod!(crate::body_clusters::BodyFlightState {
    fly_enabled: bool,
    carried_run: f32,
    // The hold is snapshot state for the same reason the value it guards is: a
    // rewind into the middle of a launch that restored the speed and not the
    // window would resimulate a body that coasts for the wrong duration.
    carried_hold: f32,
    // A launch written by a reaction and not yet drained by `step_motion` is
    // state a rewind must restore: without it, a rollback across the frame a hit
    // landed would resimulate a body that was never launched — or, worse, drain
    // a launch that the new timeline never imparted.
    pending_launch: vec2,
    pending_launch_flinchless: bool,
});

snapshot_pod!(crate::body_clusters::BodyBlinkState { cooldown: f32 });

snapshot_pod!(crate::body_clusters::BodyDodgeState {
    cooldown: f32,
    air_dodge_spent: bool,
    // DODGE STALING is rollback state: how worn the option is decides how long
    // the NEXT evade is invulnerable, so a rewind that restored the wrong count
    // gives a fighter back i-frames it had spent.
    evades_recent: u8,
    stale_decay: f32
});

snapshot_pod!(crate::body_clusters::BodyShieldState {
    active: bool,
    parry_window_timer: f32,
    // The parry's MODE: whether a caught projectile is absorbed or returned.
    // State for the same reason the window beside it is — two peers disagreeing
    // about which one a stance is doing send the same shot two different ways.
    absorb_window_timer: f32,
    depleted: f32,
    break_timer: f32,
    stun_timer: f32,
    break_total: f32,
    // A caught parry is a PRESENTATION window, and it is still rollback state:
    // a rewind that restored the guard without it would replay a parry with no
    // beat, and two peers would draw different frames from the same tick.
    parry_caught_timer: f32,
    // Both are rollback state for the same reason the timers above are: a
    // rewind that restored the guard without them would let a body re-raise a
    // shield it had already spent, with a fresh parry window.
    release_locked: bool,
    drop_lag_timer: f32,
    shield_tilt: f32,
});

snapshot_pod!(crate::body_clusters::BodyOffense {
    damage_multiplier: i32,
});

snapshot_pod!(crate::body_clusters::BodyLifetime {
    time_alive: f32,
    resets: u32,
    max_speed: f32,
    // The pending-announcement flag is SNAPSHOT state, not bookkeeping to drop:
    // a rewind into the tick between a reset and its `BodyRestarted` trigger
    // must replay the announcement, or the resimulation clears provider state
    // the original run cleared and the two diverge.
    restart_pending: bool,
});

snapshot_pod!(crate::body_clusters::BodyActionBuffer {
    attack: f32,
    pogo: f32,
    grab: f32,
    special: f32,
});

snapshot_pod!(crate::body_clusters::BodyBaseSize { base_size: vec2 });

snapshot_unit_enum!(crate::ledge_grab::LedgeGetupKind {
    Climb = 0,
    Roll = 1,
    Attack = 2,
});

snapshot_unit_enum!(crate::ledge_grab::LedgeGrabQuality {
    Precise = 0,
    Forgiving = 1,
});

snapshot_unit_enum!(crate::movement::MovementOp {
    Jump = 0,
    DoubleJump = 1,
    WallJump = 2,
    WallCling = 3,
    WallClimb = 4,
    LedgeGrab = 5,
    LedgeJump = 6,
    LedgeClimbStart = 7,
    LedgeClimbFinish = 8,
    LedgeDrop = 9,
    LedgeRoll = 10,
    LedgeGetupAttack = 11,
    SwimStroke = 12,
    Dash = 13,
    DoubleDash = 14,
    DodgeRoll = 15,
    FlyToggle = 16,
    Blink = 17,
    PrecisionBlink = 18,
    Pogo = 19,
    Rebound = 20,
    Slash = 21,
    Reset = 22,
    ShieldUp = 23,
    // APPENDED, never renumbered. The codes here are the WIRE FORMAT — the
    // enum's declaration order is not, which is why these two sit next to their
    // sibling crawl ops in `ops.rs` and at the end of the list here.
    CrawlAttach = 24,
    CrawlDetach = 25,
    AirDodge = 26,
    Tumble = 27,
    Tech = 28,
    Knockdown = 29,
    Getup = 30,
    GetupRoll = 31,
    GetupAttack = 32,
    ShieldBreak = 33,
    Footstool = 34,
    SpotDodge = 35,
});

// The hang state machine (`Option<LedgeGrabState>`) is axis-policy maneuver
// state and rides in the `MotionModel` codec (motion_codec.rs); the shared
// cluster keeps only the re-grab lockout.
snapshot_pod!(crate::body_clusters::BodyLedgeState {
    release_cooldown: f32,
});

/// The recent-movement trace a combo/chain rule reads. A `Vec`, so its order IS its
/// meaning: the ops go out in the order they went in.
impl SnapshotState for crate::body_clusters::BodyComboTrace {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u32(out, self.combo.len() as u32);
        for mark in &self.combo {
            mark.op.encode(out);
            put_f32(out, mark.age);
        }
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use crate::movement::{ComboMark, MovementOp};
        let n = r.u32()?;
        let combo = (0..n)
            .map(|_| {
                Some(ComboMark {
                    op: MovementOp::decode(r)?,
                    age: r.f32()?,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(crate::body_clusters::BodyComboTrace { combo })
    }
}

snapshot_pod!(crate::body_clusters::SweepSample {
    prev: vec2,
    curr: vec2,
    vel: vec2,
    half: vec2,
});

snapshot_pod!(crate::geometry::CenteredAabb {
    center: vec2,
    half_size: vec2,
});

snapshot_pod!(crate::player_state::ResourceMeter {
    current: f32,
    max: f32,
    regen_rate: f32,
    decay_rate: f32,
});

impl SnapshotState for crate::body_clusters::BodyMana {
    fn encode(&self, out: &mut Vec<u8>) {
        self.meter.encode(out);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::body_clusters::BodyMana {
            meter: crate::player_state::ResourceMeter::decode(r)?,
        })
    }
}

snapshot_unit_enum!(crate::reference_frame::GameplayFramePolicy {
    ControlledBodyLocal = 0,
    AccelerationFrame = 1,
    WorldSpace = 2,
    ScreenSpace = 3,
});

impl SnapshotState for crate::body_clusters::BodyKinematics {
    fn encode(&self, out: &mut Vec<u8>) {
        put_vec2(out, self.pos);
        put_vec2(out, self.vel);
        put_vec2(out, self.size);
        put_f32(out, self.facing);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::body_clusters::BodyKinematics {
            pos: r.vec2()?,
            vel: r.vec2()?,
            size: r.vec2()?,
            facing: r.f32()?,
        })
    }
}
