//! Wolfenstein's conductor: it walks the monster and performs his moves.
//!
//! His scripted pattern decides WHICH move and WHEN (its `Special` keys: tell,
//! strike, rest). This decides everything that is the move's own: where he
//! goes, the volume each move swings, the row he is drawn with
//! (`ambition.boss.conducted_pose`).
//!
//! ⭐ THE FIGHT'S SHAPE (`docs/planning/game/bosses.md`, "Wolfenstein — the
//! leash"). He was built to obey, and the wolf in him cannot. He runs on the
//! charge in the bolts of his neck, and each big move spends it. Every cycle
//! ends in the galvanize: lightning on the marks, and then his charge is
//! spent and he goes flat. That is the long punish window. He is a biped the
//! whole fight (Jon, 2026-10-17).
//!
//! - The rake: claws across in front of him, a step on each blow. Two blows,
//!   three from the transition on (the last one high).
//! - The hammer: both fists down; a shock runs the floor both ways. His fists
//!   stick in the floor after it.
//! - The lunge: he drops low, then bites across most of the room. He skids.
//! - The bound (from the transition on): a lunge into the wall behind you, a
//!   kick off it, and a lunge back. He lands winded.
//! - The howl: a ring that pushes you off him. From the transition on it
//!   calls two Failed Prototypes when none are left.
//! - The galvanize: hands on his bolts; sparks mark three places (one under
//!   you); lightning strikes them. Then he is flat until his next tell.
//!
//! Before his first move he is dead where he stands, and the coils wake him.
//! Between the phases lightning strikes him, he shakes, and he howls.
//!
//! Every volume is measured from his art (the renderer's `wolfenstein`), in
//! sheet pixels from the frame's top left, and carried into the world by
//! [`PX`] about the middle of his body box, which is his position.

use ambition_boss_special_port::{
    BossConduct, BossConductPort, BossSummon, BossSummonPort, ConductedPose, ConductedPosePort, DrawnRow, DrawnRowPort,
    Pose,
};
use ambition_combat_port::{
    BodySound, BodySoundPort, Burst, BurstPort, CameraShake, CameraShakePort, HeldDamageBox, HeldDamageBoxPort,
    RidingHitbox, RidingHitboxPort, RidingKnockback,
};
use ambition_extension_sdk::{
    phases::BOSS_CONDUCT, record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};
use bevy_math::Vec2;

/// The behaviour id of the boss it conducts.
pub const WOLFENSTEIN_ID: &str = "wolfenstein_boss";

/// World units per sheet pixel: the 320 px frame is drawn at the sheet's
/// `collision_scale` 2.01 times his 100-unit body box.
pub const PX: f32 = 2.01 * 100.0 / 320.0;
/// The middle of his body box (`body_pixel_bbox` 131, 81, 41 x 159), in sheet
/// pixels: his position.
const BODY_CENTRE: Vec2 = Vec2::new(151.5, 160.5);

/// A sheet point (pixels, art facing right) as an offset from his position,
/// unflipped.
fn art(x: f32, y: f32) -> Vec2 {
    (Vec2::new(x, y) - BODY_CENTRE) * PX
}

/// A sheet box (pixels: left, top, right, bottom) as `(centre, half)` from his
/// position, unflipped.
fn art_box(x0: f32, y0: f32, x1: f32, y1: f32) -> (Vec2, Vec2) {
    let (a, b) = (art(x0, y0), art(x1, y1));
    ((a + b) * 0.5, (b - a) * 0.5)
}

/// His soles (pixel 240): standing, his position is this far above the floor.
pub fn feet_below() -> f32 {
    art(0.0, 240.0).y
}
/// How far his body reaches ahead of and behind his position.
fn body_half() -> f32 {
    art(172.0, 0.0).x
}
/// The bolts in his neck, standing.
fn bolts() -> Vec2 {
    art(151.0, 92.0)
}

/// His body. It hurts to touch only while a move is live.
pub fn trunk() -> (Vec2, Vec2) {
    art_box(131.0, 81.0, 172.0, 240.0)
}
const TRUNK_DAMAGE: i32 = 1;
const TRUNK_KNOCKBACK: f32 = 1.2;
const TRUNK_REARM: f32 = 0.6;

/// He lurches toward you between moves (`walk`: 8 frames of 125 ms), faster
/// from the transition on.
const WALK_SPEED: f32 = 80.0;
const WALK_SPEED_UNLEASHED: f32 = 120.0;
/// He stops this far (position to you) to rake.
const STANDOFF: f32 = 46.0;

