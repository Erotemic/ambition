//! The Mockingbird's conductor: the air chase.
//!
//! Its scripted pattern decides WHICH move and WHEN (its `Special` keys:
//! tell, strike, rest). This decides everything that is the move's own: where
//! it flies, the volumes it swings, the shots it fires, its guard, the row it
//! is drawn with. It holds its pose (`ambition.boss.conducted_pose`): it is a
//! jet that holds one side of an open sky, which no shared movement profile
//! flies.
//!
//! ⭐ THE FIGHT'S SHAPE (`docs/planning/game/bosses.md`, "The Mockingbird").
//! Jon (2026-10-06): a fast air chase. The Mockingbird holds the left of the
//! screen; burning sharks flee across a sky that never stops, and they are
//! the footing. The flock drifts toward it, so standing still carries you
//! into its jaws.
//!
//! Out at its side, its armoured hull turns every blow (`ambition.boss.guard`:
//! they clang off). It fires from there: a missile off its wingtip, a spread
//! of fireballs from its throat, and a snap at anyone who rides a shark into
//! reach. Then it dives across the sky at you, nose first, and bites, which
//! opens it to damage: winded after the bite, it hangs in the middle of the
//! sky on stuttering rotors, guard down, and flies home. That is the punish
//! window, and the only one.
//!
//! Phase 2 (it screeches, and LIGHTS ITSELF ON FIRE: not because it is hurt,
//! because it is angry): the missiles come in salvoes, the fireballs in a
//! wider fan, the dive comes twice, turning at the end of the first bite, and
//! it spits burning lightsabers that spin out across the sky and come back.
//!
//! Phase 3 (Jon, 2026-10-09): its red fire turns BLUE, a cold fire, and it is
//! angrier. The sky climbs into SPACE (`ascent`). There its hull turns every
//! blow, in each of its moves: the only way to kill a mockingbird is to hit
//! it with THE MOON. The moon crosses the room again and again
//! ([`moon_at`]). The Mockingbird keeps out of its way, and it cannot while
//! it dives or hangs winded after a bite: the moon strikes it then
//! (`moonstruck`), and the fight is over. So you lure its dive into the path
//! of the moon. Its strafing run still rains fire, and a low pass back under.
//!
//! What it burns with and where the moon is are in its record for the view
//! that draws them; nothing here draws.
//!
//! Every point on its body is a socket its sheet publishes
//! (`mockingbird_boss_v2_actor.ron`), in sheet pixels, carried into the world
//! by [`PX`] about the frame's centre, which is its position (the boss
//! placement law).

use ambition_boss_special_port::{
    BossConduct, BossConductPort, BossGuard, BossGuardPort, ConductedPose, ConductedPosePort, DrawnRow, DrawnRowPort,
    Pose,
};
use ambition_combat_port::{
    BodySound, BodySoundPort, Burst, BurstPort, CameraShake, CameraShakePort, HeldDamageBox, HeldDamageBoxPort, RidingHitbox,
    RidingHitboxPort, RidingKnockback,
};
use ambition_extension_sdk::{
    phases::BOSS_CONDUCT, record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};
use ambition_projectile_spec::{ProjectileSpawn, ProjectileSpawnPort};
use bevy_math::Vec2;

/// The behaviour id of the boss it conducts.
pub const MOCKINGBIRD_ID: &str = "mockingbird";

/// World units per sheet pixel: its sky's 130 x 130 spawn drawn at the
/// sheet's `collision_scale` 1.9, over the 368 px frame
/// (`the_mockingbird_is_drawn_at_the_scale_its_volumes_are_measured`).
pub const PX: f32 = 130.0 * 1.9 / 368.0;
/// The frame's centre, in sheet pixels: its position.
const FRAME_CENTRE: Vec2 = Vec2::new(275.5, 184.0);

/// A sheet point (pixels, art facing right) as an offset from its position,
/// unflipped.
pub fn art(x: f32, y: f32) -> Vec2 {
    (Vec2::new(x, y) - FRAME_CENTRE) * PX
}

/// Its sockets (`mockingbird_boss_v2_actor.ron`).
pub fn mouth() -> Vec2 {
    art(490.8, 185.4)
}
pub fn missile_nose() -> Vec2 {
    art(286.0, 228.5)
}
fn thruster() -> Vec2 {
    art(117.0, 183.0)
}
fn core() -> Vec2 {
    art(276.0, 184.0)
}
/// The hull's half-width: the body box (53..534 px wide).
pub fn hull_half_width() -> f32 {
    (534.0 - 53.0) * 0.5 * PX
}

/// Where it holds the sky: where it was placed (its sky places it at the
/// left), at a height it eases toward yours within this fraction of the room
/// either side of its spot, bobbing on its rotors.
const HOME_REACH_Y: f32 = 0.2;
const HOME_EASE: f32 = 1.5;
const BOB_PX: f32 = 10.0;
const SWAY_PX: f32 = 18.0;
/// How fast it flies home, and between moves.
const HOVER_SPEED: f32 = 420.0;
/// It is home (and guarded again) within this of its spot.
const HOME_REACHED: f32 = 36.0;

/// The bite: its jaws, a little ahead of the mouth socket.
fn bite_box() -> (Vec2, Vec2) {
    (mouth() + Vec2::new(26.0, 6.0), Vec2::new(52.0, 44.0))
}
const BITE_DAMAGE: i32 = 2;
const BITE_KNOCKBACK: f32 = 1.5;
/// The snap at a body that rides into reach of it at its side: a short
/// lunge, then back.
const SNAP_LUNGE: f32 = 110.0;
const SNAP_LUNGE_S: f32 = 0.14;

/// The dive: from where it is to past where you were when it set off, nose
/// first, at this speed, easing in. The bite at its end is harder.
const DIVE_SPEED: f32 = 1050.0;
const DIVE_OVERSHOOT: f32 = 70.0;
const DIVE_MIN_S: f32 = 0.32;
const DIVE_RAMP: f32 = 0.35;
const CHOMP_DAMAGE: i32 = 3;
const CHOMP_KNOCKBACK: f32 = 1.8;
/// Winded after the bite: guard down, sinking on stuttering rotors, for this
/// long, then home. Shorter as the fight goes on.
const STUN_S: [f32; 3] = [2.1, 1.75, 1.45];
const STUN_SINK: f32 = 40.0;
/// Its body hurts to touch while it dives.
const RAM_DAMAGE: i32 = 2;
const RAM_KNOCKBACK: f32 = 1.6;

