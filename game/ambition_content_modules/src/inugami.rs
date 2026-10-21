//! The Inugami's conductor: Wolfenstein's head, risen as a spirit, and its
//! fight.
//!
//! Its scripted pattern decides WHICH move and WHEN (its `Special` keys:
//! tell, strike, rest). This decides everything that is the move's own: where
//! it flies, the volume each move swings, the fire it throws, the row it is
//! drawn with (`ambition.boss.conducted_pose`).
//!
//! ⭐ THE FIGHT'S SHAPE (`docs/planning/game/bosses.md`, "The Inugami").
//! Jon (2026-10-20): "the head resurrects as an inugami and you fight the
//! floating spectral head as an additional boss". The regime sewed a wolf's
//! head onto a soldier and gave it orders. When the soldier dies, the wolf is
//! free: the seam round his neck pops, the head tears off his body, and the
//! red ghost fire of an inugami catches in it. It is its own boss (its own
//! placement, health, name card and save record) that sleeps until his
//! placement is cleared (`BossConfig::wakes_after`), and it rises from where
//! his body lies (`BossConduct::predecessor`).
//!
//! - The bite: it draws back, its jaws wide, then darts straight at you and
//!   snaps twice. It tumbles past and shakes its head: the window.
//! - The dive: it flies up over you, turns nose down, and drops; its teeth
//!   sink into the floor, and it wrenches at them: the long window.
//! - The onibi: fire balls kindle round it, and it flings them at you.
//! - The howl: it throws its head back, and a ring pushes you off it. From
//!   the transition on the ring throws fire balls all round.
//! - The vanish: it fades out, and comes back behind you (a bite follows).
//! - The gutter: its fire is spent at the end of each cycle. It sinks to the
//!   floor and lies there, dim, until its next tell kindles it again.
//!
//! When it dies the fire goes out of it, the spirit leaves as a wisp, and
//! his own head drops to the floor.
//!
//! Every volume is measured on its sheet (the renderer's
//! `wolfenstein_inugami`), in the head's own pixels from the middle of its
//! body box (the skull, the muzzle, the ruff and the jar), and carried into
//! the world by [`Head`].

use ambition_boss_special_port::{
    BossConduct, BossConductPort, BossGuard, BossGuardPort, BossScale, BossScalePort, BossStage, BossStagePort,
    ConductedPose, ConductedPosePort, DrawnRow, DrawnRowPort, Pose, Predecessor,
};
use ambition_combat_port::{
    BodySound, BodySoundPort, Burst, BurstPort, CameraShake, CameraShakePort, RidingHitbox, RidingHitboxPort,
    RidingKnockback,
};
use ambition_extension_sdk::{
    phases::BOSS_CONDUCT, record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};
use ambition_projectile_spec::{ProjectileSpawn, ProjectileSpawnPort};
use bevy_math::Vec2;

use crate::wolfenstein;

/// The behaviour id of the boss it conducts.
pub const INUGAMI_ID: &str = "wolfenstein_inugami";

/// Its body box on its sheet, in head pixels: 55 tall, its middle this far
/// from the middle of the head.
const BODY_PX: f32 = 55.0;
const BODY_MID: Vec2 = Vec2::new(3.0, -10.5);

/// Its body as big as it is now: world units per head pixel, from its body
/// box ([`BossConduct::size`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Head {
    k: f32,
}

impl Head {
    pub fn of_height(height: f32) -> Self {
        Self { k: height.max(1.0) / BODY_PX }
    }

    /// A length in head pixels, in world units.
    fn len(self, px: f32) -> f32 {
        px * self.k
    }

    /// A head point (pixels, art facing right) as an offset from its
    /// position, unflipped.
    fn art(self, x: f32, y: f32) -> Vec2 {
        (Vec2::new(x, y) - BODY_MID) * self.k
    }

    fn art_box(self, x0: f32, y0: f32, x1: f32, y1: f32) -> (Vec2, Vec2) {
        let (a, b) = (self.art(x0, y0), self.art(x1, y1));
        ((a + b) * 0.5, (b - a) * 0.5)
    }

    /// Its jaws, level (the bite).
    pub fn jaws(self) -> (Vec2, Vec2) {
        self.art_box(6.0, -8.0, 28.0, 14.0)
    }

    /// Its jaws, nose down (the dive): the head turns about its own middle.
    pub fn jaws_down(self) -> (Vec2, Vec2) {
        self.art_box(-17.0 + BODY_MID.x, 16.5 + BODY_MID.y, 5.0 + BODY_MID.x, 38.5 + BODY_MID.y)
    }

    /// How far under its position its chin is when it lies on the floor
    /// (its `guttered` and dead rows).
    pub fn lying_low(self) -> f32 {
        self.art(0.0, 22.0).y
    }

    /// Where the ring of its fire balls is, about its middle.
    fn onibi(self, degrees: f32) -> Vec2 {
        let a = degrees.to_radians();
        self.art(-1.0, -6.0) + Vec2::new(a.cos(), a.sin() * 0.8) * self.len(37.0)
    }
}

/// Where a body of the predecessor's dead head is, and how big its head is
/// drawn there: the Inugami rises from it at that size.
fn dead_head_of(p: &Predecessor) -> (Vec2, f32) {
    let body = wolfenstein::Body::of_height(p.size[1]);
    let off = body.dead_head();
    (Vec2::from(p.position) + Vec2::new(off.x * p.side, off.y), body.head_px())
}

/// Between moves it hangs this far to the side of you and over you (head
/// pixels: a giant head keeps a giant's distance).
const HOVER_DX: f32 = 70.0;
const HOVER_DY: f32 = 42.0;
const HOVER_SPEED: f32 = 150.0;
const HOVER_SPEED_FRENZIED: f32 = 210.0;
/// The top of the room it flies to, over its floor.
const CEILING: f32 = 740.0;
/// Its room's side walls.
const WALL: f32 = 16.0;

