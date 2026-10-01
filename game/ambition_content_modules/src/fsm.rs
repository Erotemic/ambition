//! The Flying Spaghetti Monster's conductor: it flies the god and performs its
//! moves. Migrated from the native conductor (fast-iteration I7): the first
//! conducted boss.
//!
//! The boss's scripted pattern decides WHICH move and WHEN (its `Special`
//! keys, telegraph then strike then rest). This decides everything that is
//! the move's own: where the god goes for it, the volume it swings, the shots
//! it throws, the row it is drawn with. It holds the god's pose
//! (`ambition.boss.conducted_pose`) because none of the shared movement
//! profiles can do what it does — swim in beats like a jellyfish, plunge onto
//! the floor where you stood, and lie there tangled until its next move lifts
//! it. A participant who drives the god owns its locomotion instead, except
//! through the dive.
//!
//! ⭐ THE FIGHT'S SHAPE. The god hovers so its noodles hang to just above your
//! head, and they STING: the bell is the target, the curtain under it is not.
//! The bell can be reached two ways — from the arena's ledges, or when it
//! dives and lies stranded on the floor (the fight's one long punish window).
//! Each move pushes you between those two places: the lash sweeps the ledges,
//! the pulse throws everything near it away, the grasp drags you up into the
//! curtain, and the meatballs find you anywhere.

use ambition_boss_special_port::{
    BossConduct, BossConductPort, BossSummon, BossSummonPort, ConductedPose, ConductedPosePort, DrawnRow,
    DrawnRowPort, Pose,
};
use ambition_combat_port::{
    BodySound, BodySoundPort, Burst, BurstPort, HeldDamageBox, HeldDamageBoxPort, RidingHitbox, RidingHitboxPort,
    RidingKnockback,
};
use ambition_extension_sdk::{
    phases::BOSS_CONDUCT, record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};
use ambition_projectile_spec::{ProjectileSpawn, ProjectileSpawnPort};
use bevy_math::Vec2;

/// The behaviour id of the boss it conducts.
pub const FSM_ID: &str = "flying_spaghetti_monster_boss";

/// The bell's half-extent: the boss's `combat_size` is (190, 88).
pub const BELL_HALF: Vec2 = Vec2::new(95.0, 44.0);
/// Where it swims: the bell's centre this far above the floor, so its noodles
/// (~140 long) hang to just over a standing player's head and its bell is out
/// of reach of a jump from the floor. From a ledge, a jump reaches it.
pub const HOVER_ABOVE_FLOOR: f32 = 265.0;
/// One swim beat: squeeze, push, glide.
const BEAT_S: f32 = 1.6;
const BEAT_LIFT: f32 = 16.0;
/// Its gliding speed, and how far it keeps from the walls.
const SWIM_SPEED: f32 = 150.0;
/// It swims BESIDE you, not over you. Bell centre to you.
const SWIM_STANDOFF: f32 = 175.0;
const WALL_MARGIN: f32 = 150.0;

/// The curtain of noodles under the bell. One sting per pass.
const STING_HALF: Vec2 = Vec2::new(88.0, 65.0);
const STING_DAMAGE: i32 = 1;
const STING_KNOCKBACK: f32 = 0.8;
const STING_REARM: f32 = 1.0;

/// The lash: a taut noodle straight out from the bell's side, a little below
/// its centre — the height of a player standing on a ledge.
const LASH_REACH: f32 = 270.0;
const LASH_HALF_H: f32 = 18.0;
const LASH_DROP: f32 = 40.0;
const LASH_DAMAGE: i32 = 2;
const LASH_KNOCKBACK: f32 = 1.3;
/// It lines up this far from you to lash, so the tip lands where you stand.
const LASH_STANDOFF: f32 = LASH_REACH * 0.72;

/// The volley: meatballs lobbed to land where you are, a little spread.
const MEATBALL_HALF: Vec2 = Vec2::new(14.0, 14.0);
const MEATBALL_GRAVITY: f32 = 900.0;
const MEATBALL_FLIGHT_S: f32 = 0.95;
const MEATBALL_DAMAGE: i32 = 2;
const MEATBALL_SPREAD: f32 = 90.0;

/// The pulse: the skirt thrown open, everything near the bell thrown away.
const PULSE_RADIUS: f32 = 205.0;
const PULSE_DAMAGE: i32 = 2;
const PULSE_KNOCKBACK: f32 = 1.6;