/// The rake: the claws across in front of him, from his shoulder to a
/// standing player's knees (`slash` frames 3-6).
pub fn rake_box() -> (Vec2, Vec2) {
    art_box(160.0, 90.0, 238.0, 222.0)
}
/// The high last blow of three (`attack_up` frames 3-5): over his head and
/// in front, down to a standing player's head.
pub fn overhead_box() -> (Vec2, Vec2) {
    art_box(140.0, 30.0, 226.0, 190.0)
}
const RAKE_DAMAGE: i32 = 1;
const RAKE_KNOCKBACK: f32 = 1.2;
const OVERHEAD_DAMAGE: i32 = 2;
const OVERHEAD_KNOCKBACK: f32 = 1.6;
/// Each blow starts this far into the strike, and steps him this far on.
const BLOW_AT: [f32; 3] = [0.0, 0.3, 0.6];
const BLOW_S: f32 = 0.3;
const RAKE_STEP: f32 = 22.0;

/// The hammer: both fists into the floor in front of him (`attack_down`
/// frames 5-7).
pub fn fists_box() -> (Vec2, Vec2) {
    art_box(160.0, 186.0, 236.0, 242.0)
}
/// Where the fists meet the floor: the shocks start here.
fn fists_x() -> f32 {
    art(205.0, 0.0).x
}
const HAMMER_DAMAGE: i32 = 2;
const HAMMER_KNOCKBACK: f32 = 1.6;
/// A shock runs the floor both ways from his fists (held boxes 0 and 1).
const SHOCK_SPEED: f32 = 460.0;
const SHOCK_HALF: Vec2 = Vec2::new(22.0, 16.0);
const SHOCK_DAMAGE: i32 = 1;
const SHOCK_KNOCKBACK: f32 = 1.1;
const SHOCK_LIFETIME_S: f32 = 4.0;
/// His fists stay in the floor this long after the strike, then he pulls them
/// out (`attack_down` frames 8-9).
const STUCK_S: f32 = 0.7;
const PULL_S: f32 = 0.25;

/// The lunge: his whole body low and forward, jaws first (`attack_side`
/// frames 3-5).
pub fn lunge_box() -> (Vec2, Vec2) {
    art_box(150.0, 96.0, 246.0, 240.0)
}
const LUNGE_DAMAGE: i32 = 2;
const LUNGE_KNOCKBACK: f32 = 1.5;
/// He bites across this much of the room at most, and stops short of the wall.
const LUNGE_REACH: f32 = 620.0;
const WALL_GAP: f32 = 12.0;
/// He slides on this far after the strike, over this long.
const SKID: f32 = 44.0;
const SKID_S: f32 = 0.4;

/// The bound: up into the wall behind you (to this high over the floor), a
/// moment on the wall, then the lunge back to the floor this far from the
/// wall. Fractions of the strike.
const BOUND_HIGH: f32 = 130.0;
const BOUND_BACK: f32 = 460.0;
const BOUND_ON_WALL: f32 = 0.36;
const BOUND_OFF_WALL: f32 = 0.5;
/// He lands winded: the window after the bound, until his next tell.
const WINDED_S: f32 = 1.6;

/// The howl: a ring round him that pushes you off.
pub fn howl_box() -> (Vec2, Vec2) {
    art_box(30.0, 60.0, 273.0, 242.0)
}
const HOWL_DAMAGE: i32 = 1;
const HOWL_KNOCKBACK: f32 = 2.2;
/// From the transition on, a howl calls this many of the pack when none are
/// left.
const PACK: u32 = 2;
const PACK_CHARACTER: &str = "npc_evil_failed_prototype";
const PACK_HALF: [f32; 2] = [18.0, 20.0];
const PACK_HEALTH: u32 = 3;

/// The galvanize: three marks, one under you and one each side; lightning
/// strikes each from the ceiling (held boxes 2-4).
const MARK_SPREAD: f32 = 230.0;
const BOLT_HALF: Vec2 = Vec2::new(24.0, 420.0);
const BOLT_DAMAGE: i32 = 2;
const BOLT_KNOCKBACK: f32 = 1.4;
const BOLT_S: f32 = 0.3;

/// The underside of the throne room's ceiling over its floor
/// (`castle_godwin_throne_area.ron`: floor 832, ceiling 48 thick).
const CEILING: f32 = 784.0;

const SHAKE_HAMMER: f32 = 9.0;
const SHAKE_BOLT: f32 = 8.0;
const SHAKE_HOWL: f32 = 5.0;
const SHAKE_LANDING: f32 = 6.0;

const DUST: [f32; 4] = [0.74, 0.70, 0.66, 1.0];
const SPARK: [f32; 4] = [1.0, 0.93, 0.72, 1.0];
const ARC: [f32; 4] = [0.66, 0.84, 1.0, 1.0];
const RING: [f32; 4] = [0.90, 0.88, 0.80, 0.9];

