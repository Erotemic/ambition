//! The Supreme Leader's conductor: it walks him, runs his network, and
//! performs his moves.
//!
//! His scripted pattern decides WHICH move and WHEN (its `Special` keys: tell,
//! strike, rest). This decides everything that is the move's own: where he
//! stands, what fires, the volume each move swings, and the row he is drawn
//! with (`ambition.boss.conducted_pose`).
//!
//! ⭐ THE FIGHT'S SHAPE (`docs/planning/game/bosses.md`, "The Supreme
//! Leader"). A ruler who tries to control every part of the battlefield: his
//! objective is total control, and the fight is that objective failing. His
//! orders go through a COMMAND UPLINK, a mast he calls down into the room
//! (one at a time: a summon of his encounter, so the count of its living
//! summons says whether it stands). While it stands, a tether runs from its
//! dish to his sceptre and he cannot be hurt while he gives an order. Break
//! the uplink and the order still comes, but nothing answers it: he stands
//! with a hand to his ear, shouting into it (`report`), open to every blow.
//!
//! - The lines (an order): sight lines from the uplink's two pods down the
//!   room; the low line fires, then the high one. Jump the low line and be
//!   down again before the high one.
//! - The decree (an order): reticles under you and either side; shells fall
//!   from the ceiling on them.
//! - The crossfire (an order, from the Disruption on): the high pod's sight
//!   line tracks you, locks, then fires along where it locked.
//! - Precision: the sceptre levelled; bolts that lead you. A guard turns
//!   them back on him.
//! - The sweep: the sceptre wound back and swung through in front of him.
//! - The gate step (from the Disruption on): a ring opens behind you; he
//!   steps through it and sweeps.
//! - THE MANDATE (his Emergency Powers, the signature): the orb's beam swept
//!   from high in front of him down to the floor at his feet. Nothing in
//!   front of him and inside its reach is safe; behind him is. The sceptre
//!   overheats after it: the long window.
//!
//! Between his first two phases (the Contingency) he sends the uplink away
//! and calls a new one down at the other end of the room. At the Emergency
//! Powers he drains the uplink into his sceptre: it goes, he grows, and no
//! order needs it again. When he falls he goes down on one knee on his
//! sceptre, defiant; the orb opens a gate behind him and he falls back
//! through it. He is beaten, not killed: the gate takes him away.
//!
//! Every volume is measured from his art (the renderer's `supreme_leader`,
//! its 640 x 480 frame), in its frame pixels, and carried into the world by
//! [`Body`] about the middle of his body box, which is his position.

use ambition_boss_special_port::{
    BossConduct, BossConductPort, BossDismissSummon, BossDismissSummonPort, BossGuard, BossGuardPort, BossScale,
    BossScalePort, BossSummon, BossSummonPort, ConductedPose, ConductedPosePort, DrawnRow, DrawnRowPort, Pose,
    SummonSide,
};
use ambition_combat_port::{
    BodySound, BodySoundPort, Burst, BurstPort, CameraShake, CameraShakePort, HeldRay, HeldRayPort, Lightning,
    LightningAura, LightningAuraPort, LightningPort, RidingHitbox, RidingHitboxPort, RidingKnockback, Spot,
};
use ambition_extension_sdk::{
    phases::BOSS_CONDUCT, record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};
use ambition_projectile_spec::{ProjectileSpawn, ProjectileSpawnPort};
use bevy_math::Vec2;

/// The behaviour id of the boss it conducts.
pub const SUPREME_LEADER_ID: &str = "supreme_leader";

/// His body box in the art: the union of his `idle` frame (hair to boots,
/// sceptre to cape), 144 x 322 frame pixels about (315, 261).
const BODY_PX: f32 = 322.0;
const BODY_CENTRE: Vec2 = Vec2::new(315.0, 261.0);
/// The height of the body his lengths were authored for, in world units.
const AUTHORED_HEIGHT: f32 = 150.0;

/// His body as big as it is in the world: every volume is measured on his
/// art, and his box ([`BossConduct::size`]) says how many world units a
/// frame pixel is. The Emergency Powers grow him, and every reach grows with
/// him.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    px: f32,
}

impl Body {
    pub const AUTHORED: Body = Body { px: AUTHORED_HEIGHT / BODY_PX };

    pub fn of_height(height: f32) -> Self {
        Self { px: height.max(1.0) / BODY_PX }
    }

    /// How many times the authored body he is.
    pub fn grown(self) -> f32 {
        self.px / Self::AUTHORED.px
    }

    fn len(self, authored: f32) -> f32 {
        authored * self.grown()
    }

    /// A frame point (pixels, art facing right) as an offset from his
    /// position, unflipped.
    fn art(self, x: f32, y: f32) -> Vec2 {
        (Vec2::new(x, y) - BODY_CENTRE) * self.px
    }

    /// His soles (pixel 420): standing, his position is this far above the
    /// floor.
    pub fn feet_below(self) -> f32 {
        self.art(0.0, 420.0).y
    }

    /// Half his body's width.
    fn body_half(self) -> f32 {
        self.art(387.0, 0.0).x
    }

    /// The orb of his sceptre as he stands (`idle` frame 0): the tether's
    /// end.
    fn orb_standing(self) -> Vec2 {
        self.art(368.2, 126.1)
    }

    /// The orb with the sceptre levelled (`aim`, from its frame 3): where
    /// his precision bolts leave.
    fn orb_levelled(self) -> Vec2 {
        self.art(532.8, 187.4)
    }

    /// The orb held out in both hands (`mandate`): where the Mandate leaves.
    fn orb_mandate(self) -> Vec2 {
        self.art(526.7, 195.1)
    }

    /// The sweep's arc, in front of him: from his chest to past the orb at
    /// the end of the swing (`sweep` frames 6-9, the orb out to 549 px),
    /// hair to boots.
    fn sweep_box(self) -> (Vec2, Vec2) {
        let (a, b) = (self.art(352.0, 100.0), self.art(566.0, 420.0));
        ((a + b) * 0.5, (b - a) * 0.5)
    }
}

// ── The uplink ──

/// The uplink's character (`npc_command_uplink`) and its box: the renderer's
/// `command_uplink`, 94 x 313 frame pixels drawn 47 x 156.
const UPLINK_CHARACTER: &str = "npc_command_uplink";
const UPLINK_HALF: [f32; 2] = [24.0, 78.0];
/// Blows it takes: an uplink of the Command, and the one called down at the
/// Contingency.
const UPLINK_HEALTH: [u32; 2] = [6, 8];
const UPLINK_LABEL: &str = "uplink";
/// Where it stands: this far in from the wall at its end of the room (the
/// space behind it, by the wall, is out of its lines).
const UPLINK_INSET: f32 = 344.0;
/// Its pods, over the floor (the renderer's `LOW_POD` and `HIGH_POD` at 2
/// frame pixels a unit), and out from its middle; its dish.
const POD_LOW: f32 = 22.0;
const POD_HIGH: f32 = 104.0;
const POD_OUT: f32 = 15.0;
const DISH: f32 = 141.0;
/// A summon is made the tick after it is asked for; until it has been seen
/// alive, a count of none is not a loss.
const UPLINK_GRACE_S: f32 = 0.5;

// ── His moves ──

