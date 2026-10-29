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
//! Its identity (Jon, 2026-10-22, "it just doesn't have a strong boss
//! identity or attack patterns"): a hunting dog of the regime, set free. Each
//! move is a hunting dog's verb, and each reads from its shape and its fire.
//!
//! - The stalk: between moves it keeps a hunting distance at your height and
//!   paces you.
//! - The lunge (`inu_bite`): it gathers back, jaws open, then darts straight
//!   at you and snaps. It skids past and shakes its head: the window.
//! - The pounce (`inu_dive`): it climbs over you, points its nose down and
//!   drops; a shock runs the floor both ways, and its teeth are in the floor:
//!   the long window.
//! - The storm (`inu_storm`): it was his head, and his lightning is still in
//!   it (Jon, 2026-10-24: "Lightning should still be a theme in the inugami
//!   fight"). It throws its head back, three marks spark on the floor (one
//!   under you, one each side), and lightning strikes each from the vault.
//! - The howl: it throws its head back, and a ring pushes you off it. From
//!   the transition on, lightning answers the howl on each side of it.
//! - The broken leash (`inu_leash`, its signature): the stitches at its neck
//!   burn and a ghost chain unrolls; its tip cracks along the floor from
//!   behind it to in front of it: jump it as it passes you. Then the chain
//!   falls slack and it pants: a window.
//! - The gutter: its fire is spent once in a long cycle. It sinks to the
//!   floor and lies there, dim, until its next tell kindles it again.
//!
//! ⭐ THE WARD AND THE CHASE (Jon, 2026-10-24). It has no ghost fire round
//! it in its first phase: it is hit as any boss. At its transition the blue
//! hitodama kindle round it (`warded`): while they burn it cannot be hurt,
//! and they are what make you run. It crashes through the arena's wall and
//! hunts you down the drop and west under the castle to the final hall, the
//! room's end farthest from where it rose. There Kagero comes to your aid
//! (`crate::kagero`): her spell breaks the ward, and you fight it there.
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
    BossSummon, BossSummonPort, ConductedPose, ConductedPosePort, DrawnRow, DrawnRowPort, Pose, Predecessor, SummonSide,
};
use ambition_combat_port::{
    BodySound, BodySoundPort, Burst, BurstPort, CameraShake, CameraShakePort, HeldDamageBox, HeldDamageBoxPort, HeldRay,
    HeldRayPort, Lightning, LightningPort, RidingHitbox, RidingHitboxPort, RidingKnockback, Spot,
};
use ambition_extension_sdk::{
    phases::BOSS_CONDUCT, record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};
use bevy_math::Vec2;

use crate::wolfenstein;
mod chase;

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

    /// Its neck, where his stitches were: the leash comes out of it.
    fn neck(self) -> Vec2 {
        self.art(-4.0, 24.0)
    }
}

/// Where a body of the predecessor's dead head is, and how big its head is
/// drawn there: the Inugami rises from it at that size.
fn dead_head_of(p: &Predecessor) -> (Vec2, f32) {
    let body = wolfenstein::Body::of_height(p.size[1]);
    let off = body.dead_head();
    (Vec2::from(p.position) + Vec2::new(off.x * p.side, off.y), body.head_px())
}

/// THE STALK. Between moves it keeps a hunting distance at your height: this
/// far to your side (head pixels: a giant head keeps a giant's distance), its
/// bottom `STALK_CLEAR` over your feet, and it paces you, `STALK_PACE` head
/// pixels either way at `STALK_PACE_HZ`.
const STALK_DX: f32 = 80.0;
const STALK_CLEAR: f32 = 24.0;
const STALK_PACE: f32 = 14.0;
const STALK_PACE_HZ: f32 = 0.35;
const HOVER_SPEED: f32 = 150.0;
const HOVER_SPEED_FRENZIED: f32 = 210.0;
/// The top of the room it flies to, over its floor.
const CEILING: f32 = 740.0;
/// Its room's side walls.
const WALL: f32 = 16.0;

/// It dives from this high over you (world units), or the top of the room.
const DIVE_HIGH: f32 = 440.0;
/// THE FINAL HALL: the `FINAL_W` at the end of its room farthest from where
/// it rose, where the fight is finished (phase 2 and the enrage).
const FINAL_W: f32 = 1600.0;
/// Its transition: it howls and its ward kindles round it for `FLARE_S`,
/// then it crashes through the arena's wall on the way out (Jon, 2026-10-22:
/// "maybe when he enrages he crashes into a wall so you can do the run and
/// chase"): the wall of its arena farther from where it rose. It goes at the
/// wall at `CRASH_SPEED`, and the wall breaks when the encounter's second
/// phase starts (the wall's `boss.past_first_phase`). Then it draws back off
/// the wall, `RECOIL` and up to the vault, so you can get under it to the gap.
const FLARE_S: f32 = 0.9;
const CRASH_SPEED: f32 = 1300.0;
const RECOIL: f32 = 320.0;
const RECOIL_SPEED: f32 = 520.0;
/// The pursuit follows the target's route. Its speed follows the target's
/// measured pace, with a continuous correction toward this separation.
/// A stopped body is caught; no camera state affects pursuit.
const RUN_GAP: f32 = 220.0;
/// It hunts over the floor, its bottom this far over your feet: a head of
/// its size that kept to your height dragged along the floor and was held
/// on a 48 px step at the throne room's door (measured, 2026-10-22), and at
/// 64 over your feet it was held on the run's 80 px barricades (measured,
/// 2026-10-24). The run is 640 clear.
const RUN_CLEAR: f32 = 120.0;
const RUN_FAR: f32 = 520.0;
const RUN_BEAT_S: f32 = 2.4;
/// Its lunge in the run: it gathers this long, then darts this long past you.
const RUN_GATHER_S: f32 = 0.45;
const RUN_DART_S: f32 = 0.35;
/// It howls over the final hall for this long when it comes into it.
const ARRIVE_S: f32 = 1.1;
/// Its lightning on the run: a mark this far ahead of you the way you run,
/// sparking for `RUN_BOLT_TELL_S`, then a strike out of the air from
/// `RUN_BOLT_HIGH` over your feet down to the floor.
const RUN_BOLT_AHEAD: f32 = 300.0;
const RUN_BOLT_TELL_S: f32 = 0.7;
const RUN_BOLT_HIGH: f32 = 460.0;
/// THE SCENT. It hunts you along where you went, not in a straight line at
/// you: a straight line held it 10 s in the stair tower's top corner, under
/// the battlements' floor (measured, 2026-10-22). It is a hunting dog. A point
/// of your feet every `SCENT_STEP` you go, the last `SCENT_MAX` of them; it
/// has a point when it is within `SCENT_REACH` of it.
const SCENT_STEP: f32 = 48.0;
const SCENT_MAX: u32 = 96;
const SCENT_REACH: f32 = 40.0;

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

/// THE STORM, in place of the pack (Jon, 2026-10-24: no ghost fire round it
/// but in the chase, and lightning stays the fight's theme): three marks on
/// the floor, one under you and one `MARK_SPREAD` each side, spark through
/// its tell; then lightning strikes each from `STORM_HIGH` over the floor (a
/// ray down to the floor, `BOLT_WIDTH` wide) for `BOLT_S`.
const MARK_SPREAD: f32 = 240.0;
const STORM_HIGH: f32 = 760.0;
const BOLT_WIDTH: f32 = 48.0;
const BOLT_DAMAGE: i32 = 2;
const BOLT_KNOCKBACK: f32 = 1.4;
const BOLT_S: f32 = 0.3;