/// Every cue his body makes. ⛔ A cue the bank does not hold plays NOTHING
/// and says nothing: these name cues `sfx.bank.txt` ships. His own voice is
/// still to be auditioned; these are borrowed.
pub const BODY: [&str; 11] = [
    SFX_RAKE, SFX_BITE, SFX_LUNGE, SFX_HAMMER, SFX_SHOCK, SFX_HOWL, SFX_ARC, SFX_BOLT, SFX_SPUTTER, SFX_STEP, SFX_LAND,
];
const SFX_RAKE: &str = "boss.bear_mauler.swipe";
const SFX_BITE: &str = "boss.trex.chomp";
const SFX_LUNGE: &str = "enemy.robot.guardian_lunge";
const SFX_HAMMER: &str = "boss.gnu_ton.stomp";
const SFX_SHOCK: &str = "vfx.explosion.shockwave";
const SFX_HOWL: &str = "boss.generic.intro_roar";
const SFX_ARC: &str = "vfx.generic_world.electric_arc";
const SFX_BOLT: &str = "vfx.generic_world.electric_burst";
const SFX_SPUTTER: &str = "enemy.robot.death_spark";
const SFX_STEP: &str = "player.land";
const SFX_LAND: &str = "boss.trex.stomp";

/// His moves: the `Special` keys his pattern names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Rake,
    RakeThrice,
    Hammer,
    Lunge,
    Bound,
    Howl,
    Galvanize,
}

impl Move {
    pub const ALL: [Move; 7] =
        [Move::Rake, Move::RakeThrice, Move::Hammer, Move::Lunge, Move::Bound, Move::Howl, Move::Galvanize];

    pub fn key(self) -> &'static str {
        match self {
            Move::Rake => "wolf_rake",
            Move::RakeThrice => "wolf_rake_thrice",
            Move::Hammer => "wolf_hammer",
            Move::Lunge => "wolf_lunge",
            Move::Bound => "wolf_bound",
            Move::Howl => "wolf_howl",
            Move::Galvanize => "wolf_galvanize",
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

    /// How many blows a rake strikes.
    fn blows(self) -> usize {
        match self {
            Move::RakeThrice => 3,
            _ => 2,
        }
    }
}

record! {
    /// The conductor's memory, on the monster.
    pub struct Conductor = SchemaKey::new(crate::PROVIDER, "wolfenstein.conductor", 1);
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
    /// Where the strike started, and how many of its blows have struck.
    12 step_from: f32,
    13 blows_struck: u32,
    /// The strike's one-shot onset has been spent.
    14 fired: bool,
    /// Where the lunge goes.
    15 lunge_to: f32,
    /// The bound: the wall it goes to, and the floor it comes back to.
    16 wall_x: f32,
    17 bound_back_x: f32,
    18 bound_from_y: f32,
    /// After-states, each with seconds since it began: the skid, the fists
    /// stuck in the floor, winded after the bound, flat after the galvanize.
    19 skid: Option<f32>,
    20 stuck: Option<f32>,
    21 winded: Option<f32>,
    22 flat: Option<f32>,
    /// The shocks rolling the floor: where each is (none when it has hit a
    /// wall), and its generation.
    23 shock_left: Option<f32>,
    24 shock_right: Option<f32>,
    25 shock_generation: u32,
    /// The galvanize's three marks, and its generation.
    26 mark0: f32,
    27 mark1: f32,
    28 mark2: f32,
    29 bolt_generation: u32,
    /// Seconds until his body can hurt again.
    30 trunk_rearm: f32,
    31 stride: f32,
    /// His first move has started: the coils have woken him.
    32 woke: bool,
    /// He has been through a transition: the leash is broken.
    33 unleashed: bool,
    /// Shaking in the lightning between two phases: seconds into it.
    34 rearing: Option<f32>,
    /// How many times a howl has called the pack.
    35 pack_calls: u32,
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
    /// The move being performed and whether it is striking, for tests and
    /// inspectors.
    pub fn performing(&self) -> Option<(Move, bool)> {
        self.part().map(|p| (p.mv, p.striking))
    }

    /// Flat after the galvanize, his charge spent, for tests and inspectors.
    pub fn flat(&self) -> bool {
        self.flat.is_some()
    }

    /// Winded after the bound, for tests and inspectors.
    pub fn winded(&self) -> bool {
        self.winded.is_some()
    }

    /// His fists in the floor after the hammer, for tests and inspectors.
    pub fn stuck(&self) -> bool {
        self.stuck.is_some()
    }

    /// A shock is rolling the floor, for tests and inspectors.
    pub fn shock_rolling(&self) -> bool {
        self.shock_left.is_some() || self.shock_right.is_some()
    }

