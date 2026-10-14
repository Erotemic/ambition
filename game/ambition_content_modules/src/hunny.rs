//! The Hunny Horror's conductor: it walks the bear and performs his moves.
//!
//! His scripted pattern decides WHICH move and WHEN (its `Special` keys: tell,
//! strike, rest). This decides everything that is the move's own: where he
//! walks, the volume each move swings, the row he is drawn with
//! (`ambition.boss.conducted_pose`).
//!
//! ⭐ THE FIGHT'S SHAPE (`docs/planning/game/bosses.md`, "The Hunny Horror").
//! His face is the tell. Sweet, he is harmless: his body does not hurt to
//! touch while he walks, stands or sits dazed. Each attack turns him into the
//! monster first (the tell), and only then does anything hurt. Every move has
//! one dodge and a recovery that is the punish window.
//!
//! - The swipe (phase 1) and the maul (phase 2 on, two blows): claws in front
//!   of him. Back off or jump.
//! - The hug: arms open wide and a giggle. Caught, you are squeezed and thrown
//!   across the room; mash to get free (`ambition.combat.body_hold`).
//! - The honey: globs lobbed at you in arcs. Each lands in a puddle of honey
//!   that slows whoever stands in it (`ambition.combat.slow_bodies`), any
//!   actor, not only you.
//! - The red balloon (his own move; Jon, 2026-10-14): a red balloon in his paw,
//!   he rises on it over the room toward you, lets go and pounces. He lands
//!   dazed, with bees round his head: the long punish window. The balloon
//!   floats on and pops on the ceiling.
//!
//! Every volume is measured from his art (the renderer's `hunny_horror_boss`),
//! in sheet pixels from the frame's top left, and carried into the world by
//! [`PX`] about the frame's centre, which is his position (the boss placement
//! law, `BossSheetSpec::drawn_anchor`).

use ambition_boss_special_port::{BossConduct, BossConductPort, ConductedPose, ConductedPosePort, DrawnRow, DrawnRowPort, Pose};
use ambition_combat_port::{
    BodyHold, BodyHoldPort, BodySound, BodySoundPort, Burst, BurstPort, CameraShake, CameraShakePort, RidingHitbox,
    RidingHitboxPort, RidingKnockback, SlowBodies, SlowBodiesPort,
};
use ambition_extension_sdk::{
    phases::BOSS_CONDUCT, record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};
use ambition_projectile_spec::{ProjectileSpawn, ProjectileSpawnPort};
use bevy_math::Vec2;

/// The behaviour id of the boss it conducts.
pub const HUNNY_ID: &str = "hunny_horror_boss";

/// World units per sheet pixel: his 180-unit body drawn at the sheet's
/// `collision_scale` 1.3, over the 212 px frame.
pub const PX: f32 = 180.0 * 1.3 / 212.0;
/// The frame's centre, in sheet pixels: his position.
const FRAME_CENTRE: Vec2 = Vec2::new(144.0, 106.0);

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

/// His soles (pixel 200): standing, his position is this far above the floor.
pub fn feet_below() -> f32 {
    art(0.0, 200.0).y
}
/// How far his snout reaches ahead of his position standing; how far his rump
/// reaches behind.
fn head_front() -> f32 {
    art(200.0, 0.0).x
}
fn rump_back() -> f32 {
    -art(89.0, 0.0).x
}

/// His body, standing. It hurts to touch only while he is the monster.
pub fn trunk() -> (Vec2, Vec2) {
    art_box(95.0, 40.0, 190.0, 200.0)
}
const TRUNK_DAMAGE: i32 = 1;
const TRUNK_KNOCKBACK: f32 = 1.2;
const TRUNK_REARM: f32 = 0.6;

/// He lumbers toward you between moves (`walk`: 8 frames of 115 ms).
const WALK_SPEED: f32 = 70.0;
/// He stops this far (position to you) to swipe.
const STANDOFF: f32 = 95.0;

/// The swipe: the paw back (frames 0-2), then the claws across in front of
/// him (frames 3-5).
pub fn swipe_box() -> (Vec2, Vec2) {
    art_box(168.0, 80.0, 236.0, 170.0)
}
const SWIPE_DAMAGE: i32 = 2;
const SWIPE_KNOCKBACK: f32 = 1.4;
const SWIPE_STEP: f32 = 30.0;
const SWIPE_STEP_S: f32 = 0.14;

/// The maul: both paws up (frames 0-2), down in front (3-4), up (5), down
/// again further on (6-7). Each blow is a stride on.
fn maul_box() -> (Vec2, Vec2) {
    art_box(172.0, 80.0, 252.0, 170.0)
}
const MAUL_DAMAGE: i32 = 2;
const MAUL_KNOCKBACK: f32 = 1.5;
const MAUL_STEP: f32 = 36.0;
/// The second blow, this far into the strike.
const SECOND_BLOW_AT_S: f32 = 0.4;