/// The lines: the low one fires as the order's strike begins and holds
/// `LINE_S`; the high one `HIGH_AT` later.
const LINE_WIDTH: f32 = 12.0;
const LINE_DAMAGE: i32 = 1;
const LINE_KNOCKBACK: f32 = 1.0;
const LINE_S: f32 = 0.2;
const HIGH_AT: f32 = 0.55;
/// The sight lines show from this far into the tell.
const SIGHT_FROM: f32 = 0.12;

/// The decree: marks this far apart (three in the Command, five after), and
/// a shell for each, one after another.
const DECREE_SPREAD: f32 = 150.0;
const SHELL_EVERY_S: f32 = 0.1;
const SHELL_DROP: f32 = 560.0;
const SHELL_SPEED: f32 = 300.0;
const SHELL_GRAVITY: f32 = 1600.0;
const SHELL_HALF: Vec2 = Vec2::new(8.0, 14.0);
const SHELL_SPLASH: f32 = 34.0;
const SHELL_DAMAGE: i32 = 1;

/// Precision: when each bolt leaves (three, four, five by phase).
const BOLT_AT: [f32; 5] = [0.1, 0.28, 0.46, 0.64, 0.8];
const BOLT_SPEED: f32 = 560.0;
const BOLT_HALF: Vec2 = Vec2::new(9.0, 9.0);
const BOLT_DAMAGE: i32 = 1;
/// How far ahead of you a bolt is aimed: at most this long of your motion.
const LEAD_MAX_S: f32 = 0.5;

/// The sweep: it hurts this far into the strike, this long.
const SWEEP_AT: f32 = 0.08;
const SWEEP_LIVE_S: f32 = 0.18;
const SWEEP_DAMAGE: i32 = 2;
const SWEEP_KNOCKBACK: f32 = 1.4;
const SWEEP_STEP: f32 = 24.0;

/// The crossfire: the high pod's sight line follows you until this long
/// before the tell ends, then holds; the line fires along it.
const CROSSFIRE_LOCK_S: f32 = 0.45;
const CROSSFIRE_WIDTH: f32 = 14.0;
const CROSSFIRE_S: f32 = 0.35;

/// The gate step: the ring opens this far past you, on the far side of you
/// from him; he steps out of it, and sweeps (`SWEEP_AT` after `GATE_IN_S`).
const GATE_BEHIND: f32 = 90.0;
const GATE_IN_S: f32 = 0.3;
const GATE_GLOW: [f32; 4] = [0.62, 0.32, 1.0, 0.7];

/// The Mandate: from `MANDATE_FROM` degrees (up in front of him) to
/// `MANDATE_TO` (the floor at his feet), as far as `MANDATE_REACH`.
const MANDATE_FROM: f32 = -32.0;
const MANDATE_TO: f32 = 40.0;
const MANDATE_REACH: f32 = 720.0;
const MANDATE_WIDTH: f32 = 22.0;
const MANDATE_DAMAGE: i32 = 2;
const MANDATE_KNOCKBACK: f32 = 1.4;

/// The windows after his moves.
const RECOVER_S: f32 = 0.45;
/// Reporting a failed order: as long as the order's strike would have been
/// and the rest after it, unless a new tell comes first.
const REPORT_S: f32 = 2.2;

/// Where he keeps from you between moves (he commands from the back), and
/// how fast he walks there, by phase.
const STANDOFF: [f32; 3] = [300.0, 220.0, 120.0];
const WALK_SPEED: [f32; 3] = [70.0, 95.0, 125.0];
const WALL_GAP: f32 = 12.0;

/// His intro (as long as the encounter's `intro_seconds`): the address, and
/// the uplink called down `INTRO_UPLINK_AT` into it.
pub const INTRO_S: f32 = 2.88;
const INTRO_UPLINK_AT: f32 = 0.6;

/// The beat between two phases (the encounter's `transition_seconds`).
const BEAT_S: f32 = 1.8;
/// The Contingency: the old uplink goes at once, the new one comes this far in.
const REDEPLOY_AT: f32 = 0.9;
/// The Emergency Powers: the drain until `DRAIN_S`, the growth after it.
const DRAIN_S: f32 = 0.8;
const EMERGENCY_GROWTH: f32 = 1.1;

/// The death: thrown back, down on one knee, the gate, through it (his
/// `death` row, 20 frames of 0.09 s).
const DEATH_FD: f32 = 0.09;
const DEATH_GATE_AT: f32 = 0.55 * 19.0 * DEATH_FD;
const DEATH_GONE_AT: f32 = 19.0 * DEATH_FD;

/// A held effect is asked again each tick for a little longer than a tick.
const HOLD_S: f32 = 0.06;

const CRIMSON: [f32; 4] = [1.0, 0.22, 0.18, 1.0];
const TETHER: [f32; 4] = [1.0, 0.2, 0.14, 0.45];
const GOLD: [f32; 4] = [1.0, 0.82, 0.4, 1.0];
const SMOKE: [f32; 4] = [0.5, 0.48, 0.52, 0.9];
const DUST: [f32; 4] = [0.74, 0.70, 0.66, 1.0];
const VIOLET: [f32; 4] = [0.7, 0.45, 1.0, 1.0];

const SFX_ORDER: &str = "faction.military.radio_chirp";
const SFX_LINE: &str = "ability.eye_laser.ignite";
const SFX_LAUNCH: &str = "boss.mockingbird.missile_launch";
const SFX_CHARGE: &str = "projectile.energy_orb.charge";
const SFX_BOLT: &str = "projectile.magic_missile.cast";
const SFX_SWEEP: &str = "swing.heavy";
const SFX_GATE_OUT: &str = "world.portal.enter";
const SFX_GATE_IN: &str = "story.gate_ripple";
const SFX_MANDATE: &str = "weapon.lasersword.fire";
const SFX_OVERHEAT: &str = "vfx.explosion.smoke_burst";
const SFX_DRAIN: &str = "ability.lightning.charge";
const SFX_ZAP: &str = "boss.wolfenstein.zap";
const SFX_LAND: &str = "boss.gnu_ton.stomp";
const SFX_BREAK: &str = "vfx.explosion.classic_burst";

/// His moves: the `Special` keys his pattern names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Lines,
    Decree,
    Crossfire,
    Precision,
    Sweep,
    GateStep,
    Mandate,
}

impl Move {
    pub const ALL: [Move; 7] =
        [Move::Lines, Move::Decree, Move::Crossfire, Move::Precision, Move::Sweep, Move::GateStep, Move::Mandate];

    pub fn key(self) -> &'static str {
        match self {
            Move::Lines => "sl_lines",
            Move::Decree => "sl_decree",
            Move::Crossfire => "sl_crossfire",
            Move::Precision => "sl_precision",
            Move::Sweep => "sl_sweep",
            Move::GateStep => "sl_gate_step",
            Move::Mandate => "sl_mandate",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mv| mv.key() == key)
    }

    fn code(self) -> u32 {
        Self::ALL.iter().position(|mv| *mv == self).unwrap_or(0) as u32 + 1
    }

    fn of_code(code: u32) -> Option<Self> {
        code.checked_sub(1).and_then(|i| Self::ALL.get(i as usize).copied())
    }

    /// An order: the uplink carries it out, and he is guarded while it does.
    pub fn is_order(self) -> bool {
        matches!(self, Move::Lines | Move::Decree | Move::Crossfire)
    }
}

