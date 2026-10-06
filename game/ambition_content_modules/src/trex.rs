//! The Tyrant King's conductor: it walks the T-rex and performs his moves.
//!
//! His scripted pattern decides WHICH move and WHEN (its `Special` keys: tell,
//! strike, rest). This decides everything that is the move's own: where he
//! walks, the volume each move swings, the row he is drawn with. It holds his
//! pose (`ambition.boss.conducted_pose`): he is a grounded body that walks the
//! floor, lunges, charges the hall and lies stunned against its wall, which no
//! shared movement profile does.
//!
//! ⭐ THE FIGHT'S SHAPE (`docs/planning/game/bosses.md`, "The Tyrant King").
//! A pattern fight: every move has one tell, one dodge and a recovery that is
//! the punish window, and no place is safe. His trunk hurts to touch; under his
//! chin the bite comes down to your height; under his tail the whip sweeps low;
//! on a ledge he snaps up at you. The charge is the long window: he cannot
//! stop, hits the wall and lies stunned.
//!
//! Phase 2, the Quake: his stomp sends a shock along the floor both ways and
//! shakes rocks from the ceiling (each falls where dust trickles first), and
//! his bite comes twice. Enraged: a roar that blows you back and brings the
//! ceiling down, a charge that turns once at the wall and comes back, and a
//! leap that lands where you stood, with a quake.
//!
//! The jaw grab (phase 2 on): a low lunge with the jaws wide; caught, you are
//! clamped in his jaws and thrashed, then flung across the hall. Mash to break
//! free (the engine's capture relation, `ambition.combat.body_hold`).
//!
//! The call: the king of the dinosaurs shrieks for his kin. Stochastic parrots
//! swoop in from the high corners; enraged, raptors run in along the floor
//! too. He calls only while their number is below `MINION_CAP`.
//!
//! Every volume is measured from his art (`scripts/measure_part_track_boxes.py
//! trex_enemy`), in sheet pixels from the frame's top left, and carried into
//! the world by [`PX`] about the frame's centre, which is his position (the
//! boss placement law, `BossSheetSpec::drawn_anchor`).

use ambition_boss_special_port::{
    BossConduct, BossConductPort, BossSummon, BossSummonPort, ConductedPose, ConductedPosePort, DrawnRow, DrawnRowPort,
    Pose,
};
use ambition_combat_port::{
    BodyAttachmentsPort, BodyHold, BodyHoldPort, BodySound, BodySoundPort, Burst, BurstPort, CameraShake, CameraShakePort,
    HeldDamageBox, HeldDamageBoxPort, RidingHitbox, RidingHitboxPort, RidingKnockback,
};
use ambition_extension_sdk::{
    phases::BOSS_CONDUCT, record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};
use ambition_projectile_spec::{ProjectileSpawn, ProjectileSpawnPort};
use bevy_math::Vec2;

/// The behaviour id of the boss it conducts.
pub const TREX_ID: &str = "trex_boss";

/// World units per sheet pixel: his arena's 140 x 112 spawn drawn at the
/// sheet's `collision_scale` 1.6, over the 300 px frame
/// (`trex_volumes_are_measured_at_the_scale_he_is_drawn`).
pub const PX: f32 = 224.0 / 300.0;
/// The frame's centre, in sheet pixels: his position.
const FRAME_CENTRE: Vec2 = Vec2::new(228.0, 150.0);

/// A sheet point (pixels, art facing right) as an offset from his position,
/// unflipped.
fn art(x: f32, y: f32) -> Vec2 {
    (Vec2::new(x, y) - FRAME_CENTRE) * PX
}

/// A sheet box (pixels: left, top, right, bottom) as `(centre, half)` from his
/// position, unflipped.
fn art_box(x0: f32, y0: f32, x1: f32, y1: f32) -> (Vec2, Vec2) {
    let (a, b) = (art(x0, y0), art(x1, y1));
    ((a + b) * 0.5, (b - a) * 0.5)
}

/// His feet (pixel 279): standing, his position is this far above the floor.
pub fn feet_below() -> f32 {
    art(0.0, 279.0).y
}
/// How far his snout reaches ahead of his position standing, and lowered for
/// the charge; how far his tail reaches behind.
fn head_front() -> f32 {
    art(404.0, 0.0).x
}
pub fn ram_front() -> f32 {
    art(434.0, 0.0).x
}
fn tail_back() -> f32 {
    -art(21.0, 0.0).x
}

/// The trunk: torso, thighs and shins, standing. It hurts to touch, which is
/// what makes standing under him a mistake.
pub fn trunk() -> (Vec2, Vec2) {
    art_box(116.0, 104.0, 295.0, 279.0)
}
const TRUNK_DAMAGE: i32 = 1;
const TRUNK_KNOCKBACK: f32 = 1.3;
const TRUNK_REARM: f32 = 0.7;

/// He stalks you between moves, at the pace his feet are drawn to plant
/// (`walk`: 36 px a half-stride, 4 frames of 90 ms).
const WALK_SPEED: f32 = 75.0;
/// He stops this far (position to you) to bite.
const STANDOFF: f32 = 170.0;

/// The bite: the head rears (frames 0-1), then snaps down and forward to your
/// height (frames 2-4), and he lunges one stride with it.
pub fn bite_box() -> (Vec2, Vec2) {
    art_box(314.0, 150.0, 452.0, 244.0)
}
const BITE_DAMAGE: i32 = 2;
const BITE_KNOCKBACK: f32 = 1.4;
const BITE_LUNGE: f32 = 64.0;
const BITE_LUNGE_S: f32 = 0.16;
/// The double bite: one tell, then the bite twice, the second this far into
/// the strike and lunging again from wherever the first left him. The second
/// is the one that catches a dodge back.
const SECOND_BITE_AT_S: f32 = 0.42;

/// The tail whip: the tail coils up (frames 0-2), then sweeps low behind him
/// (frames 3-4). Jump it.
fn tail_box() -> (Vec2, Vec2) {
    art_box(18.0, 150.0, 112.0, 246.0)
}
const TAIL_DAMAGE: i32 = 2;
const TAIL_KNOCKBACK: f32 = 1.3;

/// The charge: the head down at the hall (the `charge` row), at this speed,
/// until his snout meets the wall.
fn ram_box() -> (Vec2, Vec2) {
    art_box(300.0, 84.0, 434.0, 200.0)
}
const CHARGE_SPEED: f32 = 620.0;
const CHARGE_RAMP_S: f32 = 0.25;
const RAM_DAMAGE: i32 = 2;
const RAM_KNOCKBACK: f32 = 1.8;
/// The crash knocks him back this far off the wall, onto his side.
pub const CRASH_RECOIL: f32 = 26.0;