    /// The leash is broken (he has been through a transition), for tests and
    /// inspectors.
    pub fn unleashed(&self) -> bool {
        self.unleashed
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

    fn marks(&self) -> [f32; 3] {
        [self.mark0, self.mark1, self.mark2]
    }

    /// He stands still in a window or a story beat: no walk, no move of his own.
    fn holding_still(&self) -> bool {
        self.skid.is_some() || self.stuck.is_some() || self.winded.is_some() || self.flat.is_some() || self.rearing.is_some()
    }
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "wolfenstein"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![Conductor::schema()],
        entries: vec![EntryDescriptor {
            key: "conduct".into(),
            phase: BOSS_CONDUCT,
            trigger: TriggerBinding { port: BossConductPort::KEY, selector: WOLFENSTEIN_ID.into() },
            reads: Vec::new(),
            writes: vec![Conductor::KEY],
            requests: vec![
                ConductedPosePort::KEY,
                DrawnRowPort::KEY,
                BurstPort::KEY,
                CameraShakePort::KEY,
                BossSummonPort::KEY,
                RidingHitboxPort::KEY,
                HeldDamageBoxPort::KEY,
                BodySoundPort::KEY,
            ],
            after: Vec::new(),
            limits: Limits { max_requests: 48 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(conduct),
        }],
    }
}

fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t) * (1.0 - t)
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

/// A lightning strike from the ceiling to the floor at `x`: sparks down its
/// length, so it reads from across the room.
fn lightning(inv: &mut Invocation<'_>, hall: Hall, x: f32, ceiling: f32) -> Result<(), Fault> {
    let steps = 6;
    for i in 0..=steps {
        let y = ceiling + (hall.floor - ceiling) * i as f32 / steps as f32;
        burst(inv, Vec2::new(x, y), 5, 160.0, ARC, "spark")?;
    }
    burst(inv, Vec2::new(x, hall.floor - 4.0), 16, 300.0, SPARK, "spark")
}

fn conduct(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let wolf: BossConduct = inv.trigger::<BossConductPort>()?.clone();
    let dt = inv.dt();
    let mut c = Conductor::load(inv)?;
    if !c.hall_known {
        let Some(h) = wolf.hall else {
            return Ok(());
        };
        c.hall_known = true;
        c.hall_floor = h.floor;
        c.hall_left = h.left;
        c.hall_right = h.right;
    }
    let hall = Hall { floor: c.hall_floor, left: c.hall_left, right: c.hall_right };
    let ceiling = hall.floor - CEILING;
    let mut side = wolf.side;
    c.clock += dt;
    c.ticks = c.ticks.wrapping_add(1);
    let at = Vec2::from(wolf.position);
    let stand_y = hall.floor - feet_below();
    let target = wolf.target.map_or(Vec2::new((hall.left + hall.right) * 0.5, hall.floor - 24.0), Vec2::from);
    // Where he can stand: his body inside the room.
    let clamp_x = |x: f32| x.clamp(hall.left + body_half() + WALL_GAP, hall.right - body_half() - WALL_GAP);
    let face = |x: f32| if target.x < x { -1.0 } else { 1.0 };
    let neck = |pos: Vec2, side: f32| pos + Vec2::new(bolts().x * side, bolts().y);

    // ── Dead: he settles on the floor and is drawn by the engine ──
    if !wolf.alive {
        inv.submit::<DrawnRowPort>(DrawnRow::body_as_engine())?;
        let pos = Vec2::new(at.x, stand_y);
        inv.submit::<ConductedPosePort>(ConductedPose {
            pose: Some(Pose { position: pos.into(), velocity: ((pos - at) / dt).into() }),
            side: Some(side),
        })?;
        return c.store(inv);
    }

    // ── Which part of which move is live ──
    let live = match (&wolf.active, &wolf.telegraph) {
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
                c.woke = true;
                if !striking {
                    // A tell ends every window: he gathers himself and turns
                    // to you.
                    c.skid = None;
                    c.stuck = None;
                    c.winded = None;
                    if c.flat.take().is_some() {
                        // The coils jolt him back up.
                        play(inv, SFX_ARC, neck(at, side))?;
                        burst(inv, neck(at, side), 14, 220.0, ARC, "spark")?;
                    }
                    side = face(at.x);
                    if mv == Move::Galvanize {
                        // One mark under you, one each side of it.
                        let x = target.x.clamp(hall.left + 40.0, hall.right - 40.0);
                        c.mark0 = x;
                        c.mark1 = (x - MARK_SPREAD).max(hall.left + 40.0);
                        c.mark2 = (x + MARK_SPREAD).min(hall.right - 40.0);
                    }
                } else {
                    c.fired = false;
                    c.step_from = at.x;
                    c.blows_struck = 0;
                    match mv {
                        Move::Lunge => {
                            let reach = (target.x - at.x).abs().clamp(160.0, LUNGE_REACH) + 60.0;
                            c.lunge_to = clamp_x(at.x + side * reach.min(LUNGE_REACH));
                        }
                        Move::Bound => {
                            // The wall behind you, and back the other way.
                            c.wall_x = if side > 0.0 {
                                hall.right - body_half() - WALL_GAP
                            } else {
                                hall.left + body_half() + WALL_GAP
                            };
                            c.bound_back_x = clamp_x(c.wall_x - side * BOUND_BACK);
                            c.bound_from_y = at.y;
                        }
                        _ => {}
                    }
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
                Move::Lunge => c.skid = Some(0.0),
                Move::Hammer => c.stuck = Some(0.0),
                Move::Bound => {
                    c.winded = Some(0.0);
                    let feet = Vec2::new(at.x, hall.floor - 4.0);
                    play(inv, SFX_LAND, feet)?;
                    shake(inv, SHAKE_LANDING)?;
                    burst(inv, feet, 24, 280.0, DUST, "dust")?;
                }
                Move::Galvanize => {
                    c.flat = Some(0.0);
                    play(inv, SFX_SPUTTER, neck(at, side))?;
                }
                _ => {}
            }
        }
    }
    c.set_part(part);
    c.last_remaining = live.map_or(0.0, |(_, _, remaining)| remaining);