/// THE BROKEN LEASH, its signature: a ghost chain from its neck, its tip
/// `LEASH_REACH` of its body heights out, cracks along the floor from behind
/// it to in front of it. Only the tip hurts (`LEASH_TIP_HALF`): the chain over
/// it is fire, drawn. Then it falls slack for `LEASH_SLACK_S`, and the head
/// pants: a window.
const LEASH_REACH: f32 = 2.2;
const LEASH_TIP_HALF: Vec2 = Vec2::new(34.0, 20.0);
const LEASH_DAMAGE: i32 = 2;
const LEASH_KNOCKBACK: f32 = 1.3;
const LEASH_LINKS: u32 = 9;
const LEASH_SLACK_S: f32 = 1.2;

/// The pounce's shock along the floor, both ways from where its teeth go in.
const SHOCK_SPEED: f32 = 420.0;
const SHOCK_HALF: Vec2 = Vec2::new(22.0, 16.0);
const SHOCK_DAMAGE: i32 = 1;
const SHOCK_KNOCKBACK: f32 = 1.1;
const SHOCK_LIFETIME_S: f32 = 4.0;
/// Held damage box and ray slots: the two shocks, the leash's tip, the
/// storm's three bolts, the run's bolt.
const SLOT_SHOCK: [u32; 2] = [0, 1];
const SLOT_LEASH: u32 = 2;
const SLOT_STORM: [u32; 3] = [3, 4, 5];
const SLOT_RUN_BOLT: u32 = 6;

const HOWL_DAMAGE: i32 = 1;
const HOWL_KNOCKBACK: f32 = 2.2;
/// The ring's radius, in its body heights.
const HOWL_RADIUS: f32 = 0.9;

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

/// Kagero comes to your aid when it has howled over the final hall and you
/// are in it (Jon, 2026-10-24: "The ninja is who dispels the flames and lets
/// you fight once you reach the end of the hall"; `crate::kagero`). She comes
/// beside you, `ALLY_BESIDE` toward it, `ALLY_OVER` over your middle, and no
/// nearer a wall of the hall than `ALLY_WALL`: her lines are drawn over her,
/// in view. Her spell holds it from when she comes; it breaks its ward at
/// `kagero::DISPEL_AT_S` and shakes itself free over `DISPEL_S`.
const ALLY_BESIDE: f32 = 70.0;
const ALLY_OVER: f32 = 30.0;
const ALLY_WALL: f32 = 220.0;
const DISPEL_S: f32 = 0.9;
/// She comes when it has stalked you to within this across (world px): her
/// spell, its ward breaking and you are in one picture.
const ALLY_NEAR: f32 = 400.0;
/// Her health: she does not stay long enough to fall.
const ALLY_HEALTH: u32 = 200;

/// Its fire, and dust.
const FIRE: [f32; 4] = [1.0, 0.36, 0.34, 1.0];
const FIRE_CORE: [f32; 4] = [1.0, 0.88, 0.84, 1.0];
/// Its hitodama are blue: cold fire about the red head (Jon, 2026-10-20).
const WARD_FIRE: [f32; 4] = [0.45, 0.75, 1.0, 1.0];
/// Its lightning: his own colours.
const SPARK: [f32; 4] = [1.0, 0.93, 0.72, 1.0];
const ARC: [f32; 4] = [0.66, 0.84, 1.0, 1.0];
const DUST: [f32; 4] = [0.74, 0.70, 0.66, 1.0];

/// Its howl, a described one (the SFX renderer's `howl` mode), at three
/// sizes: lower as it grows (Jon, 2026-10-21: "pitch shift them down for the
/// inugami especially as it grows in size"). The pitch is over its size: 520
/// Hz at its build, 452 at 1.15x, 400 at 1.3x.
const SFX_HOWLS: [(f32, &str); 3] =
    [(1.08, "boss.inugami.howl"), (1.22, "boss.inugami.howl_large"), (f32::INFINITY, "boss.inugami.howl_giant")];

/// Every cue it plays: each must have a recipe, or it plays silence.
pub const SOUNDS: [&str; 15] = [
    SFX_HOWLS[0].1, SFX_HOWLS[1].1, SFX_HOWLS[2].1, SFX_BITE, SFX_DART, SFX_SLAM, SFX_WHIP, SFX_GUTTER,
    SFX_CATCH, SFX_STING, SFX_SHOCK, SFX_PANT, SFX_ZAP, SFX_STRIKE, SFX_WARD_BREAK,
];

/// The theremin of his march, as the seam round his neck pops: the cue that
/// something is wrong with the head (Jon, 2026-10-22). Iron Toccata comes in
/// when the head tears off (`music_intro_from` of its encounter).
const SFX_STING: &str = "boss.inugami.unsewn_sting";
/// The first stitch pops this long into his `unsewn` row (frame 4 of 14, 80 ms).
const FIRST_STITCH: f32 = 0.28;

/// The howl for a head built at `scale` times its build.
pub fn howl_cue(scale: f32) -> &'static str {
    SFX_HOWLS.iter().find(|(below, _)| scale < *below).map_or(SFX_HOWLS[2].1, |(_, cue)| cue)
}
const SFX_BITE: &str = "boss.trex.chomp";
const SFX_DART: &str = "enemy.burning_flying_shark.flame_flyby";
const SFX_SLAM: &str = "boss.trex.stomp";
/// His lightning: a mark that sparks zaps, a bolt that lands strikes.
const SFX_ZAP: &str = "boss.wolfenstein.zap";
const SFX_STRIKE: &str = "boss.wolfenstein.strike";
/// Its ward breaks under her spell.
const SFX_WARD_BREAK: &str = "vfx.generic_world.ice_shatter";
const SFX_WHIP: &str = "boss.trex.tail_whip";
const SFX_SHOCK: &str = "vfx.explosion.shockwave";
const SFX_PANT: &str = "boss.trex.growl_chuff_a";
const SFX_GUTTER: &str = "ability.fire.power_down";
const SFX_CATCH: &str = "ability.fire.release";

/// Its moves: the `Special` keys of its pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    /// The lunge.
    Bite,
    /// The pounce.
    Dive,
    /// Lightning on three marks.
    Storm,
    Howl,
    Leash,
    Gutter,
}

impl Move {
    pub const ALL: [Move; 6] = [Move::Bite, Move::Dive, Move::Storm, Move::Howl, Move::Leash, Move::Gutter];