/// The missile: off the near wingtip at where you are, faster as the fight
/// goes on. A salvo is three, each aimed again.
const MISSILE_SPEED: [f32; 3] = [470.0, 540.0, 620.0];
const MISSILE_HALF: Vec2 = Vec2::new(20.0, 7.0);
const MISSILE_DAMAGE: i32 = 2;
const MISSILE_LIFE_S: f32 = 3.2;
const SALVO_EVERY_S: f32 = 0.24;
const SALVO_COUNT: u32 = 3;
const SALVO_SPREAD_DEG: f32 = 7.0;
pub const MISSILE_VISUAL: &str = "mockingbird_missile";

/// The fireballs: spat in a fan centred on you.
const FIRE_SPEED: f32 = 360.0;
const FIRE_HALF: Vec2 = Vec2::new(13.0, 13.0);
const FIRE_DAMAGE: i32 = 2;
const FIRE_LIFE_S: f32 = 3.4;
const FAN_DEG: f32 = 13.0;
pub const FIRE_VISUAL: &str = "mockingbird_fireball";

/// The lightsabers (phase 2 on): spat from its mouth in a fan centred on you,
/// burning. They spin out, slow, and come back the way they went.
const SABER_SPEED: f32 = 520.0;
const SABER_HALF: Vec2 = Vec2::new(26.0, 26.0);
const SABER_DAMAGE: i32 = 2;
const SABER_LIFE_S: f32 = 2.6;
const SABER_RETURN_S: f32 = 0.85;
const SABER_FAN_DEG: f32 = 16.0;
pub const SABER_VISUAL: &str = "mockingbird_lightsaber";
/// What it throws once its fire is cold.
pub const COLD_SABER_VISUAL: &str = "mockingbird_lightsaber_cold";
pub const COLD_FIRE_VISUAL: &str = "mockingbird_coldfire";

/// The climb into space (phase 3): how long the sky takes to become space,
/// and to become sky again when the Mockingbird is dead.
pub const ASCENT_S: f32 = 7.0;
const DESCENT_S: f32 = 5.0;

/// The moon (in space). It crosses the room from the right to the left in
/// `MOON_CROSS_S`, one time each `MOON_EVERY_S`, on a lane a little above the
/// middle that is highest at the middle of the room.
pub const MOON_RADIUS: f32 = 120.0;
const MOON_FIRST_S: f32 = 2.0;
const MOON_EVERY_S: f32 = 10.0;
const MOON_CROSS_S: f32 = 5.0;
const MOON_LANE: f32 = 0.46;
const MOON_ARC: f32 = 0.07;
/// The moon strikes a Mockingbird whose core is this near its edge.
const MOON_REACH: f32 = 70.0;
/// The moon hurts a body it rolls over.
/// How far ahead of the moon, past its radius, it starts to get out of the
/// moon's way, how far above the moon's top it holds, and how fast it goes
/// there.
const MOON_DODGE_AHEAD: f32 = 460.0;
const MOON_DODGE_CLEAR: f32 = 100.0;
const MOON_DODGE_EASE: f32 = 4.5;
/// The slot of the moon's damage box.
const MOON_SLOT: u32 = 0;
const MOON_DAMAGE: i32 = 2;
const MOON_KNOCKBACK: f32 = 1.7;
/// How fast the moon throws it out of the fight.
const MOON_THROW: f32 = 620.0;

/// The strafing run (enraged): up to the top of the sky, across it raining
/// fire on the flock, down the far side and back low under it.
const STRAFE_SPEED: f32 = 760.0;
const STRAFE_TOP: f32 = 0.15;
const STRAFE_LOW: f32 = 0.84;
const STRAFE_FAR: f32 = 0.88;
const BOMB_EVERY_S: f32 = 0.13;
const BOMB_GRAVITY: f32 = 950.0;

/// How hard each moment shakes the camera.
const SHAKE_SCREECH: f32 = 9.0;
const SHAKE_CHOMP: f32 = 7.0;
const SHAKE_LAUNCH: f32 = 3.0;
const SHAKE_DEATH: f32 = 13.0;
const SHAKE_MOON: f32 = 22.0;

/// Its death: shot down, it falls out of the sky trailing smoke, to here
/// below its bottom edge: out of sight (its hull is half this tall), and
/// inside the room's fall margin, past which a body has left the world.
const FALL_GRAVITY: f32 = 520.0;
const FALLEN_BELOW: f32 = 140.0;

const SMOKE: [f32; 4] = [0.36, 0.34, 0.38, 1.0];
const FIRE: [f32; 4] = [1.0, 0.56, 0.18, 1.0];
const STEEL: [f32; 4] = [0.82, 0.84, 0.88, 1.0];
/// Its fire once it is cold.
const COLD: [f32; 4] = [0.36, 0.72, 1.0, 1.0];
const MOONDUST: [f32; 4] = [0.86, 0.86, 0.80, 1.0];

// ⛔ A cue the bank does not hold plays NOTHING: every cue here is held to a
// recipe (`every_cue_the_mockingbird_plays_has_a_recipe`).
const SFX_SCREECH: &str = "boss.mockingbird.screech";
const SFX_LAUNCH: &str = "boss.mockingbird.missile_launch";
const SFX_SPIT: &str = "boss.mockingbird.spit";
const SFX_CHOMP: &str = "boss.mockingbird.chomp";
const SFX_DIVE: &str = "boss.mockingbird.dive";
const SFX_STALL: &str = "boss.mockingbird.stall";
const SFX_DEFEAT: &str = "boss.mockingbird.defeat";

/// Every cue it plays itself (its pattern's tells name their own).
pub const SOUNDS: [&str; 7] = [SFX_SCREECH, SFX_LAUNCH, SFX_SPIT, SFX_CHOMP, SFX_DIVE, SFX_STALL, SFX_DEFEAT];