    // ── Between phases: lightning finds him, he shakes, he howls ──
    if wolf.between_phases {
        if c.rearing.is_none() {
            side = face(at.x);
            c.skid = None;
            c.stuck = None;
            c.winded = None;
            c.flat = None;
            c.unleashed = true;
            play(inv, SFX_BOLT, neck(at, side))?;
            lightning(inv, hall, at.x, ceiling)?;
            shake(inv, SHAKE_BOLT)?;
        }
        let t = c.rearing.map_or(0.0, |t| t + dt);
        if t >= REAR_SHAKE_S && t - dt < REAR_SHAKE_S {
            play(inv, SFX_HOWL, neck(at, side))?;
            burst(inv, neck(at, side), 24, 300.0, RING, "spark")?;
            shake(inv, SHAKE_HOWL)?;
        }
        c.rearing = Some(t);
    } else {
        c.rearing = None;
    }

    // ── Where he goes ──
    let mut walking = false;
    let pos = match part {
        Some(Part { mv: Move::Rake | Move::RakeThrice, striking: true, t, .. }) => {
            let blow = BLOW_AT.iter().take(part.map_or(2, |p| p.mv.blows())).filter(|at| t >= **at).count().max(1);
            let into = t - BLOW_AT[blow - 1];
            let x = c.step_from + side * RAKE_STEP * ((blow - 1) as f32 + ease(into / BLOW_S));
            Vec2::new(clamp_x(x), stand_y)
        }
        Some(Part { mv: Move::Lunge, striking: true, t, dur }) => {
            Vec2::new(c.step_from + (c.lunge_to - c.step_from) * ease_out(t / dur), stand_y)
        }
        Some(Part { mv: Move::Bound, striking: true, t, dur }) => {
            let u = t / dur;
            if u < BOUND_ON_WALL {
                // Up into the wall behind you: an arc through where you are.
                let k = u / BOUND_ON_WALL;
                let x = c.step_from + (c.wall_x - c.step_from) * ease_out(k);
                let y = c.bound_from_y + (stand_y - BOUND_HIGH - c.bound_from_y) * (k * (2.0 - k));
                Vec2::new(x, y)
            } else if u < BOUND_OFF_WALL {
                // On the wall, turned to come back.
                side = if c.wall_x > c.bound_back_x { -1.0 } else { 1.0 };
                Vec2::new(c.wall_x, stand_y - BOUND_HIGH)
            } else {
                side = if c.wall_x > c.bound_back_x { -1.0 } else { 1.0 };
                let k = (u - BOUND_OFF_WALL) / (1.0 - BOUND_OFF_WALL);
                let x = c.wall_x + (c.bound_back_x - c.wall_x) * ease_out(k);
                Vec2::new(x, stand_y - BOUND_HIGH * (1.0 - k * k))
            }
        }
        // Telling and the other strikes, he holds his ground.
        Some(_) => Vec2::new(at.x, stand_y),
        None if c.skid.is_some() => {
            let t = c.skid.unwrap_or(0.0);
            let x = clamp_x(c.lunge_to + side * SKID * ease_out(t / SKID_S));
            Vec2::new(x, stand_y)
        }
        None if c.holding_still() || !c.woke => Vec2::new(at.x, stand_y),
        None => {
            // Between moves he turns to you and lurches into raking range.
            side = face(at.x);
            let goal = clamp_x(target.x - side * STANDOFF);
            let to = goal - at.x;
            let speed = if c.unleashed { WALK_SPEED_UNLEASHED } else { WALK_SPEED };
            let stride = speed * dt;
            if to.abs() > stride {
                walking = true;
                Vec2::new(at.x + to.signum() * stride, stand_y)
            } else {
                Vec2::new(goal, stand_y)
            }
        }
    };
    if walking {
        c.stride += dt;
        // A heavy footfall every half-stride (four frames of the walk).
        if c.stride >= 0.5 {
            c.stride -= 0.5;
            let foot = Vec2::new(pos.x, hall.floor - 4.0);
            play(inv, SFX_STEP, foot)?;
            burst(inv, foot, 4, 70.0, DUST, "dust")?;
        }
    }