/// The hug: arms wide through the tell, then the arms close on whoever is in
/// front of his belly (`hug` frames 3-5) inside this window of the strike.
fn hug_reach() -> (Vec2, Vec2) {
    art_box(150.0, 60.0, 226.0, 200.0)
}
const HUG_STEP: f32 = 40.0;
const HUG_STEP_S: f32 = 0.16;
const HUG_FROM_S: f32 = 0.08;
const HUG_TO_S: f32 = 0.3;
/// Where he holds a body: in front of his belly, between his paws in
/// `squeeze`.
fn hug_hold() -> Vec2 {
    art(178.0, 112.0)
}
const SQUEEZE_S: f32 = 1.4;
const SQUEEZE_EVERY_S: f32 = 0.45;
const SQUEEZE_DAMAGE: i32 = 1;
const THROW_DAMAGE: i32 = 2;
const THROW_SPEED: f32 = 760.0;
const THROW_DIR: [f32; 2] = [0.62, -0.78];
const THROW_ROW_S: f32 = 0.4;
const HOLD_S: f32 = SQUEEZE_S + 0.6;

/// The spit: globs lobbed from his mouth at where you stand and either side
/// of it, landing this long after.
fn mouth() -> Vec2 {
    art(199.0, 94.0)
}
const GLOB_FLIGHT_S: f32 = 0.85;
const GLOB_GRAVITY: f32 = 1300.0;
const GLOB_HALF: Vec2 = Vec2::new(11.0, 11.0);
const GLOB_DAMAGE: i32 = 1;
const GLOB_SPREAD: f32 = 110.0;
/// How many globs a spit throws, and enraged (at most five: the puddle slots).
const GLOBS_EARLY: u32 = 3;
const GLOBS_LATE: u32 = 5;
/// A puddle lies this long after its glob lands. It does not hurt: it holds.
/// A body in it moves at `PUDDLE_SLOW` of its speed, and for `PUDDLE_STICKS_S`
/// after it steps out.
const PUDDLE_S: f32 = 4.0;
const PUDDLE_HALF: Vec2 = Vec2::new(34.0, 10.0);
const PUDDLE_SLOW: f32 = 0.4;
const PUDDLE_STICKS_S: f32 = 0.8;

/// The red balloon: it rises from his paw (`float` frame 0) and its picture
/// hangs its string to the paw; the projectile's body is the middle of the
/// picture, this far over the paw (`red_balloon` drawn 56 wide).
fn paw_up() -> Vec2 {
    art(110.0, 24.0)
}
const BALLOON_HALF: Vec2 = Vec2::new(18.0, 64.0);
const BALLOON_OVER_PAW: f32 = 68.0;
/// He rises this long to this far over the floor, toward a place this far
/// behind you, and lets go.
const RISE_S: f32 = 1.1;
const RISE_TO: f32 = 330.0;
const RISE_BEHIND: f32 = 110.0;
/// The pounce: claws first at where you are when he lets go, this fast.
const POUNCE_SPEED: f32 = 900.0;
fn pounce_box() -> (Vec2, Vec2) {
    art_box(176.0, 86.0, 262.0, 168.0)
}
/// The landing: his belly and his claws come down together.
fn landing_box() -> (Vec2, Vec2) {
    art_box(100.0, 90.0, 262.0, 170.0)
}
/// The lowest point of his pounce (pixel 165): there his claws meet the floor.
fn pounce_low() -> f32 {
    art(0.0, 165.0).y
}
const POUNCE_DAMAGE: i32 = 2;
/// The underside of the ceiling over the floor (`hunny_horror_arena_area.ron`:
/// 656): where his balloon pops.
const CEILING: f32 = 656.0;
const POUNCE_KNOCKBACK: f32 = 1.6;

const SHAKE_POUNCE: f32 = 10.0;
const SHAKE_SQUEEZE: f32 = 3.0;
const SHAKE_THROW: f32 = 6.0;

const DUST: [f32; 4] = [0.82, 0.74, 0.58, 1.0];
const BALLOON_RED: [f32; 4] = [0.84, 0.12, 0.16, 1.0];
const HONEY: [f32; 4] = [0.96, 0.66, 0.12, 1.0];
const STUFFING: [f32; 4] = [0.96, 0.94, 0.86, 1.0];