/// The upward snap at a ledge: the head thrown straight up from the rear of
/// the roar (frame 1), and a little ahead.
fn snap_box() -> (Vec2, Vec2) {
    let (centre, half) = art_box(214.0, -90.0, 380.0, 60.0);
    (centre, half)
}
const SNAP_DAMAGE: i32 = 2;
const SNAP_KNOCKBACK: f32 = 1.4;

/// The stomp: he rears onto his far leg (frames 0-1) and stamps the near foot
/// down (frame 2), this far ahead of his position. A shock runs the floor
/// from it both ways: jump it.
fn stomp_foot() -> f32 {
    art(300.0, 0.0).x
}
const WAVE_SPEED: f32 = 430.0;
const WAVE_HALF: Vec2 = Vec2::new(24.0, 18.0);
const WAVE_DAMAGE: i32 = 1;
const WAVE_KNOCKBACK: f32 = 1.1;
/// An upper bound: a wave ends at a wall long before.
const WAVE_LIFETIME_S: f32 = 4.0;

/// Rocks shaken from the ceiling: one over where you stand and the rest
/// spread from it, each falling after its dust has trickled for a while.
const ROCK_SPACING: f32 = 170.0;
/// Under the ceiling (`trex_arena_area.ron`: its underside is 656 above the
/// floor).
const ROCK_DROP_HEIGHT: f32 = 620.0;
const ROCK_WARN_S: f32 = 0.7;
const ROCK_STAGGER_S: f32 = 0.14;
const ROCK_HALF: Vec2 = Vec2::new(14.0, 14.0);
const ROCK_GRAVITY: f32 = 1500.0;
const ROCK_DAMAGE: i32 = 1;
/// How many each shake brings down.
const ROCKS_STOMP: u32 = 3;
const ROCKS_ROAR: u32 = 5;
const ROCKS_LANDING: u32 = 4;
const ROCKS_CRASH: u32 = 4;

/// The enrage roar: the blast leaves his open mouth and blows you back.
fn roar_box() -> (Vec2, Vec2) {
    art_box(300.0, 40.0, 700.0, 279.0)
}
const ROAR_DAMAGE: i32 = 1;
const ROAR_KNOCKBACK: f32 = 2.6;

/// The leap: he springs at where you stood (captured as the strike starts),
/// flies this long on an arc this high, and lands with a quake. The rest of
/// the strike is the landing.
const LEAP_FLIGHT_S: f32 = 0.75;
const LEAP_HEIGHT: f32 = 240.0;
const LAND_DAMAGE: i32 = 2;
const LAND_KNOCKBACK: f32 = 1.6;

// ⛔ A cue the bank does not hold plays NOTHING and says nothing: these name
// cues `sfx.bank.txt` ships (`VOICE` and `BODY` are held to the recipes by
// `every_cue_the_trex_plays_has_a_recipe`).
//
// His VOICE is one throat (the SFX renderer's `creature` mode, Jon's picks of
// 2026-10-06). Each tell's growl and the roar's roar are his pattern's
// telegraph cues (`boss_profiles.ron`). Here: the scream into phase 2, the
// call's roar, a growl as he seizes you and a snarl as he flings you, a hurt
// yelp when he crashes, huffs and low growls while he stalks (a growl's two
// takes in turn), and the death wail once.
const SFX_ROAR: &str = "boss.trex.roar";
const SFX_DEATH: &str = "boss.trex.death";
const SFX_SCREAM: &str = "boss.trex.scream";
const GROWL_LOW: [&str; 2] = ["boss.trex.growl_low_a", "boss.trex.growl_low_b"];
const GROWL_SNARL: [&str; 2] = ["boss.trex.growl_snarl_a", "boss.trex.growl_snarl_b"];
const GROWL_HUFF: [&str; 2] = ["boss.trex.growl_huff_a", "boss.trex.growl_huff_b"];
const GROWL_CHUFF: [&str; 2] = ["boss.trex.growl_chuff_a", "boss.trex.growl_chuff_b"];
const GROWL_GRUNT: [&str; 2] = ["boss.trex.growl_grunt_a", "boss.trex.growl_grunt_b"];
const GROWL_RISE: [&str; 2] = ["boss.trex.growl_rise_a", "boss.trex.growl_rise_b"];
const GROWL_HURT: [&str; 2] = ["boss.trex.growl_hurt_a", "boss.trex.growl_hurt_b"];
/// Stalking between moves, he huffs or growls when he has been quiet this long.
const IDLE_VOICE_S: f32 = 3.2;

/// Every cue his body makes (his voice is `VOICE`).
pub const BODY: [&str; 6] = [SFX_BITE, SFX_TAIL, SFX_STEP, SFX_STOMP, SFX_CRASH, SFX_RUBBLE];

/// Every cue his voice can play.
pub const VOICE: [&str; 17] = [
    SFX_ROAR,
    SFX_SCREAM,
    SFX_DEATH,
    GROWL_LOW[0],
    GROWL_LOW[1],
    GROWL_SNARL[0],
    GROWL_SNARL[1],
    GROWL_HUFF[0],
    GROWL_HUFF[1],
    GROWL_CHUFF[0],
    GROWL_CHUFF[1],
    GROWL_GRUNT[0],
    GROWL_GRUNT[1],
    GROWL_RISE[0],
    GROWL_RISE[1],
    GROWL_HURT[0],
    GROWL_HURT[1],
];
const SFX_BITE: &str = "boss.trex.chomp";
const SFX_TAIL: &str = "boss.trex.tail_whip";
const SFX_STEP: &str = "enemy.trex.footstep";
const SFX_STOMP: &str = "boss.trex.stomp";
const SFX_CRASH: &str = "boss.trex.crash";
const SFX_RUBBLE: &str = "world.rock.break";