/// Its moves: the `Special` keys its pattern names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Missile,
    Salvo,
    Fireballs,
    FireFan,
    Snap,
    Dive,
    DoubleDive,
    Strafe,
    Lightsabers,
}

impl Move {
    pub const ALL: [Move; 9] = [
        Move::Missile,
        Move::Salvo,
        Move::Fireballs,
        Move::FireFan,
        Move::Snap,
        Move::Dive,
        Move::DoubleDive,
        Move::Strafe,
        // New moves go at the end: a move's code is its place here.
        Move::Lightsabers,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Move::Missile => "mockingbird_missile",
            Move::Salvo => "mockingbird_salvo",
            Move::Fireballs => "mockingbird_fireballs",
            Move::FireFan => "mockingbird_fire_fan",
            Move::Snap => "mockingbird_snap",
            Move::Dive => "mockingbird_dive",
            Move::DoubleDive => "mockingbird_double_dive",
            Move::Strafe => "mockingbird_strafe",
            Move::Lightsabers => "mockingbird_lightsabers",
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
    /// The conductor's memory, on the Mockingbird.
    pub struct Conductor = SchemaKey::new(crate::PROVIDER, "mockingbird.conductor", 1);
    /// The room, measured once.
    1 room_known: bool,
    2 room_w: f32,
    3 room_h: f32,
    /// The live part: its move (`Move::code`, 0 for none), whether striking,
    /// seconds into it, its full length, and last tick's time remaining in it.
    4 part_move: u32,
    5 part_striking: bool,
    6 part_t: f32,
    7 part_dur: f32,
    8 last_remaining: f32,
    9 clock: f32,
    10 ticks: u32,
    /// The strike's onset has been spent.
    11 fired: bool,
    /// The height it holds at its side, eased toward yours.
    12 hover_y: f32,
    /// Diving: from where, to where, seconds into this leg, and how many
    /// more legs (a double dive turns once).
    13 diving: bool,
    14 dive_from_x: f32,
    15 dive_from_y: f32,
    16 dive_to_x: f32,
    17 dive_to_y: f32,
    18 dive_t: f32,
    19 dives_left: u32,
    /// Winded after a dive's bite: seconds so far, and flying home after.
    20 stunned: Option<f32>,
    21 returning: bool,
    /// Shots of this strike so far (a salvo's missiles, a strafe's bombs).
    22 shots: u32,
    /// The strafing run: seconds into it.
    23 strafing: Option<f32>,
    24 strafe_from_x: f32,
    25 strafe_from_y: f32,
    /// Screeching through the beat between two phases: seconds into it, and
    /// how many times it has.
    26 screeching: Option<f32>,
    27 screeches: u32,
    /// Its phase, from the moves it has been asked for (0, 1, 2: the pattern
    /// is the authority; enraged is the trigger's).
    28 phase: u32,
    /// Its death: seen alive (a corpse it was built as is not mourned),
    /// mourned, and the fall's speed.
    29 seen_alive: bool,
    30 mourned: bool,
    31 fall_v: f32,
    /// The snap's lunge started here.
    32 snap_from_x: f32,
    /// The guard it last asked for.
    33 guarded: bool,
    /// Seconds since it died.
    34 dead_t: f32,
    /// Where it is: this module's alone. The body is held here every tick;
    /// what the body integrator does to it in between (gravity, a shark it
    /// would land on) is not read back.
    35 pos_known: bool,
    36 pos_x: f32,
    37 pos_y: f32,
    /// Its spot: where it was placed.
    38 home_x: f32,
    39 home_y: f32,
    /// How far the sky has become space: 0 is sky, 1 is space. It climbs in
    /// phase 3 and comes back when the Mockingbird is dead.
    40 ascent: f32,
    /// The moon: seconds since the room came into space, and where it is
    /// while it is in the room.
    41 moon_t: f32,
    42 moon_out: bool,
    43 moon_x: f32,
    44 moon_y: f32,
    /// The moon has struck it. Its game lands the moon's blow for that
    /// (`strike_it_with_the_moon`).
    45 moonstruck: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Part {
    mv: Move,
    striking: bool,
    t: f32,
    dur: f32,
}

impl Conductor {
    /// The move being performed and whether it is striking, for tests and
    /// inspectors.
    pub fn performing(&self) -> Option<(Move, bool)> {
        self.part().map(|p| (p.mv, p.striking))
    }

    /// Winded after a dive: the punish window.
    pub fn is_stunned(&self) -> bool {
        self.stunned.is_some()
    }

    /// Whether it is out of its guard: diving, winded, flying home after, or
    /// passing low on a strafe.
    pub fn is_open(&self) -> bool {
        !self.guarded
    }

    /// Screeching between two phases.
    pub fn is_screeching(&self) -> bool {
        self.screeching.is_some()
    }

    /// The room it measured, once it has.
    pub fn room(&self) -> Option<Vec2> {
        self.room_known.then_some(Vec2::new(self.room_w, self.room_h))
    }

    /// Its phase: 0, then 1 (on fire), then 2 (its fire is cold; space).
    pub fn phase(&self) -> u32 {
        self.phase.min(2)
    }

    /// How far the sky has become space, 0 to 1.
    pub fn ascent(&self) -> f32 {
        self.ascent
    }

    /// Where the moon's centre is, while the moon is in the room.
    pub fn moon(&self) -> Option<Vec2> {
        self.moon_out.then_some(Vec2::new(self.moon_x, self.moon_y))
    }

    /// The moon has struck it.
    pub fn is_moonstruck(&self) -> bool {
        self.moonstruck
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

    fn stage(&self) -> usize {
        (self.phase as usize).min(2)
    }
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "mockingbird"),
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
                selector: MOCKINGBIRD_ID.into(),
            },
            reads: Vec::new(),
            writes: vec![Conductor::KEY],
            requests: vec![
                ConductedPosePort::KEY,
                DrawnRowPort::KEY,
                BossGuardPort::KEY,
                BurstPort::KEY,
                CameraShakePort::KEY,
                RidingHitboxPort::KEY,
                ProjectileSpawnPort::KEY,
                BodySoundPort::KEY,
                HeldDamageBoxPort::KEY,
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

/// Accelerating from rest and arriving at speed: a dive, nose first.
fn dive_ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    // Mostly linear, with an eased start.
    if t < DIVE_RAMP {
        let u = t / DIVE_RAMP;
        DIVE_RAMP * 0.5 * u * u
    } else {
        DIVE_RAMP * 0.5 + (t - DIVE_RAMP) / (1.0 - DIVE_RAMP) * (1.0 - DIVE_RAMP * 0.5)
    }
}

/// Toward `to` at no more than `speed`.
fn glide(from: Vec2, to: Vec2, speed: f32, dt: f32) -> Vec2 {
    let d = to - from;
    let step = speed * dt;
    if d.length() <= step {
        to
    } else {
        from + d.normalize() * step
    }
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

/// A volume riding its body, from an unflipped art box turned to `side`.
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

fn turned(v: Vec2, side: f32) -> Vec2 {
    Vec2::new(v.x * side, v.y)
}

fn rotated(dir: Vec2, degrees: f32) -> Vec2 {
    let (s, c) = degrees.to_radians().sin_cos();
    Vec2::new(dir.x * c - dir.y * s, dir.x * s + dir.y * c)
}

#[allow(clippy::too_many_arguments)]
fn shoot(
    inv: &mut Invocation<'_>,
    origin: Vec2,
    dir: Vec2,
    speed: f32,
    half: Vec2,
    damage: i32,
    life: f32,
    gravity: f32,
    visual: &str,
) -> Result<(), Fault> {
    shoot_returning(inv, origin, dir, speed, half, damage, life, gravity, visual, None)
}

/// [`shoot`], for a shot that comes back after `return_s` (a thrown blade).
#[allow(clippy::too_many_arguments)]
fn shoot_returning(
    inv: &mut Invocation<'_>,
    origin: Vec2,
    dir: Vec2,
    speed: f32,
    half: Vec2,
    damage: i32,
    life: f32,
    gravity: f32,
    visual: &str,
    return_s: Option<f32>,
) -> Result<(), Fault> {
    inv.submit::<ProjectileSpawnPort>(ProjectileSpawn {
        origin,
        dir,
        speed,
        damage,
        max_lifetime: life,
        half_extent: half,
        gravity,
        visual_id: visual.to_string(),
        bounces: 0,
        bounce_on_world_contact: false,
        splash_half_extent: 0.0,
        boomerang_return_s: return_s,
    })
}

/// Where the moon's centre is `t` seconds after the room came into space:
/// `None` between two crossings.
pub fn moon_at(t: f32, room: Vec2) -> Option<Vec2> {
    let since = t - MOON_FIRST_S;
    if since < 0.0 {
        return None;
    }
    let u = (since % MOON_EVERY_S) / MOON_CROSS_S;
    if u > 1.0 {
        return None;
    }
    let [from, to, lane, arc] = moon_lane(room);
    let rise = (u * std::f32::consts::PI).sin();
    Some(Vec2::new(from + (to - from) * u, lane - arc * rise))
}

/// The moon's lane across `room`: the x it comes in at, the x it goes out at,
/// the height of the lane's two ends, and how far its middle is above them.
/// A drawing of the lane shows the player where to lure it.
pub fn moon_lane(room: Vec2) -> [f32; 4] {
    [room.x + MOON_RADIUS + 40.0, -MOON_RADIUS - 40.0, room.y * MOON_LANE, room.y * MOON_ARC]
}

/// The strafing run's waypoints, from where it set off.
fn strafe_path(from: Vec2, room: Vec2, home_x: f32) -> [Vec2; 5] {
    [
        from,
        Vec2::new(from.x + 80.0, room.y * STRAFE_TOP),
        Vec2::new(room.x * STRAFE_FAR, room.y * STRAFE_TOP),
        Vec2::new(room.x * STRAFE_FAR, room.y * STRAFE_LOW),
        Vec2::new(home_x + 60.0, room.y * STRAFE_LOW),
    ]
}

/// Where a strafing run is `t` seconds in, and on which leg (`None` once it
/// has run its course).
fn strafe_at(path: &[Vec2; 5], t: f32) -> Option<(Vec2, usize)> {
    let mut left = t * STRAFE_SPEED;
    for (leg, pair) in path.windows(2).enumerate() {
        let len = (pair[1] - pair[0]).length();
        if left <= len {
            let u = if len > 0.0 { left / len } else { 1.0 };
            return Some((pair[0].lerp(pair[1], u), leg));
        }
        left -= len;
    }
    None
}

fn conduct(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let bird: BossConduct = inv.trigger::<BossConductPort>()?.clone();
    let dt = inv.dt();
    let mut c = Conductor::load(inv)?;
    if bird.alive && c.dead_t > 0.0 {
        // Brought back (a Hall of Bosses life switch): it starts again from
        // where its body was put.
        let room = (c.room_known, c.room_w, c.room_h);
        c = Conductor::default();
        (c.room_known, c.room_w, c.room_h) = room;
    }
    if !c.pos_known {
        c.pos_known = true;
        c.pos_x = bird.position[0];
        c.pos_y = bird.position[1];
        c.home_x = c.pos_x;
        c.home_y = c.pos_y;
    }
    let at = Vec2::new(c.pos_x, c.pos_y);
    if !c.room_known {
        let Some([w, h]) = bird.room else {
            return Ok(());
        };
        c.room_known = true;
        c.room_w = w;
        c.room_h = h;
        c.hover_y = at.y;
    }
    let room = Vec2::new(c.room_w, c.room_h);
    c.clock += dt;
    c.ticks = c.ticks.wrapping_add(1);
    let mut side = bird.side;
    let target = bird.target.map_or(Vec2::new(room.x * 0.6, room.y * 0.5), Vec2::from);
    let home_x = c.home_x;
    let (low, high) = (
        (c.home_y - room.y * HOME_REACH_Y).max(room.y * 0.12),
        (c.home_y + room.y * HOME_REACH_Y).min(room.y * 0.8),
    );
    // It keeps out of the moon's way: with the moon coming at its side of the
    // sky, it holds above the moon. It cannot do this while it dives or hangs
    // winded, which is the lure.
    let dodge = c
        .moon()
        .filter(|moon| (moon.x - home_x).abs() < MOON_RADIUS + MOON_DODGE_AHEAD)
        .map(|moon| (moon.y - MOON_RADIUS - MOON_DODGE_CLEAR).max(room.y * 0.08));
    let (goal, pull) = match dodge {
        Some(over) => (over, MOON_DODGE_EASE),
        None => (target.y.clamp(low, high.max(low)), HOME_EASE),
    };
    c.hover_y += (goal - c.hover_y) * (pull * dt).min(1.0);
    let home = Vec2::new(
        home_x + SWAY_PX * (0.5 * c.clock).sin(),
        c.hover_y + BOB_PX * (1.4 * c.clock).sin(),
    );

    // ── The sky and the moon ──
    // Its fire is cold in phase 3, and the sky climbs into space while it
    // lives. With it dead the sky comes back, and a crossing that is under
    // way ends.
    let cold = c.phase >= 2;
    if bird.alive && cold {
        c.ascent = (c.ascent + dt / ASCENT_S).min(1.0);
    } else if !bird.alive {
        c.ascent = (c.ascent - dt / DESCENT_S).max(0.0);
    }
    if bird.alive && c.ascent >= 1.0 || c.moon_out {
        c.moon_t += dt;
        match moon_at(c.moon_t, room) {
            Some(moon) => {
                c.moon_out = true;
                c.moon_x = moon.x;
                c.moon_y = moon.y;
                // It hurts a body it rolls over: one box for each crossing,
                // which goes with the moon.
                inv.submit::<HeldDamageBoxPort>(HeldDamageBox {
                    slot: MOON_SLOT,
                    generation: ((c.moon_t - MOON_FIRST_S) / MOON_EVERY_S) as u32,
                    center: moon.into(),
                    half_extent: Vec2::splat(MOON_RADIUS * 0.72).into(),
                    damage: MOON_DAMAGE,
                    knockback: MOON_KNOCKBACK,
                    lifetime_s: MOON_CROSS_S + 1.0,
                })?;
            }
            None => c.moon_out = false,
        }
    }
    let fire = if cold { COLD } else { FIRE };

    // ── Dead: shot down, it falls out of the sky trailing smoke ──
    if bird.alive {
        c.seen_alive = true;
    }
    if !bird.alive {
        if c.seen_alive && !c.mourned {
            c.mourned = true;
            // Struck by the moon, it is thrown up and away the way the moon
            // goes. Shot down (a fight whose script lets a hit kill), it falls.
            c.fall_v = if c.moonstruck { -300.0 } else { -120.0 };
            play(inv, SFX_DEFEAT, at)?;
            shake(inv, if c.moonstruck { SHAKE_MOON } else { SHAKE_DEATH })?;
            burst(inv, at + turned(core(), side), 40, 420.0, fire, "spark")?;
            burst(inv, at + turned(core(), side), 24, 260.0, SMOKE, "dust")?;
            burst(inv, at + turned(core(), side), 14, 300.0, STEEL, "shard")?;
        }
        c.stunned = None;
        c.diving = false;
        c.strafing = None;
        let pos = if c.seen_alive {
            c.fall_v += FALL_GRAVITY * dt;
            let drift = if c.moonstruck { -MOON_THROW * (1.0 - c.dead_t / 2.5).max(0.1) } else { -25.0 * side };
            let pos = at + Vec2::new(drift * dt, c.fall_v * dt);
            if pos.y < room.y + FALLEN_BELOW && c.ticks % 4 == 0 {
                burst(inv, pos + turned(thruster(), side), 3, 90.0, SMOKE, "dust")?;
                burst(inv, pos + turned(core(), side), 2, 160.0, fire, "spark")?;
            }
            Vec2::new(pos.x, pos.y.min(room.y + FALLEN_BELOW))
        } else {
            // A corpse it was built as: out of sight below the sky.
            Vec2::new(at.x, room.y + FALLEN_BELOW)
        };
        if c.guarded {
            c.guarded = false;
            inv.submit::<BossGuardPort>(BossGuard { guarded: false })?;
        }
        c.dead_t += dt;
        c.pos_x = pos.x;
        c.pos_y = pos.y;
        inv.submit::<DrawnRowPort>(DrawnRow { name: Some("death".into()), elapsed: c.dead_t, looping: false })?;
        inv.submit::<ConductedPosePort>(ConductedPose {
            pose: Some(Pose { position: pos.into(), velocity: [0.0, 0.0] }),
            side,
        })?;
        return c.store(inv);
    }

    // ── Which part of which move is live ──
    let live = match (&bird.active, &bird.telegraph) {
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
                // Its phase, from what it is asked for.
                match mv {
                    Move::Strafe => c.phase = c.phase.max(2),
                    Move::Salvo | Move::FireFan | Move::DoubleDive | Move::Lightsabers => c.phase = c.phase.max(1),
                    _ => {}
                }
                if bird.enraged {
                    c.phase = 2;
                }
                if striking {
                    c.fired = false;
                    c.shots = 0;
                    match mv {
                        Move::Dive | Move::DoubleDive => {
                            c.diving = true;
                            c.dives_left = u32::from(mv == Move::DoubleDive);
                            c.stunned = None;
                            c.returning = false;
                            aim_dive(&mut c, at, target, room);
                            play(inv, SFX_DIVE, at)?;
                        }
                        Move::Strafe => {
                            c.strafing = Some(0.0);
                            c.strafe_from_x = at.x;
                            c.strafe_from_y = at.y;
                            c.stunned = None;
                            c.returning = false;
                            play(inv, SFX_DIVE, at)?;
                        }
                        Move::Snap => c.snap_from_x = at.x,
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
    c.set_part(part);
    c.last_remaining = live.map_or(0.0, |(_, _, remaining)| remaining);

    // ── Between phases: it rears and screeches, the core blazing ──
    if bird.between_phases {
        if c.screeching.is_none() {
            c.screeches += 1;
            c.diving = false;
            c.strafing = None;
            c.stunned = None;
            c.returning = true;
            c.phase = c.phase.max(c.screeches.min(2));
            play(inv, SFX_SCREECH, at + turned(mouth(), side))?;
            shake(inv, SHAKE_SCREECH)?;
            // The first time it lights itself; the second, its fire goes cold.
            let lit = if c.phase >= 2 { COLD } else { FIRE };
            burst(inv, at + turned(core(), side), 30, 380.0, lit, "spark")?;
            burst(inv, at + turned(thruster(), side), 16, 300.0, lit, "spark")?;
            burst(inv, at + turned(mouth(), side), 16, 300.0, lit, "spark")?;
        }
        c.screeching = Some(c.screeching.map_or(0.0, |t| t + dt));
    } else {
        c.screeching = None;
    }

    // ── Where it goes ──
    let mut chomped = false;
    let mut low_pass = false;
    let pos = if c.diving {
        c.dive_t += dt;
        let from = Vec2::new(c.dive_from_x, c.dive_from_y);
        let to = Vec2::new(c.dive_to_x, c.dive_to_y);
        let travel_s = ((to - from).length() / DIVE_SPEED).max(DIVE_MIN_S);
        let u = c.dive_t / travel_s;
        side = if to.x < from.x { -1.0 } else { 1.0 };
        let pos = from.lerp(to, dive_ease(u));
        if u >= 1.0 {
            chomped = true;
            if c.dives_left > 0 {
                // It turns at the end of the first bite and comes again.
                c.dives_left -= 1;
                aim_dive(&mut c, pos, target, room);
            } else {
                c.diving = false;
                c.stunned = Some(0.0);
            }
        }
        pos
    } else if let Some(t) = c.strafing.map(|t| t + dt) {
        c.strafing = Some(t);
        let path = strafe_path(Vec2::new(c.strafe_from_x, c.strafe_from_y), room, home_x);
        match strafe_at(&path, t) {
            Some((pos, leg)) => {
                side = if leg == 3 { -1.0 } else { 1.0 };
                low_pass = leg >= 2;
                // Fire rains from it along the top of the sky.
                if leg == 1 && t >= c.shots as f32 * BOMB_EVERY_S {
                    c.shots += 1;
                    let bomb = if cold { COLD_FIRE_VISUAL } else { FIRE_VISUAL };
                    shoot(inv, pos + turned(core(), side) + Vec2::new(0.0, 40.0), Vec2::Y, 60.0, FIRE_HALF, FIRE_DAMAGE, 2.6, BOMB_GRAVITY, bomb)?;
                }
                pos
            }
            None => {
                c.strafing = None;
                c.returning = true;
                at
            }
        }
    } else if let Some(t) = c.stunned.map(|t| t + dt) {
        c.stunned = Some(t);
        if t >= STUN_S[c.stage()] {
            c.stunned = None;
            c.returning = true;
        }
        // Winded: it sinks, rocking, and its rotors stutter.
        let pos = at + Vec2::new(0.0, STUN_SINK * dt);
        Vec2::new(pos.x, pos.y.min(room.y * 0.8))
    } else if c.returning {
        let pos = glide(at, home, HOVER_SPEED * 1.3, dt);
        if (pos - home).length() <= HOME_REACHED {
            c.returning = false;
        }
        pos
    } else {
        match part {
            Some(Part { mv: Move::Snap, striking: true, t, .. }) => {
                // A short lunge at you, then back.
                let out = ease(t / SNAP_LUNGE_S) - ease((t - SNAP_LUNGE_S * 1.6) / SNAP_LUNGE_S);
                Vec2::new(c.snap_from_x + side * SNAP_LUNGE * out, glide(at, home, HOVER_SPEED, dt).y)
            }
            _ => glide(at, home, HOVER_SPEED, dt),
        }
    };
    // Home and not striking, it faces the sky it holds.
    if !c.diving && c.strafing.is_none() {
        side = 1.0;
    }

    // ── The moon strikes it ──
    // It keeps out of the moon's way, and it cannot while it dives or hangs
    // winded. Its death is its game's (`strike_it_with_the_moon`, which
    // reads `moonstruck`), not this module's.
    if let Some(moon) = c.moon().filter(|_| !c.moonstruck && (c.diving || c.stunned.is_some())) {
        let hull = pos + turned(core(), side);
        if (hull - moon).length() <= MOON_RADIUS + MOON_REACH {
            c.moonstruck = true;
            c.diving = false;
            c.stunned = Some(0.0);
            shake(inv, SHAKE_MOON)?;
            play(inv, SFX_CHOMP, hull)?;
            let between = hull + (moon - hull) * 0.5;
            burst(inv, between, 36, 460.0, MOONDUST, "shard")?;
            burst(inv, between, 28, 380.0, COLD, "spark")?;
        }
    }

    // ── Its guard: up out at its side, down when it comes in ──
    // With its fire cold no blow gets through, in any of its moves: only the
    // moon ends it.
    let open = !cold && (c.diving || c.stunned.is_some() || c.returning && c.screeching.is_none() || low_pass);
    if c.guarded == open || c.ticks == 1 {
        c.guarded = !open;
        inv.submit::<BossGuardPort>(BossGuard { guarded: c.guarded })?;
    }

    // ── The strike's own onset, and its shots ──
    if let Some(part) = part.filter(|part| part.striking) {
        let stage = c.stage();
        match part.mv {
            Move::Missile | Move::Salvo => {
                let count = if part.mv == Move::Salvo { SALVO_COUNT } else { 1 };
                if c.shots < count && part.t >= c.shots as f32 * SALVO_EVERY_S {
                    let origin = pos + turned(missile_nose(), side);
                    let aim = (target - origin).try_normalize().unwrap_or(Vec2::X);
                    let spread = if count > 1 { (c.shots as f32 - 1.0) * SALVO_SPREAD_DEG } else { 0.0 };
                    shoot(inv, origin, rotated(aim, spread), MISSILE_SPEED[stage], MISSILE_HALF, MISSILE_DAMAGE, MISSILE_LIFE_S, 0.0, MISSILE_VISUAL)?;
                    c.shots += 1;
                    play(inv, SFX_LAUNCH, origin)?;
                    shake(inv, SHAKE_LAUNCH)?;
                    burst(inv, origin - turned(Vec2::new(90.0 * PX, 0.0), side), 10, 140.0, SMOKE, "dust")?;
                    burst(inv, origin, 6, 220.0, FIRE, "spark")?;
                }
            }
            Move::Fireballs | Move::FireFan if !c.fired => {
                c.fired = true;
                let origin = pos + turned(mouth(), side);
                let aim = (target - origin).try_normalize().unwrap_or(Vec2::X);
                let fan: &[f32] = if part.mv == Move::FireFan { &[-2.0, -1.0, 0.0, 1.0, 2.0] } else { &[-1.0, 0.0, 1.0] };
                let visual = if cold { COLD_FIRE_VISUAL } else { FIRE_VISUAL };
                for k in fan {
                    shoot(inv, origin, rotated(aim, k * FAN_DEG), FIRE_SPEED, FIRE_HALF, FIRE_DAMAGE, FIRE_LIFE_S, 0.0, visual)?;
                }
                play(inv, SFX_SPIT, origin)?;
                burst(inv, origin, 14, 240.0, fire, "spark")?;
            }
            Move::Lightsabers if !c.fired => {
                // Burning lightsabers from its mouth: three, in a fan centred
                // on you. They spin out and come back the way they went.
                c.fired = true;
                let origin = pos + turned(mouth(), side);
                let aim = (target - origin).try_normalize().unwrap_or(Vec2::X);
                let visual = if cold { COLD_SABER_VISUAL } else { SABER_VISUAL };
                for k in [-1.0, 0.0, 1.0] {
                    shoot_returning(
                        inv,
                        origin,
                        rotated(aim, k * SABER_FAN_DEG),
                        SABER_SPEED,
                        SABER_HALF,
                        SABER_DAMAGE,
                        SABER_LIFE_S,
                        0.0,
                        visual,
                        Some(SABER_RETURN_S),
                    )?;
                }
                play(inv, SFX_SPIT, origin)?;
                burst(inv, origin, 18, 260.0, fire, "spark")?;
            }
            Move::Snap if !c.fired => {
                c.fired = true;
                riding(inv, bite_box(), side, BITE_DAMAGE, BITE_KNOCKBACK, part.dur * 0.7, "mockingbird_snap")?;
                play(inv, SFX_CHOMP, pos + turned(mouth(), side))?;
            }
            _ => {}
        }
    }
    if c.diving && c.ticks % 6 == 0 {
        // The ram: its whole hull hurts as it comes, its jaws ahead of it.
        let half = Vec2::new(hull_half_width() * 0.8, 70.0);
        riding(inv, (core(), half), side, RAM_DAMAGE, RAM_KNOCKBACK, 0.12, "mockingbird_ram")?;
        burst(inv, pos + turned(thruster(), side), 4, 160.0, FIRE, "spark")?;
    }
    if c.strafing.is_some() && low_pass && c.ticks % 6 == 0 {
        let half = Vec2::new(hull_half_width() * 0.8, 70.0);
        riding(inv, (core(), half), side, RAM_DAMAGE, RAM_KNOCKBACK, 0.12, "mockingbird_ram")?;
    }
    if chomped {
        riding(inv, bite_box(), side, CHOMP_DAMAGE, CHOMP_KNOCKBACK, 0.18, "mockingbird_chomp")?;
        let jaws = pos + turned(mouth(), side);
        play(inv, SFX_CHOMP, jaws)?;
        shake(inv, SHAKE_CHOMP)?;
        burst(inv, jaws, 12, 260.0, STEEL, "spark")?;
        if c.stunned.is_some() {
            play(inv, SFX_STALL, pos)?;
        }
    }
    if let Some(t) = c.stunned {
        // Smoke off the thruster and sparks off the hull while it hangs there.
        if c.ticks % 5 == 0 {
            burst(inv, pos + turned(thruster(), side), 3, 80.0, SMOKE, "dust")?;
        }
        if c.ticks % 11 == 0 && t > 0.2 {
            burst(inv, pos + turned(core(), side), 4, 200.0, FIRE, "spark")?;
        }
    }
    if c.phase >= 1 && c.ticks % 5 == 0 {
        // It burns: embers off its hull, from one end to the other in turn.
        // (The flames themselves are drawn by the view that reads its phase.)
        let along = [core(), thruster(), mouth(), missile_nose()][(c.ticks / 5 % 4) as usize];
        burst(inv, pos + turned(along, side) + Vec2::new(0.0, -30.0), 2, 120.0, fire, "spark")?;
    }

    // ── Where it is, and what it is drawn as ──
    let flying = c.diving || c.strafing.is_some() || c.returning;
    let conducts = !bird.driven || flying || c.stunned.is_some();
    if !conducts {
        side = if bird.facing < 0.0 { -1.0 } else { 1.0 };
    }
    c.pos_x = pos.x;
    c.pos_y = pos.y;
    // Held still between ticks: where it flies is this module's alone. A
    // velocity here would be integrated again, and its hull swept against
    // the flock it flies through (a shark is a solid to a body that moves).
    inv.submit::<ConductedPosePort>(ConductedPose {
        pose: conducts.then(|| Pose { position: pos.into(), velocity: [0.0, 0.0] }),
        side,
    })?;
    let row = drawn_row(&c, part, chomped);
    inv.submit::<DrawnRowPort>(DrawnRow { name: Some(row.0.into()), elapsed: row.1, looping: row.2 })?;
    c.store(inv)
}

/// Set off a dive from `from` at past where `target` is now, inside the sky.
fn aim_dive(c: &mut Conductor, from: Vec2, target: Vec2, room: Vec2) {
    let dir = (target - from).try_normalize().unwrap_or(Vec2::X);
    let to = target + dir * DIVE_OVERSHOOT;
    // Its whole hull stays in the sky: a bite at the edge must not leave it
    // half off the screen.
    let margin = Vec2::new(hull_half_width() + 8.0, 90.0);
    let to = to.clamp(margin, (room - margin).max(margin));
    c.dive_from_x = from.x;
    c.dive_from_y = from.y;
    c.dive_to_x = to.x;
    c.dive_to_y = to.y;
    c.dive_t = 0.0;
}

/// The row it is drawn with: (row, seconds in, looping).
fn drawn_row(c: &Conductor, part: Option<Part>, chomped: bool) -> (&'static str, f32, bool) {
    // (tell frames, total frames, seconds per frame): the sheet's own.
    let split = |name, tell: usize, total: usize, fd: f32, part: Part| {
        let u = (part.t / part.dur).clamp(0.0, 0.999);
        let frame = if part.striking { tell as f32 + u * (total - tell) as f32 } else { u * tell as f32 };
        (name, frame * fd, false)
    };
    if let Some(t) = c.screeching {
        // Rearing through the first frames, then the scream shudders.
        let frame = if t < 0.3 { 3.0 * t / 0.3 } else { 3.0 + ((t - 0.3) / 0.1) % 3.0 };
        return ("screech", frame * 0.1, false);
    }
    if chomped {
        return ("chomp", 0.14, false);
    }
    if c.diving {
        // The chomp's first frames as it arrives, the dive before.
        return ("dive", c.dive_t, true);
    }
    if let Some(t) = c.stunned {
        // The bite closes, then it hangs winded.
        return if t < 0.25 { ("chomp", 0.14 + t, false) } else { ("stunned", t, true) };
    }
    if c.strafing.is_some() {
        return ("thrust", c.clock, true);
    }
    if c.returning {
        return ("thrust", c.clock, true);
    }
    match part {
        Some(p @ Part { mv: Move::Missile | Move::Salvo, .. }) => split("missile", 3, 6, 0.085, p),
        Some(p @ Part { mv: Move::Fireballs | Move::FireFan | Move::Lightsabers, .. }) => split("slash", 4, 6, 0.088, p),
        Some(p @ Part { mv: Move::Snap, .. }) => split("chomp", 1, 5, 0.07, p),
        // The dive's tell: it tips over toward you, its jet spooling up. The
        // double dive's: it rears and screams first (each tell is its own
        // pose and sound, so the two read apart).
        Some(Part { mv: Move::Dive, striking: false, t, .. }) => ("dive", t.min(0.07 * 0.999), false),
        Some(p @ Part { mv: Move::DoubleDive, striking: false, .. }) => split("screech", 6, 6, 0.1, p),
        Some(Part { mv: Move::Strafe, striking: false, .. }) => ("thrust", c.clock, true),
        _ => ("rest", c.clock, true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Its sockets are where its art is: the mouth ahead of the core, the
    /// thruster behind it, the wingtip missile below and a little behind the
    /// mouth.
    #[test]
    fn its_sockets_are_where_its_art_puts_them() {
        assert!(mouth().x > 130.0 && mouth().x < 170.0, "the mouth {:?}", mouth());
        assert!(thruster().x < -90.0, "the thruster {:?}", thruster());
        assert!(missile_nose().y > 25.0, "the missile hangs under the wing: {:?}", missile_nose());
        assert!(core().length() < 2.0, "the core is the frame's centre");
    }

    /// A strafing run goes up, across the top, down the far side and back low,
    /// and ends.
    #[test]
    fn a_strafing_run_covers_the_sky_and_ends() {
        let room = Vec2::new(1600.0, 900.0);
        let path = strafe_path(Vec2::new(240.0, 400.0), room, 240.0);
        let (top, leg) = strafe_at(&path, 0.7).expect("under way");
        assert_eq!(leg, 1, "across the top");
        assert!(top.y < room.y * 0.2);
        let total: f32 = path.windows(2).map(|p| (p[1] - p[0]).length()).sum();
        let (low, leg) = strafe_at(&path, total / STRAFE_SPEED - 0.1).expect("still under way");
        assert_eq!(leg, 3, "back low");
        assert!(low.y > room.y * 0.8);
        assert!(strafe_at(&path, total / STRAFE_SPEED + 0.1).is_none(), "and it ends");
    }

    /// The moon crosses the room from the right to the left, a little above
    /// the middle, and is out of the room between two crossings. Its lane is
    /// one a dive can reach: inside the margins a dive is held to.
    #[test]
    fn the_moon_crosses_the_room_and_is_gone_between_crossings() {
        let room = Vec2::new(1280.0, 720.0);
        assert!(moon_at(MOON_FIRST_S - 0.1, room).is_none(), "the moon is there before its first crossing");
        let enters = moon_at(MOON_FIRST_S + 0.01, room).expect("entering");
        let middle = moon_at(MOON_FIRST_S + MOON_CROSS_S * 0.5, room).expect("in the middle");
        let leaves = moon_at(MOON_FIRST_S + MOON_CROSS_S - 0.01, room).expect("leaving");
        assert!(enters.x > room.x + MOON_RADIUS && leaves.x < -MOON_RADIUS, "{enters:?} to {leaves:?}");
        assert!((middle.x - room.x * 0.5).abs() < 2.0 && middle.y < enters.y, "{middle:?}");
        assert!(middle.y > 90.0 + MOON_RADIUS * 0.5 && middle.y < room.y - 90.0, "a dive cannot reach its lane: {middle:?}");
        assert!(moon_at(MOON_FIRST_S + MOON_CROSS_S + 1.0, room).is_none(), "the moon is in the room between two crossings");
        assert!(moon_at(MOON_FIRST_S + MOON_EVERY_S + 0.5, room).is_some(), "the moon does not come again");
    }

    /// A dive ends where it was aimed, inside the sky.
    #[test]
    fn a_dive_is_aimed_past_you_and_stays_in_the_sky() {
        let mut c = Conductor::default();
        let room = Vec2::new(1600.0, 900.0);
        aim_dive(&mut c, Vec2::new(240.0, 400.0), Vec2::new(900.0, 600.0), room);
        let to = Vec2::new(c.dive_to_x, c.dive_to_y);
        assert!(to.x > 900.0 && to.y > 600.0, "past where you were: {to:?}");
        aim_dive(&mut c, Vec2::new(240.0, 400.0), Vec2::new(1590.0, 890.0), room);
        let to = Vec2::new(c.dive_to_x, c.dive_to_y);
        assert!(to.x < room.x && to.y < room.y, "inside the sky: {to:?}");
        assert!((dive_ease(1.0) - 1.0).abs() < 1e-4 && dive_ease(0.0) == 0.0);
    }
}