record! {
    /// The conductor's memory, on the Leader.
    pub struct Conductor = SchemaKey::new(crate::PROVIDER, "supreme_leader.conductor", 1);
    /// The room, measured once.
    1 hall_known: bool,
    2 hall_floor: f32,
    3 hall_left: f32,
    4 hall_right: f32,
    /// The live part: its move (`Move::code`, 0 for none), whether
    /// striking, seconds into it, and its full length.
    5 part_move: u32,
    6 part_striking: bool,
    7 part_t: f32,
    8 part_dur: f32,
    9 last_remaining: f32,
    10 ticks: u32,
    /// What the strike has spent: its onset, and how many shots it has fired
    /// (lines, shells, bolts).
    11 fired: bool,
    12 shots: u32,
    /// His intro is over (or his first move has started): he fights.
    13 woke: bool,
    14 intro: Option<f32>,
    /// The uplink: asked for and not yet lost (`uplink_up`), seen alive
    /// since (`uplink_seen`), seconds since it was asked for, its number,
    /// where it stands and which way its lines fire.
    15 uplink_up: bool,
    16 uplink_seen: bool,
    17 uplink_age: f32,
    18 uplink_serial: u32,
    19 uplink_x: f32,
    20 uplink_dir: f32,
    /// Windows, each seconds since it began: reporting a failed order (or
    /// a lost uplink), coming back from a swing, the overheated sceptre.
    21 report: Option<f32>,
    22 recover: Option<f32>,
    23 overheat: Option<f32>,
    /// The phase beats he has begun (1 the Contingency, 2 the Emergency
    /// Powers), and seconds into the one running.
    24 transitions: u32,
    25 beat: Option<f32>,
    /// The Emergency Powers are his.
    26 emergency: bool,
    /// The guard he asked for last.
    27 guarded: bool,
    /// The decree's marks.
    28 mark0: f32,
    29 mark1: f32,
    30 mark2: f32,
    31 mark3: f32,
    32 mark4: f32,
    33 marks: u32,
    /// The crossfire's aim (a unit direction from the high pod).
    34 aim_x: f32,
    35 aim_y: f32,
    /// The gate step's ring.
    36 ring_x: f32,
    /// Where you were last tick (to lead you).
    37 last_tx: f32,
    38 last_ty: f32,
    39 ray_generation: u32,
    40 step_from: f32,
    /// Dead: seconds since he fell.
    41 dead_for: Option<f32>,
    /// He has turned round: seconds into coming round (`turn`).
    42 turning: Option<f32>,
    /// The encounter held a beat between phases last tick: a beat begins
    /// where one starts.
    43 was_between: bool,
    /// Seconds a tell has been gone from his pattern (a blow's flinch hides
    /// it for a few ticks): it is held, not begun again, unless it stays gone.
    44 tell_gap: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Part {
    mv: Move,
    striking: bool,
    t: f32,
    dur: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Hall {
    floor: f32,
    left: f32,
    right: f32,
}

impl Conductor {
    /// The move being performed and whether it is striking.
    pub fn performing(&self) -> Option<(Move, bool)> {
        self.part().map(|p| (p.mv, p.striking))
    }

    /// An uplink stands for him (asked for and not lost).
    pub fn uplink_up(&self) -> bool {
        self.uplink_up
    }

    /// How many uplinks he has called down.
    pub fn uplinks_called(&self) -> u32 {
        self.uplink_serial
    }

    /// He is reporting a failed order: open to every blow.
    pub fn reporting(&self) -> bool {
        self.report.is_some()
    }

    /// His sceptre is overheated after the Mandate.
    pub fn overheated(&self) -> bool {
        self.overheat.is_some()
    }

    /// The Emergency Powers are his.
    pub fn emergency(&self) -> bool {
        self.emergency
    }

    /// He asked to be guarded, last.
    pub fn guarded(&self) -> bool {
        self.guarded
    }

    /// The phase beats he has begun.
    pub fn transitions(&self) -> u32 {
        self.transitions
    }

    fn part(&self) -> Option<Part> {
        Move::of_code(self.part_move).map(|mv| Part { mv, striking: self.part_striking, t: self.part_t, dur: self.part_dur })
    }

    fn set_part(&mut self, part: Option<Part>) {
        match part {
            Some(p) => {
                self.part_move = p.mv.code();
                self.part_striking = p.striking;
                self.part_t = p.t;
                self.part_dur = p.dur;
            }
            None => {
                self.part_move = 0;
                self.part_striking = false;
                self.part_t = 0.0;
                self.part_dur = 0.0;
            }
        }
    }

    fn marks(&self) -> Vec<f32> {
        [self.mark0, self.mark1, self.mark2, self.mark3, self.mark4].into_iter().take(self.marks as usize).collect()
    }

    fn set_marks(&mut self, marks: &[f32]) {
        let mut m = [0.0f32; 5];
        for (slot, x) in m.iter_mut().zip(marks) {
            *slot = *x;
        }
        [self.mark0, self.mark1, self.mark2, self.mark3, self.mark4] = m;
        self.marks = marks.len().min(5) as u32;
    }

    /// Which phase he fights in: 0 the Command, 1 the Disruption, 2 the
    /// Emergency Powers.
    fn phase(&self) -> usize {
        (self.transitions as usize).min(2)
    }
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "supreme_leader"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            build: None,
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![Conductor::schema()],
        entries: vec![EntryDescriptor {
            replies: Vec::new(),
            key: "conduct".into(),
            phase: BOSS_CONDUCT,
            trigger: TriggerBinding { port: BossConductPort::KEY, selector: SUPREME_LEADER_ID.into() },
            reads: Vec::new(),
            writes: vec![Conductor::KEY],
            requests: vec![
                ConductedPosePort::KEY,
                DrawnRowPort::KEY,
                BurstPort::KEY,
                LightningPort::KEY,
                LightningAuraPort::KEY,
                CameraShakePort::KEY,
                BossSummonPort::KEY,
                BossDismissSummonPort::KEY,
                RidingHitboxPort::KEY,
                HeldRayPort::KEY,
                ProjectileSpawnPort::KEY,
                BodySoundPort::KEY,
                BossGuardPort::KEY,
                BossScalePort::KEY,
            ],
            after: Vec::new(),
            limits: Limits { max_requests: 64 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(conduct),
        }],
    }
}

fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn play(inv: &mut Invocation<'_>, cue: &str, at: Vec2) -> Result<(), Fault> {
    inv.submit::<BodySoundPort>(BodySound { cue: cue.into(), at: ambition_combat_port::Place::World(at.into()) })
}

fn shake(inv: &mut Invocation<'_>, amplitude_px: f32) -> Result<(), Fault> {
    inv.submit::<CameraShakePort>(CameraShake { amplitude_px })
}

fn burst(inv: &mut Invocation<'_>, at: Vec2, count: u32, speed: f32, color: [f32; 4], kind: &str) -> Result<(), Fault> {
    inv.submit::<BurstPort>(Burst { at: at.into(), count, speed, color, kind: kind.into() })
}

/// A line of `look` from `from` to `to`, held `hold` seconds; asked again
/// under the same `key`, the same line held on.
fn line(inv: &mut Invocation<'_>, key: &str, from: Spot, to: Spot, look: &str, glow: Option<[f32; 4]>, hold: f32) -> Result<(), Fault> {
    inv.submit::<LightningPort>(Lightning { key: key.into(), from, to, look: look.into(), glow, hold_s: hold })
}

/// A sight line: where a shot is about to go.
fn sight(inv: &mut Invocation<'_>, key: &str, from: Vec2, to: Vec2) -> Result<(), Fault> {
    line(inv, key, Spot::World(from.into()), Spot::World(to.into()), "sight", None, HOLD_S)
}

/// A shot down a line: a ray from `from` along `dir`, as far as `reach` or
/// the first wall.
#[allow(clippy::too_many_arguments)]
fn ray(
    inv: &mut Invocation<'_>,
    slot: u32,
    generation: u32,
    from: Spot,
    dir: Vec2,
    reach: f32,
    width: f32,
    damage: i32,
    knockback: f32,
    life: f32,
) -> Result<(), Fault> {
    inv.submit::<HeldRayPort>(HeldRay {
        slot,
        generation,
        from,
        toward: dir.into(),
        reach,
        width,
        damage,
        knockback,
        launch_dir: None,
        lifetime_s: life,
        look: "laser".into(),
        jump: None,
    })
}

/// Where a line from `from` along `dir` meets the room's walls, floor or the
/// height `top` over the floor, whichever is first.
fn to_wall(from: Vec2, dir: Vec2, hall: Hall, top: f32) -> Vec2 {
    let mut s = f32::INFINITY;
    if dir.x > 1e-4 {
        s = s.min((hall.right - from.x) / dir.x);
    } else if dir.x < -1e-4 {
        s = s.min((hall.left - from.x) / dir.x);
    }
    if dir.y > 1e-4 {
        s = s.min((hall.floor - from.y) / dir.y);
    } else if dir.y < -1e-4 {
        s = s.min((from.y - (hall.floor - top)) / -dir.y);
    }
    if !s.is_finite() {
        s = 0.0;
    }
    from + dir * s.max(0.0)
}

/// The uplink's pods, its dish: where its lines leave and where his tether
/// ends.
fn pods(c: &Conductor, hall: Hall) -> (Vec2, Vec2, Vec2) {
    let x = c.uplink_x + c.uplink_dir * POD_OUT;
    (
        Vec2::new(x, hall.floor - POD_LOW),
        Vec2::new(x, hall.floor - POD_HIGH),
        Vec2::new(c.uplink_x, hall.floor - DISH),
    )
}

/// Call an uplink down at `x`, its lines firing along `dir`: a bolt from the
/// ceiling sets it on the floor.
fn call_uplink(inv: &mut Invocation<'_>, c: &mut Conductor, hall: Hall, x: f32, dir: f32) -> Result<(), Fault> {
    c.uplink_serial += 1;
    c.uplink_up = true;
    c.uplink_seen = false;
    c.uplink_age = 0.0;
    c.uplink_x = x;
    c.uplink_dir = dir;
    let health = UPLINK_HEALTH[(c.uplink_serial as usize - 1).min(1)];
    inv.submit::<BossSummonPort>(BossSummon {
        label: UPLINK_LABEL.into(),
        serial: vec![c.uplink_serial],
        position: [x, hall.floor - UPLINK_HALF[1]],
        half_size: UPLINK_HALF,
        character_id: UPLINK_CHARACTER.into(),
        health: Some(health),
        keeps_contact_damage: false,
        // The boss's own side: his lines pass through it, your blows land.
        side: SummonSide::Boss,
        holds: Vec::new(),
    })?;
    let feet = Vec2::new(x, hall.floor - 4.0);
    line(inv, "uplink_drop", Spot::World([x, hall.floor - 520.0]), Spot::World([x, hall.floor - DISH]), "strike", Some(TETHER), 0.3)?;
    play(inv, SFX_ZAP, feet)?;
    play(inv, SFX_LAND, feet)?;
    shake(inv, 6.0)?;
    burst(inv, feet, 30, 300.0, DUST, "dust")?;
    burst(inv, Vec2::new(x, hall.floor - DISH), 18, 260.0, CRIMSON, "spark")
}

/// Send the standing uplink away.
fn dismiss_uplink(inv: &mut Invocation<'_>, c: &mut Conductor, hall: Hall) -> Result<(), Fault> {
    if c.uplink_up {
        inv.submit::<BossDismissSummonPort>(BossDismissSummon { label: UPLINK_LABEL.into(), serial: vec![c.uplink_serial] })?;
        burst(inv, Vec2::new(c.uplink_x, hall.floor - 80.0), 24, 260.0, SMOKE, "dust")?;
    }
    c.uplink_up = false;
    c.uplink_seen = false;
    Ok(())
}

fn conduct(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let boss: BossConduct = inv.trigger::<BossConductPort>()?.clone();
    let b = Body::of_height(boss.size[1]);
    let dt = inv.dt();
    let mut c = Conductor::load(inv)?;
    if !c.hall_known {
        let Some(h) = boss.hall else {
            return Ok(());
        };
        c.hall_known = true;
        c.hall_floor = h.floor;
        c.hall_left = h.left;
        c.hall_right = h.right;
    }
    if boss.alive && c.dead_for.is_some() {
        // Brought back (the hall's life switch): a new fight, from his
        // address, at his own size.
        c = Conductor { hall_known: true, hall_floor: c.hall_floor, hall_left: c.hall_left, hall_right: c.hall_right, ..Default::default() };
        inv.submit::<BossScalePort>(BossScale { scale: 1.0 })?;
        inv.submit::<BossGuardPort>(BossGuard { guarded: false })?;
    }
    let hall = Hall { floor: c.hall_floor, left: c.hall_left, right: c.hall_right };
    c.ticks = c.ticks.wrapping_add(1);
    let at = Vec2::from(boss.position);
    let mut side = if boss.side < 0.0 { -1.0 } else { 1.0 };
    let stand_y = hall.floor - b.feet_below();
    let target = boss
        .target_body
        .map(|t| Vec2::from(t.position))
        .or(boss.target.map(Vec2::from))
        .unwrap_or(Vec2::new((hall.left + hall.right) * 0.5, hall.floor - 24.0));
    // How you move: to lead you.
    let target_vel = if c.ticks > 1 && dt > 0.0 { (target - Vec2::new(c.last_tx, c.last_ty)) / dt } else { Vec2::ZERO };
    c.last_tx = target.x;
    c.last_ty = target.y;
    let clamp_x = |x: f32| x.clamp(hall.left + b.body_half() + WALL_GAP, hall.right - b.body_half() - WALL_GAP);
    let face = |x: f32| if target.x < x { -1.0 } else { 1.0 };
    let width = hall.right - hall.left;

    // ── The uplink: is it still standing? ──
    if c.uplink_up {
        c.uplink_age += dt;
        if boss.minions > 0 {
            c.uplink_seen = true;
        } else if c.uplink_seen || c.uplink_age > UPLINK_GRACE_S {
            // Broken: his network is down. A blow there costs him a beat.
            c.uplink_up = false;
            c.uplink_seen = false;
            let dish = Vec2::new(c.uplink_x, hall.floor - DISH);
            play(inv, SFX_BREAK, dish)?;
            shake(inv, 7.0)?;
            burst(inv, dish, 36, 340.0, GOLD, "spark")?;
            burst(inv, dish, 20, 200.0, SMOKE, "dust")?;
            if boss.alive && c.woke && c.beat.is_none() {
                c.report = Some(0.0);
                play(inv, SFX_ORDER, at)?;
            }
        }
    }

    // ── Beaten: down on one knee on his sceptre; a gate behind him, and
    // he falls back through it ──
    if !boss.alive {
        let t = c.dead_for.map_or(0.0, |t| t + dt);
        let was = t - dt;
        let first = c.dead_for.is_none();
        let crossed = |mark: f32| t >= mark && (was < mark || first);
        if first {
            dismiss_uplink(inv, &mut c, hall)?;
            if c.guarded {
                c.guarded = false;
                inv.submit::<BossGuardPort>(BossGuard { guarded: false })?;
            }
            burst(inv, at, 30, 320.0, CRIMSON, "spark")?;
            shake(inv, 8.0)?;
        }
        if crossed(DEATH_GATE_AT) {
            play(inv, SFX_GATE_OUT, at)?;
            burst(inv, at + Vec2::new(0.0, -b.len(30.0)), 28, 240.0, VIOLET, "spark")?;
        }
        if crossed(DEATH_GONE_AT) {
            play(inv, SFX_GATE_IN, at)?;
            burst(inv, at, 40, 300.0, VIOLET, "spark")?;
        }
        inv.submit::<DrawnRowPort>(DrawnRow::body("death", t.min(DEATH_GONE_AT), false))?;
        c.dead_for = Some(t);
        let pos = Vec2::new(at.x, stand_y);
        inv.submit::<ConductedPosePort>(ConductedPose {
            pose: Some(Pose { position: pos.into(), velocity: Vec2::ZERO.into() }),
            side: Some(side),
        })?;
        return c.store(inv);
    }

    // ── His intro: the address; the uplink comes down into it ──
    if !c.woke {
        if boss.awake {
            c.intro = Some(c.intro.map_or(0.0, |t| t + dt));
        }
        let t = c.intro.unwrap_or(0.0);
        if c.intro.is_some() && c.uplink_serial == 0 && t >= INTRO_UPLINK_AT {
            let (x, dir) = uplink_spot(hall, 0);
            call_uplink(inv, &mut c, hall, x, dir)?;
        }
        if t >= INTRO_S || boss.telegraph.is_some() || boss.active.is_some() {
            c.woke = true;
        } else {
            side = face(at.x);
            let row = if c.intro.is_some() { ("address", t, true) } else { ("idle", c.ticks as f32 / 60.0, true) };
            inv.submit::<DrawnRowPort>(DrawnRow::body(row.0, row.1, row.2))?;
            let pos = Vec2::new(at.x, stand_y);
            inv.submit::<ConductedPosePort>(ConductedPose {
                pose: Some(Pose { position: pos.into(), velocity: Vec2::ZERO.into() }),
                side: Some(side),
            })?;
            return c.store(inv);
        }
    }
    if c.uplink_serial == 0 {
        // A fight that skipped his intro still has its network.
        let (x, dir) = uplink_spot(hall, 0);
        call_uplink(inv, &mut c, hall, x, dir)?;
    }

    // ── The beats between his phases ──
    let between_starts = boss.between_phases && !c.was_between;
    c.was_between = boss.between_phases;
    let beat_starts = c.beat.is_none()
        && c.transitions < 2
        && ((between_starts && (c.transitions == 0 || boss.enraged)) || (boss.enraged && !boss.between_phases));
    if beat_starts {
        c.transitions = if boss.enraged { 2 } else { c.transitions + 1 };
        c.beat = Some(0.0);
        c.report = None;
        c.recover = None;
        c.overheat = None;
        side = face(at.x);
        if c.transitions == 1 {
            // The Contingency: this uplink is done with.
            dismiss_uplink(inv, &mut c, hall)?;
            play(inv, SFX_ORDER, at)?;
        } else {
            play(inv, SFX_DRAIN, at)?;
        }
    }
    let beat = c.beat.map(|t| t + dt);
    if let Some(t) = beat {
        let was = t - dt;
        let crossed = |mark: f32| t >= mark && was < mark;
        let orb = at + Vec2::new(b.orb_standing().x * side, b.orb_standing().y);
        if c.transitions == 1 {
            if crossed(REDEPLOY_AT) || (t >= BEAT_S && !c.uplink_up) {
                let (x, dir) = uplink_spot(hall, 1);
                call_uplink(inv, &mut c, hall, x, dir)?;
            }
        } else {
            // The Emergency Powers: the uplink's power into his sceptre.
            if t < DRAIN_S && c.uplink_up {
                let (_, _, dish) = pods(&c, hall);
                line(inv, "drain", Spot::World(dish.into()), Spot::World(orb.into()), "force", Some(TETHER), HOLD_S)?;
                if c.ticks % 3 == 0 {
                    burst(inv, dish, 4, 160.0, CRIMSON, "spark")?;
                }
            }
            if crossed(DRAIN_S) || (t >= DRAIN_S && c.uplink_up) {
                dismiss_uplink(inv, &mut c, hall)?;
                play(inv, SFX_ZAP, orb)?;
                shake(inv, 9.0)?;
                burst(inv, orb, 40, 360.0, CRIMSON, "spark")?;
            }
            if t >= DRAIN_S {
                let u = ((t - DRAIN_S) / (BEAT_S - DRAIN_S)).min(1.0);
                inv.submit::<BossScalePort>(BossScale { scale: 1.0 + (EMERGENCY_GROWTH - 1.0) * ease(u) })?;
            }
        }
        if t >= BEAT_S {
            c.beat = None;
            if c.transitions >= 2 {
                c.emergency = true;
                inv.submit::<BossScalePort>(BossScale { scale: EMERGENCY_GROWTH })?;
            }
        } else {
            c.beat = Some(t);
        }
    }

    // ── Which part of which move is live ──
    // A beat or a failed order stops him performing his pattern.
    let holding = c.beat.is_some();
    let live = if holding { (None, None) } else { (boss.active.as_ref(), boss.telegraph.as_ref()) };
    let live = match live {
        (Some(active), _) if Move::from_key(&active.key).is_some() => {
            Move::from_key(&active.key).map(|mv| (mv, true, active.remaining))
        }
        (_, Some(tell)) => Move::from_key(&tell.key).map(|mv| (mv, false, tell.remaining)),
        _ => None,
    };
    let prev = c.part();
    // A tell does not end in nothing (it ends in its strike): one that is
    // gone for a moment is a flinch, and is held where it was.
    let live = match (live, prev) {
        (None, Some(p)) if !p.striking && !holding && c.tell_gap + dt <= TELL_GAP_S => {
            c.tell_gap += dt;
            Some((p.mv, false, c.last_remaining))
        }
        (live, _) => {
            c.tell_gap = 0.0;
            live
        }
    };
    let fresh = match (prev, live) {
        (Some(prev), Some((mv, striking, remaining))) => {
            prev.mv != mv || prev.striking != striking || remaining > c.last_remaining + 1e-3
        }
        (None, Some(_)) => true,
        _ => false,
    };
    let part = match live {
        Some((mv, striking, remaining)) => {
            if fresh {
                c.woke = true;
                c.fired = false;
                c.shots = 0;
                c.step_from = at.x;
                if !striking {
                    // A tell ends every window: he gathers himself.
                    c.report = None;
                    c.recover = None;
                    c.overheat = None;
                    side = face(at.x);
                    match mv {
                        Move::Decree => {
                            let n: i32 = if c.phase() == 0 { 1 } else { 2 };
                            let x = target.x.clamp(hall.left + 40.0, hall.right - 40.0);
                            let marks: Vec<f32> = (-n..=n)
                                .map(|k| x + k as f32 * DECREE_SPREAD)
                                .filter(|m| *m > hall.left + 24.0 && *m < hall.right - 24.0)
                                .collect();
                            c.set_marks(&marks);
                        }
                        Move::GateStep => {
                            let away = if target.x >= at.x { 1.0 } else { -1.0 };
                            c.ring_x = clamp_x(target.x + away * GATE_BEHIND);
                        }
                        Move::Crossfire => {
                            let (_, high, _) = pods(&c, hall);
                            let d = (target - high).normalize_or(Vec2::new(c.uplink_dir, 0.0));
                            c.aim_x = d.x;
                            c.aim_y = d.y;
                        }
                        _ => {}
                    }
                } else if mv.is_order() && !c.uplink_up {
                    // The order goes out, and nothing answers it.
                    c.report = Some(0.0);
                    play(inv, SFX_ORDER, at)?;
                }
                Some(Part { mv, striking, t: 0.0, dur: remaining.max(1e-3) })
            } else {
                prev.map(|p| Part { t: p.t + dt, ..p })
            }
        }
        None => None,
    };
    // A strike that ends leaves its window behind it.
    if let (Some(prev), None | Some(Part { striking: false, .. })) = (prev, part) {
        if prev.striking {
            match prev.mv {
                Move::Sweep | Move::GateStep => c.recover = Some(0.0),
                Move::Mandate => {
                    c.overheat = Some(0.0);
                    let orb = at + Vec2::new(b.orb_mandate().x * side, b.orb_mandate().y);
                    play(inv, SFX_OVERHEAT, orb)?;
                    burst(inv, orb, 24, 140.0, SMOKE, "dust")?;
                }
                _ => {}
            }
        }
    }
    c.set_part(part);
    c.last_remaining = live.map_or(0.0, |(_, _, remaining)| remaining);

    // ── His guard: only while an order runs through a standing uplink ──
    let guarded = c.uplink_up
        && c.report.is_none()
        && c.beat.is_none()
        && part.is_some_and(|p| p.mv.is_order());
    if guarded != c.guarded {
        c.guarded = guarded;
        inv.submit::<BossGuardPort>(BossGuard { guarded })?;
    }
    if c.uplink_up && c.uplink_seen && (guarded || c.ticks % 20 < 2) {
        // The tether from its dish to his orb: taut while it guards him, a
        // flicker otherwise.
        let (_, _, dish) = pods(&c, hall);
        let orb = Vec2::new(b.orb_standing().x * side, b.orb_standing().y);
        line(inv, "tether", Spot::World(dish.into()), Spot::Body(orb.into()), "force", Some(TETHER), HOLD_S)?;
    }

    // ── Where he goes ──
    let phase = c.phase();
    let mut walking = false;
    let pos = match part {
        Some(Part { mv: Move::GateStep, striking: true, .. }) => Vec2::new(c.ring_x, stand_y),
        Some(Part { mv: Move::Sweep, striking: true, t, dur }) => {
            let k = ease(t / (dur * 0.4));
            Vec2::new(clamp_x(c.step_from + side * b.len(SWEEP_STEP) * k), stand_y)
        }
        Some(_) => Vec2::new(at.x, stand_y),
        None if c.report.is_some() || c.recover.is_some() || c.overheat.is_some() || c.beat.is_some() => {
            Vec2::new(at.x, stand_y)
        }
        None => {
            // Between moves he turns to you and keeps his distance.
            side = face(at.x);
            let goal = clamp_x(target.x - side * STANDOFF[phase]);
            let to = goal - at.x;
            let stride = b.len(WALK_SPEED[phase]) * dt;
            if to.abs() > stride {
                walking = true;
                Vec2::new(at.x + to.signum() * stride, stand_y)
            } else {
                Vec2::new(goal, stand_y)
            }
        }
    };
    if let Some(Part { mv: Move::GateStep, striking: true, .. }) = part {
        // Out of the ring, facing back the way he came, toward you.
        side = face(pos.x);
    }

    // ── The moves' own tells and strikes ──
    // Reporting, he does nothing else: the order went unanswered.
    if let Some(p) = part.filter(|_| c.report.is_none()) {
        let (low, high, _) = pods(&c, hall);
        let dir = Vec2::new(c.uplink_dir, 0.0);
        match (p.mv, p.striking) {
            (Move::Lines, false) if c.uplink_up && p.t >= SIGHT_FROM => {
                sight(inv, "sight_low", low, to_wall(low, dir, hall, 0.0))?;
                sight(inv, "sight_high", high, to_wall(high, dir, hall, 0.0))?;
            }
            (Move::Lines, true) => {
                if c.shots == 0 {
                    c.shots = 1;
                    c.ray_generation = c.ray_generation.wrapping_add(1);
                    ray(inv, 0, c.ray_generation, Spot::World(low.into()), dir, width, LINE_WIDTH, LINE_DAMAGE, LINE_KNOCKBACK, LINE_S)?;
                    play(inv, SFX_LINE, low)?;
                    burst(inv, low, 10, 200.0, CRIMSON, "spark")?;
                }
                if c.shots == 1 && p.t >= HIGH_AT {
                    c.shots = 2;
                    c.ray_generation = c.ray_generation.wrapping_add(1);
                    ray(inv, 1, c.ray_generation, Spot::World(high.into()), dir, width, LINE_WIDTH, LINE_DAMAGE, LINE_KNOCKBACK, LINE_S)?;
                    play(inv, SFX_LINE, high)?;
                    burst(inv, high, 10, 200.0, CRIMSON, "spark")?;
                }
                if c.shots == 1 && p.t >= HIGH_AT - 0.35 {
                    // The high line's sight comes back before it fires.
                    sight(inv, "sight_high", high, to_wall(high, dir, hall, 0.0))?;
                }
            }
            (Move::Decree, false) if c.uplink_up => {
                for (i, x) in c.marks().into_iter().enumerate() {
                    let top = Vec2::new(x, (hall.floor - SHELL_DROP).max(hall.floor - 2000.0));
                    if p.t >= SIGHT_FROM {
                        sight(inv, &format!("reticle_{i}"), top, Vec2::new(x, hall.floor))?;
                    }
                    if c.ticks % 8 == (i as u32 % 8) {
                        burst(inv, Vec2::new(x, hall.floor - 4.0), 3, 60.0, CRIMSON, "spark")?;
                    }
                }
            }
            (Move::Decree, true) => {
                let marks = c.marks();
                if !c.fired {
                    play(inv, SFX_LAUNCH, Vec2::new(c.uplink_x, hall.floor - DISH))?;
                }
                while (c.shots as usize) < marks.len() && p.t >= c.shots as f32 * SHELL_EVERY_S {
                    let x = marks[c.shots as usize];
                    inv.submit::<ProjectileSpawnPort>(ProjectileSpawn {
                        reply: None,
                        properties: Default::default(),
                        guard: Default::default(),
                        origin: Vec2::new(x, hall.floor - SHELL_DROP),
                        dir: Vec2::Y,
                        speed: SHELL_SPEED,
                        damage: SHELL_DAMAGE,
                        max_lifetime: 3.0,
                        half_extent: SHELL_HALF,
                        gravity: SHELL_GRAVITY,
                        visual_id: "supreme_leader_shell".to_string(),
                        bounces: 0,
                        bounce_on_world_contact: false,
                        boomerang_return_s: None,
                        splash_half_extent: SHELL_SPLASH,
                    })?;
                    c.shots += 1;
                }
            }
            (Move::Crossfire, false) if c.uplink_up => {
                if p.dur - p.t > CROSSFIRE_LOCK_S {
                    let d = (target - high).normalize_or(dir);
                    c.aim_x = d.x;
                    c.aim_y = d.y;
                } else if c.shots == 0 {
                    // Locked.
                    c.shots = 1;
                    play(inv, SFX_CHARGE, high)?;
                    burst(inv, high, 12, 160.0, CRIMSON, "spark")?;
                }
                let aim = Vec2::new(c.aim_x, c.aim_y);
                sight(inv, "sight_cross", high, to_wall(high, aim, hall, 1000.0))?;
            }
            (Move::Crossfire, true) if !c.fired => {
                let aim = Vec2::new(c.aim_x, c.aim_y);
                c.ray_generation = c.ray_generation.wrapping_add(1);
                ray(inv, 2, c.ray_generation, Spot::World(high.into()), aim, width * 1.5, CROSSFIRE_WIDTH, LINE_DAMAGE, LINE_KNOCKBACK, CROSSFIRE_S)?;
                play(inv, SFX_LINE, high)?;
                burst(inv, high, 14, 220.0, CRIMSON, "spark")?;
            }
            (Move::Precision, false) => {
                let orb = pos + Vec2::new(b.orb_levelled().x * side, b.orb_levelled().y);
                if p.t >= SIGHT_FROM {
                    let aim = lead(orb, target, target_vel);
                    sight(inv, "sight_aim", orb, to_wall(orb, aim, hall, 1000.0))?;
                }
            }
            (Move::Precision, true) => {
                let orb = pos + Vec2::new(b.orb_levelled().x * side, b.orb_levelled().y);
                let count = 3 + phase;
                while (c.shots as usize) < count && p.t >= BOLT_AT[c.shots as usize] {
                    let aim = lead(orb, target, target_vel);
                    inv.submit::<ProjectileSpawnPort>(ProjectileSpawn {
                        reply: None,
                        properties: Default::default(),
                        guard: Default::default(),
                        origin: orb,
                        dir: aim,
                        speed: BOLT_SPEED,
                        damage: BOLT_DAMAGE,
                        max_lifetime: 2.5,
                        half_extent: BOLT_HALF,
                        gravity: 0.0,
                        visual_id: "supreme_leader_bolt".to_string(),
                        bounces: 0,
                        bounce_on_world_contact: false,
                        boomerang_return_s: None,
                        splash_half_extent: 0.0,
                    })?;
                    play(inv, SFX_BOLT, orb)?;
                    burst(inv, orb, 6, 160.0, CRIMSON, "spark")?;
                    c.shots += 1;
                }
            }
            (Move::Sweep, true) => {
                if c.shots == 0 && p.t >= SWEEP_AT {
                    c.shots = 1;
                    swing(inv, b, side)?;
                }
            }
            (Move::GateStep, false) => {
                // The ring opens where he will come out.
                let ring = Vec2::new(c.ring_x, stand_y);
                let grow = ease(p.t / p.dur.max(0.1));
                inv.submit::<LightningAuraPort>(LightningAura {
                    key: "ring".into(),
                    center: (ring - pos).into(),
                    half: [b.len(18.0 + 14.0 * grow), b.len(40.0 + 34.0 * grow)],
                    look: "aura".into(),
                    glow: Some(GATE_GLOW),
                    hold_s: HOLD_S,
                })?;
                if c.ticks % 4 == 0 {
                    burst(inv, ring, 4, 120.0, VIOLET, "spark")?;
                }
            }
            (Move::GateStep, true) => {
                if !c.fired {
                    play(inv, SFX_GATE_IN, pos)?;
                    burst(inv, pos, 30, 260.0, VIOLET, "spark")?;
                }
                if c.shots == 0 && p.t >= GATE_IN_S + SWEEP_AT {
                    c.shots = 1;
                    swing(inv, b, side)?;
                }
            }
            (Move::Mandate, false) => {
                let orb = Vec2::new(b.orb_mandate().x * side, b.orb_mandate().y);
                let from_dir = mandate_dir(0.0, side);
                if p.t >= SIGHT_FROM {
                    sight(inv, "sight_mandate", pos + orb, pos + orb + from_dir * MANDATE_REACH * b.grown())?;
                }
                if c.ticks % 3 == 0 {
                    burst(inv, pos + orb, 4, 140.0, CRIMSON, "spark")?;
                }
            }
            (Move::Mandate, true) => {
                let orb = Vec2::new(b.orb_mandate().x * side, b.orb_mandate().y);
                if c.ray_generation == 0 || !c.fired {
                    c.ray_generation = c.ray_generation.wrapping_add(1);
                    play(inv, SFX_MANDATE, pos + orb)?;
                    shake(inv, 6.0)?;
                }
                let u = ease(p.t / p.dur);
                ray(
                    inv,
                    3,
                    c.ray_generation,
                    Spot::Body(orb.into()),
                    mandate_dir(u, side),
                    MANDATE_REACH * b.grown(),
                    MANDATE_WIDTH,
                    MANDATE_DAMAGE,
                    MANDATE_KNOCKBACK,
                    HOLD_S * 2.0,
                )?;
                if (p.t / 0.3).floor() != ((p.t - dt) / 0.3).floor() {
                    shake(inv, 3.0)?;
                }
            }
            _ => {}
        }
        // (Each tell's cue is its pattern's: the encounter plays it.)
        if p.striking {
            c.fired = true;
        }
    }

    // ── The windows run out ──
    for (state, length) in [(&mut c.report, REPORT_S), (&mut c.recover, RECOVER_S)] {
        if let Some(t) = state.as_mut() {
            *t += dt;
            if *t >= length {
                *state = None;
            }
        }
    }
    if let Some(t) = c.overheat.as_mut() {
        // Overheated until his next tell: the sceptre vents.
        *t += dt;
        if (*t / 0.3).floor() != ((*t - dt) / 0.3).floor() {
            let orb = pos + Vec2::new(b.orb_standing().x * side, b.orb_standing().y);
            burst(inv, orb, 5, 80.0, SMOKE, "dust")?;
        }
    }
    if c.emergency && c.ticks % 2 == 0 {
        // The Emergency Powers burn in him.
        inv.submit::<LightningAuraPort>(LightningAura {
            key: "powers".into(),
            center: [0.0, 0.0],
            half: [b.len(40.0), b.len(84.0)],
            look: "aura".into(),
            glow: Some(TETHER),
            hold_s: HOLD_S * 2.0,
        })?;
    }

    // ── Where he is, and what he is drawn as ──
    let conducts = !boss.driven || c.beat.is_some() || c.report.is_some();
    if !conducts {
        side = if boss.facing < 0.0 { -1.0 } else { 1.0 };
    }
    inv.submit::<ConductedPosePort>(ConductedPose {
        pose: conducts.then(|| Pose { position: pos.into(), velocity: ((pos - at) / dt.max(1e-4)).into() }),
        side: Some(side),
    })?;
    c.turning = c.turning.map(|t| t + dt).filter(|t| *t < TURN_S);
    if side != boss.side && part.is_none_or(|p| !p.striking) {
        c.turning = Some(0.0);
    }
    let row = drawn_row(&c, part, walking);
    inv.submit::<DrawnRowPort>(DrawnRow::body(row.0, row.1, row.2))?;
    c.store(inv)
}

/// He comes round over this long when he turns (`turn`).
const TURN_S: f32 = 0.24;
/// A tell hidden for at most this long is a flinch, held through.
const TELL_GAP_S: f32 = 0.3;

/// Where the uplink stands in phase `n` (0 the Command, 1 the Disruption)
/// and which way its lines fire: the west end first, then the east.
fn uplink_spot(hall: Hall, n: u32) -> (f32, f32) {
    if n == 0 {
        (hall.left + UPLINK_INSET, 1.0)
    } else {
        (hall.right - UPLINK_INSET, -1.0)
    }
}

/// The direction a bolt from `from` leaves to meet you, moving at `vel`: as
/// far ahead of you as the bolt takes to get there, at most `LEAD_MAX_S`.
fn lead(from: Vec2, target: Vec2, vel: Vec2) -> Vec2 {
    let ahead = ((target - from).length() / BOLT_SPEED).min(LEAD_MAX_S);
    let vel = if vel.length() > 1200.0 { Vec2::ZERO } else { vel };
    (target + vel * ahead - from).normalize_or(Vec2::X)
}

/// The Mandate's direction `u` (0 to 1) through its sweep, facing `side`.
fn mandate_dir(u: f32, side: f32) -> Vec2 {
    let deg = MANDATE_FROM + (MANDATE_TO - MANDATE_FROM) * u;
    let r = deg.to_radians();
    Vec2::new(r.cos() * side, r.sin())
}

/// The sceptre's arc in front of him.
fn swing(inv: &mut Invocation<'_>, b: Body, side: f32) -> Result<(), Fault> {
    let (centre, half) = b.sweep_box();
    inv.submit::<RidingHitboxPort>(RidingHitbox {
        offset: Vec2::new(centre.x * side, centre.y).into(),
        half_extent: half.into(),
        circle_radius: None,
        damage: SWEEP_DAMAGE,
        knockback: RidingKnockback::FeelScale(SWEEP_KNOCKBACK),
        launch_dir: None,
        lifetime_s: SWEEP_LIVE_S,
        name: "sl_sweep".into(),
    })?;
    play(inv, SFX_SWEEP, Vec2::ZERO)
}

/// The row he is drawn with, `(name, seconds into it, looping)`: each part
/// of each move has its own frames (his sheet, `supreme_leader.py`).
fn drawn_row(c: &Conductor, part: Option<Part>, walking: bool) -> (&'static str, f32, bool) {
    // Frames `from..to` of a row across `u` (0-1) of a part, `fd` seconds a
    // frame.
    let span = |name, from: f32, to: f32, fd: f32, u: f32| (name, (from + u.clamp(0.0, 0.999) * (to - from)) * fd, false);
    if let Some(t) = c.beat {
        if c.transitions >= 2 {
            return span("emergency", 0.0, 13.0, 0.08, t / BEAT_S);
        }
        return if t < REDEPLOY_AT { ("report", t, true) } else { span("order", 0.0, 6.0, 0.075, (t - REDEPLOY_AT) / 0.5) };
    }
    if let Some(t) = c.report {
        return ("report", t, true);
    }
    if let Some(p) = part {
        let u = p.t / p.dur.max(1e-3);
        return match (p.mv, p.striking) {
            (Move::Lines | Move::Crossfire, false) => span("order", 0.0, 6.0, 0.075, u),
            (Move::Lines | Move::Crossfire, true) => span("order", 7.0, 11.0, 0.075, u * 3.0),
            (Move::Decree, false) => span("decree", 0.0, 7.0, 0.075, u),
            (Move::Decree, true) => span("decree", 7.0, 11.0, 0.075, u * 2.0),
            (Move::Precision, false) => span("aim", 0.0, 3.0, 0.07, u),
            (Move::Precision, true) => span("aim", 4.0, 9.0, 0.07, u),
            (Move::Sweep, false) => span("sweep", 0.0, 4.0, 0.06, u),
            (Move::Sweep, true) => span("sweep", 5.0, 9.0, 0.06, u),
            (Move::GateStep, false) => span("gate_out", 0.0, 7.0, 0.06, u),
            (Move::GateStep, true) if p.t < GATE_IN_S => span("gate_in", 0.0, 5.0, 0.06, p.t / GATE_IN_S),
            (Move::GateStep, true) => span("sweep", 5.0, 9.0, 0.06, (p.t - GATE_IN_S) / (p.dur - GATE_IN_S).max(0.1)),
            (Move::Mandate, _) => ("mandate", p.t, true),
        };
    }
    if let Some(t) = c.overheat {
        return ("overheat", t, true);
    }
    if let Some(t) = c.recover {
        return span("sweep", 9.0, 9.0, 0.06, t / RECOVER_S);
    }
    if let Some(t) = c.turning {
        return span("turn", 0.0, 5.0, 0.07, t / TURN_S);
    }
    let clock = c.ticks as f32 / 60.0;
    if walking {
        ("walk", clock, true)
    } else {
        ("idle", clock, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_move_key_round_trips_and_only_the_orders_need_the_uplink() {
        for mv in Move::ALL {
            assert_eq!(Move::from_key(mv.key()), Some(mv));
            assert_eq!(Move::of_code(mv.code()), Some(mv));
        }
        let orders: Vec<_> = Move::ALL.into_iter().filter(|m| m.is_order()).collect();
        assert_eq!(orders, vec![Move::Lines, Move::Decree, Move::Crossfire]);
    }

    #[test]
    fn the_mandate_sweeps_from_high_in_front_of_him_down_to_the_floor() {
        let start = mandate_dir(0.0, 1.0);
        let end = mandate_dir(1.0, 1.0);
        assert!(start.x > 0.0 && start.y < 0.0, "starts up and forward: {start:?}");
        assert!(end.x > 0.0 && end.y > 0.0, "ends down and forward: {end:?}");
        // Facing left, it is the mirror.
        assert!(mandate_dir(0.5, -1.0).x < 0.0);
        // The floor it reaches at the end is in front of him, inside its reach.
        let b = Body::AUTHORED;
        let orb = b.orb_mandate();
        let over_floor = b.feet_below() - orb.y;
        let lands = over_floor / end.y * end.x;
        assert!(lands > 0.0 && lands < MANDATE_REACH, "the beam lands {lands} in front of him");
    }

    #[test]
    fn the_uplinks_lines_span_a_standing_body_low_and_a_jumping_one_high() {
        // A standing body is 48 tall; a jump rises 88.
        assert!(POD_LOW + LINE_WIDTH * 0.5 < 48.0, "the low line meets a standing body");
        assert!(POD_HIGH - LINE_WIDTH * 0.5 > 48.0, "the high line passes over a standing body");
        assert!(POD_HIGH - LINE_WIDTH * 0.5 < 48.0 + 88.0, "and meets a body at the top of a jump");
    }

    #[test]
    fn his_body_measures_stand_him_on_the_floor() {
        let b = Body::AUTHORED;
        assert!((b.feet_below() - 74.5).abs() < 1.0, "his feet are half his height below him: {}", b.feet_below());
        let (centre, half) = b.sweep_box();
        assert!(centre.x - half.x > 0.0, "the sweep is all in front of him");
    }
}