/// The jaw grab. The strike lunges like the bite but further, and the jaws
/// close (`grab_reach` frame 3) inside this window of it: a body in the reach
/// then is caught. Caught, it is held between his jaws and thrashed for
/// `SHAKE_S`, bitten every `PUMMEL_EVERY_S`, then flung forward and up.
const GRAB_LUNGE: f32 = 92.0;
const GRAB_LUNGE_S: f32 = 0.18;
const GRAB_CLAMP_FROM_S: f32 = 0.06;
const GRAB_CLAMP_TO_S: f32 = 0.22;
fn grab_reach() -> (Vec2, Vec2) {
    art_box(330.0, 150.0, 470.0, 279.0)
}
/// Where his jaws hold a body: the `jaw` attachment of his body rig. His art
/// states it on his jaw joint (`trex_enemy_body_rig.ron`). A hold names it
/// (`BodyHold::Seize::hold_at`) and the engine places it from his rig's pose
/// each tick, so a held body rides his jaws through the reach, the thrash and
/// the fling. ⛔ No number for it is kept here: a pixel typed into this file
/// stayed where it was when the art's head moved, and the body hung in the
/// air under a raised head.
const JAW: &str = "jaw";
const SHAKE_S: f32 = 1.3;
const PUMMEL_EVERY_S: f32 = 0.42;
const PUMMEL_DAMAGE: i32 = 1;
/// The fling: forward and up, at a launch speed that crosses the hall.
const THROW_DAMAGE: i32 = 2;
const THROW_SPEED: f32 = 820.0;
const THROW_DIR: [f32; 2] = [0.74, -0.67];
const THROW_ROW_S: f32 = 0.4;
/// The hold's own deadline, past the throw: the shake ends in a throw, never
/// in the hold running out.
const HOLD_S: f32 = SHAKE_S + 0.6;

/// The call. Birds are dinosaurs: the parrots come down from the high corners
/// of the hall; enraged, raptors run in at the walls.
const PARROT: &str = "stochastic_parrot";
const PARROT_HALF: [f32; 2] = [18.0, 16.0];
const RAPTOR: &str = "npc_raptor_stalker";
const RAPTOR_HALF: [f32; 2] = [24.0, 40.0];
/// His summons alive at once, at most.
const MINION_CAP: u32 = 4;
/// How far in from each wall, and how high the parrots enter.
const CALL_INSET: f32 = 90.0;
const PARROT_HEIGHT: f32 = 470.0;

/// How hard each blow shakes the camera (world pixels, before the player's
/// shake setting caps it).
const SHAKE_SKID: f32 = 5.0;
const SHAKE_STOMP: f32 = 8.0;
const SHAKE_ROAR: f32 = 9.0;
const SHAKE_LANDING: f32 = 11.0;
const SHAKE_CRASH: f32 = 12.0;
const SHAKE_THRASH: f32 = 4.0;
const SHAKE_THROW: f32 = 7.0;

const DUST: [f32; 4] = [0.82, 0.74, 0.58, 1.0];

/// His moves: the `Special` keys his pattern names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Bite,
    TailWhip,
    Charge,
    SnapUp,
    Stomp,
    Roar,
    Leap,
    DoubleBite,
    JawGrab,
    Call,
}

impl Move {
    pub const ALL: [Move; 10] = [
        Move::Bite,
        Move::TailWhip,
        Move::Charge,
        Move::SnapUp,
        Move::Stomp,
        Move::Roar,
        Move::Leap,
        Move::DoubleBite,
        Move::JawGrab,
        Move::Call,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Move::Bite => "trex_bite",
            Move::TailWhip => "trex_tail_whip",
            Move::Charge => "trex_charge",
            Move::SnapUp => "trex_snap_up",
            Move::Stomp => "trex_stomp",
            Move::Roar => "trex_roar",
            Move::Leap => "trex_leap",
            Move::DoubleBite => "trex_double_bite",
            Move::JawGrab => "trex_jaw_grab",
            Move::Call => "trex_call",
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
}

record! {
    /// The conductor's memory, on the T-rex.
    pub struct Conductor = SchemaKey::new(crate::PROVIDER, "trex.conductor", 1);
    /// The hall, measured once.
    1 hall_known: bool,
    2 hall_floor: f32,
    3 hall_left: f32,
    4 hall_right: f32,
    /// The live part: its move (`Move::code`, 0 for none), whether striking,
    /// seconds into it, and its full length.
    5 part_move: u32,
    6 part_striking: bool,
    7 part_t: f32,
    8 part_dur: f32,
    /// Last tick's time remaining in the live part: a part that restarts shows
    /// as this jumping back up.
    9 last_remaining: f32,
    10 clock: f32,
    11 ticks: u32,
    /// The strike's one-shot effect has been spent.
    12 fired: bool,
    /// Where the bite's lunge started.
    13 lunge_from: f32,
    /// Charging: seconds into the run (it outlives the strike: he runs until
    /// he hits the wall).
    14 charging: Option<f32>,
    /// Lying stunned against the wall: seconds since the crash.
    15 stunned: Option<f32>,
    /// Seconds until his trunk can hurt again.
    16 trunk_rearm: f32,
    /// The stride clock, for footfalls.
    17 stride: f32,
    /// A rock fall: seconds since the shake, the x it centres on, how many
    /// rocks and how many have fallen.
    18 rocks_t: Option<f32>,
    19 rocks_x: f32,
    20 rocks_count: u32,
    21 rocks_dropped: u32,
    /// The shocks rolling the floor: where each is and which way it runs. A
    /// stamp starts a new generation of both.
    22 wave_generation: u32,
    23 wave0_x: Option<f32>,
    24 wave0_dir: f32,
    25 wave1_x: Option<f32>,
    26 wave1_dir: f32,
    /// Turns left in this charge (enraged, he turns once at the wall).
    27 charge_turns: u32,
    /// The leap: from where, to where, and whether he has landed.
    28 leap_from: f32,
    29 leap_to: f32,
    30 landed: bool,
    /// The double bite's second snap has been spent.
    31 bit_again: bool,
    /// Holding a body in his jaws: seconds of the thrash so far.
    32 thrashing: Option<f32>,
    /// The thrash's last pummel, in its seconds.
    33 pummelled_at: f32,
    /// The fling's follow-through: seconds since the throw.
    34 flinging: Option<f32>,
    /// His voice: when he last made a sound, how many takes he has used (he
    /// alternates a growl's two), and whether his death wail has sounded.
    35 voiced_at: f32,
    36 takes: u32,
    37 mourned: bool,
    /// Rearing through the beat between two phases: seconds into it. And
    /// whether he has screamed (into phase 2, the first).
    38 rearing: Option<f32>,
    39 screamed: bool,
    /// He has been seen alive. A boss cleared before you walked in loads dead
    /// and never was: his wail is for a death, not for a corpse.
    40 seen_alive: bool,
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
    /// Holding a body in his jaws, for tests and inspectors.
    pub fn thrashing(&self) -> bool {
        self.thrashing.is_some()
    }

    /// Rearing through the beat between two phases.
    pub fn rearing(&self) -> bool {
        self.rearing.is_some()
    }

    fn wave(&mut self, slot: usize) -> (&mut Option<f32>, &mut f32) {
        if slot == 0 {
            (&mut self.wave0_x, &mut self.wave0_dir)
        } else {
            (&mut self.wave1_x, &mut self.wave1_dir)
        }
    }