/// The grasp: one noodle reaching down and out for you; what it catches it
/// drags up toward the bell (into the curtain).
const GRASP_REACH: f32 = 290.0;
const GRASP_TIP_RADIUS: f32 = 34.0;
const GRASP_DAMAGE: i32 = 1;
const GRASP_PULL: f32 = 560.0;
/// The yank: TOWARD the god (a launch's `x` is mirrored away from its source)
/// and up, into the curtain under the bell.
const GRASP_PULL_DIR: [f32; 2] = [-0.5, -0.87];

/// The dive: up and over you on the tell, then down onto the floor.
const DIVE_RISE: f32 = 70.0;
const DIVE_TRACK_SPEED: f32 = 320.0;
const DIVE_FALL_S: f32 = 0.32;
/// Lying on the floor, its bell's centre is this far above it.
const STRANDED_ABOVE_FLOOR: f32 = 50.0;
const RISE_S: f32 = 0.8;
/// The landing throws a shock each way along the floor: jump it.
const WAVE_SPEED: f32 = 430.0;
const WAVE_HALF: Vec2 = Vec2::new(24.0, 18.0);
const WAVE_DAMAGE: i32 = 1;
const WAVE_KNOCKBACK: f32 = 1.1;
/// The shock's lifetime: longer than it takes to cross a hall.
const WAVE_LIFETIME_S: f32 = 4.0;

/// The lesser appendages: a pair of noodlings either side of the god.
pub const NOODLING: &str = "npc_fsm_noodling";
const NOODLING_HALF: Vec2 = Vec2::new(20.0, 20.0);
const NOODLING_SPREAD: f32 = 120.0;

const SFX_WHIP: &str = "boss.flying_spaghetti_monster.noodle_whip";
const SFX_MEATBALL: &str = "boss.flying_spaghetti_monster.meatball_impact";
const SFX_SPLASH: &str = "boss.flying_spaghetti_monster.sauce_splash";

const DUST: [f32; 4] = [0.93, 0.85, 0.62, 1.0];

/// The god's moves: the `Special` keys its pattern names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Lash,
    Volley,
    Pulse,
    Dive,
    Grasp,
    Appendages,
}

impl Move {
    pub const ALL: [Move; 6] = [Move::Lash, Move::Volley, Move::Pulse, Move::Dive, Move::Grasp, Move::Appendages];

    pub fn key(self) -> &'static str {
        match self {
            Move::Lash => "noodle_lash",
            Move::Volley => "meatball_volley",
            Move::Pulse => "noodly_pulse",
            Move::Dive => "noodly_dive",
            Move::Grasp => "noodly_grasp",
            Move::Appendages => "lesser_appendages",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mv| mv.key() == key)
    }

    /// The move's number in its record: 0 is none.
    fn code(self) -> u32 {
        Self::ALL.iter().position(|mv| *mv == self).unwrap_or(0) as u32 + 1
    }

    fn of_code(code: u32) -> Option<Self> {
        code.checked_sub(1).and_then(|i| Self::ALL.get(i as usize).copied())
    }

    /// The row it is drawn with, and how that row's frames split between the
    /// tell and the strike: (row, tell frames, total frames, seconds per
    /// frame) — the sheet's own.
    fn row(self) -> (&'static str, usize, usize, f32) {
        match self {
            Move::Lash => ("noodle_whip", 3, 7, 0.086),
            Move::Volley => ("meatball_volley", 3, 7, 0.088),
            Move::Pulse => ("pulse", 3, 8, 0.084),
            Move::Appendages => ("summon", 3, 8, 0.096),
            Move::Dive => ("dive", 4, 8, 0.084),
            Move::Grasp => ("grasp", 3, 8, 0.090),
        }
    }
}

record! {
    /// The conductor's memory, on the god.
    pub struct Conductor = SchemaKey::new(crate::PROVIDER, "fsm.conductor", 1);
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
    /// Last tick's time remaining in the live part: a part that restarts
    /// shows as this jumping back up.
    9 last_remaining: f32,
    /// The swim clock.
    10 clock: f32,
    11 ticks: u32,
    /// Where the dive will land, and where it started falling from.
    12 dive_x: f32,
    13 dive_from: [f32; 2],
    /// Lying on the floor after a dive: seconds since it landed.
    14 stranded: Option<f32>,
    /// Rising from where it lay: from where, and for how long so far.
    15 rising_from: Option<[f32; 2]>,
    16 rising_t: f32,
    /// The strike's one-shot effect has been spent.
    17 fired: bool,
    /// Seconds until the curtain can sting again.
    18 sting_rearm: f32,
    /// The two shocks rolling along the floor: where each is, and which way.
    19 wave0_x: Option<f32>,
    20 wave0_dir: f32,
    21 wave1_x: Option<f32>,
    22 wave1_dir: f32,
    /// The shocks' held-box generation: one for each landing.
    23 wave_generation: u32,
}