    // ── The strike's own onsets ──
    if let Some(part @ Part { striking: true, .. }) = part {
        let onset = !c.fired;
        c.fired = true;
        match part.mv {
            Move::Rake | Move::RakeThrice => {
                let blows = part.mv.blows();
                while (c.blows_struck as usize) < blows && part.t >= BLOW_AT[c.blows_struck as usize] {
                    let last_high = part.mv == Move::RakeThrice && c.blows_struck as usize == blows - 1;
                    let (volume, damage, knockback) = if last_high {
                        (overhead_box(), OVERHEAD_DAMAGE, OVERHEAD_KNOCKBACK)
                    } else {
                        (rake_box(), RAKE_DAMAGE, RAKE_KNOCKBACK)
                    };
                    riding(inv, volume, side, damage, knockback, BLOW_S * 0.6, "wolf_rake")?;
                    play(inv, SFX_RAKE, pos + Vec2::new(side * body_half() * 2.0, 0.0))?;
                    c.blows_struck += 1;
                }
            }
            Move::Hammer if onset => {
                riding(inv, fists_box(), side, HAMMER_DAMAGE, HAMMER_KNOCKBACK, 0.15, "wolf_hammer")?;
                let fists = Vec2::new(pos.x + side * fists_x(), hall.floor - 4.0);
                c.shock_generation = c.shock_generation.wrapping_add(1);
                c.shock_left = Some(fists.x);
                c.shock_right = Some(fists.x);
                play(inv, SFX_HAMMER, fists)?;
                play(inv, SFX_SHOCK, fists)?;
                shake(inv, SHAKE_HAMMER)?;
                burst(inv, fists, 30, 340.0, DUST, "dust")?;
                burst(inv, fists, 10, 260.0, DUST, "shard")?;
            }
            Move::Lunge | Move::Bound if onset => {
                play(inv, SFX_LUNGE, pos)?;
            }
            Move::Howl if onset => {
                riding(inv, howl_box(), side, HOWL_DAMAGE, HOWL_KNOCKBACK, 0.3, "wolf_howl")?;
                play(inv, SFX_HOWL, neck(pos, side))?;
                shake(inv, SHAKE_HOWL)?;
                burst(inv, neck(pos, side), 28, 340.0, RING, "spark")?;
                if c.unleashed && wolf.minions == 0 {
                    c.pack_calls += 1;
                    for i in 0..PACK {
                        // In from both ends of the room.
                        let x = if i % 2 == 0 { hall.left + 60.0 } else { hall.right - 60.0 };
                        let at = Vec2::new(x, hall.floor - PACK_HALF[1]);
                        inv.submit::<BossSummonPort>(BossSummon {
                            label: "wolfenstein_pack".into(),
                            serial: vec![c.pack_calls, i],
                            position: at.into(),
                            half_size: PACK_HALF,
                            character_id: PACK_CHARACTER.into(),
                            health: Some(PACK_HEALTH),
                            keeps_contact_damage: true,
                            // His own pack: his volumes pass through them.
                            on_boss_side: true,
                        })?;
                        burst(inv, at, 16, 220.0, DUST, "dust")?;
                    }
                }
            }
            Move::Galvanize if onset => {
                c.bolt_generation = c.bolt_generation.wrapping_add(1);
                play(inv, SFX_BOLT, neck(pos, side))?;
                shake(inv, SHAKE_BOLT)?;
                for x in c.marks() {
                    lightning(inv, hall, x, ceiling)?;
                }
            }
            _ => {}
        }
        // The bites ride the body while it flies.
        let flying = match part.mv {
            Move::Lunge => part.t < part.dur * 0.85,
            Move::Bound => {
                let u = part.t / part.dur;
                u < BOUND_ON_WALL || u >= BOUND_OFF_WALL
            }
            _ => false,
        };
        if flying && c.ticks % 4 == 0 {
            riding(inv, lunge_box(), side, LUNGE_DAMAGE, LUNGE_KNOCKBACK, 0.1, "wolf_lunge")?;
            burst(inv, Vec2::new(pos.x, hall.floor - 6.0), 3, 120.0, DUST, "dust")?;
        }
        if part.mv == Move::Lunge && part.t >= part.dur * 0.2 && part.t - dt < part.dur * 0.2 {
            play(inv, SFX_BITE, pos + Vec2::new(side * body_half() * 3.0, 0.0))?;
        }
    }
    // ── The galvanize: the sparks on its marks, then the lightning ──
    if let Some(Part { mv: Move::Galvanize, striking, t, .. }) = part {
        if !striking && c.ticks % 3 == 0 {
            for (i, x) in c.marks().into_iter().enumerate() {
                let across = ((c.ticks / 3 + i as u32 * 5) % 5) as f32 / 4.0 * 2.0 - 1.0;
                burst(inv, Vec2::new(x + across * BOLT_HALF.x, hall.floor - 3.0), 3, 90.0, ARC, "spark")?;
            }
            // And on his bolts, as the charge builds.
            burst(inv, neck(pos, side), 2, 120.0, ARC, "spark")?;
        }
        if striking && t < BOLT_S {
            for (slot, x) in c.marks().into_iter().enumerate() {
                inv.submit::<HeldDamageBoxPort>(HeldDamageBox {
                    slot: 2 + slot as u32,
                    generation: c.bolt_generation,
                    center: [x, hall.floor - BOLT_HALF.y],
                    half_extent: BOLT_HALF.into(),
                    damage: BOLT_DAMAGE,
                    knockback: BOLT_KNOCKBACK,
                    lifetime_s: BOLT_S,
                })?;
            }
        }
    }