/// Every cue his body makes. ⛔ A cue the bank does not hold plays NOTHING
/// and says nothing: these name cues `sfx.bank.txt` ships. His own voice is
/// still to be auditioned; these are borrowed.
pub const BODY: [&str; 8] = [SFX_SWIPE, SFX_CLAMP, SFX_SPIT, SFX_SPLAT, SFX_STOMP, SFX_POP, SFX_THROW, SFX_STEP];
const SFX_SWIPE: &str = "boss.bear_mauler.swipe";
const SFX_CLAMP: &str = "boss.mockingbird.chomp";
const SFX_SPIT: &str = "enemy.slime.attack";
const SFX_SPLAT: &str = "vfx.generic_exotic.slime_splat";
const SFX_STOMP: &str = "boss.trex.stomp";
const SFX_POP: &str = "projectile.generic.impact";
const SFX_ROAR: &str = "boss.bear_mauler.roar";
const SFX_THROW: &str = "player.robot.slash.air";
const SFX_STEP: &str = "player.land";

/// His moves: the `Special` keys his pattern names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Swipe,
    Maul,
    Hug,
    Spit,
    Balloon,
}

impl Move {
    pub const ALL: [Move; 5] = [Move::Swipe, Move::Maul, Move::Hug, Move::Spit, Move::Balloon];