/// A part of a move, read out of the record.
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

impl Hall {
    fn center_x(&self) -> f32 {
        (self.left + self.right) * 0.5
    }
}

impl Conductor {
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

    /// The dive moves the god by its own arc, even while a participant
    /// drives the god.
    fn scripts_pose(&self, part: Option<Part>) -> bool {
        self.stranded.is_some() || self.rising_from.is_some() || matches!(part, Some(Part { mv: Move::Dive, .. }))
    }

    fn wave(&mut self, slot: usize) -> (&mut Option<f32>, &mut f32) {
        if slot == 0 {
            (&mut self.wave0_x, &mut self.wave0_dir)
        } else {
            (&mut self.wave1_x, &mut self.wave1_dir)
        }
    }
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "fsm"),
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
                selector: FSM_ID.into(),
            },
            reads: Vec::new(),
            writes: vec![Conductor::KEY],
            requests: vec![
                ConductedPosePort::KEY,
                DrawnRowPort::KEY,
                BurstPort::KEY,
                BossSummonPort::KEY,
                HeldDamageBoxPort::KEY,
                RidingHitboxPort::KEY,
                ProjectileSpawnPort::KEY,
                BodySoundPort::KEY,
            ],
            after: Vec::new(),
            limits: Limits { max_requests: 32 },
            // A living god is never idle.
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(conduct),
        }],
    }
}

fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The swim: where the bell wants to be this tick, gliding in beats toward a
/// spot over the player (or `x_goal`), kept off the walls.
fn swim_goal(hall: &Hall, x_goal: f32, clock: f32) -> Vec2 {
    let beat = (clock / BEAT_S).fract();
    // A beat: a quick push up, a slow settle.
    let lift = if beat < 0.25 { ease(beat / 0.25) } else { 1.0 - ease((beat - 0.25) / 0.75) };
    let x = x_goal.clamp(hall.left + WALL_MARGIN, hall.right - WALL_MARGIN);
    Vec2::new(x, hall.floor - HOVER_ABOVE_FLOOR - BEAT_LIFT * lift)
}

/// Step from `at` toward `goal` at no more than `speed`.
fn glide(at: Vec2, goal: Vec2, speed: f32, dt: f32) -> Vec2 {
    let to = goal - at;
    let step = speed * dt;
    if to.length() <= step {
        goal
    } else {
        at + to.normalize() * step
    }
}

/// A meatball's launch so it lands on `target` after `MEATBALL_FLIGHT_S`.
pub fn lob(origin: Vec2, target: Vec2) -> (Vec2, f32) {
    let t = MEATBALL_FLIGHT_S;
    let v = (target - origin) / t - Vec2::new(0.0, 0.5 * MEATBALL_GRAVITY * t);
    (v.normalize_or_zero(), v.length())
}

fn play(inv: &mut Invocation<'_>, cue: &str, at: Vec2) -> Result<(), Fault> {
    inv.submit::<BodySoundPort>(BodySound { cue: cue.into(), at: at.into() })
}

fn burst(inv: &mut Invocation<'_>, at: Vec2, count: u32, speed: f32, color: [f32; 4], kind: &str) -> Result<(), Fault> {
    inv.submit::<BurstPort>(Burst { at: at.into(), count, speed, color, kind: kind.into() })
}

#[allow(clippy::too_many_arguments)]
fn riding(
    inv: &mut Invocation<'_>,
    offset: Vec2,
    half: Vec2,
    circle: Option<f32>,
    damage: i32,
    knockback: RidingKnockback,
    launch_dir: Option<[f32; 2]>,
    life: f32,
    name: &str,
) -> Result<(), Fault> {
    inv.submit::<RidingHitboxPort>(RidingHitbox {
        offset: offset.into(),
        half_extent: half.into(),
        circle_radius: circle,
        damage,
        knockback,
        launch_dir,
        lifetime_s: life,
        name: name.into(),
    })
}