    pub fn key(self) -> &'static str {
        match self {
            Move::Bite => "inu_bite",
            Move::Dive => "inu_dive",
            Move::Storm => "inu_storm",
            Move::Howl => "inu_howl",
            Move::Leash => "inu_leash",
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
    pub struct Conductor = SchemaKey::new(crate::PROVIDER, "inugami.conductor", 5);
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
    /// Its howl when it comes into the final hall: seconds into it.
    33 appear: Option<f32>,
    /// Kagero: whether it has called her (once).
    35 ally_called: bool,
    /// The run to the final hall after its transition: seconds into it.
    36 run: Option<f32>,
    /// It is in the final hall: it ran you there, on its own wings. Until
    /// then it flies the whole room.
    37 arrived: bool,
    /// Its crash into the arena's wall has hit the wall.
    38 crashed: bool,
    /// Your scent while it hunts you: where your feet were, a point every
    /// `SCENT_STEP`, oldest first. It follows these points in order.
    39 scent: Vec<[f32; 2]> [max SCENT_MAX],
    /// Where the run's bolt will strike, while it sparks: its x, its floor and
    /// seconds into its tell.
    40 run_bolt_x: f32,
    /// Its leash fallen slack, it pants: seconds into it.
    41 slack: Option<f32>,
    /// The pounce's shocks rolling the floor (their x), and the floor's line.
    42 shock_left: Option<f32>,
    43 shock_right: Option<f32>,
    44 shock_floor: f32,
    /// The generation of its held damage boxes (a new shock, a new whip).
    45 box_generation: u32,
    /// Its lunge in the run: seconds into it (gather, dart, skid).
    46 lunge: Option<f32>,
    /// The beats of its run: lunges and bolts, in turn.
    47 run_beats: u32,
    /// The way you run (-1 or 1), from your scent: it hunts on that side of
    /// you.
    48 run_dir: f32,
    /// Its ward of blue fire is lit: no blow hurts it.
    49 warded: bool,
    /// Kagero's spell holds it: seconds since she came.
    50 binding: Option<f32>,
    /// The storm's marks on the floor (their x), and the floor's line.
    51 marks: Vec<f32> [max 3],
    52 marks_floor: f32,
    /// The generation of its held rays (a new storm, a new run bolt).
    53 bolt_generation: u32,
    /// This transition crashes out of the arena (the first; a later one only
    /// rears where it is).
    54 rear_crash: bool,
    /// The run's bolt: its floor, and seconds into its tell.
    55 run_bolt_floor: f32,
    56 run_bolt: Option<f32>,
    /// The ward's own clock: seconds since it kindled, while it burns and
    /// while it is blown out. Its spirits circle on it, whatever row the head
    /// is drawn in (`ward_layer`).
    57 ward_t: Option<f32>,
    /// The side it was drawn facing, and seconds into its turn round when it
    /// turns (`turned`).
    58 drawn_side: f32,
    59 turning: Option<f32>,
    /// The target's last feet, measured pace and route direction.
    60 run_last_feet: Option<[f32; 2]>,
    61 run_pace: f32,
    62 run_forward: [f32; 2],
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

    /// It has called Kagero to your aid, for tests and inspectors.
    pub fn ally_called(&self) -> bool {
        self.ally_called
    }

    /// Its leash fallen slack (the window after the whip), for tests and
    /// inspectors.
    pub fn slack(&self) -> bool {
        self.slack.is_some()
    }

    /// Its ward of blue fire is lit (no blow hurts it), for tests and
    /// inspectors.
    pub fn warded(&self) -> bool {
        self.warded
    }

    /// It hunts you to the final hall, for tests and inspectors.
    pub fn running(&self) -> bool {
        self.run.is_some()
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
                HeldDamageBoxPort::KEY,
                HeldRayPort::KEY,
                LightningPort::KEY,
                BodySoundPort::KEY,
                BossGuardPort::KEY,
                BossScalePort::KEY,
                BossStagePort::KEY,
                BossSummonPort::KEY,
            ],
            after: Vec::new(),
            limits: Limits { max_requests: 64 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(conduct),
        }],
    }
}

/// `v` within `lo..=hi`, or their middle when the room is too small for the
/// body (`lo > hi`): a body pushed where its bounds cross, as a head drawn
/// after you through the arena's floor, is not a fault (`f32::clamp` panics).
fn within(v: f32, lo: f32, hi: f32) -> f32 {
    if lo > hi {
        (lo + hi) * 0.5
    } else {
        v.clamp(lo, hi)
    }
}

/// A flash of `peak` that dies away over a quarter of a second, `t` after it,
/// and is out at the end of [`FLASH_S`].
fn flash_after(t: f32, peak: f32) -> f32 {
    if t >= FLASH_S {
        0.0
    } else {
        peak * (-t / 0.25).exp()
    }
}
const FLASH_S: f32 = 0.8;

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

/// Lightning out of the air at `x`, from `high` over the floor's line
/// `floor` down to the floor: a ray in `slot` that hurts for `life`, drawn as
/// a strike, and the sparks where it lands.
fn bolt_down(inv: &mut Invocation<'_>, slot: u32, generation: u32, x: f32, floor: f32, high: f32, life: f32) -> Result<(), Fault> {
    bolt_down_with_launch(inv, slot, generation, x, floor, high, life, None)
}

fn bolt_down_with_launch(inv: &mut Invocation<'_>, slot: u32, generation: u32, x: f32, floor: f32, high: f32, life: f32,
    launch_dir: Option<ambition_combat_port::LaunchDirection>) -> Result<(), Fault> {
    inv.submit::<HeldRayPort>(HeldRay {
        slot,
        generation,
        from: Spot::World([x, floor - high]),
        toward: [0.0, 1.0],
        reach: high + 8.0,
        width: BOLT_WIDTH,
        damage: BOLT_DAMAGE,
        knockback: BOLT_KNOCKBACK,
        launch_dir,
        lifetime_s: life,
        look: "strike".into(),
    })?;
    burst(inv, Vec2::new(x, floor - 4.0), 14, 300.0, SPARK, "spark")
}

/// A mark on the floor where its lightning will strike: sparks, and an arc
/// that does not hurt from over it.
fn mark(inv: &mut Invocation<'_>, x: f32, floor: f32, k: u32, ticks: u32) -> Result<(), Fault> {
    if ticks % 3 == k % 3 {
        let across = ((ticks / 3 + k * 5) % 5) as f32 / 4.0 * 2.0 - 1.0;
        burst(inv, Vec2::new(x + across * BOLT_WIDTH * 0.5, floor - 3.0), 3, 90.0, ARC, "spark")?;
    }
    Ok(())
}