    pub fn key(self) -> &'static str {
        match self {
            Move::Swipe => "hunny_swipe",
            Move::Maul => "hunny_maul",
            Move::Hug => "hunny_hug",
            Move::Spit => "hunny_spit",
            Move::Balloon => "hunny_balloon",
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
    /// The conductor's memory, on the bear.
    pub struct Conductor = SchemaKey::new(crate::PROVIDER, "hunny.conductor", 2);
    /// The room, measured once.
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
    /// Where the strike's step started.
    13 step_from: f32,
    /// Sitting dazed after a pounce: seconds since he landed.
    14 dazed: Option<f32>,
    /// Seconds until his body can hurt again.
    15 trunk_rearm: f32,
    16 stride: f32,
    /// The maul's second blow has been spent.
    17 struck_again: bool,
    /// Holding a body in his arms: seconds of the squeeze so far.
    18 squeezing: Option<f32>,
    19 squeezed_at: f32,
    /// The throw's follow-through: seconds since the throw.
    20 throwing: Option<f32>,
    /// The puddles of the last spit: seconds since it, where each lands, and
    /// how many there are.
    21 puddles_t: Option<f32>,
    22 puddle0: f32,
    23 puddle1: f32,
    24 puddle2: f32,
    25 puddle3: f32,
    26 puddle4: f32,
    27 puddle_count: u32,
    /// The balloon: where he rose from and to; where he pounces from and to
    /// (from when he lets go); whether he has landed.
    28 rise_from_x: f32,
    29 rise_from_y: f32,
    30 rise_to_x: f32,
    31 rise_to_y: f32,
    32 pounce_from: Option<[f32; 2]>,
    33 pounce_to_x: f32,
    34 pounce_to_y: f32,
    35 pounced: bool,
    /// The balloon he let go of, rising on: where it is, how fast it rises,
    /// and where the ceiling is.
    36 balloon_x: f32,
    37 balloon_y: Option<f32>,
    38 balloon_vx: f32,
    39 balloon_vy: f32,
    /// Roaring through the beat between two phases: seconds into it.
    40 rearing: Option<f32>,
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
    /// Holding a body in his arms, for tests and inspectors.
    pub fn squeezing(&self) -> bool {
        self.squeezing.is_some()
    }

    /// Sitting dazed after a pounce, for tests and inspectors.
    pub fn dazed(&self) -> bool {
        self.dazed.is_some()
    }

    /// Up on his balloon, rising (before he lets go), for tests and
    /// inspectors.
    pub fn rising(&self) -> bool {
        matches!(self.part(), Some(Part { mv: Move::Balloon, striking: true, .. })) && self.pounce_from.is_none()
    }

    /// Pouncing: he has let go and not landed, for tests and inspectors.
    pub fn pouncing(&self) -> bool {
        self.pounce_from.is_some() && !self.pounced
    }

    /// Whether honey puddles lie on the floor, for tests and inspectors.
    pub fn puddles_live(&self) -> bool {
        self.puddles_t.is_some_and(|t| t >= GLOB_FLIGHT_S)
    }

    /// The move being performed and whether it is striking, for tests and
    /// inspectors.
    pub fn performing(&self) -> Option<(Move, bool)> {
        self.part().map(|p| (p.mv, p.striking))
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

    fn puddle(&self, i: u32) -> f32 {
        [self.puddle0, self.puddle1, self.puddle2, self.puddle3, self.puddle4][i as usize]
    }

    fn set_puddle(&mut self, i: u32, x: f32) {
        *[&mut self.puddle0, &mut self.puddle1, &mut self.puddle2, &mut self.puddle3, &mut self.puddle4][i as usize] = x;
    }
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "hunny"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![Conductor::schema()],
        entries: vec![EntryDescriptor {
            key: "conduct".into(),
            phase: BOSS_CONDUCT,
            trigger: TriggerBinding { port: BossConductPort::KEY, selector: HUNNY_ID.into() },
            reads: Vec::new(),
            writes: vec![Conductor::KEY],
            requests: vec![
                ConductedPosePort::KEY,
                DrawnRowPort::KEY,
                BurstPort::KEY,
                CameraShakePort::KEY,
                BodyHoldPort::KEY,
                RidingHitboxPort::KEY,
                SlowBodiesPort::KEY,
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

fn play(inv: &mut Invocation<'_>, cue: &str, at: Vec2) -> Result<(), Fault> {
    inv.submit::<BodySoundPort>(BodySound { cue: cue.into(), at: ambition_combat_port::Place::World(at.into()) })
}

fn shake(inv: &mut Invocation<'_>, amplitude_px: f32) -> Result<(), Fault> {
    inv.submit::<CameraShakePort>(CameraShake { amplitude_px })
}

fn burst(inv: &mut Invocation<'_>, at: Vec2, count: u32, speed: f32, color: [f32; 4], kind: &str) -> Result<(), Fault> {
    inv.submit::<BurstPort>(Burst { at: at.into(), count, speed, color, kind: kind.into() })
}

/// A volume riding his body, from an unflipped art box turned to `side`.
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

/// A thing that flies on its own: a glob of honey he spits, or his balloon.
#[allow(clippy::too_many_arguments)]
fn fly(
    inv: &mut Invocation<'_>,
    visual: &str,
    origin: Vec2,
    velocity: Vec2,
    gravity: f32,
    damage: i32,
    half: Vec2,
    life: f32,
) -> Result<(), Fault> {
    let speed = velocity.length();
    inv.submit::<ProjectileSpawnPort>(ProjectileSpawn {
        origin,
        dir: if speed > 1e-3 { velocity / speed } else { Vec2::NEG_Y },
        speed: speed.max(0.01),
        damage,
        max_lifetime: life,
        half_extent: half,
        gravity,
        visual_id: visual.to_string(),
        bounces: 0,
        bounce_on_world_contact: false,
        splash_half_extent: 0.0,
        boomerang_return_s: None,
    })
}

/// Where glob `i` of a spit at `x` lands: the first on `x`, then alternately
/// either side, inside the room.
fn spread_x(hall: Hall, x: f32, i: u32, spacing: f32) -> f32 {
    let k = i.div_ceil(2) as f32 * if i % 2 == 1 { -1.0 } else { 1.0 };
    (x + k * spacing).clamp(hall.left + 40.0, hall.right - 40.0)
}

fn conduct(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let bear: BossConduct = inv.trigger::<BossConductPort>()?.clone();
    let dt = inv.dt();
    let mut c = Conductor::load(inv)?;
    if !c.hall_known {
        let Some(h) = bear.hall else {
            return Ok(());
        };
        c.hall_known = true;
        c.hall_floor = h.floor;
        c.hall_left = h.left;
        c.hall_right = h.right;
    }
    let hall = Hall { floor: c.hall_floor, left: c.hall_left, right: c.hall_right };
    let mut side = bear.side;
    c.clock += dt;
    c.ticks = c.ticks.wrapping_add(1);
    let at = Vec2::from(bear.position);
    let stand_y = hall.floor - feet_below();
    let target = bear.target.map_or(Vec2::new((hall.left + hall.right) * 0.5, hall.floor - 24.0), Vec2::from);
    // Where he can stand facing `side`: his snout and his rump inside the room.
    let clamp_x = |x: f32, side: f32| {
        let (front, back) = (head_front(), rump_back());
        let (lo, hi) = if side > 0.0 { (hall.left + back, hall.right - front) } else { (hall.left + front, hall.right - back) };
        x.clamp(lo, hi.max(lo))
    };
    let face = |x: f32| if target.x < x { -1.0 } else { 1.0 };
    // Where his raised paw is, standing at `pos` facing `side`.
    let paw = |pos: Vec2, side: f32| pos + Vec2::new(paw_up().x * side, paw_up().y);

    // ── The balloon he let go of rises on, and pops on the ceiling ──
    if let Some(y) = c.balloon_y.as_mut() {
        *y += c.balloon_vy * dt;
        c.balloon_x += c.balloon_vx * dt;
        let ceiling = hall.floor - CEILING;
        if *y - BALLOON_HALF.y <= ceiling {
            let top = Vec2::new(c.balloon_x, ceiling + 30.0);
            c.balloon_y = None;
            play(inv, SFX_POP, top)?;
            burst(inv, top, 18, 260.0, BALLOON_RED, "shard")?;
        }
    }

    // ── Dead: he settles on the floor and is drawn by the engine ──
    if !bear.alive {
        inv.submit::<DrawnRowPort>(DrawnRow::body_as_engine())?;
        let pos = Vec2::new(at.x, stand_y);
        inv.submit::<ConductedPosePort>(ConductedPose {
            pose: Some(Pose { position: pos.into(), velocity: ((pos - at) / dt).into() }),
            side: Some(side),
        })?;
        return c.store(inv);
    }

    // ── Which part of which move is live ──
    let live = match (&bear.active, &bear.telegraph) {
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
    let part = match live {
        Some((mv, striking, remaining)) => {
            if fresh {
                if !striking {
                    // A tell gets him up and turned to you.
                    c.dazed = None;
                    side = face(at.x);
                    if mv == Move::Balloon {
                        // A red balloon in his raised paw while he giggles. It
                        // is the one he rises on: the strike's balloon takes
                        // its place as this one ends.
                        let held = paw(Vec2::new(at.x, stand_y), side) - Vec2::new(0.0, BALLOON_OVER_PAW);
                        fly(inv, "red_balloon", held, Vec2::ZERO, 0.0, 0, BALLOON_HALF, remaining.max(dt))?;
                        burst(inv, held + Vec2::new(0.0, -40.0), 8, 120.0, BALLOON_RED, "spark")?;
                    }
                } else {
                    c.fired = false;
                    c.step_from = at.x;
                    c.struck_again = false;
                    if mv == Move::Balloon {
                        // Up over the room, to a place behind where you are.
                        let to_x = (target.x - side * RISE_BEHIND).clamp(hall.left + 80.0, hall.right - 80.0);
                        c.rise_from_x = at.x;
                        c.rise_from_y = at.y;
                        c.rise_to_x = to_x;
                        c.rise_to_y = hall.floor - RISE_TO;
                        c.pounce_from = None;
                        c.pounced = false;
                        let v = (Vec2::new(c.rise_to_x, c.rise_to_y) - at) / RISE_S;
                        // The balloon he rises on, rising with him; it rises on
                        // when he lets go.
                        let held = paw(at, side) - Vec2::new(0.0, BALLOON_OVER_PAW);
                        fly(inv, "red_balloon", held, v, 0.0, 0, BALLOON_HALF, RISE_S)?;
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

    // ── Between phases: he stops, turns to you and roars ──
    if bear.between_phases {
        if c.rearing.is_none() {
            side = face(at.x);
            c.dazed = None;
            play(inv, SFX_ROAR, at + Vec2::new(side * head_front(), -60.0))?;
        }
        c.rearing = Some(c.rearing.map_or(0.0, |t| t + dt));
    } else {
        c.rearing = None;
    }

    // ── Where he goes ──
    let mut walking = false;
    let mut landed = false;
    let step = |c: &Conductor, distance: f32, over_s: f32, t: f32, side: f32| {
        clamp_x(c.step_from + side * distance * ease(t / over_s), side)
    };
    let balloon = matches!(part, Some(Part { mv: Move::Balloon, striking: true, .. }));
    let pos = if balloon && !c.pounced {
        let t = part.map_or(0.0, |p| p.t);
        if t < RISE_S {
            // Up on the balloon.
            let u = t / RISE_S;
            Vec2::new(c.rise_from_x, c.rise_from_y).lerp(Vec2::new(c.rise_to_x, c.rise_to_y), u)
        } else {
            if c.pounce_from.is_none() {
                // He lets go: the balloon rises on alone, and he pounces at
                // where you are now.
                let from = Vec2::new(c.rise_to_x, c.rise_to_y);
                side = face(from.x);
                c.pounce_from = Some(from.into());
                // His claws, not his middle, at you.
                c.pounce_to_x = clamp_x(target.x - side * pounce_box().0.x, side);
                c.pounce_to_y = hall.floor - pounce_low();
                let held = paw(from, side) - Vec2::new(0.0, BALLOON_OVER_PAW);
                c.balloon_x = held.x;
                c.balloon_y = Some(held.y);
                c.balloon_vx = (c.rise_to_x - c.rise_from_x) / RISE_S;
                c.balloon_vy = (c.rise_to_y - c.rise_from_y) / RISE_S;
                fly(inv, "red_balloon", held, Vec2::new(c.balloon_vx, c.balloon_vy), 0.0, 0, BALLOON_HALF, 4.0)?;
                play(inv, SFX_ROAR, from + Vec2::new(side * head_front(), -40.0))?;
            }
            let from = Vec2::from(c.pounce_from.unwrap_or([at.x, at.y]));
            let to = Vec2::new(c.pounce_to_x, c.pounce_to_y);
            let span = (to - from).length().max(1.0);
            let u = ((t - RISE_S) * POUNCE_SPEED / span).min(1.0);
            if u >= 1.0 {
                c.pounced = true;
                landed = true;
            }
            from.lerp(to, u)
        }
    } else if c.dazed.is_some() || c.squeezing.is_some() || c.throwing.is_some() || c.rearing.is_some() {
        Vec2::new(at.x, stand_y)
    } else {
        let x = match part {
            Some(Part { mv: Move::Swipe, striking: true, t, .. }) => step(&c, SWIPE_STEP, SWIPE_STEP_S, t, side),
            Some(Part { mv: Move::Maul, striking: true, t, .. }) => {
                if t >= SECOND_BLOW_AT_S && !c.struck_again {
                    c.struck_again = true;
                    c.step_from = at.x;
                    let life = part.map_or(0.2, |p| (p.dur - SECOND_BLOW_AT_S) * 0.6);
                    riding(inv, maul_box(), side, MAUL_DAMAGE, MAUL_KNOCKBACK, life, "hunny_maul_again")?;
                    play(inv, SFX_SWIPE, at + Vec2::new(side * head_front(), 0.0))?;
                }
                let since = if c.struck_again { t - SECOND_BLOW_AT_S } else { t };
                step(&c, MAUL_STEP, SWIPE_STEP_S, since, side)
            }
            Some(Part { mv: Move::Hug, striking: true, t, .. }) => step(&c, HUG_STEP, HUG_STEP_S, t, side),
            // Telling and striking, he holds his ground.
            Some(_) => at.x,
            None => {
                // Between moves he turns to you and lumbers to swiping range.
                side = face(at.x);
                let goal = clamp_x(target.x - side * STANDOFF, side);
                let to = goal - at.x;
                let stride = WALK_SPEED * dt;
                if to.abs() > stride {
                    walking = true;
                    at.x + to.signum() * stride
                } else {
                    goal
                }
            }
        };
        Vec2::new(x, stand_y)
    };
    if landed {
        // Claws into the floor: he sits there dazed until his next tell.
        let feet = Vec2::new(pos.x, hall.floor - 4.0);
        riding(inv, landing_box(), side, POUNCE_DAMAGE, POUNCE_KNOCKBACK, 0.12, "hunny_pounce_landing")?;
        play(inv, SFX_STOMP, feet)?;
        shake(inv, SHAKE_POUNCE)?;
        burst(inv, feet, 30, 300.0, DUST, "dust")?;
        burst(inv, feet + Vec2::new(0.0, -40.0), 12, 200.0, STUFFING, "dust")?;
        c.dazed = Some(0.0);
    }
    if c.pouncing() && c.ticks % 4 == 0 {
        // The claws, refreshed while he dives.
        riding(inv, pounce_box(), side, POUNCE_DAMAGE, POUNCE_KNOCKBACK, 0.1, "hunny_pounce")?;
    }
    if let Some(dazed) = c.dazed.as_mut() {
        *dazed += dt;
    }
    if walking {
        c.stride += dt;
        // A footfall every half-stride (four frames of the walk).
        if c.stride >= 0.46 {
            c.stride -= 0.46;
            let foot = Vec2::new(pos.x, hall.floor - 4.0);
            play(inv, SFX_STEP, foot)?;
            burst(inv, foot, 4, 70.0, DUST, "dust")?;
        }
    }

    // ── The strike's own onset ──
    if let Some(part) = part {
        if part.striking && !c.fired {
            c.fired = true;
            match part.mv {
                Move::Swipe => {
                    riding(inv, swipe_box(), side, SWIPE_DAMAGE, SWIPE_KNOCKBACK, part.dur * 0.55, "hunny_swipe")?;
                    play(inv, SFX_SWIPE, pos + Vec2::new(side * head_front(), 0.0))?;
                }
                Move::Maul => {
                    riding(inv, maul_box(), side, MAUL_DAMAGE, MAUL_KNOCKBACK, SECOND_BLOW_AT_S * 0.6, "hunny_maul")?;
                    play(inv, SFX_SWIPE, pos + Vec2::new(side * head_front(), 0.0))?;
                }
                Move::Spit => {
                    let count = if bear.enraged { GLOBS_LATE } else { GLOBS_EARLY };
                    let from = pos + Vec2::new(mouth().x * side, mouth().y);
                    c.puddles_t = Some(0.0);
                    c.puddle_count = count;
                    for i in 0..count {
                        let land = Vec2::new(spread_x(hall, target.x, i, GLOB_SPREAD), hall.floor - GLOB_HALF.y);
                        c.set_puddle(i, land.x);
                        let flight = GLOB_FLIGHT_S;
                        let v = Vec2::new(
                            (land.x - from.x) / flight,
                            (land.y - from.y - 0.5 * GLOB_GRAVITY * flight * flight) / flight,
                        );
                        fly(inv, "honey_glob", from, v, GLOB_GRAVITY, GLOB_DAMAGE, GLOB_HALF, 2.5)?;
                    }
                    play(inv, SFX_SPIT, from)?;
                    burst(inv, from, 10, 160.0, HONEY, "dust")?;
                }
                Move::Hug | Move::Balloon => {}
            }
        }
    }

    // ── The hug: the arms close, the squeeze, the throw ──
    if let Some(Part { mv: Move::Hug, striking: true, t, .. }) = part {
        if c.squeezing.is_none() && !bear.holding && (HUG_FROM_S..=HUG_TO_S).contains(&t) {
            let (centre, half) = hug_reach();
            inv.submit::<BodyHoldPort>(BodyHold::Seize {
                reach_offset: centre.into(),
                reach_half: half.into(),
                hold_at: None,
                hold_offset: hug_hold().into(),
                hold_s: HOLD_S,
            })?;
        }
    }
    if bear.holding && c.squeezing.is_none() && c.throwing.is_none() {
        // Caught (the seize lands a tick before the trigger reports it).
        c.squeezing = Some(0.0);
        c.squeezed_at = 0.0;
        play(inv, SFX_CLAMP, pos + Vec2::new(side * hug_hold().x, hug_hold().y))?;
    }
    if let Some(squeeze) = c.squeezing.as_mut() {
        if !bear.holding {
            // Mashed free.
            c.squeezing = None;
        } else {
            *squeeze += dt;
            let t = *squeeze;
            if t >= SQUEEZE_S {
                inv.submit::<BodyHoldPort>(BodyHold::Throw {
                    damage: THROW_DAMAGE,
                    knockback: THROW_SPEED,
                    growth: 0.0,
                    launch_dir: THROW_DIR,
                })?;
                c.squeezing = None;
                c.throwing = Some(0.0);
                play(inv, SFX_THROW, pos)?;
                shake(inv, SHAKE_THROW)?;
            } else {
                inv.submit::<BodyHoldPort>(BodyHold::Carry { hold_at: None, hold_offset: hug_hold().into() })?;
                if t - c.squeezed_at >= SQUEEZE_EVERY_S {
                    c.squeezed_at = t;
                    inv.submit::<BodyHoldPort>(BodyHold::Pummel { damage: SQUEEZE_DAMAGE })?;
                    let at = pos + Vec2::new(side * hug_hold().x, hug_hold().y);
                    play(inv, SFX_CLAMP, at)?;
                    shake(inv, SHAKE_SQUEEZE)?;
                    burst(inv, at, 6, 150.0, HONEY, "spark")?;
                }
            }
        }
    }
    if let Some(throw) = c.throwing.as_mut() {
        *throw += dt;
        if *throw >= THROW_ROW_S {
            c.throwing = None;
        }
    }

    // ── The honey puddles: each holds whoever steps in it ──
    if let Some(t) = c.puddles_t.as_mut() {
        *t += dt;
        let t = *t;
        if t >= GLOB_FLIGHT_S + PUDDLE_S {
            c.puddles_t = None;
        } else if t >= GLOB_FLIGHT_S {
            let landing = t - dt < GLOB_FLIGHT_S;
            for i in 0..c.puddle_count {
                let x = c.puddle(i);
                let floor = Vec2::new(x, hall.floor - 3.0);
                if landing {
                    play(inv, SFX_SPLAT, floor)?;
                    burst(inv, floor, 16, 180.0, HONEY, "dust")?;
                } else if c.ticks % 3 == i % 3 {
                    // The puddle: honey across its width, bubbling. It must read
                    // from across the room.
                    let across = ((c.ticks / 3 + i * 7) % 5) as f32 / 4.0 * 2.0 - 1.0;
                    burst(inv, floor + Vec2::new(across * PUDDLE_HALF.x * 0.8, 0.0), 3, 40.0, HONEY, "dust")?;
                }
                inv.submit::<SlowBodiesPort>(SlowBodies {
                    center: [x, hall.floor - PUDDLE_HALF.y],
                    half_extent: PUDDLE_HALF.into(),
                    speed_scale: PUDDLE_SLOW,
                    seconds: PUDDLE_STICKS_S,
                })?;
            }
        }
    }

    // ── His body hurts to touch while he is the monster ──
    // Not through the hug (his arms are the move, and a blow from his belly as
    // they close would knock the body out of them), and not on the balloon,
    // where his pounce has its own claws.
    let gentle = matches!(part, Some(Part { mv: Move::Hug | Move::Balloon, .. }));
    let monster = (part.is_some() && !gentle) || c.rearing.is_some();
    c.trunk_rearm -= dt;
    if monster && c.trunk_rearm <= 0.0 && c.squeezing.is_none() {
        c.trunk_rearm = TRUNK_REARM;
        riding(inv, trunk(), side, TRUNK_DAMAGE, TRUNK_KNOCKBACK, TRUNK_REARM, "hunny_trunk")?;
    }

    // ── Where he is, and what he is drawn as ──
    let conducts = !bear.driven || balloon || c.dazed.is_some() || c.squeezing.is_some() || c.throwing.is_some();
    if !conducts {
        side = if bear.facing < 0.0 { -1.0 } else { 1.0 };
    }
    inv.submit::<ConductedPosePort>(ConductedPose {
        pose: conducts.then(|| Pose { position: pos.into(), velocity: ((pos - at) / dt).into() }),
        side: Some(side),
    })?;
    let row = drawn_row(&c, part, walking);
    inv.submit::<DrawnRowPort>(DrawnRow::body(row.0, row.1, row.2))?;
    c.store(inv)
}

/// The row he is drawn with: the live move's, its tell frames filling the
/// telegraph and the rest the strike; on his balloon; dazed; walking or idling.
fn drawn_row(c: &Conductor, part: Option<Part>, walking: bool) -> (&'static str, f32, bool) {
    // (row, tell frames, total frames, seconds per frame): the sheet's own.
    let split = |name, tell: usize, total: usize, fd: f32, part: Part| {
        let u = (part.t / part.dur).clamp(0.0, 0.999);
        let frame = if part.striking { tell as f32 + u * (total - tell) as f32 } else { u * tell as f32 };
        (name, frame * fd, false)
    };
    if let Some(t) = c.rearing {
        // He roars through the beat between phases, his jaws open.
        let frame = if t < 0.4 { 4.0 * t / 0.4 } else { 4.0 + ((t - 0.4) / 0.1) % 4.0 };
        return ("taunt", frame * 0.1, false);
    }
    if let Some(dazed) = c.dazed {
        return ("stunned", dazed, true);
    }
    if let Some(squeeze) = c.squeezing {
        return ("squeeze", squeeze, true);
    }
    if let Some(throw) = c.throwing {
        return ("throw", throw, false);
    }
    match part {
        Some(p @ Part { mv: Move::Swipe, .. }) => split("slash", 3, 8, 0.085, p),
        Some(p @ Part { mv: Move::Maul, .. }) => split("punch", 3, 10, 0.08, p),
        Some(p @ Part { mv: Move::Hug, .. }) => split("hug", 3, 6, 0.1, p),
        Some(p @ Part { mv: Move::Spit, .. }) => split("spit", 3, 6, 0.09, p),
        // The tell: his paw up with the balloon in it, the sweet frames.
        Some(Part { mv: Move::Balloon, striking: false, t, .. }) => ("float", (t * 0.5) % 0.24, false),
        // Up on the balloon: the sweet frames, the monster's at the top. Then
        // the dive.
        Some(Part { mv: Move::Balloon, t, .. }) if c.pounce_from.is_none() => {
            ("float", (t / RISE_S).clamp(0.0, 0.999) * 6.0 * 0.12, false)
        }
        Some(Part { mv: Move::Balloon, t, .. }) => ("pounce", t, true),
        None if walking => ("walk", c.clock, true),
        None => ("idle", c.clock, true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// He stands on the floor: his soles, not his frame, meet it.
    #[test]
    fn his_feet_are_where_his_art_draws_them() {
        assert!((feet_below() - 94.0 * PX).abs() < 1e-4);
    }

    /// The swipe reaches a standing player in front of him: the box comes down
    /// to a player's height and starts ahead of his belly.
    #[test]
    fn the_swipe_reaches_a_standing_player_in_front_of_him() {
        let (swipe, half) = swipe_box();
        let floor = feet_below();
        assert!(swipe.y + half.y > floor - 44.0, "the swipe passes over a standing player");
        assert!(swipe.x + half.x > STANDOFF, "the swipe stops short of where he stands to swipe");
    }

    /// A held body is in front of him, inside the reach that caught it.
    #[test]
    fn he_holds_a_body_where_his_arms_close() {
        let (reach, half) = hug_reach();
        let hold = hug_hold();
        assert!(hold.x > 0.0);
        assert!((hold - reach).abs().cmple(half).all());
    }

    /// His pounce lands claws first on the floor: its claws reach the floor
    /// where his lowest point does.
    #[test]
    fn the_pounce_reaches_the_floor() {
        let (claws, half) = pounce_box();
        assert!(claws.y + half.y >= pounce_low() - 1.0);
    }
}