/// PHASE 1, THE CHASE (Jon, 2026-10-20): it chases you down the long gallery
/// from the throne room. It hangs this far behind you and over you (head
/// pixels), at this pace, and faster when you have got far ahead of it.
const CHASE_BEHIND: f32 = 55.0;
const CHASE_OVER: f32 = 34.0;
const CHASE_SPEED: f32 = 230.0;
const CHASE_CATCH_UP: f32 = 1.8;
const CHASE_FAR: f32 = 600.0;
/// In the chase its dive lands this far ahead of you (head pixels): in your
/// way, its teeth in the floor for you to turn on.
const DIVE_AHEAD: f32 = 40.0;
/// It dives from this high over you (world units), or the top of the room.
const DIVE_HIGH: f32 = 440.0;
/// THE FINAL HALL: when you have hurt it enough (its transition) the chase
/// is over. It vanishes and comes back in the hall at the end of the gallery
/// (the last `FINAL_W` of its room the way it chased you), and the fight is
/// finished there (phase 2 and the enrage).
const FINAL_W: f32 = 1600.0;
/// Its transition: the flare, then the vanish (`vanish`, 8 x 60 ms); it
/// comes back (`appear`) when phase 2 starts.
const FLARE_S: f32 = 0.5;
const VANISH_S: f32 = 0.48;
const APPEAR_S: f32 = 0.48;

const BITE_REACH: f32 = 560.0;
const BITE_DAMAGE: i32 = 2;
const BITE_KNOCKBACK: f32 = 1.4;
/// Past you: it tumbles and shakes its head (`bite_miss`, 12 x 70 ms).
const MISS_S: f32 = 0.84;
const MISS_DRIFT: f32 = 40.0;

const DIVE_DAMAGE: i32 = 2;
const DIVE_KNOCKBACK: f32 = 1.6;
/// It is over you for the part of the tell after this.
const DIVE_RISE_U: f32 = 0.6;
/// Its teeth in the floor (`buried`), then it wrenches them out
/// (`pull_free`, 6 x 70 ms).
const BURIED_S: f32 = 1.5;
const PULL_S: f32 = 0.42;
/// How deep its teeth go into the floor, head pixels.
const SINK_PX: f32 = 6.0;

const ONIBI_VISUAL: &str = "inugami_onibi";
const ONIBI_DAMAGE: i32 = 1;
const ONIBI_HALF: Vec2 = Vec2::new(14.0, 14.0);
const ONIBI_SPEED: f32 = 280.0;
const ONIBI_SPEED_FRENZIED: f32 = 340.0;
const ONIBI_LIFE: f32 = 4.5;
/// The fire balls round it at the end of the tell (its sheet's ring, turned
/// by the tell), and the two more it kindles from the transition on.
const ONIBI_AT: [f32; 5] = [-110.0, 10.0, 130.0, -50.0, 70.0];
/// From the transition on its howl throws this many all round.
const HOWL_ONIBI: u32 = 6;

const HOWL_DAMAGE: i32 = 1;
const HOWL_KNOCKBACK: f32 = 2.2;
/// The ring's radius, in its body heights.
const HOWL_RADIUS: f32 = 0.9;

/// It comes back this far behind you, and this far over you.
/// (Head pixels.)
const BEHIND: f32 = 80.0;
const OVER: f32 = 26.0;
/// It swells as the fight goes on: from the transition, and more at the
/// enrage (Jon, 2026-10-20: "the head should also get bigger in the fight"),
/// this much of its build each second.
const SWELL_FRENZIED: f32 = 1.15;
const SWELL_ENRAGED: f32 = 1.3;
const SWELL_RATE: f32 = 0.25;

/// It comes up out of the floor over this long when it kindles again
/// (`rekindle`, 6 x 60 ms).
const REKINDLE_S: f32 = 0.36;

const SHAKE_DIVE: f32 = 9.0;
const SHAKE_HOWL: f32 = 5.0;
const SHAKE_TEAR: f32 = 6.0;

/// Its fire, and dust.
const FIRE: [f32; 4] = [1.0, 0.36, 0.34, 1.0];
const FIRE_CORE: [f32; 4] = [1.0, 0.88, 0.84, 1.0];
/// Its onibi are blue: cold fire about the red head (Jon, 2026-10-20).
const ONIBI_FIRE: [f32; 4] = [0.45, 0.75, 1.0, 1.0];
const DUST: [f32; 4] = [0.74, 0.70, 0.66, 1.0];

const SFX_HOWL: &str = "boss.generic.intro_roar";
const SFX_BITE: &str = "boss.trex.chomp";
const SFX_DART: &str = "enemy.burning_flying_shark.flame_flyby";
const SFX_SLAM: &str = "boss.trex.stomp";
const SFX_FLING: &str = "projectile.fireball.shoot";
const SFX_VANISH: &str = "enemy.shadow_oni.vanish";
const SFX_GUTTER: &str = "ability.fire.power_down";
const SFX_CATCH: &str = "ability.fire.release";

/// Its moves: the `Special` keys of its pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Bite,
    Dive,
    Onibi,
    Howl,
    Vanish,
    Gutter,
}

impl Move {
    pub const ALL: [Move; 6] = [Move::Bite, Move::Dive, Move::Onibi, Move::Howl, Move::Vanish, Move::Gutter];