/// A strike that shows and does not hurt: from `from` to `to`.
fn arc(inv: &mut Invocation<'_>, key: &str, from: Vec2, to: Vec2, hold: f32) -> Result<(), Fault> {
    inv.submit::<LightningPort>(Lightning {
        key: key.into(),
        from: Spot::World(from.into()),
        to: Spot::World(to.into()),
        look: "spark_gap".into(),
        glow: None,
        hold_s: hold,
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

/// Put it at `pos`, facing `side`, drawn with `row`, and its ward's `layer`
/// over it (or none).
#[allow(clippy::too_many_arguments)]
fn held_at(
    inv: &mut Invocation<'_>,
    at: Vec2,
    pos: Vec2,
    side: f32,
    row: (&'static str, f32, bool),
    layer: Option<(&'static str, f32, bool)>,
    dt: f32,
) -> Result<(), Fault> {
    inv.submit::<ConductedPosePort>(ConductedPose {
        pose: Some(Pose { position: pos.into(), velocity: ((pos - at) / dt.max(1e-4)).into() }),
        side: Some(side),
    })?;
    inv.submit::<DrawnRowPort>(DrawnRow::body(row.0, row.1, row.2))?;
    let (names, elapsed, looping) = layer.map_or((Vec::new(), 0.0, false), |(name, t, looping)| (vec![name.into()], t, looping));
    inv.submit::<DrawnRowPort>(DrawnRow { on: ambition_boss_special_port::DrawnPart::Layer, names, elapsed, looping })
}

/// How long its spirits take to go round once (`wolfenstein_inugami`'s
/// `SPIRIT_TURN_S`), and the beat of the ring: its four spirits are where
/// they were each quarter turn.
const RING_TURN_S: f32 = 1.12;
const RING_BEAT_S: f32 = RING_TURN_S / 4.0;

/// Its ward's spirits, a layer over its row on the ward's own clock: they
/// catch while it kindles, circle while it burns, and are blown out when
/// Kagero's spell breaks it. Each ring row starts its spirits where the
/// last left them, so they circle on through every row the head is drawn
/// in (Jon, 2026-10-25: the ring "jumping where one warded row hands over
/// to the next").
fn ward_layer(c: &Conductor) -> Option<(&'static str, f32, bool)> {
    let w = c.ward_t?;
    let at_dispel = crate::kagero::DISPEL_AT_S;
    if let Some(b) = c.binding.filter(|b| *b >= at_dispel) {
        // Its spirits were on a beat when she cast (she is called on one):
        // the row starts them there, and its own clock is past it by as much
        // as the ward's was.
        let past_beat = (w - b + at_dispel).rem_euclid(RING_BEAT_S);
        return Some(("ward_ring_dispel", past_beat + b - at_dispel, false));
    }
    match c.rearing {
        Some(t) if c.rear_crash && t < FLARE_S => Some(("ward_ring_kindle", w, false)),
        _ if c.warded => Some(("ward_ring", w, true)),
        _ => None,
    }
}

/// How long it takes to turn round (`turn`, 8 x 30 ms).
const TURN_S: f32 = 0.24;

/// The rows it turns round in: where it floats, hunts or draws up to a move.
/// Anywhere else (a strike, a crash, its spell) it faces the new way at once.
fn turns_in(row: &str) -> bool {
    matches!(row, "idle" | "drift" | "hunt" | "bite_tell" | "dive_tell" | "storm_tell" | "howl_tell" | "leash_tell")
}

/// `row`, or its turn round when it has just come to face the other way
/// (Jon, 2026-10-25: it flipped). `facing`: the side it is drawn facing
/// this tick.
fn turned(c: &mut Conductor, facing: f32, row: (&'static str, f32, bool), dt: f32) -> (&'static str, f32, bool) {
    if c.drawn_side != 0.0 && facing != c.drawn_side && turns_in(row.0) {
        c.turning = Some(0.0);
    }
    c.drawn_side = facing;
    match c.turning {
        Some(t) if t < TURN_S && turns_in(row.0) => {
            c.turning = Some(t + dt);
            ("turn", t, false)
        }
        _ => {
            c.turning = None;
            row
        }
    }
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
    let at = Vec2::from(inu.position);
    let mut side = inu.side;
    let built = Head::of_height(inu.size[1] / c.scale.max(0.1));
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
    // The final hall: the end of its room farthest from where it rose.
    let toward_final = if c.rise_x < room_w * 0.5 { 1.0 } else { -1.0 };
    let (final_lo, final_hi) = if toward_final > 0.0 { (room_w - FINAL_W, room_w - WALL) } else { (WALL, FINAL_W) };
    let in_final_hall = |x: f32| x >= final_lo && x <= final_hi;
    // The run crosses the room; then the hall holds it. ⛔ Not "it is
    // frenzied and not running": it is frenzied from the first tick of its
    // transition, before its run, and the hall's bounds put it 7000 px down
    // the castle on that tick (Jon, 2026-10-22: "The head still teleports to
    // the other side").
    let in_final = c.arrived;
    let (lo, hi) = if in_final { (final_lo, final_hi) } else { (WALL, room_w - WALL) };
    let hall = Hall { floor, left: lo, right: hi };
    let half = Vec2::new(inu.size[0], inu.size[1]) * 0.5;
    // Chasing you it flies the whole room, up the tower too; in the final
    // hall it keeps under the hall's sky.
    let top = if in_final { (hall.floor - CEILING).max(48.0) } else { 48.0 } + half.y;
    let ground = hall.floor - h.lying_low();
    c.clock += dt;
    c.ticks = c.ticks.wrapping_add(1);
    let target = inu.target_body.as_ref().map(|body| body.position).or(inu.target).map_or(Vec2::new((hall.left + hall.right) * 0.5, hall.floor - 24.0), Vec2::from);
    let bottom = if c.run.is_some() {
        inu.room.map_or(hall.floor, |room| room[1])
    } else {
        hall.floor
    };
    let clamp = |p: Vec2| {
        Vec2::new(
            within(p.x, hall.left + half.x + 8.0, hall.right - half.x - 8.0),
            within(p.y, top, bottom - half.y - 8.0),
        )
    };
    let face = |x: f32| if target.x < x { -1.0 } else { 1.0 };
    // The arena's wall on its way out, as it is measured under it now (the
    // gate stands in it until the second phase): the one farther from where
    // it rose. It crashes into it, its face against the wall.
    let walls = inu.hall.map_or((hall.left, hall.right), |w| (w.left, w.right));
    let crash_dir = if walls.1 - c.rise_x > c.rise_x - walls.0 { 1.0 } else { -1.0 };
    let crash_wall = if crash_dir > 0.0 { walls.1 } else { walls.0 };
    let crash_at = |at: Vec2| Vec2::new(crash_wall - crash_dir * half.x, at.y);
    // Your feet, under the box it tracks.
    let feet = inu.target_body.as_ref().map_or(target.y + 24.0, |b| b.position[1] + b.size[1] * 0.5);
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
            play(inv, howl_cue(c.scale), at)?;
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
        let x = if lived { at.x } else { within(rise_from.x + side * h.len(30.0), hall.left + half.x, hall.right - half.x) };
        let from_y = if lived { c.from_y } else { ground };
        let u = ((t / DEATH_S - DEATH_FALL.0) / (DEATH_FALL.1 - DEATH_FALL.0)).clamp(0.0, 1.0);
        // Landed, it lies where its body came to rest. Held on the floor's line
        // it shook: each tick the floor put it back up 4 px and the pose put it
        // down again (Jon, 2026-10-21: "The head also shakes violently after
        // you do kill it").
        let y = if t / DEATH_S < DEATH_FALL.0 {
            if lived { at.y } else { ground }
        } else if u >= 1.0 && t - dt >= DEATH_FALL.1 * DEATH_S {
            at.y
        } else {
            from_y + (ground - from_y) * u * u
        };
        let lit = SPIRIT_LIT + (1.0 - SPIRIT_LIT) * ease((t - DEATH_LEAVES_U * DEATH_S) / 2.0);
        inv.submit::<BossStagePort>(BossStage { lit: if lived { lit } else { 1.0 }, flash: 0.0, card: 0.0 })?;
        c.dead_for = Some(t);
        c.warded = false;
        c.ward_t = None;
        held_at(inv, at, Vec2::new(x, y), side, ("death", t, false), None, dt)?;
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
    // While it runs you to the hall, howls over it and waits there warded
    // for Kagero, and while her spell holds it, its pattern waits. (Its
    // pattern let in between them started a pounce that her spell cut off,
    // and the pounce's window put it 196 px down in one tick: measured
    // 2026-10-24.)
    let live = match (inu.active.as_ref(), inu.telegraph.as_ref()) {
        _ if c.run.is_some() || c.appear.is_some() || c.binding.is_some() || (c.arrived && c.warded) => None,
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
                    c.slack = None;
                    side = face(at.x);
                    if mv == Move::Dive {
                        // Over you, near the top of the room.
                        let over = clamp(Vec2::new(target.x, (target.y - DIVE_HIGH).max(top)));
                        c.to_x = over.x;
                        c.to_y = over.y;
                    }
                    if mv == Move::Storm {
                        // One mark under you, one each side.
                        let x = within(target.x, hall.left + 40.0, hall.right - 40.0);
                        c.marks = [x - MARK_SPREAD, x, x + MARK_SPREAD]
                            .into_iter()
                            .map(|m| within(m, hall.left + 40.0, hall.right - 40.0))
                            .collect();
                        c.marks_floor = hall.floor;
                    }
                    // Its tell's own sound is its pattern's (`telegraph.cue`).
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
                        Move::Storm => {
                            c.bolt_generation = c.bolt_generation.wrapping_add(1);
                        }
                        Move::Leash => {
                            // A new whip: its tip is a new held box.
                            c.box_generation = c.box_generation.wrapping_add(1);
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
                Move::Leash => {
                    c.slack = Some(0.0);
                    play(inv, SFX_PANT, at)?;
                }
                _ => {}
            }
        }
    }
    c.set_part(part);
    c.last_remaining = live.map_or(0.0, |(_, _, remaining)| remaining);

    // ── Between phases: it howls; the first time its ward kindles ──
    if inu.between_phases {
        if c.rearing.is_none() {
            side = face(at.x);
            c.miss = None;
            c.buried = None;
            c.guttered = None;
            c.slack = None;
            // The first transition crashes out of the arena; a later one
            // only rears where it is.
            c.rear_crash = !c.frenzied;
            c.frenzied = true;
            play(inv, howl_cue(c.scale), at)?;
            burst(inv, at, 40, 360.0, FIRE, "spark")?;
            shake(inv, SHAKE_HOWL)?;
            inv.submit::<BossStagePort>(BossStage { lit: SPIRIT_LIT, flash: 0.4, card: 0.0 })?;
            if c.rear_crash {
                // Its ward kindles round it: the blue fire that makes you run.
                c.warded = true;
                c.ward_t = Some(0.0);
                inv.submit::<BossGuardPort>(BossGuard { guarded: true })?;
                play(inv, SFX_CATCH, at)?;
            }
        }
        let t = c.rearing.map_or(0.0, |t| t + dt);
        // The flash of its howl dies away (a stage's flash holds until it is
        // set again: held, it washed the picture white for the whole
        // transition).
        inv.submit::<BossStagePort>(BossStage { lit: SPIRIT_LIT, flash: flash_after(t, 0.4), card: 0.0 })?;
        if c.rear_crash && c.ticks % 4 == 0 && t < FLARE_S {
            burst(inv, at, 4, 220.0, WARD_FIRE, "spark")?;
        }
        if t >= FLARE_S && t - dt < FLARE_S {
            burst(inv, at, 30, 320.0, FIRE_CORE, "spark")?;
        }
        // It meets the arena's wall: stone and fire, and a slam.
        if c.rear_crash && t >= FLARE_S && !c.crashed && (crash_at(at) - at).x.abs() < 2.0 {
            c.crashed = true;
            let face = Vec2::new(at.x + crash_dir * half.x, at.y);
            play(inv, SFX_SLAM, face)?;
            burst(inv, face, 40, 380.0, DUST, "dust")?;
            burst(inv, face, 24, 300.0, WARD_FIRE, "spark")?;
            shake(inv, SHAKE_DIVE * 1.5)?;
        }
        c.rearing = Some(t);
    } else if c.rearing.take().is_some() {
        inv.submit::<BossStagePort>(BossStage { lit: SPIRIT_LIT, flash: 0.0, card: 0.0 })?;
        if c.rear_crash {
            // The wall is down (the second phase): it hunts you through the
            // gap, the way it crashed.
            c.run = Some(0.0);
            c.run_dir = crash_dir;
            c.run_forward = [crash_dir, 0.0];
            c.run_last_feet = None;
            c.run_pace = 0.0;
            c.scent.clear();
        }
    }

    let pursuit = c.run.map(|_| {
        chase::Trail {
            points: &mut c.scent,
            last_feet: &mut c.run_last_feet,
            pace: &mut c.run_pace,
            forward: &mut c.run_forward,
        }.advance(at, Vec2::new(target.x, feet), RUN_CLEAR + half.y, dt, clamp)
    });
    if let Some(step) = &pursuit {
        if step.forward.x.abs() > 0.2 {
            c.run_dir = step.forward.x.signum();
        }
    }

    // ── The run: it hunts you to the final hall ──
    // It comes into the hall on its own wings, after you: if you are there
    // first, it is still hunting you there (Jon, 2026-10-22: "if you get to the
    // final arena before he does he just teleports there, that is no good").
    let head_home = at.x - half.x >= final_lo && at.x + half.x <= final_hi;
    if let Some(t) = c.run {
        let t = t + dt;
        if head_home {
            // In the hall: it howls over it, and the fight goes on there.
            c.run = None;
            c.lunge = None;
            c.run_bolt = None;
            c.arrived = true;
            c.appear = Some(0.0);
            play(inv, howl_cue(c.scale), at)?;
            burst(inv, at, 30, 320.0, WARD_FIRE, "spark")?;
        } else {
            // On your heels, a beat every `RUN_BEAT_S`: a lunge at your back,
            // or lightning on the way ahead of you, in turn.
            let on_heels = pursuit.as_ref().is_some_and(|step| step.distance < RUN_FAR);
            let beat = (t / RUN_BEAT_S).floor() != ((t - dt) / RUN_BEAT_S).floor();
            if c.lunge.is_none() && c.run_bolt.is_none() && on_heels && beat {
                if c.run_beats % 2 == 0 {
                    c.lunge = Some(0.0);
                    c.from_x = at.x;
                    c.from_y = at.y;
                } else {
                    c.run_bolt = Some(0.0);
                    c.run_bolt_x = target.x + c.run_dir * (c.run_pace * 0.25).min(RUN_BOLT_AHEAD);
                    c.run_bolt_floor = feet;
                    play(inv, SFX_ZAP, Vec2::new(c.run_bolt_x, feet))?;
                }
                c.run_beats = c.run_beats.wrapping_add(1);
            }
            if let Some(l) = c.lunge {
                let l = l + dt;
                let was = l - dt;
                if l >= RUN_GATHER_S && was < RUN_GATHER_S {
                    // It darts past where you are now.
                    let from = Vec2::new(c.from_x, c.from_y);
                    let aim = (target - from).normalize_or(Vec2::X * c.run_dir);
                    let to = clamp(from + aim * ((target - from).length() + 120.0).min(BITE_REACH));
                    c.to_x = to.x;
                    c.to_y = to.y;
                    inv.submit::<RidingHitboxPort>(RidingHitbox {
                        offset: [0.0, 0.0],
                        half_extent: half.into(),
                        circle_radius: None,
                        damage: BITE_DAMAGE,
                        knockback: RidingKnockback::LaunchSpeed { base: 420.0, growth: None },
                        launch_dir: Some(ambition_combat_port::LaunchDirection::World(
                            chase::escape_launch(Vec2::from(c.run_forward)).into(),
                        )),
                        lifetime_s: RUN_DART_S,
                        name: "Inugami ghost fire".into(),
                    })?;
                    play(inv, SFX_DART, at)?;
                    play(inv, SFX_BITE, at)?;
                }
                // Its ward stays lit through the skid: in the chase nothing
                // hurts it, and you run.
                c.lunge = if l >= RUN_GATHER_S + RUN_DART_S + MISS_S { None } else { Some(l) };
            }
            if let Some(b) = c.run_bolt {
                let b = b + dt;
                if b < RUN_BOLT_TELL_S {
                    mark(inv, c.run_bolt_x, c.run_bolt_floor, 0, c.ticks)?;
                } else if b < RUN_BOLT_TELL_S + BOLT_S {
                    if b - dt < RUN_BOLT_TELL_S {
                        c.bolt_generation = c.bolt_generation.wrapping_add(1);
                        play(inv, SFX_STRIKE, Vec2::new(c.run_bolt_x, c.run_bolt_floor))?;
                        shake(inv, SHAKE_HOWL)?;
                    }
                    bolt_down_with_launch(inv, SLOT_RUN_BOLT, c.bolt_generation, c.run_bolt_x, c.run_bolt_floor, RUN_BOLT_HIGH, RUN_BOLT_TELL_S + BOLT_S - b,
                        Some(ambition_combat_port::LaunchDirection::World(chase::escape_launch(Vec2::from(c.run_forward)).into())))?;
                }
                c.run_bolt = if b >= RUN_BOLT_TELL_S + BOLT_S { None } else { Some(b) };
            }
            if c.ticks % 6 == 0 {
                burst(inv, at, 3, 200.0, WARD_FIRE, "spark")?;
            }
            c.run = Some(t);
        }
    }

    // ── Kagero comes to your aid in the final hall: her spell breaks its ward ──
    let near = (at.x - target.x).abs() < ALLY_NEAR;
    // She is called on a beat of the ward's ring, so her spell breaks it with
    // its spirits where its `ward_ring_dispel` row starts them.
    let on_beat = c.ward_t.is_none_or(|w| (w + crate::kagero::DISPEL_AT_S).rem_euclid(RING_BEAT_S) < dt);
    if c.arrived && c.warded && !c.ally_called && c.appear.is_none() && in_final_hall(target.x) && near && on_beat {
        c.ally_called = true;
        c.binding = Some(0.0);
        let toward_it = if at.x < target.x { -1.0 } else { 1.0 };
        let x = within(target.x + toward_it * ALLY_BESIDE, hall.left + ALLY_WALL, hall.right - ALLY_WALL);
        let y = within(target.y - ALLY_OVER, top, hall.floor - 80.0);
        inv.submit::<BossSummonPort>(BossSummon {
            label: "kagero".into(),
            serial: Vec::new(),
            position: [x, y],
            half_size: crate::kagero::HALF_SIZE,
            character_id: crate::kagero::KAGERO_ID.into(),
            health: Some(ALLY_HEALTH),
            keeps_contact_damage: false,
            side: SummonSide::Player,
            holds: vec![crate::kagero::VISIT.into()],
        })?;
    }
    if let Some(b) = c.binding {
        let b = b + dt;
        let at_dispel = crate::kagero::DISPEL_AT_S;
        if b >= at_dispel && b - dt < at_dispel {
            // The ward breaks: its blows land from here.
            c.warded = false;
            inv.submit::<BossGuardPort>(BossGuard { guarded: false })?;
            play(inv, SFX_WARD_BREAK, at)?;
            play(inv, SFX_GUTTER, at)?;
            burst(inv, at, 60, 420.0, WARD_FIRE, "spark")?;
            burst(inv, at, 24, 300.0, FIRE_CORE, "streak")?;
            shake(inv, SHAKE_TEAR)?;
        }
        if b >= at_dispel {
            // The flash of its ward breaking, dying away.
            inv.submit::<BossStagePort>(BossStage { lit: SPIRIT_LIT, flash: flash_after(b - at_dispel, 0.5), card: 0.0 })?;
        }
        c.binding = if b >= at_dispel + DISPEL_S { None } else { Some(b) };
        if c.binding.is_none() {
            // Blown out: the ward is gone.
            c.ward_t = None;
        }
    }

    for timer in [&mut c.miss, &mut c.buried, &mut c.guttered, &mut c.rekindle, &mut c.appear, &mut c.slack, &mut c.ward_t] {
        if let Some(t) = timer.as_mut() {
            *t += dt;
        }
    }
    if c.rekindle.is_some_and(|t| t >= REKINDLE_S) {
        c.rekindle = None;
    }
    if c.appear.is_some_and(|t| t >= ARRIVE_S) {
        // It has howled over the hall: it waits there for you, warded.
        c.appear = None;
        shake(inv, SHAKE_HOWL)?;
        burst(inv, at, 40, 360.0, WARD_FIRE, "spark")?;
    }
    if c.miss.is_some_and(|t| t >= MISS_S) {
        c.miss = None;
    }
    if c.buried.is_some_and(|t| t >= BURIED_S + PULL_S) {
        c.buried = None;
    }
    if c.slack.is_some_and(|t| t >= LEASH_SLACK_S) {
        c.slack = None;
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
        // It stalks you: at your height, a hunting distance to your side,
        // pacing. In the final hall, while you are not in it yet, it waits
        // over its middle.
        let waiting = in_final && (target.x < final_lo || target.x > final_hi);
        let pace = h.len(STALK_PACE) * (c.clock * std::f32::consts::TAU * STALK_PACE_HZ).sin();
        let goal = if waiting {
            Vec2::new((final_lo + final_hi) * 0.5, top + h.len(20.0) + bob)
        } else {
            Vec2::new(target.x - side * h.len(STALK_DX) + pace, feet - STALK_CLEAR - half.y + bob * 0.5)
        };
        let speed = if c.frenzied { HOVER_SPEED_FRENZIED } else { HOVER_SPEED };
        let goal = clamp(goal);
        let step = goal - at;
        let stride = speed * dt;
        if step.length() > stride { at + step.normalize() * stride } else { goal }
    };
    // A position the run's lunge sets over its hunt.
    let mut return_pos: Option<Vec2> = None;
    let pos = match part {
        Some(Part { mv: Move::Bite, striking: true, t, dur }) => from + (to - from) * ease_out(t / dur),
        Some(Part { mv: Move::Dive, striking: false, t, dur }) => from + (to - from) * ease(t / (dur * DIVE_RISE_U)),
        Some(Part { mv: Move::Dive, striking: true, t, dur }) => {
            let u = (t / dur).min(1.0);
            from + (to - from) * u * u
        }
        Some(Part { mv: Move::Gutter, striking: false, t, dur }) => {
            let u = t / dur;
            Vec2::new(at.x, from.y + (ground - from.y) * u * u)
        }
        Some(Part { mv: Move::Gutter, striking: true, .. }) => Vec2::new(at.x, ground),
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
        // Its leash slack, it pants where it is.
        None if c.slack.is_some() => at,
        None if c.miss.is_some() => {
            let t = c.miss.unwrap_or(0.0);
            let along = (to - from).normalize_or(Vec2::ZERO);
            // It tumbles on past you, slowing.
            at + along * (MISS_DRIFT / MISS_S) * (1.0 - t / MISS_S).max(0.0) * dt
        }
        None if c.rearing.is_some() => {
            // It flares where it is, then goes at the wall; off it, it draws
            // back up under the vault, so you can get under it to the gap. A
            // later transition only rears where it is.
            let (goal, speed) = if !c.rear_crash || c.rearing.is_some_and(|t| t < FLARE_S) {
                (at, 0.0)
            } else if !c.crashed {
                side = crash_dir;
                (crash_at(at), CRASH_SPEED)
            } else {
                side = crash_dir;
                (Vec2::new(crash_wall - crash_dir * (half.x + RECOIL), top + h.len(8.0)), RECOIL_SPEED)
            };
            let step = goal - at;
            let stride = speed * dt;
            if step.length() > stride { at + step.normalize() * stride } else { goal }
        }
        None if c.appear.is_some() || c.binding.is_some() => at,
        None if c.run.is_some() => {
            let hunt = pursuit.as_ref().expect("a running head has a pursuit step");
            side = if hunt.position.x < at.x { -1.0 } else { 1.0 };
            // Its lunge at your back: it gathers back, darts past you, skids.
            if let Some(l) = c.lunge {
                let from = Vec2::new(c.from_x, c.from_y);
                let to = Vec2::new(c.to_x, c.to_y);
                let dart = (l - RUN_GATHER_S) / RUN_DART_S;
                return_pos = Some(if l < RUN_GATHER_S {
                    from - Vec2::new(c.run_dir * h.len(6.0) * ease(l / RUN_GATHER_S), 0.0)
                } else if dart < 1.0 {
                    side = if to.x < from.x { -1.0 } else { 1.0 };
                    from + (to - from) * ease_out(dart)
                } else {
                    let skid = (l - RUN_GATHER_S - RUN_DART_S) / MISS_S;
                    let along = (to - from).normalize_or(Vec2::ZERO);
                    side = if along.x < 0.0 { -1.0 } else { 1.0 };
                    at + along * (MISS_DRIFT / MISS_S) * (1.0 - skid).max(0.0) * dt
                });
            }
            if c.lunge.is_some() && c.ticks % 2 == 0 {
                burst(inv, at, 5, 180.0, WARD_FIRE, "spark")?;
            }
            clamp(return_pos.take().unwrap_or(hunt.position))
        }
        None => {
            side = face(at.x);
            hover(side)
        }
    };
    let pos = if c.run.is_some() { clamp(pos) } else { clamp(pos).min(Vec2::new(f32::MAX, ground)) };

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
            Move::Storm => {
                // Lightning strikes each of its marks from over the floor.
                if part.t < BOLT_S {
                    for (k, x) in c.marks.clone().into_iter().enumerate() {
                        bolt_down(inv, SLOT_STORM[k.min(2)], c.bolt_generation, x, c.marks_floor, STORM_HIGH, BOLT_S - part.t)?;
                    }
                }
                if onset {
                    play(inv, SFX_STRIKE, Vec2::new(target.x, c.marks_floor))?;
                    shake(inv, SHAKE_DIVE)?;
                }
            }
            Move::Leash => {
                // The chain from its neck down to the floor under it, and
                // along the floor to its tip, which sweeps from behind it to
                // in front of it. The tip hurts; the chain is fire.
                let u = (part.t / part.dur).clamp(0.0, 1.0);
                let reach = LEASH_REACH * inu.size[1];
                let tip_x = pos.x + side * reach * (2.0 * ease(u) - 1.0);
                let floor = hall.floor - LEASH_TIP_HALF.y;
                let neck = h.neck();
                let neck = pos + Vec2::new(neck.x * side, neck.y);
                let under = Vec2::new(pos.x, floor);
                let tip = Vec2::new(tip_x, floor);
                if onset {
                    play(inv, SFX_WHIP, pos)?;
                }
                if c.ticks % 2 == 0 {
                    for k in 0..=LEASH_LINKS {
                        let f = k as f32 / LEASH_LINKS as f32;
                        let link = if f < 0.4 { neck + (under - neck) * (f / 0.4) } else { under + (tip - under) * ((f - 0.4) / 0.6) };
                        burst(inv, link, 1, 40.0, FIRE, "spark")?;
                    }
                    burst(inv, tip, 3, 160.0, FIRE_CORE, "spark")?;
                }
                inv.submit::<HeldDamageBoxPort>(HeldDamageBox {
                    slot: SLOT_LEASH,
                    generation: c.box_generation,
                    center: tip.into(),
                    half_extent: (LEASH_TIP_HALF * c.scale).into(),
                    damage: LEASH_DAMAGE,
                    knockback: LEASH_KNOCKBACK,
                    lifetime_s: part.dur + 0.1,
                })?;
            }
            Move::Howl if onset => {
                let radius = inu.size[1] * HOWL_RADIUS;
                riding(inv, (Vec2::ZERO, Vec2::splat(radius)), side, Some(radius), HOWL_DAMAGE, HOWL_KNOCKBACK, 0.3, "inu_howl")?;
                play(inv, howl_cue(c.scale), pos)?;
                shake(inv, SHAKE_HOWL)?;
                burst(inv, pos, 30, 360.0, FIRE_CORE, "spark")?;
                // From the transition lightning answers the howl, a bolt each
                // side of it.
                if c.frenzied {
                    c.bolt_generation = c.bolt_generation.wrapping_add(1);
                    let out = radius + 60.0;
                    for (slot, x) in [(SLOT_STORM[0], pos.x - out), (SLOT_STORM[2], pos.x + out)] {
                        let x = within(x, hall.left + 40.0, hall.right - 40.0);
                        bolt_down(inv, slot, c.bolt_generation, x, hall.floor, STORM_HIGH, BOLT_S)?;
                    }
                    play(inv, SFX_STRIKE, pos)?;
                }
            }
            Move::Gutter if onset => {
                play(inv, SFX_SLAM, Vec2::new(pos.x, hall.floor - 4.0))?;
                burst(inv, Vec2::new(pos.x, hall.floor - 4.0), 16, 200.0, DUST, "dust")?;
            }
            _ => {}
        }
    }
    // The dive lands: its teeth go into the floor, and a shock runs the
    // floor both ways.
    if c.buried.is_some_and(|t| t <= dt) {
        let jaws = Vec2::new(pos.x, hall.floor - 4.0);
        play(inv, SFX_SLAM, jaws)?;
        play(inv, SFX_SHOCK, jaws)?;
        shake(inv, SHAKE_DIVE)?;
        burst(inv, jaws, 30, 320.0, DUST, "dust")?;
        burst(inv, jaws, 16, 260.0, FIRE, "spark")?;
        c.box_generation = c.box_generation.wrapping_add(1);
        c.shock_left = Some(pos.x);
        c.shock_right = Some(pos.x);
        c.shock_floor = hall.floor;
    }
    // The shocks roll the floor to its walls.
    for (slot, wave, dir) in [(SLOT_SHOCK[0], &mut c.shock_left, -1.0f32), (SLOT_SHOCK[1], &mut c.shock_right, 1.0)] {
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
            burst(inv, Vec2::new(x, c.shock_floor - 8.0), 5, 200.0, DUST, "dust")?;
            burst(inv, Vec2::new(x, c.shock_floor - 8.0), 2, 160.0, FIRE, "spark")?;
        }
        inv.submit::<HeldDamageBoxPort>(HeldDamageBox {
            slot,
            generation: c.box_generation,
            center: [x, c.shock_floor - SHOCK_HALF.y],
            half_extent: SHOCK_HALF.into(),
            damage: SHOCK_DAMAGE,
            knockback: SHOCK_KNOCKBACK,
            lifetime_s: SHOCK_LIFETIME_S,
        })?;
    }

    // Its storm's tell: the marks spark, and his lightning crawls over it.
    if let Some(Part { mv: Move::Storm, striking: false, .. }) = part {
        for (k, x) in c.marks.clone().into_iter().enumerate() {
            mark(inv, x, c.marks_floor, k as u32, c.ticks)?;
        }
        if c.ticks % 5 == 0 {
            let ear = |x: f32, y: f32| pos + Vec2::new(h.art(x, y).x * side, h.art(x, y).y);
            arc(inv, "storm_ears", ear(-10.0, -34.0), ear(12.0, -30.0), 0.1)?;
            play(inv, SFX_ZAP, pos)?;
        }
    }
    let row = drawn_row(&c, part);
    // The side it is drawn facing is the one it faces now: the side asked
    // for here is drawn from the next tick (filmed 2026-10-25: started on
    // the asking tick, the turn's first frame was drawn the old way round).
    let row = turned(&mut c, inu.side, row, dt);
    let layer = ward_layer(&c);
    held_at(inv, at, pos, side, row, layer, dt)?;
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
        return held_at(inv, at, from, side, ("unseen", 0.0, true), None, dt);
    };
    let start = rise_at(inu);
    let was = t - dt;
    let crossed = |s: f32| t >= s && was < s;
    inv.submit::<BossStagePort>(rise_stage_at(t, start))?;
    let sting = if inu.predecessor.is_some() { wolfenstein::UNSEW_AT + FIRST_STITCH } else { 0.0 };
    if crossed(sting) {
        play(inv, SFX_STING, from)?;
    }
    if t < start {
        set_scale(c, inv, c.rise_scale)?;
        return held_at(inv, at, from, side, ("unseen", 0.0, true), None, dt);
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
        play(inv, howl_cue(c.scale), at)?;
        burst(inv, at, 30, 360.0, FIRE_CORE, "spark")?;
        shake(inv, SHAKE_HOWL)?;
    }
    let pos = from - Vec2::new(0.0, RISE_UP * ease(u.min(1.0)));
    let row = if u < 1.0 { span("rise", 0.0, 22.0, 0.08, u) } else { ("idle", t, true) };
    held_at(inv, at, pos, side, row, None, dt)
}

/// The row it is drawn with: each phase of a move has its own row; then its
/// windows; then its float.
fn drawn_row(c: &Conductor, part: Option<Part>) -> (&'static str, f32, bool) {
    let u = |p: Part| p.t / p.dur;
    let tell = |name, frames: f32, fd: f32, u: f32| span(name, 0.0, frames, fd, (u / 0.7).min(1.0));
    if let Some(t) = c.rekindle {
        return span("rekindle", 0.0, 6.0, 0.06, t / REKINDLE_S);
    }
    if let Some(t) = c.binding {
        // Her spell holds it, then its ward breaks.
        let at = crate::kagero::DISPEL_AT_S;
        return if t < at { ("ward_bound", t, true) } else { span("dispel", 0.0, 12.0, 0.075, (t - at) / DISPEL_S) };
    }
    if let Some(t) = c.rearing {
        // It howls as its ward kindles, then goes at the wall jaws first.
        return match (c.rear_crash, t < FLARE_S) {
            (true, true) => span("ward_kindle", 0.0, 12.0, 0.075, t / FLARE_S),
            (true, false) => ("crash", t - FLARE_S, true),
            (false, _) => ("howl", t, false),
        };
    }
    if let Some(t) = c.appear {
        // It howls over the hall it came into.
        return ("howl", t, false);
    }
    if let Some(l) = c.lunge {
        // Its lunge at your back: gather, dart, skid.
        return if l < RUN_GATHER_S {
            span("bite_tell", 0.0, 8.0, 0.07, l / RUN_GATHER_S)
        } else if l < RUN_GATHER_S + RUN_DART_S {
            span("bite", 0.0, 8.0, 0.05, (l - RUN_GATHER_S) / RUN_DART_S)
        } else {
            span("bite_miss", 0.0, 12.0, 0.07, (l - RUN_GATHER_S - RUN_DART_S) / MISS_S)
        };
    }
    if let Some(t) = c.run {
        // It hunts: ears back, low, at a run.
        return ("hunt", t, true);
    }
    match part {
        Some(p @ Part { mv: Move::Bite, striking: false, .. }) => tell("bite_tell", 8.0, 0.07, u(p)),
        Some(p @ Part { mv: Move::Bite, .. }) => span("bite", 0.0, 8.0, 0.05, u(p)),
        Some(p @ Part { mv: Move::Dive, striking: false, .. }) => tell("dive_tell", 8.0, 0.08, u(p)),
        Some(Part { mv: Move::Dive, t, .. }) => ("dive", t, true),
        Some(p @ Part { mv: Move::Storm, striking: false, .. }) => tell("storm_tell", 10.0, 0.08, u(p)),
        Some(Part { mv: Move::Storm, t, .. }) => ("howl", t, false),
        Some(p @ Part { mv: Move::Leash, striking: false, .. }) => span("leash_tell", 0.0, 10.0, 0.08, u(p)),
        Some(p @ Part { mv: Move::Leash, .. }) => span("leash_whip", 0.0, 10.0, 0.06, u(p)),
        Some(p @ Part { mv: Move::Howl, striking: false, .. }) => tell("howl_tell", 6.0, 0.08, u(p)),
        Some(Part { mv: Move::Howl, t, .. }) => ("howl", t, false),
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
            } else if let Some(t) = c.slack {
                ("leash_slack", t, true)
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

    /// It howls lower as it grows: its build, then its frenzy's 1.15x, then
    /// its enrage's 1.3x.
    #[test]
    fn its_howl_is_lower_as_it_grows() {
        assert_eq!(howl_cue(0.4), "boss.inugami.howl");
        assert_eq!(howl_cue(1.0), "boss.inugami.howl");
        assert_eq!(howl_cue(SWELL_FRENZIED), "boss.inugami.howl_large");
        assert_eq!(howl_cue(SWELL_ENRAGED), "boss.inugami.howl_giant");
    }
}