    // ── The shocks roll the floor ──
    for (slot, wave, dir) in [(0u32, &mut c.shock_left, -1.0f32), (1, &mut c.shock_right, 1.0)] {
        let Some(x) = wave.as_mut() else {
            continue;
        };
        *x += dir * SHOCK_SPEED * dt;
        if *x < hall.left + SHOCK_HALF.x || *x > hall.right - SHOCK_HALF.x {
            *wave = None;
            continue;
        }
        let x = *x;
        if c.ticks % 2 == 0 {
            burst(inv, Vec2::new(x, hall.floor - 8.0), 6, 200.0, DUST, "dust")?;
        }
        if c.ticks % 6 == 0 {
            burst(inv, Vec2::new(x, hall.floor - 4.0), 3, 240.0, DUST, "shard")?;
        }
        inv.submit::<HeldDamageBoxPort>(HeldDamageBox {
            slot,
            generation: c.shock_generation,
            center: [x, hall.floor - SHOCK_HALF.y],
            half_extent: SHOCK_HALF.into(),
            damage: SHOCK_DAMAGE,
            knockback: SHOCK_KNOCKBACK,
            lifetime_s: SHOCK_LIFETIME_S,
        })?;
    }

    // ── The windows run out ──
    for (state, length) in [(&mut c.skid, SKID_S), (&mut c.stuck, STUCK_S + PULL_S), (&mut c.winded, WINDED_S)] {
        if let Some(t) = state.as_mut() {
            *t += dt;
            if *t >= length {
                *state = None;
            }
        }
    }
    if let Some(t) = c.flat.as_mut() {
        // Flat until his next tell: the bolts sputter.
        *t += dt;
        let sputter = (*t / 0.45).floor() != ((*t - dt) / 0.45).floor();
        if sputter {
            burst(inv, neck(pos, side), 4, 90.0, SPARK, "spark")?;
        }
    }
    if !c.woke && c.ticks % 5 == 0 {
        // Dead where he stands: the charge creeps into his bolts.
        burst(inv, neck(pos, side), 2, 80.0, ARC, "spark")?;
    }

    // ── His body hurts to touch while a move is live ──
    c.trunk_rearm -= dt;
    if part.is_some() && c.trunk_rearm <= 0.0 {
        c.trunk_rearm = TRUNK_REARM;
        riding(inv, trunk(), side, TRUNK_DAMAGE, TRUNK_KNOCKBACK, TRUNK_REARM, "wolf_trunk")?;
    }

    // ── Where he is, and what he is drawn as ──
    let flying = matches!(part, Some(Part { mv: Move::Lunge | Move::Bound, striking: true, .. }));
    let conducts = !wolf.driven || flying || c.holding_still();
    if !conducts {
        side = if wolf.facing < 0.0 { -1.0 } else { 1.0 };
    }
    inv.submit::<ConductedPosePort>(ConductedPose {
        pose: conducts.then(|| Pose { position: pos.into(), velocity: ((pos - at) / dt).into() }),
        side: Some(side),
    })?;
    let row = drawn_row(&c, part, walking);
    inv.submit::<DrawnRowPort>(DrawnRow::body(row.0, row.1, row.2))?;
    c.store(inv)
}

/// He shakes in the lightning this long between two phases, then howls.
const REAR_SHAKE_S: f32 = 0.9;