fn conduct(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let god: BossConduct = inv.trigger::<BossConductPort>()?.clone();
    let dt = inv.dt();
    let mut c = Conductor::load(inv)?;
    if !c.hall_known {
        let Some(h) = god.hall else {
            // No hall yet: nothing moves, nothing is remembered.
            return Ok(());
        };
        c.hall_known = true;
        c.hall_floor = h.floor;
        c.hall_left = h.left;
        c.hall_right = h.right;
    }
    let hall = Hall { floor: c.hall_floor, left: c.hall_left, right: c.hall_right };
    let mut side = god.side;
    c.clock += dt;
    c.ticks = c.ticks.wrapping_add(1);
    let at = Vec2::from(god.position);
    let target = god.target.map_or(Vec2::new(hall.center_x(), hall.floor - 24.0), Vec2::from);

    // ── The landing's shocks roll out along the floor ──
    // Before the move runs: a shock moves first on the tick after its landing.
    let mut waves: [Option<(f32, f32)>; 2] = [None; 2];
    for (slot, live) in waves.iter_mut().enumerate() {
        let (x, dir) = c.wave(slot);
        let Some(mut wx) = *x else {
            continue;
        };
        wx += *dir * WAVE_SPEED * dt;
        let gone = wx < hall.left + WAVE_HALF.x || wx > hall.right - WAVE_HALF.x;
        if gone {
            *x = None;
            continue;
        }
        *x = Some(wx);
        *live = Some((wx, *dir));
        if c.ticks % 3 == 0 {
            burst(inv, Vec2::new(wx, hall.floor - WAVE_HALF.y), 3, 120.0, DUST, "dust")?;
        }
    }

    // ── Dead: its shocks end with it; it sinks, limp, to the floor, and is
    // drawn by the engine. ──
    if !god.alive {
        c.wave0_x = None;
        c.wave1_x = None;
        inv.submit::<DrawnRowPort>(DrawnRow { name: None, elapsed: 0.0, looping: false })?;
        let rest = Vec2::new(at.x, hall.floor - STRANDED_ABOVE_FLOOR);
        let pos = glide(at, rest, 120.0, dt);
        inv.submit::<ConductedPosePort>(ConductedPose {
            pose: Some(Pose { position: pos.into(), velocity: ((pos - at) / dt).into() }),
            side,
        })?;
        return c.store(inv);
    }

    // ── Which part of which move is live ──
    let live = match (&god.active, &god.telegraph) {
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
                // A move starting anywhere but its own strike lifts a stranded
                // god off the floor.
                if c.stranded.is_some() && !(mv == Move::Dive && striking) {
                    c.stranded = None;
                    c.rising_from = Some(at.into());
                    c.rising_t = 0.0;
                }
                if !striking {
                    side = if target.x < at.x { -1.0 } else { 1.0 };
                } else {
                    c.fired = false;
                    if mv == Move::Dive {
                        c.dive_from = at.into();
                        c.dive_x = at.x;
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

    // ── Where the god goes ──
    // Between moves it turns to face you.
    if part.is_none() && c.stranded.is_none() {
        side = if target.x < at.x { -1.0 } else { 1.0 };
    }
    let swim = swim_goal(&hall, target.x - side * SWIM_STANDOFF, c.clock);
    let mut landed = None;
    let pos = if let Some(stranded) = c.stranded.as_mut() {
        *stranded += dt;
        Vec2::new(at.x, hall.floor - STRANDED_ABOVE_FLOOR)
    } else if let Some(from) = c.rising_from {
        c.rising_t += dt;
        let from = Vec2::from(from);
        let u = ease(c.rising_t / RISE_S);
        let pos = from + (swim - from) * u;
        if c.rising_t >= RISE_S {
            c.rising_from = None;
        }
        pos
    } else {
        match part {
            Some(Part { mv: Move::Dive, striking: false, t, dur }) => {
                // Up, and over where you stand.
                let over = Vec2::new(target.x, swim.y - DIVE_RISE * ease(t / dur));
                let over = Vec2::new(over.x.clamp(hall.left + BELL_HALF.x, hall.right - BELL_HALF.x), over.y);
                glide(at, over, DIVE_TRACK_SPEED, dt)
            }
            Some(Part { mv: Move::Dive, striking: true, t, .. }) => {
                let floor_y = hall.floor - STRANDED_ABOVE_FLOOR;
                let u = (t / DIVE_FALL_S).clamp(0.0, 1.0);
                // Falling: slow off the top, fast at the floor.
                let from = Vec2::from(c.dive_from);
                let y = from.y + (floor_y - from.y) * u * u;
                if u >= 1.0 && !c.fired {
                    c.fired = true;
                    c.stranded = Some(0.0);
                    landed = Some(c.dive_x);
                }
                Vec2::new(c.dive_x, y)
            }
            Some(Part { mv: Move::Lash, striking: false, .. }) => {
                // Line up at lash range, at your side.
                let goal = Vec2::new(target.x - side * LASH_STANDOFF, swim.y);
                glide(at, swim_goal(&hall, goal.x, c.clock), SWIM_SPEED * 1.6, dt)
            }
            // Striking, it holds where it is; otherwise it swims over you.
            Some(Part { striking: true, .. }) => at,
            _ => glide(at, swim, SWIM_SPEED, dt),
        }
    };
    if let Some(x) = landed {
        // The dive lands: a shock each way along the floor, and a splash.
        let feet = Vec2::new(x, hall.floor - WAVE_HALF.y);
        c.wave_generation = c.wave_generation.wrapping_add(1);
        for (slot, dir) in [-1.0f32, 1.0].into_iter().enumerate() {
            let wx = x + dir * BELL_HALF.x;
            let (wave_x, wave_dir) = c.wave(slot);
            *wave_x = Some(wx);
            *wave_dir = dir;
            waves[slot] = Some((wx, dir));
        }
        play(inv, SFX_SPLASH, feet)?;
        burst(inv, feet, 30, 340.0, DUST, "dust")?;
    }

    // ── Who moves the god ──
    // The module, unless a participant drives it outside the dive.
    let conducts = !god.driven || c.scripts_pose(part);
    if !conducts {
        side = if god.facing < 0.0 { -1.0 } else { 1.0 };
    }
    // A driven god's moves start where the participant put it.
    let pos_now = if conducts { pos } else { at };

    // ── The strike's own onset ──
    if let Some(part) = part {
        if part.striking && !c.fired {
            match part.mv {
                Move::Lash => {
                    c.fired = true;
                    let half = Vec2::new(LASH_REACH * 0.5, LASH_HALF_H);
                    let offset = Vec2::new(side * (BELL_HALF.x * 0.6 + LASH_REACH * 0.5), LASH_DROP);
                    riding(inv, offset, half, None, LASH_DAMAGE, RidingKnockback::FeelScale(LASH_KNOCKBACK), None, part.dur, "fsm_lash")?;
                    play(inv, SFX_WHIP, pos_now)?;
                }
                Move::Volley => {
                    c.fired = true;
                    let count = if god.enraged { 5 } else { 3 };
                    // From the throwing meatball, on the bell's front half.
                    let origin = pos_now + Vec2::new(side * BELL_HALF.x * 0.5, -10.0);
                    for k in 0..count {
                        let spread = (k as f32 - (count - 1) as f32 * 0.5) * MEATBALL_SPREAD;
                        let (dir, speed) = lob(origin, Vec2::new(target.x + spread, target.y));
                        inv.submit::<ProjectileSpawnPort>(ProjectileSpawn {
                            origin,
                            dir,
                            speed,
                            damage: MEATBALL_DAMAGE,
                            max_lifetime: 3.0,
                            half_extent: MEATBALL_HALF,
                            gravity: MEATBALL_GRAVITY,
                            visual_id: "meatball".to_string(),
                            // A meatball bounces once off the floor and rolls
                            // on: jump it, don't stand under it.
                            bounces: 1,
                            bounce_on_world_contact: true,
                            splash_half_extent: 0.0,
                            boomerang_return_s: None,
                        })?;
                    }
                    play(inv, SFX_MEATBALL, origin)?;
                }
                Move::Pulse => {
                    c.fired = true;
                    riding(
                        inv,
                        Vec2::ZERO,
                        Vec2::splat(PULSE_RADIUS),
                        Some(PULSE_RADIUS),
                        PULSE_DAMAGE,
                        RidingKnockback::FeelScale(PULSE_KNOCKBACK),
                        None,
                        part.dur.min(0.3),
                        "fsm_pulse",
                    )?;
                    play(inv, SFX_SPLASH, pos_now)?;
                    burst(inv, pos_now, 24, 380.0, [0.95, 0.88, 0.7, 1.0], "spark")?;
                }
                Move::Grasp => {
                    c.fired = true;
                    // Reach for where you are, within reach, and never up.
                    let base = pos_now + Vec2::new(side * BELL_HALF.x * 0.5, BELL_HALF.y);
                    let mut to = target - base;
                    to.y = to.y.max(to.x.abs() * 0.2);
                    let reach = to.length().min(GRASP_REACH);
                    let tip = base + to.normalize_or_zero() * reach;
                    riding(
                        inv,
                        tip - pos_now,
                        Vec2::splat(GRASP_TIP_RADIUS),
                        Some(GRASP_TIP_RADIUS),
                        GRASP_DAMAGE,
                        RidingKnockback::LaunchSpeed { base: GRASP_PULL, growth: Some(0.0) },
                        Some(GRASP_PULL_DIR),
                        part.dur,
                        "fsm_grasp",
                    )?;
                    play(inv, SFX_WHIP, tip)?;
                }
                Move::Appendages => {
                    c.fired = true;
                    for (i, s) in [-1.0f32, 1.0].into_iter().enumerate() {
                        let x = (pos_now.x + s * NOODLING_SPREAD)
                            .clamp(hall.left + NOODLING_HALF.x * 2.0, hall.right - NOODLING_HALF.x * 2.0);
                        inv.submit::<BossSummonPort>(BossSummon {
                            label: "fsm_noodling".into(),
                            serial: vec![c.ticks, i as u32],
                            position: [x, pos_now.y],
                            half_size: NOODLING_HALF.into(),
                            character_id: NOODLING.into(),
                            health: None,
                            keeps_contact_damage: true,
                            // Its OWN appendages: the god's side, so its sting
                            // and its pulse pass through them.
                            on_boss_side: true,
                        })?;
                    }
                    play(inv, SFX_SPLASH, pos_now)?;
                }
                // The dive fires on landing, above.
                Move::Dive => {}
            }
        }
    }

    // ── The curtain stings ──
    c.sting_rearm -= dt;
    let curtain_down =
        c.stranded.is_none() && c.rising_from.is_none() && !matches!(part, Some(Part { mv: Move::Dive, .. }));
    if curtain_down && c.sting_rearm <= 0.0 {
        c.sting_rearm = STING_REARM;
        riding(
            inv,
            Vec2::new(0.0, BELL_HALF.y + STING_HALF.y),
            STING_HALF,
            None,
            STING_DAMAGE,
            RidingKnockback::FeelScale(STING_KNOCKBACK),
            None,
            STING_REARM,
            "fsm_sting",
        )?;
    }

    // ── The shocks, held while they roll ──
    for (slot, wave) in waves.iter().enumerate() {
        if let Some((x, _)) = wave {
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

    // ── Where it is, and what it is drawn as ──
    inv.submit::<ConductedPosePort>(ConductedPose {
        pose: conducts.then(|| Pose { position: pos.into(), velocity: ((pos - at) / dt).into() }),
        side,
    })?;
    let speed = if conducts { (pos - at).length() / dt } else { Vec2::from(god.velocity).length() };
    let row = drawn_row(&c, part, speed);
    inv.submit::<DrawnRowPort>(match row {
        Some((name, elapsed, looping)) => DrawnRow { name: Some(name.into()), elapsed, looping },
        None => DrawnRow { name: None, elapsed: 0.0, looping: false },
    })?;
    c.store(inv)
}

/// The row the god is drawn with: the live move's, played so its tell frames
/// fill the telegraph and the rest fill the strike; lying on the floor, the
/// dive's splayed last frame; otherwise swimming or idling.
fn drawn_row(c: &Conductor, part: Option<Part>, speed: f32) -> Option<(&'static str, f32, bool)> {
    if c.stranded.is_some() {
        let (name, _, total, fd) = Move::Dive.row();
        return Some((name, total as f32 * fd, false));
    }
    if c.rising_from.is_some() {
        return Some(("drift", c.clock, true));
    }
    if let Some(part) = part {
        let (name, tell, total, fd) = part.mv.row();
        let u = (part.t / part.dur).clamp(0.0, 0.999);
        let frame = if part.striking { tell as f32 + u * (total - tell) as f32 } else { u * tell as f32 };
        return Some((name, frame * fd, false));
    }
    if speed > 60.0 {
        Some(("drift", c.clock, true))
    } else {
        Some(("idle", c.clock, true))
    }
}