    /// Shocks leave `x` both ways along the floor.
    fn stamp(&mut self, x: f32) {
        self.wave_generation = self.wave_generation.wrapping_add(1);
        for (slot, dir) in [(0, -1.0), (1, 1.0)] {
            let (wave_x, wave_dir) = self.wave(slot);
            *wave_x = Some(x);
            *wave_dir = dir;
        }
    }

    /// The ceiling sheds `count` rocks centred on `x`.
    fn shake(&mut self, x: f32, count: u32) {
        self.rocks_t = Some(0.0);
        self.rocks_x = x;
        self.rocks_count = count;
        self.rocks_dropped = 0;
    }

    /// Whether the ceiling is shedding rocks, for tests and inspectors.
    pub fn rocks_falling(&self) -> bool {
        self.rocks_t.is_some()
    }

    /// Whether a shock is rolling the floor, for tests and inspectors.
    pub fn shock_rolling(&self) -> bool {
        self.wave0_x.is_some() || self.wave1_x.is_some()
    }

    /// The move being performed and whether it is striking, for tests and
    /// inspectors.
    pub fn performing(&self) -> Option<(Move, bool)> {
        self.part().map(|p| (p.mv, p.striking))
    }

    fn part(&self) -> Option<Part> {
        Move::of_code(self.part_move).map(|mv| Part {
            mv,
            striking: self.part_striking,
            t: self.part_t,
            dur: self.part_dur,
        })
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
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "trex"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![Conductor::schema()],
        entries: vec![EntryDescriptor {
            key: "conduct".into(),
            phase: BOSS_CONDUCT,
            trigger: TriggerBinding {
                port: BossConductPort::KEY,
                selector: TREX_ID.into(),
            },
            reads: vec![BodyAttachmentsPort::KEY],
            writes: vec![Conductor::KEY],
            requests: vec![
                ConductedPosePort::KEY,
                DrawnRowPort::KEY,
                BurstPort::KEY,
                CameraShakePort::KEY,
                BodyHoldPort::KEY,
                BossSummonPort::KEY,
                RidingHitboxPort::KEY,
                HeldDamageBoxPort::KEY,
                ProjectileSpawnPort::KEY,
                BodySoundPort::KEY,
            ],
            after: Vec::new(),
            limits: Limits { max_requests: 48 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(conduct),
        }],
    }
}

fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// One of a growl's two takes, the other next time.
fn voice(inv: &mut Invocation<'_>, c: &mut Conductor, takes: &[&str; 2], at: Vec2) -> Result<(), Fault> {
    let cue = takes[(c.takes % 2) as usize];
    c.takes = c.takes.wrapping_add(1);
    c.voiced_at = c.clock;
    play(inv, cue, at)
}

fn play(inv: &mut Invocation<'_>, cue: &str, at: Vec2) -> Result<(), Fault> {
    inv.submit::<BodySoundPort>(BodySound { cue: cue.into(), at: at.into() })
}

fn shake(inv: &mut Invocation<'_>, amplitude_px: f32) -> Result<(), Fault> {
    inv.submit::<CameraShakePort>(CameraShake { amplitude_px })
}

fn burst(inv: &mut Invocation<'_>, at: Vec2, count: u32, speed: f32, color: [f32; 4], kind: &str) -> Result<(), Fault> {
    inv.submit::<BurstPort>(Burst { at: at.into(), count, speed, color, kind: kind.into() })
}

/// Where rock `i` of a fall centred on `x` lands: the first on `x`, then
/// alternately either side, a little uneven, inside the hall.
fn rock_x(hall: Hall, x: f32, i: u32) -> f32 {
    let k = i.div_ceil(2) as f32 * if i % 2 == 1 { -1.0 } else { 1.0 };
    let jitter = ((i as f32 * 12.9898).sin() * 43758.547).fract() * 40.0 - 20.0;
    (x + k * ROCK_SPACING + if i == 0 { 0.0 } else { jitter }).clamp(hall.left + 30.0, hall.right - 30.0)
}

/// A volume riding his body, from an unflipped art box turned to `side`.
#[allow(clippy::too_many_arguments)]
fn riding(
    inv: &mut Invocation<'_>,
    (centre, half): (Vec2, Vec2),
    side: f32,
    damage: i32,
    knockback: f32,
    life: f32,
    name: &str,
) -> Result<(), Fault> {
    inv.submit::<RidingHitboxPort>(RidingHitbox {
        offset: Vec2::new(centre.x * side, centre.y).into(),
        half_extent: half.into(),
        circle_radius: None,
        damage,
        knockback: RidingKnockback::FeelScale(knockback),
        launch_dir: None,
        lifetime_s: life,
        name: name.into(),
    })
}

fn conduct(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let rex: BossConduct = inv.trigger::<BossConductPort>()?.clone();
    let dt = inv.dt();
    let mut c = Conductor::load(inv)?;
    if !c.hall_known {
        let Some(h) = rex.hall else {
            return Ok(());
        };
        c.hall_known = true;
        c.hall_floor = h.floor;
        c.hall_left = h.left;
        c.hall_right = h.right;
    }
    let hall = Hall { floor: c.hall_floor, left: c.hall_left, right: c.hall_right };
    let mut side = rex.side;
    c.clock += dt;
    c.ticks = c.ticks.wrapping_add(1);
    let at = Vec2::from(rex.position);
    let stand_y = hall.floor - feet_below();
    let target = rex.target.map_or(Vec2::new((hall.left + hall.right) * 0.5, hall.floor - 24.0), Vec2::from);
    // Where he can stand facing `side`: his snout and his tail inside the hall.
    let clamp_x = |x: f32, side: f32| {
        let (front, back) = (head_front(), tail_back());
        let (lo, hi) = if side > 0.0 { (hall.left + back, hall.right - front) } else { (hall.left + front, hall.right - back) };
        x.clamp(lo, hi.max(lo))
    };

    // ── Dead: he settles on the floor and is drawn by the engine ──
    if rex.alive {
        c.seen_alive = true;
    }
    if !rex.alive {
        if c.seen_alive && !c.mourned {
            c.mourned = true;
            play(inv, SFX_DEATH, at + Vec2::new(side * head_front(), -40.0))?;
        }
        inv.submit::<DrawnRowPort>(DrawnRow { name: None, elapsed: 0.0, looping: false })?;
        let pos = Vec2::new(at.x, stand_y);
        inv.submit::<ConductedPosePort>(ConductedPose {
            pose: Some(Pose { position: pos.into(), velocity: ((pos - at) / dt).into() }),
            side,
        })?;
        return c.store(inv);
    }

    // ── Which part of which move is live ──
    let live = match (&rex.active, &rex.telegraph) {
        (Some(active), _) if Move::from_key(&active.key).is_some() => {
            Move::from_key(&active.key).map(|mv| (mv, true, active.remaining))
        }
        (_, Some(tell)) => Move::from_key(&tell.key).map(|mv| (mv, false, tell.remaining)),
        _ => None,
    };
    let prev = c.part();
    let fresh = match (prev, live) {
        (Some(prev), Some((mv, striking, remaining))) => {
            prev.mv != mv || prev.striking != striking || remaining > c.last_remaining + 1e-3
        }
        (None, Some(_)) => true,
        _ => false,
    };
    // A tell that begins this tick: he voices it below, once he has turned.
    let mut told = None;
    let part = match live {
        Some((mv, striking, remaining)) => {
            if fresh {
                if !striking {
                    told = Some(mv);
                    // A tell gets him up off the wall and turned to you.
                    c.stunned = None;
                    if c.charging.is_none() {
                        side = if target.x < at.x { -1.0 } else { 1.0 };
                    }
                } else {
                    c.fired = false;
                    if matches!(mv, Move::Bite | Move::DoubleBite | Move::JawGrab) {
                        c.lunge_from = at.x;
                        c.bit_again = false;
                    }
                    if mv == Move::Charge {
                        c.charging = Some(0.0);
                        c.charge_turns = u32::from(rex.enraged);
                    }
                    if mv == Move::Leap {
                        // At where you stand now: a leap does not track you.
                        side = if target.x < at.x { -1.0 } else { 1.0 };
                        c.leap_from = at.x;
                        c.leap_to = clamp_x(target.x, side);
                        c.landed = false;
                    }
                }
                Some(Part { mv, striking, t: 0.0, dur: remaining.max(1e-3) })
            } else {
                prev.map(|p| Part { t: p.t + dt, ..p })
            }
        }
        None => None,
    };
    c.set_part(part);
    c.last_remaining = live.map_or(0.0, |(_, _, remaining)| remaining);

    // ── Between phases: he stops, turns to you and rears; the first time
    // (into phase 2), screaming ──
    if rex.between_phases {
        if c.rearing.is_none() {
            side = if target.x < at.x { -1.0 } else { 1.0 };
            c.charging = None;
            c.stunned = None;
            if !c.screamed {
                c.screamed = true;
                play(inv, SFX_SCREAM, at + Vec2::new(side * head_front(), -40.0))?;
            }
        }
        c.rearing = Some(c.rearing.map_or(0.0, |t| t + dt));
        c.voiced_at = c.clock;
    } else {
        c.rearing = None;
    }

    // ── Where he goes ──
    let mut crashed = None;
    let mut skidded = None;
    let mut walking = false;
    let mut lift = 0.0;
    let mut second_bite = false;
    let x = if let Some(run_t) = c.charging.as_mut() {
        // He runs the hall and cannot stop: his snout to the wall.
        *run_t += dt;
        let speed = CHARGE_SPEED * ease(*run_t / CHARGE_RAMP_S).max(0.15);
        let wall = if side > 0.0 { hall.right - ram_front() } else { hall.left + ram_front() };
        let next = at.x + side * speed * dt;
        if (next - wall) * side >= 0.0 && c.charge_turns > 0 {
            // Enraged, he digs in at the wall, turns and comes back.
            c.charge_turns -= 1;
            *run_t = 0.0;
            skidded = Some(wall);
            side = -side;
            wall
        } else if (next - wall) * side >= 0.0 {
            crashed = Some(wall);
            wall - side * CRASH_RECOIL
        } else {
            next
        }
    } else if c.stunned.is_some() || c.thrashing.is_some() || c.flinging.is_some() || c.rearing.is_some() {
        at.x
    } else {
        match part {
            Some(Part { mv: Move::Bite, striking: true, t, .. }) => {
                let u = ease(t / BITE_LUNGE_S);
                clamp_x(c.lunge_from + side * BITE_LUNGE * u, side)
            }
            Some(Part { mv: Move::DoubleBite, striking: true, t, .. }) => {
                if t >= SECOND_BITE_AT_S && !c.bit_again {
                    c.bit_again = true;
                    c.lunge_from = at.x;
                    second_bite = true;
                }
                let since = if c.bit_again { t - SECOND_BITE_AT_S } else { t };
                let u = ease(since / BITE_LUNGE_S);
                clamp_x(c.lunge_from + side * BITE_LUNGE * u, side)
            }
            Some(Part { mv: Move::JawGrab, striking: true, t, .. }) => {
                let u = ease(t / GRAB_LUNGE_S);
                clamp_x(c.lunge_from + side * GRAB_LUNGE * u, side)
            }
            Some(Part { mv: Move::Leap, striking: true, t, .. }) => {
                let u = (t / LEAP_FLIGHT_S).clamp(0.0, 1.0);
                lift = 4.0 * LEAP_HEIGHT * u * (1.0 - u);
                c.leap_from + (c.leap_to - c.leap_from) * u
            }
            // Telling and striking, he holds his ground.
            Some(_) => at.x,
            None => {
                // Between moves he turns to you and stalks to biting range.
                side = if target.x < at.x { -1.0 } else { 1.0 };
                let goal = clamp_x(target.x - side * STANDOFF, side);
                let to = goal - at.x;
                let step = WALK_SPEED * dt;
                if to.abs() > step {
                    walking = true;
                    at.x + to.signum() * step
                } else {
                    goal
                }
            }
        }
    };
    let pos = Vec2::new(x, stand_y - lift);
    if let Some(wall) = skidded {
        // `side` has turned: the wall is behind his snout now.
        let snout = Vec2::new(wall - side * ram_front(), hall.floor - 60.0);
        play(inv, SFX_STOMP, snout)?;
        shake(inv, SHAKE_SKID)?;
        burst(inv, Vec2::new(pos.x, hall.floor - 6.0), 24, 260.0, DUST, "dust")?;
    }
    if let Some(wall) = crashed {
        c.charging = None;
        c.stunned = Some(0.0);
        let snout = Vec2::new(wall + side * ram_front(), hall.floor - 70.0);
        play(inv, SFX_CRASH, snout)?;
        play(inv, SFX_RUBBLE, snout)?;
        shake(inv, SHAKE_CRASH)?;
        burst(inv, snout, 36, 360.0, DUST, "dust")?;
        burst(inv, Vec2::new(snout.x, hall.floor - 40.0), 14, 220.0, DUST, "shard")?;
        if rex.enraged {
            // The whole hall shakes: rocks come down on you.
            c.shake(target.x, ROCKS_CRASH);
        }
    }
    if let Some(stunned) = c.stunned.as_mut() {
        // His `stunned` row draws the stars circling his head.
        *stunned += dt;
    }
    if walking {
        c.stride += dt;
        // A footfall every half-stride (four frames of the walk).
        if c.stride >= 0.36 {
            c.stride -= 0.36;
            let foot = Vec2::new(pos.x, hall.floor - 4.0);
            play(inv, SFX_STEP, foot)?;
            burst(inv, foot, 6, 90.0, DUST, "dust")?;
        }
    }

    // ── His voice ──
    // A tell's sound is its pattern cue (`boss_profiles.ron`): the cue is half
    // of what tells one move from another, so it lives with the pose.
    let mouth = pos + Vec2::new(side * head_front(), -40.0);
    if told.is_some() {
        c.voiced_at = c.clock;
    } else if crashed.is_some() {
        voice(inv, &mut c, &GROWL_HURT, mouth)?;
    } else if part.is_none() && c.charging.is_none() && c.stunned.is_none() && c.clock - c.voiced_at >= IDLE_VOICE_S {
        // Stalking: a snort, then a low growl, then a snort...
        let idle = if (c.takes / 2) % 2 == 0 { &GROWL_HUFF } else { &GROWL_LOW };
        voice(inv, &mut c, idle, mouth)?;
    }

    // ── The strike's own onset ──
    if let Some(part) = part {
        if part.striking && !c.fired {
            c.fired = true;
            match part.mv {
                Move::Bite => {
                    riding(inv, bite_box(), side, BITE_DAMAGE, BITE_KNOCKBACK, part.dur * 0.6, "trex_bite")?;
                    play(inv, SFX_BITE, pos + Vec2::new(side * head_front(), 0.0))?;
                }
                Move::DoubleBite => {
                    riding(inv, bite_box(), side, BITE_DAMAGE, BITE_KNOCKBACK, SECOND_BITE_AT_S * 0.6, "trex_bite")?;
                    play(inv, SFX_BITE, pos + Vec2::new(side * head_front(), 0.0))?;
                }
                Move::TailWhip => {
                    riding(inv, tail_box(), side, TAIL_DAMAGE, TAIL_KNOCKBACK, part.dur * 0.6, "trex_tail")?;
                    play(inv, SFX_TAIL, pos - Vec2::new(side * tail_back(), 0.0))?;
                }
                Move::SnapUp => {
                    riding(inv, snap_box(), side, SNAP_DAMAGE, SNAP_KNOCKBACK, part.dur * 0.6, "trex_snap")?;
                    play(inv, SFX_BITE, pos + Vec2::new(0.0, -120.0))?;
                }
                Move::Stomp => {
                    let foot = Vec2::new(pos.x + side * stomp_foot(), hall.floor - 4.0);
                    c.stamp(foot.x);
                    c.shake(target.x, ROCKS_STOMP);
                    play(inv, SFX_STOMP, foot)?;
                    shake(inv, SHAKE_STOMP)?;
                    burst(inv, foot, 30, 300.0, DUST, "dust")?;
                    burst(inv, foot, 10, 200.0, DUST, "shard")?;
                }
                Move::Roar => {
                    riding(inv, roar_box(), side, ROAR_DAMAGE, ROAR_KNOCKBACK, part.dur * 0.5, "trex_roar")?;
                    shake(inv, SHAKE_ROAR)?;
                    c.shake(target.x, ROCKS_ROAR);
                }
                // The leap lands below, when it lands.
                Move::Leap => {}
                // The grab's clamp reaches below, through its window.
                Move::JawGrab => {}
                Move::Call => {
                    // Its tell chuffs; the call itself is the roar.
                    play(inv, SFX_ROAR, mouth)?;
                    c.voiced_at = c.clock;
                    shake(inv, SHAKE_ROAR)?;
                    let high = hall.floor - PARROT_HEIGHT;
                    let mut kin: Vec<(&str, [f32; 2], Vec2)> = vec![
                        (PARROT, PARROT_HALF, Vec2::new(hall.left + CALL_INSET, high)),
                        (PARROT, PARROT_HALF, Vec2::new(hall.right - CALL_INSET, high)),
                    ];
                    if rex.enraged {
                        // The raptors FIRST: they are the enraged call's new
                        // threat, and the cap spends its room in this order.
                        let low = hall.floor - RAPTOR_HALF[1] - 2.0;
                        kin.insert(0, (RAPTOR, RAPTOR_HALF, Vec2::new(hall.right - CALL_INSET, low)));
                        kin.insert(0, (RAPTOR, RAPTOR_HALF, Vec2::new(hall.left + CALL_INSET, low)));
                    }
                    let room = MINION_CAP.saturating_sub(rex.minions) as usize;
                    for (i, (character, half, at)) in kin.into_iter().take(room).enumerate() {
                        inv.submit::<BossSummonPort>(BossSummon {
                            label: "trex_kin".into(),
                            serial: vec![c.ticks, i as u32],
                            position: at.into(),
                            half_size: half,
                            character_id: character.into(),
                            health: None,
                            keeps_contact_damage: true,
                            // His own kin: his volumes pass through them.
                            on_boss_side: true,
                        })?;
                        burst(inv, at, 16, 220.0, DUST, "dust")?;
                    }
                }
                // The charge's ram rides the run, below.
                Move::Charge => {}
            }
        }
    }
    if c.charging.is_some() && c.ticks % 6 == 0 {
        // The ram, refreshed while he runs; dust off his feet.
        riding(inv, ram_box(), side, RAM_DAMAGE, RAM_KNOCKBACK, 0.12, "trex_ram")?;
        burst(inv, Vec2::new(pos.x, hall.floor - 6.0), 5, 140.0, DUST, "dust")?;
    }

    if second_bite {
        let life = part.map_or(0.2, |p| (p.dur - SECOND_BITE_AT_S) * 0.6);
        riding(inv, bite_box(), side, BITE_DAMAGE, BITE_KNOCKBACK, life, "trex_bite_again")?;
        play(inv, SFX_BITE, pos + Vec2::new(side * head_front(), 0.0))?;
    }

    // ── The jaw grab: the clamp, the thrash, the fling ──
    if let Some(Part { mv: Move::JawGrab, striking: true, t, .. }) = part {
        if c.thrashing.is_none() && !rex.holding && (GRAB_CLAMP_FROM_S..=GRAB_CLAMP_TO_S).contains(&t) {
            let (centre, half) = grab_reach();
            inv.submit::<BodyHoldPort>(BodyHold::Seize {
                reach_offset: centre.into(),
                reach_half: half.into(),
                hold_at: Some(JAW.into()),
                hold_offset: [0.0, 0.0],
                hold_s: HOLD_S,
            })?;
            if t == GRAB_CLAMP_FROM_S || (t - dt) < GRAB_CLAMP_FROM_S {
                play(inv, SFX_BITE, pos + Vec2::new(side * head_front(), 20.0))?;
            }
        }
    }
    if rex.holding && c.thrashing.is_none() && c.flinging.is_none() {
        // Caught (the seize lands a tick before the trigger reports it).
        c.thrashing = Some(0.0);
        c.pummelled_at = 0.0;
        voice(inv, &mut c, &GROWL_LOW, pos + Vec2::new(side * head_front(), -40.0))?;
    }
    if let Some(thrash) = c.thrashing.as_mut() {
        if !rex.holding {
            // Mashed free.
            c.thrashing = None;
        } else {
            *thrash += dt;
            let t = *thrash;
            if t >= SHAKE_S {
                inv.submit::<BodyHoldPort>(BodyHold::Throw {
                    damage: THROW_DAMAGE,
                    knockback: THROW_SPEED,
                    growth: 0.0,
                    launch_dir: THROW_DIR,
                })?;
                c.thrashing = None;
                c.flinging = Some(0.0);
                voice(inv, &mut c, &GROWL_SNARL, pos + Vec2::new(side * head_front(), -40.0))?;
                shake(inv, SHAKE_THROW)?;
            } else {
                // The body rides his jaws: the thrash it is shaken with is the
                // thrash his art draws (`grab_shake`), not a second one made here.
                inv.submit::<BodyHoldPort>(BodyHold::Carry {
                    hold_at: Some(JAW.into()),
                    hold_offset: [0.0, 0.0],
                })?;
                if t - c.pummelled_at >= PUMMEL_EVERY_S {
                    c.pummelled_at = t;
                    inv.submit::<BodyHoldPort>(BodyHold::Pummel { damage: PUMMEL_DAMAGE })?;
                    // The bite sounds and sparks at his jaws, where his rig's
                    // pose has them (from his position, +x the way he faces);
                    // at his snout when his rig gives no jaw.
                    let jaw = inv.observe::<BodyAttachmentsPort>().ok().and_then(|points| points.get(JAW));
                    let bite = pos + jaw.map_or(Vec2::new(side * head_front(), 0.0), |[x, y]| Vec2::new(side * x, y));
                    play(inv, SFX_BITE, bite)?;
                    shake(inv, SHAKE_THRASH)?;
                    burst(inv, bite, 6, 160.0, DUST, "spark")?;
                }
            }
        }
    }
    if let Some(fling) = c.flinging.as_mut() {
        *fling += dt;
        if *fling >= THROW_ROW_S {
            c.flinging = None;
        }
    }

    // ── The leap lands ──
    if let Some(Part { mv: Move::Leap, striking: true, t, .. }) = part {
        if t >= LEAP_FLIGHT_S && !c.landed {
            c.landed = true;
            let feet = Vec2::new(pos.x, hall.floor - 4.0);
            c.stamp(feet.x);
            c.shake(target.x, ROCKS_LANDING);
            let (centre, half) = trunk();
            riding(inv, (centre, half + Vec2::new(30.0, 0.0)), side, LAND_DAMAGE, LAND_KNOCKBACK, 0.15, "trex_landing")?;
            play(inv, SFX_STOMP, feet)?;
            play(inv, SFX_RUBBLE, feet)?;
            shake(inv, SHAKE_LANDING)?;
            burst(inv, feet, 40, 380.0, DUST, "dust")?;
            burst(inv, feet, 12, 240.0, DUST, "shard")?;
        }
    }

    // ── The shocks roll the floor ──
    let mut waves: [Option<f32>; 2] = [None; 2];
    for (slot, live) in waves.iter_mut().enumerate() {
        let (x, dir) = c.wave(slot);
        let Some(wx) = x.as_mut() else {
            continue;
        };
        *wx += *dir * WAVE_SPEED * dt;
        if *wx < hall.left + WAVE_HALF.x || *wx > hall.right - WAVE_HALF.x {
            *x = None;
            continue;
        }
        *live = Some(*wx);
    }
    for (slot, wave) in waves.iter().enumerate() {
        if let Some(x) = wave {
            // A crest of dust thrown up as it runs, and grit off its leading
            // edge: it must read from across the hall.
            if c.ticks % 2 == 0 {
                burst(inv, Vec2::new(*x, hall.floor - 8.0), 6, 200.0, DUST, "dust")?;
            }
            if c.ticks % 6 == 0 {
                burst(inv, Vec2::new(*x, hall.floor - 4.0), 3, 240.0, DUST, "shard")?;
            }
            inv.submit::<HeldDamageBoxPort>(HeldDamageBox {
                slot: slot as u32,
                generation: c.wave_generation,
                center: [*x, hall.floor - WAVE_HALF.y],
                half_extent: WAVE_HALF.into(),
                damage: WAVE_DAMAGE,
                knockback: WAVE_KNOCKBACK,
                lifetime_s: WAVE_LIFETIME_S,
            })?;
        }
    }

    // ── The ceiling sheds its rocks: dust trickles where each will fall ──
    if let Some(t) = c.rocks_t.as_mut() {
        *t += dt;
        let t = *t;
        let ceiling = hall.floor - ROCK_DROP_HEIGHT;
        for i in c.rocks_dropped..c.rocks_count {
            let x = rock_x(hall, c.rocks_x, i);
            if t >= ROCK_WARN_S + i as f32 * ROCK_STAGGER_S {
                c.rocks_dropped = i + 1;
                inv.submit::<ProjectileSpawnPort>(ProjectileSpawn {
                    origin: Vec2::new(x, ceiling),
                    dir: Vec2::Y,
                    speed: 60.0,
                    damage: ROCK_DAMAGE,
                    max_lifetime: 2.0,
                    half_extent: ROCK_HALF,
                    gravity: ROCK_GRAVITY,
                    visual_id: "trex_rock".to_string(),
                    bounces: 0,
                    bounce_on_world_contact: false,
                    splash_half_extent: 0.0,
                    boomerang_return_s: None,
                })?;
                burst(inv, Vec2::new(x, ceiling), 8, 120.0, DUST, "shard")?;
            } else if c.ticks % 5 == (i % 5) {
                // The tell: dust trickling from the ceiling, and its shadow
                // stirring the floor beneath.
                burst(inv, Vec2::new(x, ceiling), 2, 50.0, DUST, "dust")?;
                burst(inv, Vec2::new(x, hall.floor - 4.0), 1, 30.0, DUST, "dust")?;
            }
        }
        if c.rocks_dropped >= c.rocks_count {
            c.rocks_t = None;
        }
    }

    // ── His trunk hurts to touch ──
    c.trunk_rearm -= dt;
    // Not while he holds a body: a blow to the held body is a hit reaction,
    // and a reacting captive is released (`release_interrupted_captures`).
    if c.trunk_rearm <= 0.0 && c.thrashing.is_none() {
        c.trunk_rearm = TRUNK_REARM;
        riding(inv, trunk(), side, TRUNK_DAMAGE, TRUNK_KNOCKBACK, TRUNK_REARM, "trex_trunk")?;
    }

    // ── Where he is, and what he is drawn as ──
    let airborne = matches!(part, Some(Part { mv: Move::Leap, striking: true, .. }));
    let conducts = !rex.driven
        || c.charging.is_some()
        || c.stunned.is_some()
        || c.thrashing.is_some()
        || c.flinging.is_some()
        || airborne;
    if !conducts {
        side = if rex.facing < 0.0 { -1.0 } else { 1.0 };
    }
    inv.submit::<ConductedPosePort>(ConductedPose {
        pose: conducts.then(|| Pose { position: pos.into(), velocity: ((pos - at) / dt).into() }),
        side,
    })?;
    let row = drawn_row(&c, part, walking);
    inv.submit::<DrawnRowPort>(DrawnRow { name: Some(row.0.into()), elapsed: row.1, looping: row.2 })?;
    c.store(inv)
}

/// The row he is drawn with: the live move's, its tell frames filling the
/// telegraph and the rest the strike; running; stunned; walking or idling.
fn drawn_row(c: &Conductor, part: Option<Part>, walking: bool) -> (&'static str, f32, bool) {
    // (row, tell frames, total frames, seconds per frame): the sheet's own.
    let split = |name, tell: usize, total: usize, fd: f32, part: Part| {
        let u = (part.t / part.dur).clamp(0.0, 0.999);
        let frame = if part.striking { tell as f32 + u * (total - tell) as f32 } else { u * tell as f32 };
        (name, frame * fd, false)
    };
    if let Some(t) = c.rearing {
        // He rears through the roar's first frames, then his open jaws
        // shudder through the rest for as long as the scream lasts.
        let frame = if t < 0.3 { 2.0 * t / 0.3 } else { 2.0 + ((t - 0.3) / 0.104) % 4.0 };
        return ("roar", frame * 0.104, false);
    }
    if let Some(run_t) = c.charging {
        return ("charge", run_t, true);
    }
    if let Some(stunned) = c.stunned {
        return ("stunned", stunned, true);
    }
    if let Some(thrash) = c.thrashing {
        return ("grab_shake", thrash, true);
    }
    if let Some(fling) = c.flinging {
        return ("grab_throw", fling, false);
    }
    match part {
        Some(p @ Part { mv: Move::Bite, .. }) => split("bite", 2, 7, 0.078, p),
        // The double bite's strike is the bite's twice over.
        Some(p @ Part { mv: Move::DoubleBite, striking: false, .. }) => split("bite", 2, 7, 0.078, p),
        Some(Part { mv: Move::DoubleBite, t, dur, .. }) => {
            let (since, span) = if t < SECOND_BITE_AT_S {
                (t, SECOND_BITE_AT_S)
            } else {
                (t - SECOND_BITE_AT_S, (dur - SECOND_BITE_AT_S).max(1e-3))
            };
            ("bite", (2.0 + 5.0 * (since / span).clamp(0.0, 0.999)) * 0.078, false)
        }
        Some(p @ Part { mv: Move::TailWhip, .. }) => split("tail_swipe", 3, 7, 0.082, p),
        // The charge's tell is a roar and a scrape.
        Some(p @ Part { mv: Move::Charge, striking: false, .. }) => split("roar", 6, 6, 0.104, p),
        Some(Part { mv: Move::Charge, .. }) => ("charge", 0.0, true),
        Some(p @ Part { mv: Move::SnapUp, .. }) => split("snap_up", 2, 6, 0.09, p),
        Some(p @ Part { mv: Move::Stomp, .. }) => split("stomp", 2, 6, 0.092, p),
        Some(p @ Part { mv: Move::JawGrab, .. }) => split("grab_reach", 2, 6, 0.1, p),
        Some(p @ Part { mv: Move::Roar, .. }) => split("roar", 2, 6, 0.104, p),
        // The call: his head thrown back, the roar's rear held, then the
        // shriek.
        Some(p @ Part { mv: Move::Call, .. }) => split("roar", 2, 6, 0.104, p),
        // The leap: the crouch held through the tell, frames 1-4 in the air,
        // 5-7 the landing.
        Some(Part { mv: Move::Leap, striking: false, .. }) => ("leap", 0.0, false),
        Some(Part { mv: Move::Leap, t, dur, .. }) => {
            let frame = if t < LEAP_FLIGHT_S {
                1.0 + 4.0 * t / LEAP_FLIGHT_S
            } else {
                5.0 + 3.0 * ((t - LEAP_FLIGHT_S) / (dur - LEAP_FLIGHT_S).max(1e-3)).clamp(0.0, 0.999)
            };
            ("leap", frame.min(7.999) * 0.09, false)
        }
        None if walking => ("walk", c.clock, true),
        None => ("idle", c.clock, true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// He stands on the floor: his feet, not his frame, meet it.
    #[test]
    fn his_feet_are_where_his_art_draws_them() {
        // Pixel 279 of a 300 px frame, at 224/300 world units a pixel.
        assert!((feet_below() - 129.0 * PX).abs() < 1e-4);
        // His snout reaches further lowered than standing, and his tail
        // reaches as far behind.
        assert!(ram_front() > head_front());
        assert!(tail_back() > head_front());
    }

    /// The bite reaches a standing player under his chin: the box comes down
    /// to within a player's height of the floor, ahead of his trunk.
    #[test]
    fn the_bite_comes_down_to_a_standing_players_height() {
        let (bite, half) = bite_box();
        let (trunk, trunk_half) = trunk();
        let floor = feet_below();
        assert!(bite.y + half.y > floor - 44.0, "the bite stops above a standing player");
        assert!(bite.x - half.x < trunk.x + trunk_half.x + 30.0, "the bite leaves a gap under his chin");
    }
}