    pub fn key(self) -> &'static str {
        match self {
            Move::Bite => "inu_bite",
            Move::Dive => "inu_dive",
            Move::Onibi => "inu_onibi",
            Move::Howl => "inu_howl",
            Move::Vanish => "inu_vanish",
            Move::Gutter => "inu_gutter",
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
    /// The conductor's memory, on the head.
    pub struct Conductor = SchemaKey::new(crate::PROVIDER, "inugami.conductor", 1);
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
    9 last_remaining: f32,
    10 clock: f32,
    11 ticks: u32,
    /// The strike's one-shot onset has been spent.
    12 fired: bool,
    /// Where the move started and where it goes.
    13 from_x: f32,
    14 from_y: f32,
    15 to_x: f32,
    16 to_y: f32,
    /// After-states, each with seconds since it began.
    17 miss: Option<f32>,
    18 buried: Option<f32>,
    19 guttered: Option<f32>,
    20 rekindle: Option<f32>,
    /// Its rise is over (or its first move has started): it fights.
    21 woke: bool,
    /// It has been through the transition.
    22 frenzied: bool,
    /// Flaring between two phases: seconds into it.
    23 rearing: Option<f32>,
    /// Seconds since its encounter woke; `None` while it sleeps.
    24 intro: Option<f32>,
    /// Seconds since it died.
    25 dead_for: Option<f32>,
    /// It has been alive in this room (a head built dead was never seen to
    /// die: it lies where it fell).
    26 lived: bool,
    /// Where it rises from (the dead head), and how big it is there.
    27 rise_x: f32,
    28 rise_y: f32,
    29 rise_scale: f32,
    30 rise_known: bool,
    /// Its size now, as a part of its build.
    31 scale: f32,
    /// Its size when it died: it shrinks from it to his head's.
    32 dying_scale: f32,
    /// Coming back in the final hall: seconds into it.
    33 appear: Option<f32>,
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

    /// Its teeth in the floor after the dive, for tests and inspectors.
    pub fn buried(&self) -> bool {
        self.buried.is_some()
    }

    /// Its fire spent, on the floor, for tests and inspectors.
    pub fn guttered(&self) -> bool {
        self.guttered.is_some()
    }

    /// It has risen and fights, for tests and inspectors.
    pub fn woke(&self) -> bool {
        self.woke
    }

    fn part(&self) -> Option<Part> {
        Move::of_code(self.part_move).map(|mv| Part { mv, striking: self.part_striking, t: self.part_t, dur: self.part_dur })
    }

    fn set_part(&mut self, part: Option<Part>) {
        let p = part.unwrap_or(Part { mv: Move::Bite, striking: false, t: 0.0, dur: 0.0 });
        self.part_move = part.map_or(0, |p| p.mv.code());
        self.part_striking = p.striking;
        self.part_t = p.t;
        self.part_dur = p.dur;
    }
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "inugami"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![Conductor::schema()],
        entries: vec![EntryDescriptor {
            key: "conduct".into(),
            phase: BOSS_CONDUCT,
            trigger: TriggerBinding { port: BossConductPort::KEY, selector: INUGAMI_ID.into() },
            reads: Vec::new(),
            writes: vec![Conductor::KEY],
            requests: vec![
                ConductedPosePort::KEY,
                DrawnRowPort::KEY,
                BurstPort::KEY,
                CameraShakePort::KEY,
                RidingHitboxPort::KEY,
                ProjectileSpawnPort::KEY,
                BodySoundPort::KEY,
                BossGuardPort::KEY,
                BossScalePort::KEY,
                BossStagePort::KEY,
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

fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t) * (1.0 - t)
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

/// A volume riding its body, from an unflipped box turned to `side`.
#[allow(clippy::too_many_arguments)]
fn riding(
    inv: &mut Invocation<'_>,
    (centre, half): (Vec2, Vec2),
    side: f32,
    circle: Option<f32>,
    damage: i32,
    knockback: f32,
    life: f32,
    name: &str,
) -> Result<(), Fault> {
    inv.submit::<RidingHitboxPort>(RidingHitbox {
        offset: Vec2::new(centre.x * side, centre.y).into(),
        half_extent: half.into(),
        circle_radius: circle,
        damage,
        knockback: RidingKnockback::FeelScale(knockback),
        launch_dir: None,
        lifetime_s: life,
        name: name.into(),
    })
}

/// A ball of its fire, from `origin` along `dir`.
fn onibi(inv: &mut Invocation<'_>, origin: Vec2, dir: Vec2, speed: f32) -> Result<(), Fault> {
    inv.submit::<ProjectileSpawnPort>(ProjectileSpawn {
        origin,
        dir: dir.normalize_or(Vec2::X),
        speed,
        damage: ONIBI_DAMAGE,
        max_lifetime: ONIBI_LIFE,
        half_extent: ONIBI_HALF,
        gravity: 0.0,
        visual_id: ONIBI_VISUAL.to_string(),
        bounces: 0,
        bounce_on_world_contact: false,
        splash_half_extent: 0.0,
        boomerang_return_s: None,
    })
}

/// Build it at `scale` times its build (`ambition.boss.scale`), when that
/// changes.
fn set_scale(c: &mut Conductor, inv: &mut Invocation<'_>, scale: f32) -> Result<(), Fault> {
    if (c.scale - scale).abs() > 1e-4 {
        c.scale = scale;
        inv.submit::<BossScalePort>(BossScale { scale })?;
    }
    Ok(())
}

/// Put it at `pos`, facing `side`, drawn with `row`.
fn held_at(inv: &mut Invocation<'_>, at: Vec2, pos: Vec2, side: f32, row: (&'static str, f32, bool), dt: f32) -> Result<(), Fault> {
    inv.submit::<ConductedPosePort>(ConductedPose {
        pose: Some(Pose { position: pos.into(), velocity: ((pos - at) / dt.max(1e-4)).into() }),
        side: Some(side),
    })?;
    inv.submit::<DrawnRowPort>(DrawnRow::body(row.0, row.1, row.2))
}

/// Frames `from..to` of a row across `u` (0-1) of a part.
fn span(name: &'static str, from: f32, to: f32, fd: f32, u: f32) -> (&'static str, f32, bool) {
    (name, (from + u.clamp(0.0, 0.999) * (to - from)) * fd, false)
}

/// The rise runs on `Conductor::intro`, from the tick its encounter woke
/// (the tick after its predecessor's death settled). It is as long as its
/// encounter's `intro_seconds`.
const INTRO_S: f32 = 9.9;
/// The rise itself (`rise`, 22 x 80 ms): it rises this far from his body.
const RISE_S: f32 = 22.0 * 0.08;
const RISE_UP: f32 = 160.0;
/// Over the part of the rise from the fire catching, it grows from the size
/// his head was drawn at to its own.
const GROW_FROM: f32 = 0.22;
const GROW_TO: f32 = 0.5;
/// The room in its red light, while it fights.
const SPIRIT_LIT: f32 = 0.72;
/// Its death (`death`, 26 x 90 ms): the spirit leaves at this part of it,
/// and the head falls to the floor between these parts.
const DEATH_S: f32 = 26.0 * 0.09;
const DEATH_LEAVES_U: f32 = 0.44;
const DEATH_FALL: (f32, f32) = (0.44, 0.66);

/// When its rise starts, from its wake: when his head tears off his body,
/// or at once for a head that follows no body.
fn rise_at(inu: &BossConduct) -> f32 {
    if inu.predecessor.is_some() {
        wolfenstein::HEAD_OFF_AT
    } else {
        0.0
    }
}

/// How lit the room is `t` into its rise (1 before it), and its name card.
pub fn rise_stage_at(t: f32, rise_at: f32) -> BossStage {
    let u = (t - rise_at) / RISE_S;
    let lit = 1.0 - (1.0 - SPIRIT_LIT) * ease(u);
    let flash = if u >= GROW_FROM { 0.5 * (-(t - rise_at - GROW_FROM * RISE_S) / 0.25).exp() } else { 0.0 };
    let card_in = rise_at + 0.82 * RISE_S;
    let card = if t < card_in { 0.0 } else if t < INTRO_S - 0.5 { ((t - card_in) / 0.35).min(1.0) } else { ((INTRO_S - t) / 0.5).clamp(0.0, 1.0) };
    BossStage { lit, flash: flash.max(0.0), card }
}

fn conduct(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let inu: BossConduct = inv.trigger::<BossConductPort>()?.clone();
    let h = Head::of_height(inu.size[1]);
    let dt = inv.dt();
    let mut c = Conductor::load(inv)?;
    if !c.hall_known {
        let Some(hall) = inu.hall else {
            return Ok(());
        };
        c.hall_known = true;
        c.hall_floor = hall.floor;
        c.hall_left = hall.left;
        c.hall_right = hall.right;
        c.scale = 1.0;
    }
    // It flies the whole room: the floor is the one under it now (the
    // gallery has steps and rubble), and its bounds across are the room's
    // walls, or the final hall's once the chase is over.
    let floor = inu.hall.map_or(c.hall_floor, |hall| hall.floor);
    let room_w = inu.room.map_or(c.hall_right + WALL, |room| room[0]);
    let east = if c.rise_x < room_w * 0.5 { 1.0 } else { -1.0 };
    let (final_lo, final_hi) = if east > 0.0 { (room_w - FINAL_W, room_w - WALL) } else { (WALL, FINAL_W) };
    let in_final = c.frenzied || inu.enraged;
    let (lo, hi) = if in_final { (final_lo, final_hi) } else { (WALL, room_w - WALL) };
    let hall = Hall { floor, left: lo, right: hi };
    let built = Head::of_height(inu.size[1] / c.scale.max(0.1));
    let half = Vec2::new(inu.size[0], inu.size[1]) * 0.5;
    // Chasing you it flies the whole room, up the tower too; in the final
    // hall it keeps under the hall's sky.
    let top = if in_final { (hall.floor - CEILING).max(48.0) } else { 48.0 } + half.y;
    let ground = hall.floor - h.lying_low();
    c.clock += dt;
    c.ticks = c.ticks.wrapping_add(1);
    let at = Vec2::from(inu.position);
    let mut side = inu.side;
    let target = inu.target.map_or(Vec2::new((hall.left + hall.right) * 0.5, hall.floor - 24.0), Vec2::from);
    let clamp = |p: Vec2| {
        Vec2::new(
            p.x.clamp(hall.left + half.x + 8.0, hall.right - half.x - 8.0),
            p.y.clamp(top, hall.floor - half.y - 8.0),
        )
    };
    let face = |x: f32| if target.x < x { -1.0 } else { 1.0 };
    // Where it rises from: his head, at his head's size, followed until it
    // tears off; or where it was placed, at its own size.
    let before_rise = !c.woke && c.intro.is_none_or(|t| t < rise_at(&inu));
    if before_rise || !c.rise_known {
        c.rise_known = true;
        let (from, head_px) = inu.predecessor.as_ref().map_or((at, built.k), dead_head_of);
        c.rise_x = from.x;
        c.rise_y = from.y;
        c.rise_scale = (head_px / built.k).clamp(0.2, 1.0);
        if let Some(p) = &inu.predecessor {
            side = p.side;
        }
    }
    let rise_from = Vec2::new(c.rise_x, c.rise_y);

    // ── Dead: the fire goes out of it, the spirit leaves, his head drops ──
    if !inu.alive {
        let lived = c.lived;
        let t = c.dead_for.map_or(if lived { 0.0 } else { DEATH_S }, |t| t + dt);
        let was = t - dt;
        let crossed = |u: f32| t >= u * DEATH_S && was < u * DEATH_S;
        // When the spirit leaves it, it is his head again, at his head's size.
        let left = ((t / DEATH_S - DEATH_LEAVES_U) / 0.1).clamp(0.0, 1.0);
        if c.dead_for.is_none() {
            c.dying_scale = c.scale;
        }
        let giant = if lived { c.dying_scale } else { c.rise_scale };
        let shrunk = giant + (c.rise_scale - giant) * ease(left);
        set_scale(&mut c, inv, shrunk)?;
        if c.dead_for.is_none() && lived {
            play(inv, SFX_HOWL, at)?;
            burst(inv, at, 30, 300.0, FIRE, "spark")?;
        }
        if crossed(DEATH_LEAVES_U) {
            play(inv, SFX_GUTTER, at)?;
            burst(inv, at, 40, 340.0, FIRE_CORE, "spark")?;
            shake(inv, SHAKE_TEAR)?;
        }
        if crossed(DEATH_FALL.1) {
            let chin = Vec2::new(at.x, hall.floor - 4.0);
            play(inv, SFX_SLAM, chin)?;
            burst(inv, chin, 18, 220.0, DUST, "dust")?;
        }
        // A head built dead lies by his body; one that died falls where it was.
        if c.dead_for.is_none() {
            c.from_y = at.y;
        }
        let x = if lived { at.x } else { (rise_from.x + side * h.len(30.0)).clamp(hall.left + half.x, hall.right - half.x) };
        let from_y = if lived { c.from_y } else { ground };
        let u = ((t / DEATH_S - DEATH_FALL.0) / (DEATH_FALL.1 - DEATH_FALL.0)).clamp(0.0, 1.0);
        let y = if t / DEATH_S < DEATH_FALL.0 { if lived { at.y } else { ground } } else { from_y + (ground - from_y) * u * u };
        let lit = SPIRIT_LIT + (1.0 - SPIRIT_LIT) * ease((t - DEATH_LEAVES_U * DEATH_S) / 2.0);
        inv.submit::<BossStagePort>(BossStage { lit: if lived { lit } else { 1.0 }, flash: 0.0, card: 0.0 })?;
        c.dead_for = Some(t);
        held_at(inv, at, Vec2::new(x, y), side, ("death", t, false), dt)?;
        return c.store(inv);
    }
    c.lived = true;

    // ── Asleep in his body, then the rise ──
    if !c.woke {
        if inu.awake {
            c.intro = Some(c.intro.map_or(0.0, |t| t + dt));
        }
        if c.intro.is_some_and(|t| t >= INTRO_S) || inu.telegraph.is_some() || inu.active.is_some() {
            c.woke = true;
            set_scale(&mut c, inv, 1.0)?;
            inv.submit::<BossStagePort>(BossStage { lit: SPIRIT_LIT, flash: 0.0, card: 0.0 })?;
        } else {
            rise(inv, &mut c, &inu, rise_from, side, dt)?;
            return c.store(inv);
        }
    }

    // ── Which part of which move is live ──
    let live = match (inu.active.as_ref(), inu.telegraph.as_ref()) {
        (Some(active), _) if Move::from_key(&active.key).is_some() => Move::from_key(&active.key).map(|mv| (mv, true, active.remaining)),
        (_, Some(tell)) => Move::from_key(&tell.key).map(|mv| (mv, false, tell.remaining)),
        _ => None,
    };
    let prev = c.part();
    let fresh = match (prev, live) {
        (Some(prev), Some((mv, striking, remaining))) => prev.mv != mv || prev.striking != striking || remaining > c.last_remaining + 1e-3,
        (None, Some(_)) => true,
        _ => false,
    };
    let part = match live {
        Some((mv, striking, remaining)) => {
            if fresh {
                c.woke = true;
                c.fired = false;
                c.from_x = at.x;
                c.from_y = at.y;
                if !striking {
                    // A tell ends every window, and kindles a spent fire.
                    c.miss = None;
                    c.buried = None;
                    if c.guttered.take().is_some() {
                        c.rekindle = Some(0.0);
                        play(inv, SFX_CATCH, at)?;
                        burst(inv, at, 24, 280.0, FIRE, "spark")?;
                    }
                    side = face(at.x);
                    if mv == Move::Dive {
                        // Over you, near the top of the room; in the chase,
                        // over the way ahead of you.
                        let ahead = if in_final { 0.0 } else { east * h.len(DIVE_AHEAD) };
                        let over = clamp(Vec2::new(target.x + ahead, (target.y - DIVE_HIGH).max(top)));
                        c.to_x = over.x;
                        c.to_y = over.y;
                    }
                    // Its tell's own sound is its pattern's (`telegraph.cue`).
                    if mv == Move::Vanish {
                        inv.submit::<BossGuardPort>(BossGuard { guarded: true })?;
                    }
                } else {
                    match mv {
                        Move::Bite => {
                            // Straight at where you are now, a little past you.
                            let aim = (target - at).normalize_or(Vec2::X * side);
                            let reach = ((target - at).length() + 120.0).min(BITE_REACH);
                            let to = clamp(at + aim * reach);
                            c.to_x = to.x;
                            c.to_y = to.y;
                            side = if aim.x < 0.0 { -1.0 } else { 1.0 };
                        }
                        Move::Dive => {
                            c.to_x = at.x;
                            c.to_y = hall.floor - h.jaws_down().0.y - h.jaws_down().1.y + h.len(SINK_PX);
                        }
                        Move::Vanish => {
                            // Behind you: on the far side from where it was.
                            let away = if target.x < at.x { -1.0 } else { 1.0 };
                            let to = clamp(Vec2::new(target.x + away * h.len(BEHIND), target.y - h.len(OVER)));
                            c.to_x = to.x;
                            c.to_y = to.y;
                            side = -away;
                        }
                        Move::Gutter => {
                            c.to_x = at.x;
                            c.to_y = ground;
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
                Move::Bite => c.miss = Some(0.0),
                Move::Dive => c.buried = Some(0.0),
                Move::Gutter => c.guttered = Some(0.0),
                Move::Vanish => inv.submit::<BossGuardPort>(BossGuard { guarded: false })?,
                _ => {}
            }
        }
    }
    c.set_part(part);
    c.last_remaining = live.map_or(0.0, |(_, _, remaining)| remaining);

    // ── Between phases: its fire flares, and it howls ──
    if inu.between_phases {
        if c.rearing.is_none() {
            side = face(at.x);
            c.miss = None;
            c.buried = None;
            c.guttered = None;
            c.frenzied = true;
            play(inv, SFX_HOWL, at)?;
            burst(inv, at, 40, 360.0, FIRE, "spark")?;
            shake(inv, SHAKE_HOWL)?;
            inv.submit::<BossStagePort>(BossStage { lit: SPIRIT_LIT, flash: 0.4, card: 0.0 })?;
        }
        let t = c.rearing.map_or(0.0, |t| t + dt);
        if t >= FLARE_S && t - dt < FLARE_S {
            play(inv, SFX_VANISH, at)?;
        }
        c.rearing = Some(t);
    } else if c.rearing.take().is_some() {
        // It comes back in the final hall, over its middle.
        inv.submit::<BossStagePort>(BossStage { lit: SPIRIT_LIT, flash: 0.0, card: 0.0 })?;
        c.appear = Some(0.0);
        let back = Vec2::new((final_lo + final_hi) * 0.5, top + h.len(20.0));
        c.from_x = back.x;
        c.from_y = back.y;
        play(inv, SFX_VANISH, back)?;
        play(inv, SFX_HOWL, back)?;
        burst(inv, back, 40, 360.0, FIRE, "spark")?;
        shake(inv, SHAKE_HOWL)?;
    }

    for timer in [&mut c.miss, &mut c.buried, &mut c.guttered, &mut c.rekindle, &mut c.appear] {
        if let Some(t) = timer.as_mut() {
            *t += dt;
        }
    }
    if c.rekindle.is_some_and(|t| t >= REKINDLE_S) {
        c.rekindle = None;
    }
    if c.appear.is_some_and(|t| t >= APPEAR_S) {
        c.appear = None;
    }
    if c.miss.is_some_and(|t| t >= MISS_S) {
        c.miss = None;
    }
    if c.buried.is_some_and(|t| t >= BURIED_S + PULL_S) {
        c.buried = None;
    }

    // ── It swells as the fight goes on ──
    let swell = if inu.enraged { SWELL_ENRAGED } else if c.frenzied { SWELL_FRENZIED } else { 1.0 };
    if c.scale < swell {
        let grown = (c.scale + SWELL_RATE * dt).min(swell);
        set_scale(&mut c, inv, grown)?;
    }

    // ── Where it goes ──
    let from = Vec2::new(c.from_x, c.from_y);
    let to = Vec2::new(c.to_x, c.to_y);
    let bob = 8.0 * (c.clock * 2.2).sin();
    let hover = |side: f32| {
        let (goal, speed) = if in_final {
            // In the final hall: at your side, over you; it waits over the
            // middle of the hall while you are still in the gallery.
            let waiting = target.x < final_lo || target.x > final_hi;
            let goal = if waiting {
                Vec2::new((final_lo + final_hi) * 0.5, top + h.len(20.0) + bob)
            } else {
                Vec2::new(target.x - side * h.len(HOVER_DX), target.y - h.len(HOVER_DY) + bob)
            };
            (goal, if c.frenzied { HOVER_SPEED_FRENZIED } else { HOVER_SPEED })
        } else {
            // The chase: behind you, over you, and faster when far behind.
            let goal = Vec2::new(target.x - east * h.len(CHASE_BEHIND), target.y - h.len(CHASE_OVER) + bob);
            let far = (goal - at).length() > CHASE_FAR;
            (goal, CHASE_SPEED * if far { CHASE_CATCH_UP } else { 1.0 })
        };
        let goal = clamp(goal);
        let step = goal - at;
        let stride = speed * dt;
        if step.length() > stride { at + step.normalize() * stride } else { goal }
    };
    let pos = match part {
        Some(Part { mv: Move::Bite, striking: true, t, dur }) => from + (to - from) * ease_out(t / dur),
        Some(Part { mv: Move::Dive, striking: false, t, dur }) => from + (to - from) * ease(t / (dur * DIVE_RISE_U)),
        Some(Part { mv: Move::Dive, striking: true, t, dur }) => {
            let u = (t / dur).min(1.0);
            from + (to - from) * u * u
        }
        Some(Part { mv: Move::Vanish, striking: true, .. }) => to,
        Some(Part { mv: Move::Gutter, striking: false, t, dur }) => {
            let u = t / dur;
            Vec2::new(at.x, from.y + (ground - from.y) * u * u)
        }
        Some(Part { mv: Move::Gutter, striking: true, .. }) => Vec2::new(at.x, ground),
        Some(Part { mv: Move::Vanish, striking: false, .. }) => at,
        Some(Part { striking: true, .. }) => at,
        Some(_) => {
            side = face(at.x);
            hover(side)
        }
        None if c.buried.is_some() => {
            let t = c.buried.unwrap_or(0.0);
            Vec2::new(at.x, to.y - h.len(SINK_PX + 10.0) * ease((t - BURIED_S) / PULL_S))
        }
        None if c.guttered.is_some() => Vec2::new(at.x, ground),
        None if c.miss.is_some() => {
            let t = c.miss.unwrap_or(0.0);
            let along = (to - from).normalize_or(Vec2::ZERO);
            // It tumbles on past you, slowing.
            at + along * (MISS_DRIFT / MISS_S) * (1.0 - t / MISS_S).max(0.0) * dt
        }
        None if c.rearing.is_some() => at,
        None if c.appear.is_some() => from,
        None => {
            side = face(at.x);
            hover(side)
        }
    };
    let pos = clamp(pos).min(Vec2::new(f32::MAX, ground));

    // ── The strike's own onsets ──
    if let Some(part @ Part { striking: true, .. }) = part {
        let onset = !c.fired;
        c.fired = true;
        match part.mv {
            Move::Bite if onset => {
                riding(inv, h.jaws(), side, None, BITE_DAMAGE, BITE_KNOCKBACK, part.dur, "inu_bite")?;
                play(inv, SFX_DART, pos)?;
                play(inv, SFX_BITE, pos + Vec2::new(side * h.len(20.0), 0.0))?;
            }
            Move::Dive if onset => {
                riding(inv, h.jaws_down(), side, None, DIVE_DAMAGE, DIVE_KNOCKBACK, part.dur + 0.1, "inu_dive")?;
                play(inv, SFX_DART, pos)?;
            }
            Move::Onibi if onset => {
                let count = if c.frenzied { ONIBI_AT.len() } else { 3 };
                let speed = if c.frenzied { ONIBI_SPEED_FRENZIED } else { ONIBI_SPEED };
                for degrees in ONIBI_AT.iter().take(count) {
                    let off = h.onibi(*degrees);
                    let origin = pos + Vec2::new(off.x * side, off.y);
                    onibi(inv, origin, target - origin, speed)?;
                    burst(inv, origin, 6, 140.0, ONIBI_FIRE, "spark")?;
                }
                play(inv, SFX_FLING, pos)?;
            }
            Move::Howl if onset => {
                let radius = inu.size[1] * HOWL_RADIUS;
                riding(inv, (Vec2::ZERO, Vec2::splat(radius)), side, Some(radius), HOWL_DAMAGE, HOWL_KNOCKBACK, 0.3, "inu_howl")?;
                play(inv, SFX_HOWL, pos)?;
                shake(inv, SHAKE_HOWL)?;
                burst(inv, pos, 30, 360.0, FIRE_CORE, "spark")?;
                if c.frenzied {
                    for k in 0..HOWL_ONIBI {
                        let a = std::f32::consts::TAU * (k as f32 + 0.5) / HOWL_ONIBI as f32;
                        onibi(inv, pos, Vec2::new(a.cos(), a.sin()), ONIBI_SPEED)?;
                    }
                }
            }
            Move::Vanish if onset => {
                play(inv, SFX_VANISH, pos)?;
                burst(inv, pos, 20, 240.0, FIRE, "spark")?;
            }
            Move::Gutter if onset => {
                play(inv, SFX_SLAM, Vec2::new(pos.x, hall.floor - 4.0))?;
                burst(inv, Vec2::new(pos.x, hall.floor - 4.0), 16, 200.0, DUST, "dust")?;
            }
            _ => {}
        }
    }
    // The dive lands: its teeth go into the floor.
    if c.buried.is_some_and(|t| t <= dt) {
        let jaws = Vec2::new(pos.x, hall.floor - 4.0);
        play(inv, SFX_SLAM, jaws)?;
        shake(inv, SHAKE_DIVE)?;
        burst(inv, jaws, 30, 320.0, DUST, "dust")?;
        burst(inv, jaws, 16, 260.0, FIRE, "spark")?;
    }

    let row = drawn_row(&c, part);
    held_at(inv, at, pos, side, row, dt)?;
    c.store(inv)
}

/// The rise: hidden in his body until his head tears off; then its fire
/// catches, it grows to its own size, rises, and howls under its name.
#[allow(clippy::too_many_arguments)]
fn rise(
    inv: &mut Invocation<'_>,
    c: &mut Conductor,
    inu: &BossConduct,
    from: Vec2,
    side: f32,
    dt: f32,
) -> Result<(), Fault> {
    let at = Vec2::from(inu.position);
    let Some(t) = c.intro else {
        // Asleep: it is still his head, on his body.
        set_scale(c, inv, c.rise_scale)?;
        return held_at(inv, at, from, side, ("unseen", 0.0, true), dt);
    };
    let start = rise_at(inu);
    let was = t - dt;
    let crossed = |s: f32| t >= s && was < s;
    inv.submit::<BossStagePort>(rise_stage_at(t, start))?;
    if t < start {
        set_scale(c, inv, c.rise_scale)?;
        return held_at(inv, at, from, side, ("unseen", 0.0, true), dt);
    }
    let u = (t - start) / RISE_S;
    let grow = ((u - GROW_FROM) / (GROW_TO - GROW_FROM)).clamp(0.0, 1.0);
    set_scale(c, inv, c.rise_scale + (1.0 - c.rise_scale) * ease(grow))?;
    if crossed(start) {
        burst(inv, from, 20, 220.0, FIRE, "spark")?;
    }
    if crossed(start + GROW_FROM * RISE_S) {
        // The fire catches in it.
        play(inv, SFX_CATCH, at)?;
        burst(inv, at, 40, 340.0, FIRE, "spark")?;
        shake(inv, SHAKE_TEAR)?;
    }
    if crossed(start + 0.82 * RISE_S) {
        play(inv, SFX_HOWL, at)?;
        burst(inv, at, 30, 360.0, FIRE_CORE, "spark")?;
        shake(inv, SHAKE_HOWL)?;
    }
    let pos = from - Vec2::new(0.0, RISE_UP * ease(u.min(1.0)));
    let row = if u < 1.0 { span("rise", 0.0, 22.0, 0.08, u) } else { ("idle", t, true) };
    held_at(inv, at, pos, side, row, dt)
}

/// The row it is drawn with: each phase of a move has its own row; then its
/// windows; then its float.
fn drawn_row(c: &Conductor, part: Option<Part>) -> (&'static str, f32, bool) {
    let u = |p: Part| p.t / p.dur;
    let tell = |name, frames: f32, fd: f32, u: f32| span(name, 0.0, frames, fd, (u / 0.7).min(1.0));
    if let Some(t) = c.rekindle {
        return span("rekindle", 0.0, 6.0, 0.06, t / REKINDLE_S);
    }
    if let Some(t) = c.rearing {
        return if t < FLARE_S { ("howl", t, false) } else { span("vanish", 0.0, 8.0, 0.06, (t - FLARE_S) / VANISH_S) };
    }
    if let Some(t) = c.appear {
        return span("appear", 0.0, 8.0, 0.06, t / APPEAR_S);
    }
    match part {
        Some(p @ Part { mv: Move::Bite, striking: false, .. }) => tell("bite_tell", 8.0, 0.07, u(p)),
        Some(p @ Part { mv: Move::Bite, .. }) => span("bite", 0.0, 8.0, 0.05, u(p)),
        Some(p @ Part { mv: Move::Dive, striking: false, .. }) => tell("dive_tell", 8.0, 0.08, u(p)),
        Some(Part { mv: Move::Dive, t, .. }) => ("dive", t, true),
        Some(p @ Part { mv: Move::Onibi, striking: false, .. }) => span("onibi_tell", 0.0, 10.0, 0.08, u(p)),
        Some(p @ Part { mv: Move::Onibi, .. }) => span("onibi", 0.0, 8.0, 0.06, u(p)),
        Some(p @ Part { mv: Move::Howl, striking: false, .. }) => tell("howl_tell", 6.0, 0.08, u(p)),
        Some(Part { mv: Move::Howl, t, .. }) => ("howl", t, false),
        Some(p @ Part { mv: Move::Vanish, striking: false, .. }) => span("vanish", 0.0, 8.0, 0.06, u(p)),
        Some(p @ Part { mv: Move::Vanish, .. }) => span("appear", 0.0, 8.0, 0.06, u(p)),
        Some(p @ Part { mv: Move::Gutter, striking: false, .. }) => span("gutter", 0.0, 10.0, 0.08, u(p)),
        Some(Part { mv: Move::Gutter, t, .. }) => ("guttered", t, true),
        None => {
            if let Some(t) = c.buried {
                if t < BURIED_S {
                    ("buried", t, true)
                } else {
                    span("pull_free", 0.0, 6.0, 0.07, (t - BURIED_S) / PULL_S)
                }
            } else if let Some(t) = c.guttered {
                ("guttered", t, true)
            } else if let Some(t) = c.miss {
                span("bite_miss", 0.0, 12.0, 0.07, t / MISS_S)
            } else {
                ("idle", c.clock, true)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overlaps((a, ah): (Vec2, Vec2), (b, bh): (Vec2, Vec2)) -> bool {
        ((a - b).abs() - (ah + bh)).cmple(Vec2::ZERO).all()
    }

    /// Its jaws are in front of it and its dive's jaws under it.
    #[test]
    fn its_jaws_are_where_its_art_draws_them() {
        let h = Head::of_height(212.0);
        let (jaws, _) = h.jaws();
        assert!(jaws.x > 0.0, "the bite is ahead of it: {jaws:?}");
        let (down, half) = h.jaws_down();
        assert!(down.y - half.y > 0.0, "the dive's jaws are under its middle: {down:?} {half:?}");
    }

    /// The bite reaches a robot it darts at, at the end of its dash.
    #[test]
    fn the_bite_lands_on_what_it_darts_at() {
        let h = Head::of_height(212.0);
        let robot = (Vec2::new(h.jaws().0.x, 0.0), Vec2::new(14.0, 24.0));
        assert!(overlaps(h.jaws(), robot));
    }

    /// It rises from his dead head at his head's size: smaller than its own.
    #[test]
    fn it_rises_from_his_head_at_his_heads_size() {
        let p = Predecessor { position: [600.0, 760.0], size: [84.0, 140.0], side: 1.0, alive: false };
        let (at, head_px) = dead_head_of(&p);
        assert!(at.x > 600.0, "his head lies ahead of his middle when he faces right: {at:?}");
        let scale = head_px / Head::of_height(212.0).k;
        assert!((0.15..0.5).contains(&scale), "a giant head rises from his small one: {scale}");
    }

    /// The rise dims the room to its red light and shows its name only
    /// once it has risen.
    #[test]
    fn its_rise_dims_the_room_and_names_it() {
        let start = wolfenstein::HEAD_OFF_AT;
        assert!(rise_stage_at(start - 0.1, start).lit > 0.999 && rise_stage_at(start - 0.1, start).card == 0.0);
        let named = rise_stage_at(start + RISE_S + 0.3, start);
        assert!(named.card > 0.9 && named.lit < SPIRIT_LIT + 0.01, "{named:?}");
        assert!(INTRO_S > start + RISE_S, "the intro covers the rise");
    }
}