/// The row he is drawn with: the live move's, its tell frames filling the
/// telegraph and the rest the strike; his windows; walking or idling.
///
/// Every row is his sheet's (`wolfenstein.py`): `(name, frame, seconds per
/// frame)`, and the time is `frame * seconds per frame` into the row.
fn drawn_row(c: &Conductor, part: Option<Part>, walking: bool) -> (&'static str, f32, bool) {
    let at = |name, frame: f32, fd: f32| (name, frame * fd, false);
    // Frames `from..to` of a row across `u` (0-1) of a part.
    let span = |name, from: f32, to: f32, fd: f32, u: f32| at(name, from + u.clamp(0.0, 0.999) * (to - from), fd);
    if let Some(t) = c.rearing {
        if t < REAR_SHAKE_S {
            // He shakes in the lightning.
            return at("hit", 1.0 + ((t / 0.06) as u32 % 3) as f32, 0.095);
        }
        return span("taunt", 4.0, 12.0, 0.1, (t - REAR_SHAKE_S) / 0.8);
    }
    if !c.woke {
        // Dead where he stands, until the coils wake him: slumped, then the
        // jolt, then the howl.
        let t = c.clock;
        return if t < INTRO_DEAD_S {
            at("death", 1.0, 0.12)
        } else if t < INTRO_DEAD_S + 0.4 {
            span("hit", 0.0, 5.0, 0.095, (t - INTRO_DEAD_S) / 0.4)
        } else if t < INTRO_DEAD_S + 0.4 + INTRO_HOWL_S {
            span("taunt", 0.0, 12.0, 0.1, (t - INTRO_DEAD_S - 0.4) / INTRO_HOWL_S)
        } else {
            ("idle", t, true)
        };
    }
    let u = |p: Part| p.t / p.dur;
    match part {
        Some(p @ Part { mv: Move::Rake | Move::RakeThrice, striking: false, .. }) => span("slash", 0.0, 3.0, 0.08, u(p)),
        Some(Part { mv: mv @ (Move::Rake | Move::RakeThrice), striking: true, t, .. }) => {
            let blow = BLOW_AT.iter().take(mv.blows()).filter(|at| t >= **at).count().max(1);
            let into = (t - BLOW_AT[blow - 1]) / BLOW_S;
            // The blows swing alternately across and up; the last of three is
            // high.
            if blow % 2 == 1 && !(mv == Move::RakeThrice && blow == 3) {
                span("slash", 3.0, 9.0, 0.08, into)
            } else {
                span("attack_up", 3.0, 8.0, 0.08, into)
            }
        }
        Some(p @ Part { mv: Move::Hammer, striking: false, .. }) => span("attack_down", 0.0, 5.0, 0.085, u(p)),
        Some(p @ Part { mv: Move::Hammer, .. }) => span("attack_down", 5.0, 7.5, 0.085, u(p) * 2.0),
        Some(p @ Part { mv: Move::Lunge | Move::Bound, striking: false, .. }) => {
            // Low, gathered: the landing crouch held.
            span("land_hard", 1.0, 3.0, 0.09, u(p))
        }
        Some(p @ Part { mv: Move::Lunge, .. }) => span("attack_side", 3.0, 6.0, 0.08, u(p)),
        Some(p @ Part { mv: Move::Bound, .. }) => {
            let u = u(p);
            if u < BOUND_ON_WALL {
                span("jump", 1.0, 4.0, 0.1, u / BOUND_ON_WALL)
            } else if u < BOUND_OFF_WALL {
                at("fall", 0.0, 0.1)
            } else {
                at("attack_side", 4.0, 0.08)
            }
        }
        Some(p @ Part { mv: Move::Howl, striking: false, .. }) => span("taunt", 0.0, 4.0, 0.1, u(p)),
        Some(p @ Part { mv: Move::Howl, .. }) => span("taunt", 4.0, 12.0, 0.1, u(p)),
        // Hands up to his bolts, shaking with the charge.
        Some(Part { mv: Move::Galvanize, t, .. }) => at("attack_up", 3.0 + ((t / 0.05) as u32 % 2) as f32, 0.08),
        None => {
            if let Some(t) = c.skid {
                span("land_hard", 3.0, 6.0, 0.09, t / SKID_S)
            } else if let Some(t) = c.stuck {
                if t < STUCK_S {
                    at("attack_down", 7.0, 0.085)
                } else {
                    span("attack_down", 8.0, 10.0, 0.085, (t - STUCK_S) / PULL_S)
                }
            } else if let Some(t) = c.winded {
                span("land_hard", 1.0, 6.0, 0.09, t / WINDED_S)
            } else if c.flat.is_some() {
                // His charge spent: slumped, head down.
                at("death", 1.0, 0.12)
            } else if walking {
                ("walk", c.clock, true)
            } else {
                ("idle", c.clock, true)
            }
        }
    }
}

/// Before his first move, he is dead this long, then the coils jolt him
/// (0.4 s) and he howls. The boss wakes when his room does, so the three fill
/// his encounter's `intro_seconds` (2.2).
const INTRO_DEAD_S: f32 = 1.0;
const INTRO_HOWL_S: f32 = 0.8;

#[cfg(test)]
mod tests {
    use super::*;

    /// A standing robot: 48 units tall, its soles on the floor.
    fn robot_at(x: f32) -> (Vec2, Vec2) {
        (Vec2::new(x, feet_below() - 24.0), Vec2::new(14.0, 24.0))
    }

    fn overlaps((a, ah): (Vec2, Vec2), (b, bh): (Vec2, Vec2)) -> bool {
        ((a - b).abs() - (ah + bh)).cmple(Vec2::ZERO).all()
    }

    /// He stands on the floor: his soles, not his frame, meet it.
    #[test]
    fn his_feet_are_where_his_art_draws_them() {
        assert!((feet_below() - 50.0).abs() < 0.5, "his 100-unit body box has his soles at its bottom");
    }

    /// Each blow of the rake and the hammer reaches a standing robot where he
    /// stops to rake.
    #[test]
    fn his_blows_reach_a_robot_at_his_standoff() {
        let robot = robot_at(STANDOFF);
        assert!(overlaps(rake_box(), robot), "the rake");
        assert!(overlaps(overhead_box(), robot), "the high blow");
        assert!(overlaps(fists_box(), robot), "the hammer");
        assert!(overlaps(lunge_box(), robot), "the lunge");
    }

    /// The howl is the answer to a robot that stands in him or behind him.
    #[test]
    fn the_howl_reaches_both_sides_of_him() {
        assert!(overlaps(howl_box(), robot_at(0.0)));
        assert!(overlaps(howl_box(), robot_at(-STANDOFF)));
    }
}
