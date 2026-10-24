//! GNU-ton's conductor: it performs the scholar's moves with the giant gnu's
//! fists. Migrated from the native conductor (fast-iteration I4).
//!
//! The scholar's scripted pattern decides WHICH move and WHEN (its `Special`
//! keys). This decides the rest: where each fist goes (the choreography), what
//! it hits, when it is an opening, what each body is drawn as, the gnu's buck
//! and its stomp's shocks, and eureka. It reads the gnu and its fists through
//! `ambition.boss.mount`, and writes them through the limb and mount ports.
//!
//! The fight's shape: a fist that sticks in the floor is the punish window.
//! Its first blow passes on to the scholar; then it is spent until it sticks
//! again, so a phase lasts as long as its rhythm, not as long as the player
//! swings. Hurt him from the gnu's back and the gnu bucks you off.
//!
//! The giant's arms: the fight starts with them ON it (phase 1), and its fists
//! are in its picture. It punches down into a portal by its shoulder, and the
//! fist comes out of the pair's far line, over the player; the pair is a real
//! one, so the player can go up through it to the giant's back. When the
//! encounter leaves phase 1 the arms go into two lines that close on them
//! (the clamp), and from then the fists are bodies of their own. This module
//! asks the portal domain for the pair of the punch each tick it is open
//! (`ambition.portal.held_pair`), and for each look line each tick it is
//! there (`ambition.portal.light_line`).

use ambition_boss_special_port::{
    BossConduct, BossConductPort, BossGuard, BossGuardPort, ConductedPose, ConductedPosePort, DrawnPart, DrawnRow, DrawnRowPort, LimbBlow,
    LimbBlowPort, LimbBond, LimbBondPort, LimbHold, LimbOpening, LimbOpeningPort, LimbPose, LimbPosePort, LimbStrike,
    LimbStrikePort, MountLaunch, MountLaunchPort, MountWalk, MountWalkPort, FoeLaunch, FoeLaunchPort,
    MountView, MountViewPort, Pose as BodyPose, Wear, WearPort, Wearer,
};
use ambition_combat_port::{
    BodySound, BodySoundPort, Burst, BurstPort, CameraShake, CameraShakePort, Effect, EffectPort, HeldDamageBox, HeldDamageBoxPort,
    GravityWell, HeldForce, HeldForcePort,
    Place, RidingHitbox,
    RidingHitboxPort, RidingKnockback, SpeechBubble, SpeechBubblePort,
};
use ambition_portal_port::{HeldPortalPair, HeldPortalPairPort, LightLine, LightLinePort, LineOnLimb, PortalEnd};
use ambition_extension_sdk::{
    phases::BOSS_CONDUCT, record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};
use bevy_math::Vec2;

use super::choreography::blows::{self, Arm, Blow, Exit};
use super::choreography::{self as ch, Cue, Fist, Hall, Latch, Move, Pose, Stage};

/// The behaviour id of the boss it conducts.
pub const GNU_TON_ID: &str = "gnu_ton_rider";

/// The phase its entry runs in: a system of the game that reads its record in
/// the same tick is ordered after this phase's requests are lowered.
pub use ambition_extension_sdk::phases::BOSS_CONDUCT as CONDUCT_PHASE;

/// The limb slot of each fist (`ambition_characters::actor::LimbSlot`).
const FIST_SLOTS: [&str; 2] = ["hand_left", "hand_right"];

/// A falling or swinging fist.
const FIST_DAMAGE: i32 = 2;
const FIST_KNOCKBACK: f32 = 1.4;
/// The hit volume is a little smaller than the drawn fist: a graze is not a hit.
const FIST_HIT_SCALE: f32 = 0.84;
/// A long move re-arms its hit this often, so it can land again once the
/// player's mercy frames are over.
const REARM: f32 = 0.9;
/// The buck: how hard the gnu hops, how long its back throws, and how hard.
const BUCK_HOP: f32 = 520.0;
const BUCK_WINDOW: f32 = 0.16;
/// The buck THROWS whoever stands on its back off it: a light hit over the
/// back that launches up and away from him. A hit's hitstun is what makes a
/// throw a throw: a rebound pad was steered back by the player's air control.
const BUCK_THROW_SPEED: f32 = 900.0;
/// Up and out, `x` away from him.
const BUCK_THROW_DIR: [f32; 2] = [0.75, -0.66];
const BUCK_DAMAGE: i32 = 1;
const BUCK_REACH: f32 = 48.0;
/// The gnu's reflex: from his hurt to the throw (its snort and rear-up), the
/// whole of it (its `buck` row), and the rest before it can throw again.
const REFLEX_TELL: f32 = 0.35;
const REFLEX_LEN: f32 = 0.56;
const REFLEX_COOLDOWN: f32 = 1.0;
/// The stomp's shock: a floor wave each way.
const WAVE_SPEED: f32 = 540.0;
const WAVE_HALF: Vec2 = Vec2::new(26.0, 20.0);
const WAVE_DAMAGE: i32 = 2;
const WAVE_KNOCKBACK: f32 = 1.3;
const WAVE_LIFETIME_S: f32 = 6.0;
/// The held-box slots: one for each fist, then one for each shock.
const FIST_BOX_SLOT: u32 = 0;
const WAVE_BOX_SLOT: u32 = 2;
const WAVES: usize = 4;
/// The held portal pair's slot: the pair of the punch.
const PUNCH_PAIR_SLOT: u32 = 0;
/// The slot of the well's pair of portals (`ch::space::well_portals`).
const WELL_PAIR_SLOT: u32 = 1;
const INK: [f32; 4] = [1.0, 0.86, 0.45, 0.95];
const WARNING: [f32; 4] = [1.0, 0.42, 0.25, 1.0];
const GOLD: [f32; 4] = [1.0, 0.84, 0.2, 1.0];
/// The dust where he lands on the back.
const DUST: [f32; 4] = [0.86, 0.8, 0.68, 0.9];
const SFX_SNORT: &str = "boss.gnu_ton.snort";
const SFX_CHARGE: &str = "boss.gnu_ton.charge";
const SFX_STOMP: &str = "boss.gnu_ton.stomp";
/// The blows through a pair: the limb drawn back, the pair opening, the swing,
/// a blow that lands on a body, a fist into the floor, a hoof that strikes.
const SFX_WINDUP: &str = "boss.gnu_ton.blow_windup";
const SFX_PORTAL_OPEN: &str = "boss.gnu_ton.portal_open";
const SFX_WHOOSH: &str = "boss.gnu_ton.blow_whoosh";
const SFX_LAND: &str = "boss.gnu_ton.blow_land";
const SFX_FLOOR: &str = "boss.gnu_ton.blow_floor";
const SFX_HOOF: &str = "boss.gnu_ton.hoof_kick";
/// A blow that lands on a body: the camera shakes this hard, and the limb and
/// the giant hold still this long (the giant's side of the hit's freeze; the
/// body that was struck has its own).
const LAND_SHAKE: f32 = 26.0;
const LAND_FREEZE: f32 = 0.11;
/// A blow into the floor, and one into the air.
const FLOOR_SHAKE: f32 = 11.0;
const AIR_SHAKE: f32 = 4.0;
/// The hammer is the heaviest blow: a land on a body shakes the camera this
/// hard and holds the giant this long; into the floor it shakes this hard,
/// and holds the giant still a moment there too.
const HAMMER_LAND_SHAKE: f32 = 32.0;
const HAMMER_LAND_FREEZE: f32 = 0.16;
const HAMMER_FLOOR_SHAKE: f32 = 22.0;
const HAMMER_FLOOR_FREEZE: f32 = 0.07;
/// Under this part of his health a hammer into the floor sends a shock along
/// it each way, as the giant's stomp does: the fight's second lesson.
const QUAKE_UNDER: f32 = 0.86;
/// The authored effects of a blow (the sheet `gnu_ton_blow_fx`, which the
/// giant owns): the star where the limb will come out as it draws back the
/// last of the way, the lines of speed as it comes out of the far line, a
/// fist into the floor, and a blow that lands on a body. Each scale is of the
/// game's effect square (56 world units).
const FX_GLINT: &str = "gnu_blow_glint";
const FX_STREAK: &str = "gnu_blow_streak";
const FX_QUAKE: &str = "gnu_hammer_quake";
const FX_IMPACT: &str = "gnu_blow_impact";
/// Two blows that meet (`ch::clash`).
const FX_CLASH: &str = "gnu_blow_clash";
const CLASH_SCALE: f32 = 3.4;
/// The golden apple of the end, and when (seconds into its row) it hits.
const FX_APPLE: &str = "gnu_apple_bonk";
/// Phase 3: his power, the giant's liftoff, and a push of the tide.
const FX_POWER: &str = "gnu_power_up";
const POWER_SCALE: f32 = 3.2;
const FX_LIFTOFF: &str = "gnu_liftoff";
const LIFTOFF_SCALE: f32 = 6.0;
const FX_PUFF: &str = "gnu_thrust_puff";
const PUFF_SCALE: f32 = 2.6;
/// The gear he and his giant wear in space (`worn_gear.ron`). Not the
/// player: a fight that ends with no finale (the player falls, or leaves)
/// would leave the player's helmet on.
const HELMET: &str = "space_helmet";
const HELMET_WEARERS: [Wearer; 2] = [Wearer::Boss, Wearer::Mount];
/// The slot of the field of space among his fields.
const SPACE_FIELD: u32 = 1;
/// The slot of the field of the well's gravity.
const WELL_FIELD: u32 = 2;
const APPLE_SCALE: f32 = 7.0;
const APPLE_HITS: f32 = 14.0 * 0.055 * 0.42;
const GLINT_SCALE: f32 = 3.0;
const STREAK_SCALE: f32 = 5.5;
// Seen in the arena at 6 and 4: the dust stood higher than the giant's fist
// and hid it for all of its time in the floor (the opening).
const QUAKE_SCALE: f32 = 4.0;
const STOMP_QUAKE_SCALE: f32 = 3.0;
const IMPACT_SCALE: f32 = 3.0;
/// The quake's floor is this far under the middle of its square (the art's
/// floor line, 120 of its 512 design units).
const QUAKE_FLOOR: f32 = 120.0 / 512.0;
/// The side of the effect square, in world units, at scale 1.
const FX_SIZE: f32 = 56.0;
/// The flash of a blow that lands.
const FLASH: [f32; 4] = [1.0, 0.97, 0.82, 1.0];
/// The index of the hoof in the per-limb rows of the record (after the fists).
const HOOF: usize = 2;
/// A fist's size when the gnu has none.
const DEFAULT_FIST: Vec2 = Vec2::new(130.0, 100.0);

/// Eureka, start to finish: the apple, the tumble, the daze, the climb. The
/// pattern's Rest after the apple rain is authored at least this long.
pub const EUREKA_LEN: f32 = ch::eureka::APPLE + ch::eureka::TUMBLE + ch::eureka::DAZED + 0.4;

record! {
    /// The conductor's memory, on the scholar.
    pub struct Conductor = SchemaKey::new(crate::PROVIDER, "gnu_ton.conductor", 1);
    /// The latch holds where the fists stood when it first conducted.
    1 seated: bool,
    /// The hall under the gnu, last measured.
    2 hall_known: bool,
    3 hall_floor: f32,
    4 hall_left: f32,
    5 hall_right: f32,
    /// The live part: its move (`code`, 0 for none), whether striking,
    /// seconds into it, its telegraph's length and seconds into its beat.
    6 part_move: u32,
    7 part_striking: bool,
    8 part_t: f32,
    9 part_tel_dur: f32,
    10 part_beat_t: f32,
    /// Last tick's time remaining in the live part: a part that restarts
    /// (the same move twice in a row) shows as this jumping back up.
    11 last_remaining: f32,
    /// The latch: the aim, the lead (`code`, 0 for none), where each fist
    /// stood when the beat began, and the curve's phase.
    12 latch_aim: [f32; 2],
    13 latch_lead: u32,
    14 latch_from: Vec<[f32; 2]> [max 2],
    15 curve_phase: f32,
    /// Each fist's own aim (a pair's follower keeps tracking after the lead).
    16 aims: Vec<[f32; 2]> [max 2],
    17 next_lead: u32,
    18 clock: f32,
    19 ticks: u32,
    /// Seconds since the fists last had a beat: they ease home over it.
    20 resting: f32,
    /// Each fist's last pose: where it was, and whether it was stuck.
    21 last_pos: Vec<Option<[f32; 2]>> [max 3],
    22 last_stuck: Vec<bool> [max 2],
    /// Each fist has passed on its one blow since it last stuck.
    23 spent: Vec<bool> [max 2],
    /// The gnu's own reflex: seconds into it, and the rest before the next.
    24 reflex: Option<f32>,
    25 reflex_cooldown: f32,
    /// His health last tick: a drop is a hit.
    26 last_health: Option<i32>,
    /// Each fist's hit volume: held, when it was armed, and its generation.
    /// (The two fists, then the hoof.)
    27 armed: Vec<bool> [max 3],
    28 armed_at: Vec<f32> [max 3],
    29 fist_generation: Vec<u32> [max 3],
    /// The stomp's shocks: where each is (none when gone), which way, and the
    /// generation of its held box.
    30 wave_x: Vec<Option<f32>> [max 4],
    31 wave_dir: Vec<f32> [max 4],
    32 wave_generation: Vec<u32> [max 4],
    /// Where eureka drops him.
    33 landing: [f32; 2],
    /// Eureka held his pose last tick: release it when it ends.
    34 posed: bool,
    /// The giant's arms are in their two lines, and its fists are bodies of
    /// their own. Not yet: its arms are on it (phase 1).
    35 loosed: bool,
    /// Seconds into the clamp: the arms go into their lines.
    36 clamp: Option<f32>,
    /// The punch's pair is open: its near line (by the giant's shoulder, the
    /// fist goes down into it) and its far line (the fist comes down out of it).
    37 punch_open: bool,
    38 punch_near: [f32; 2],
    39 punch_far: [f32; 2],
    /// The lines that are a look and no way through: the left shoulder's, the
    /// right shoulder's, the left wrist's, the right wrist's.
    40 lines: Vec<Option<[f32; 2]>> [max 4],
    /// Half the length of each of them: wide when it opens, tight on the arm
    /// after the clamp.
    41 line_half: Vec<f32> [max 4],
    /// What he has said (a bit for each line of `Bark`): he says each once.
    42 said: u32,
    /// The blows he took from the giant's back while its arms are on it,
    /// since it last threw.
    43 back_blows: u32,
    // 44 and 45 were his two shoulders and the hop between them.
    /// The duel on the giant's back: he fights (the game suppresses the
    /// pattern's own move then), the move he chose (a place in
    /// `ch::arms::DUEL_MOVES` and 1; 0 is none), a count of his choices, and
    /// the seconds until the next.
    46 dueling: bool,
    47 duel_move: u32,
    48 duel_turn: u32,
    49 duel_wait: f32,
    /// Seconds into the giant taking its arm back out of a punch that the
    /// duel cut short.
    50 pull: Option<f32>,
    /// In the duel his book is up: a blow on him does nothing.
    51 guard: bool,
    /// On the back he is a fighter on a platform. Where he is when he is not
    /// in his saddle: from the saddle (so he goes with the giant).
    52 off: Option<[f32; 2]>,
    /// What he does there (the code of a `Footing`), and seconds into it.
    53 footing: u32,
    54 footing_t: f32,
    /// His speed in the air, when a blow launched him.
    55 air: [f32; 2],
    /// Where a roll, a way back or a hop to the saddle started and ends
    /// (from the saddle), and how long it is.
    56 path_from: [f32; 2],
    57 path_to: [f32; 2],
    58 path_len: f32,
    /// The most health he had: the part of it he lost makes a blow launch
    /// him farther.
    59 full_health: i32,
    /// The blow through the pair (`blows`): its exit (`Exit::code`, 0 for
    /// none) and the side it goes to; the blows begun; the exit of the one
    /// before (no blow is the same as the one before).
    60 blow_exit: u32,
    61 blow_side: f32,
    62 blow_count: u32,
    63 last_exit: u32,
    /// The bodies its strike had hit last tick: a count that grows is a land.
    64 blow_struck: u32,
    /// It landed: its end is not a miss.
    65 blow_landed: bool,
    /// A land holds the blow still: seconds of that left, and the seconds of
    /// the strike it held.
    66 freeze: f32,
    67 lag: f32,
    /// The normals of the pair's near and far line, and how open it is.
    68 near_normal: [f32; 2],
    69 far_normal: [f32; 2],
    70 opening: f32,
    /// The arm of the blow (`Arm::code`, 0 for the leg), the arm of the last
    /// blow of an arm and when it ended: a blow soon after is the next of a
    /// combination, and the other arm throws it.
    71 blow_arm: u32,
    72 last_arm: u32,
    73 last_blow_end: f32,
    /// The blow went into the floor; a hammer there is an opening until a
    /// blow of the player's on it has passed to him.
    74 floored: bool,
    75 blow_spent: bool,
    /// The star of its tell was shown.
    76 glinted: bool,
    /// The giant walks to a mark between its blows (`ch::stride`): where (none:
    /// it stands), seconds into its walk row, and the seconds before it may
    /// walk again.
    77 walk_to: Option<f32>,
    78 walk_t: f32,
    79 walk_wait: f32,
    /// Seconds the player has stood on its back: it looks round at him.
    80 look_t: f32,
    /// When (in the strike's own time) a blow of the player's met the limb as
    /// it came out of its far line (`ch::clash`): it goes back from there.
    81 clashed_at: Option<f32>,
    /// Seconds into the end of the fight (`ch::finale`), and where he was
    /// when he was beaten.
    82 finale_t: Option<f32>,
    83 finale_from: [f32; 2],
    /// Phase 3 (`ch::space`): seconds since the launch began (none: on the
    /// ground), how far the sky is in space (0..1), the seconds before a
    /// blow of the player's may push them back again, and whether the
    /// helmets are on.
    84 space_t: Option<f32>,
    85 ascent: f32,
    86 recoil_wait: f32,
    87 helmets: bool,
    /// His arrival (`ch::arrival`): seconds into it (none: before the hall
    /// woke, or after it), and whether it has played.
    88 arrive_t: Option<f32>,
    89 arrived: bool,
}

/// What he does on the giant's back when he is not in his saddle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Footing {
    /// He stands where he is.
    Stand,
    /// He goes to the player along the back.
    Walk,
    /// He rolls through the player.
    Roll,
    /// A blow launched him: he is in the air and can do nothing.
    Launched,
    /// He landed, and does not act yet.
    Land,
    /// The blow threw him off the back: he comes back to it.
    Recover,
    /// The fight on the back is over: he hops to his saddle.
    Home,
}

impl Footing {
    const ALL: [Footing; 7] =
        [Footing::Stand, Footing::Walk, Footing::Roll, Footing::Launched, Footing::Land, Footing::Recover, Footing::Home];

    fn code(self) -> u32 {
        Self::ALL.iter().position(|f| *f == self).unwrap_or(0) as u32
    }
}

/// What he says at the turns of the fight, once each.
#[derive(Clone, Copy)]
enum Bark {
    /// The first punch through the pair begins.
    Punch,
    /// The player is on the giant's back while its arms are on it.
    Boarded,
    /// The clamp begins.
    Clamp,
    /// The lines are tight on the arms.
    Clamped,
    /// The first kick through a pair.
    Kick,
    /// The first blow of a combination: the other arm follows.
    Combo,
    /// The first blow of the player's that meets a limb (`ch::clash`).
    Clash,
}

impl Bark {
    fn text(self) -> &'static str {
        match self {
            Bark::Punch => "Observe: action at a distance.",
            Bark::Boarded => "Off the shoulders of my giant!",
            Bark::Clamp => "Enough! Hold still.",
            Bark::Clamped => "Now we shall be rigorous.",
            Bark::Kick => "Every action, an equal kick!",
            Bark::Combo => "And for every action... another!",
            Bark::Clash => "Equal! And opposite!",
        }
    }
}

fn say(inv: &mut Invocation<'_>, c: &mut Conductor, bark: Bark, head: Vec2) -> Result<(), Fault> {
    let bit = 1u32 << bark as u32;
    if c.said & bit != 0 {
        return Ok(());
    }
    c.said |= bit;
    inv.submit::<SpeechBubblePort>(SpeechBubble { at: a(head - Vec2::new(0.0, 30.0)), text: bark.text().to_string() })
}

/// The part of a beat being performed.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Part {
    mv: Move,
    striking: bool,
    t: f32,
    tel_dur: f32,
    beat_t: f32,
}

fn move_code(mv: Move) -> u32 {
    Move::ALL.iter().position(|m| *m == mv).unwrap_or(0) as u32 + 1
}

fn move_of_code(code: u32) -> Option<Move> {
    code.checked_sub(1).and_then(|i| Move::ALL.get(i as usize).copied())
}

fn fist_code(fist: Option<Fist>) -> u32 {
    match fist {
        None => 0,
        Some(Fist::Left) => 1,
        Some(Fist::Right) => 2,
    }
}

fn fist_of_code(code: u32) -> Option<Fist> {
    match code {
        1 => Some(Fist::Left),
        2 => Some(Fist::Right),
        _ => None,
    }
}

fn v(a: [f32; 2]) -> Vec2 {
    Vec2::from(a)
}

fn a(v: Vec2) -> [f32; 2] {
    [v.x, v.y]
}

impl Conductor {
    /// The move being performed and whether it is striking, for tests and
    /// inspectors.
    pub fn performing(&self) -> Option<(Move, bool)> {
        self.part().map(|p| (p.mv, p.striking))
    }

    /// The hall under the gnu, last measured.
    pub fn hall(&self) -> Option<Hall> {
        self.hall_known.then_some(Hall { floor: self.hall_floor, left: self.hall_left, right: self.hall_right })
    }

    /// A record that has not conducted yet holds no per-fist rows: give it
    /// each one, in its starting state.
    fn sized(mut self) -> Self {
        self.latch_from.resize(2, [0.0; 2]);
        self.aims.resize(2, [0.0; 2]);
        self.last_stuck.resize(2, false);
        self.spent.resize(2, false);
        self.last_pos.resize(3, None);
        self.armed.resize(3, false);
        self.armed_at.resize(3, 0.0);
        self.fist_generation.resize(3, 0);
        self.wave_x.resize(WAVES, None);
        self.wave_dir.resize(WAVES, 0.0);
        self.wave_generation.resize(WAVES, 0);
        self.lines.resize(4, None);
        self.line_half.resize(4, 0.0);
        self
    }

    /// The blow whose pair is open, and whether it landed on a body.
    pub fn blow(&self) -> Option<(Exit, bool)> {
        Exit::of_code(self.blow_exit).filter(|_| self.punch_open).map(|exit| (exit, self.blow_landed))
    }

    /// The limb of the blow whose pair is open (the arm and the blow, or the
    /// kick), and whether its fist is in the floor and still an opening.
    pub fn blow_limb(&self) -> Option<(Blow, bool)> {
        self.blow_kind().filter(|_| self.punch_open).map(|blow| (blow, self.floored && !self.blow_spent))
    }

    /// Where the giant walks to between its blows (none: it stands).
    pub fn walk_to(&self) -> Option<f32> {
        self.walk_to
    }

    /// A blow of the player's met the limb of this blow (`ch::clash`).
    pub fn clashed(&self) -> bool {
        self.clashed_at.is_some()
    }

    /// Seconds into the end of the fight (`ch::finale`), once he is beaten.
    /// How far the sky is in space: 0 on the ground, 1 in space.
    pub fn ascent(&self) -> f32 {
        self.ascent
    }

    pub fn finale(&self) -> Option<f32> {
        self.finale_t
    }

    /// The blow of the record: its exit thrown by its arm.
    fn blow_kind(&self) -> Option<Blow> {
        Exit::of_code(self.blow_exit).map(|exit| exit.blow(Arm::of_code(self.blow_arm).unwrap_or(Arm::Right)))
    }

    /// The pair of the punch, when it is open: its near line and its far line.
    pub fn punch_pair(&self) -> Option<[Vec2; 2]> {
        self.punch_open.then(|| [v(self.punch_near), v(self.punch_far)])
    }

    /// The duel on the giant's back: whether he fights now, and the move of his
    /// table he chose (its id, and the count of his choices: a new count is a
    /// new move).
    pub fn duel(&self) -> (bool, Option<(&'static str, u32)>) {
        let chosen = (self.duel_move as usize)
            .checked_sub(1)
            .and_then(|i| ch::arms::DUEL_MOVES.get(i))
            .map(|(id, ..)| (*id, self.duel_turn));
        (self.dueling, chosen)
    }

    /// What he does on the back.
    pub fn footing(&self) -> Footing {
        Footing::ALL.get(self.footing as usize).copied().unwrap_or(Footing::Stand)
    }

    fn step_to(&mut self, footing: Footing) {
        self.footing = footing.code();
        self.footing_t = 0.0;
    }

    /// A blow launched him and he has not his feet under him yet: a move of
    /// his that plays is stopped, whatever part of it plays.
    pub fn reels(&self) -> bool {
        matches!(self.footing(), Footing::Launched | Footing::Land | Footing::Recover)
    }

    /// The lines that are a look and no way through (see `lines`): the middle
    /// of each, and half its length.
    pub fn look_lines(&self) -> [Option<(Vec2, f32)>; 4] {
        std::array::from_fn(|i| {
            let half = self.line_half.get(i).copied().unwrap_or(0.0);
            self.lines.get(i).copied().flatten().map(|at| (v(at), half))
        })
    }

    fn part(&self) -> Option<Part> {
        move_of_code(self.part_move).map(|mv| Part {
            mv,
            striking: self.part_striking,
            t: self.part_t,
            tel_dur: self.part_tel_dur,
            beat_t: self.part_beat_t,
        })
    }

    fn set_part(&mut self, part: Option<Part>) {
        let p = part.unwrap_or(Part { mv: Move::ALL[0], striking: false, t: 0.0, tel_dur: 0.0, beat_t: 0.0 });
        self.part_move = part.map_or(0, |p| move_code(p.mv));
        self.part_striking = p.striking;
        self.part_t = p.t;
        self.part_tel_dur = p.tel_dur;
        self.part_beat_t = p.beat_t;
    }

    fn latch(&self) -> Latch {
        Latch {
            aim: v(self.latch_aim),
            lead: fist_of_code(self.latch_lead),
            from: [v(self.latch_from[0]), v(self.latch_from[1])],
            curve_phase: self.curve_phase,
        }
    }

    fn set_latch(&mut self, latch: Latch) {
        self.latch_aim = a(latch.aim);
        self.latch_lead = fist_code(latch.lead);
        self.latch_from = vec![a(latch.from[0]), a(latch.from[1])];
        self.curve_phase = latch.curve_phase;
    }

    /// Latch the fists where they stand (left, right), so the first beat eases
    /// each one from there.
    fn seat(&mut self, fists: [Vec2; 2]) {
        self.latch_aim = a((fists[0] + fists[1]) * 0.5);
        self.latch_from = vec![a(fists[0]), a(fists[1])];
        self.aims = vec![a(fists[0]), a(fists[1])];
        self.seated = true;
    }
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "gnu_ton"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![Conductor::schema()],
        entries: vec![EntryDescriptor {
            key: "conduct".into(),
            phase: BOSS_CONDUCT,
            trigger: TriggerBinding { port: BossConductPort::KEY, selector: GNU_TON_ID.into() },
            reads: vec![MountViewPort::KEY],
            writes: vec![Conductor::KEY],
            requests: vec![
                ConductedPosePort::KEY,
                EffectPort::KEY,
                BossGuardPort::KEY,
                DrawnRowPort::KEY,
                LimbPosePort::KEY,
                LimbOpeningPort::KEY,
                LimbBlowPort::KEY,
                LimbBondPort::KEY,
                LimbStrikePort::KEY,
                MountLaunchPort::KEY,
                MountWalkPort::KEY,
                FoeLaunchPort::KEY,
                HeldForcePort::KEY,
                WearPort::KEY,
                CameraShakePort::KEY,
                HeldDamageBoxPort::KEY,
                HeldPortalPairPort::KEY,
                LightLinePort::KEY,
                RidingHitboxPort::KEY,
                BurstPort::KEY,
                BodySoundPort::KEY,
                SpeechBubblePort::KEY,
            ],
            after: Vec::new(),
            // The telegraph ink alone is up to 58 sparks in a tick.
            limits: Limits { max_requests: 128 },
            // A living scholar is never idle.
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(conduct),
        }],
    }
}

/// The live part of the scholar's move: its `Special` key, whether it is
/// striking, and the time left in it.
fn live_part(scholar: &BossConduct) -> Option<(Move, bool, f32)> {
    if let Some(mv) = scholar.active.as_ref().and_then(|m| Move::from_key(&m.key).map(|mv| (mv, m.remaining))) {
        return Some((mv.0, true, mv.1));
    }
    scholar.telegraph.as_ref().and_then(|m| Move::from_key(&m.key).map(|mv| (mv, false, m.remaining)))
}

/// Where each fist rests: its limb's rest about the gnu. A missing fist rests
/// at its twin's mirror.
fn homes(giant: Vec2, offsets: [Option<Vec2>; 2]) -> [Vec2; 2] {
    let mirror = |v: Vec2| Vec2::new(-v.x, v.y);
    let right = offsets[1].or(offsets[0].map(mirror)).unwrap_or(Vec2::ZERO);
    let left = offsets[0].unwrap_or(mirror(right));
    [giant + left, giant + right]
}

fn spark(inv: &mut Invocation<'_>, at: Vec2, color: [f32; 4], count: u32, speed: f32) -> Result<(), Fault> {
    inv.submit::<BurstPort>(Burst { at: a(at), count, speed, color, kind: "spark".into() })
}

fn dust(inv: &mut Invocation<'_>, at: Vec2, count: u32, speed: f32, color: [f32; 4]) -> Result<(), Fault> {
    inv.submit::<BurstPort>(Burst { at: a(at), count, speed, color, kind: "dust".into() })
}

fn burst(inv: &mut Invocation<'_>, at: Vec2, kind: &str, count: u32, speed: f32, color: [f32; 4]) -> Result<(), Fault> {
    inv.submit::<BurstPort>(Burst { at: a(at), count, speed, color, kind: kind.into() })
}

/// An authored effect at `at`, at `scale` of the effect square, its +X
/// turned along `way` (world units, +y down).
fn effect(inv: &mut Invocation<'_>, fx: &str, at: Vec2, scale: f32, way: Vec2) -> Result<(), Fault> {
    inv.submit::<EffectPort>(Effect { at: Place::World(a(at)), fx: fx.into(), scale, angle: way.y.atan2(way.x) })
}

/// The moments of a blow through the pair, each felt once. It LANDS on a body
/// (the strike its limb carries hit one): a burst where it hit, the camera
/// shakes hard, and the limb and the giant hold still for a moment. Or it is
/// full and hit nothing: into the floor (the limb is stopped by what it
/// meets) with a quake and a boom, or into the air with a rush of it. A
/// hammer is the heaviest of them. Returns where a hammer went into the floor.
#[allow(clippy::too_many_arguments)]
fn impact(
    inv: &mut Invocation<'_>,
    c: &mut Conductor,
    limb: &blows::Limb,
    blow: Blow,
    body: &ambition_boss_special_port::MountLimb,
    cue: &Cue,
    dt: f32,
    floor: f32,
    target: Option<Vec2>,
) -> Result<Option<Vec2>, Fault> {
    let now = cue.t - c.lag;
    let depth = blows::depth(limb, blows::part(limb, now, cue.dur - cue.t)).unwrap_or(limb.reach());
    // Where the fist (the hoof) is at the far end of the pair.
    let fist = v(c.punch_far) + v(c.far_normal) * depth;
    let hammer = blow.is_hammer();
    if body.struck > c.blow_struck && !c.blow_landed {
        c.blow_landed = true;
        let at = target.map_or(fist, |target| (target + fist) * 0.5);
        inv.submit::<CameraShakePort>(CameraShake { amplitude_px: if hammer { HAMMER_LAND_SHAKE } else { LAND_SHAKE } })?;
        c.freeze = if hammer { HAMMER_LAND_FREEZE } else { LAND_FREEZE };
        effect(inv, FX_IMPACT, at, IMPACT_SCALE * if hammer { 1.25 } else { 1.0 }, v(c.far_normal))?;
        spark(inv, at, FLASH, 24, 520.0)?;
        burst(inv, at, "shard", 10, 420.0, [0.8, 0.7, 0.52, 1.0])?;
        play(inv, SFX_LAND, at)?;
    }
    c.blow_struck = body.struck;
    // Held still by a land, the blow is not full this tick.
    let before = if c.freeze > 0.0 { now } else { now - dt };
    // It lands on the floor once: a freeze holds its clock back, and the
    // clock must not come to the moment of the land again.
    if c.blow_landed || c.floored || !blows::lands(limb, before, now) {
        return Ok(None);
    }
    let reach = limb.strike_half.y.max(limb.strike_half.x);
    let ground = body.contact.map(v).or_else(|| (fist.y + reach >= floor - 6.0).then(|| Vec2::new(fist.x, floor)));
    let kick = blow == Blow::Kick;
    match ground {
        Some(at) => {
            c.floored = true;
            let (shake, scale) = if hammer { (HAMMER_FLOOR_SHAKE, QUAKE_SCALE) } else { (FLOOR_SHAKE, STOMP_QUAKE_SCALE) };
            inv.submit::<CameraShakePort>(CameraShake { amplitude_px: shake })?;
            if hammer {
                c.freeze = HAMMER_FLOOR_FREEZE;
            }
            // The quake stands on the floor: its square is lifted so its
            // floor line is the floor.
            let size = FX_SIZE * scale;
            effect(inv, FX_QUAKE, at - Vec2::new(0.0, size * QUAKE_FLOOR), scale, Vec2::X)?;
            dust(inv, at, 18, 380.0, [0.62, 0.52, 0.42, 1.0])?;
            play(inv, if kick { SFX_HOOF } else { SFX_FLOOR }, at)?;
            if hammer {
                play(inv, SFX_STOMP, at)?;
                return Ok(Some(at));
            }
        }
        None => {
            inv.submit::<CameraShakePort>(CameraShake { amplitude_px: AIR_SHAKE })?;
            burst(inv, fist, "streak", 10, 420.0, [0.95, 0.92, 0.85, 0.8])?;
            play(inv, if kick { SFX_HOOF } else { SFX_WHOOSH }, fist)?;
        }
    }
    Ok(None)
}

fn play(inv: &mut Invocation<'_>, cue: &str, at: Vec2) -> Result<(), Fault> {
    inv.submit::<BodySoundPort>(BodySound { cue: cue.into(), at: Place::World(a(at)) })
}

fn draw(inv: &mut Invocation<'_>, on: DrawnPart, row: Option<(&[&str], f32, bool)>) -> Result<(), Fault> {
    let (names, elapsed, looping) = row.map_or((Vec::new(), 0.0, false), |(rows, elapsed, looping)| {
        (rows.iter().map(|r| (*r).to_string()).collect(), elapsed, looping)
    });
    inv.submit::<DrawnRowPort>(DrawnRow { on, names, elapsed, looping })
}

fn fist_part(fist: Fist) -> DrawnPart {
    DrawnPart::Limb(FIST_SLOTS[fist.index()].into())
}

/// The buck's throw: a light hit over the gnu's back, the moment it bucks.
/// It rides HIM, not the world: a hit launches away from where it came from,
/// and a world box comes from the middle of the back, which threw a player
/// standing beside him toward him. Riding him, it also rides the gnu's hop.
fn buck(inv: &mut Invocation<'_>, scholar: Vec2, back: Option<[f32; 4]>) -> Result<(), Fault> {
    inv.submit::<MountLaunchPort>(MountLaunch { velocity: [0.0, -BUCK_HOP], flinchless: true })?;
    let Some([min_x, min_y, max_x, _]) = back else {
        return Ok(());
    };
    let center = Vec2::new((min_x + max_x) * 0.5, min_y - BUCK_REACH * 0.5);
    inv.submit::<RidingHitboxPort>(RidingHitbox {
        offset: a(center - scholar),
        half_extent: [(max_x - min_x) * 0.5, BUCK_REACH * 0.5],
        circle_radius: None,
        damage: BUCK_DAMAGE,
        knockback: RidingKnockback::LaunchSpeed { base: BUCK_THROW_SPEED, growth: None },
        launch_dir: Some(BUCK_THROW_DIR),
        lifetime_s: BUCK_WINDOW,
        name: "gnu_ton_buck_throw".into(),
    })
}

fn conduct(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let scholar: BossConduct = inv.trigger::<BossConductPort>()?.clone();
    // A scholar who rides nothing has no fists to conduct.
    let giant = match inv.observe::<MountViewPort>() {
        Ok(giant) => giant.clone(),
        Err(Fault::MissingObservation(_)) => return Ok(()),
        Err(fault) => return Err(fault),
    };
    let dt = inv.dt();
    let mut c = Conductor::load(inv)?.sized();
    if dt <= 0.0 {
        // A tick with no time keeps what is open: a pair or a line that is
        // not asked for in a tick is closed.
        hold_punch_pair(inv, &c)?;
        if !scholar.alive {
            return Ok(());
        }
        hold_look_lines(inv, &c)?;
        return bond_fists(inv, &c, &giant);
    }
    // The hall under the gnu, measured every tick: the pair can be carried
    // into another room, and fists posed against the last room's floor fall
    // out of this one.
    let Some(h) = giant.hall else {
        return Ok(());
    };
    let hall = Hall { floor: h.floor, left: h.left, right: h.right };
    c.hall_known = true;
    c.hall_floor = hall.floor;
    c.hall_left = hall.left;
    c.hall_right = hall.right;
    let limb = |fist: Fist| giant.limbs.iter().find(|l| l.slot == FIST_SLOTS[fist.index()]);
    if !scholar.alive {
        finale(inv, &mut c, &scholar, &giant, hall, dt)?;
        return c.store(inv);
    }
    c.clock += dt;
    c.ticks = c.ticks.wrapping_add(1);
    // His arrival plays once, when the hall wakes. A fight that wakes in its
    // enrage (a test, a debug start) has none.
    if scholar.awake && !c.arrived {
        c.arrived = true;
        c.arrive_t = (!scholar.enraged).then_some(0.0);
    }
    let arriving = match c.arrive_t {
        Some(t) if t + dt < ch::arrival::LEN => {
            c.arrive_t = Some(t + dt);
            Some(t + dt)
        }
        Some(_) => {
            c.arrive_t = None;
            None
        }
        None => None,
    };
    let clock = c.clock;
    let ticks = c.ticks;
    let scholar_pos = v(scholar.position);
    let scholar_size = v(scholar.size);
    let giant_pos = v(giant.position);
    let giant_size = v(giant.size);

    let fist_size = Fist::BOTH.iter().find_map(|f| limb(*f).map(|l| v(l.size))).unwrap_or(DEFAULT_FIST);
    let current: [Vec2; 2] = std::array::from_fn(|i| limb(Fist::BOTH[i]).map_or(giant_pos, |l| v(l.position)));
    let rests = std::array::from_fn(|i| limb(Fist::BOTH[i]).map(|l| v(l.home_offset)));
    // The drawing faces one way; a giant that is turned has each place on
    // the other side.
    let turn = if giant.unmirrored { 1.0 } else { -1.0 };
    let from_giant = |offset: Vec2| giant_pos + Vec2::new(offset.x * turn, offset.y);
    // The line each fist's arm is in: the shoulder on its own side.
    let shoulder_of = |fist: Fist| usize::from(fist.side() * turn > 0.0);
    let shoulder = |fist: Fist| from_giant(ch::arms::SHOULDER_LINES[shoulder_of(fist)]);
    let mut stage = Stage { hall, homes: homes(giant_pos, rests), scholar: scholar_pos, fist: fist_size };
    if c.loosed || c.clamp.is_some() {
        // A fist that is a body of its own rests out from the line its arm is in.
        stage.homes = std::array::from_fn(|i| from_giant(ch::arms::rest(shoulder_of(Fist::BOTH[i]))));
    }
    if !c.seated {
        // A fist that is gone starts at its home.
        c.seat(std::array::from_fn(|i| limb(Fist::BOTH[i]).map_or(stage.homes[i], |l| v(l.position))));
    }
    let aim_now = scholar.target.map_or(scholar_pos, v);
    let head = scholar_pos - Vec2::new(0.0, scholar_size.y * 0.5);

    // ── Which part of which beat, and did it just begin? ──
    // In the launch (the beat before the enrage, `ch::space`) he performs no
    // move: the pattern's move of the phase before is not his now.
    let in_launch = c.space_t.is_some_and(|t| t < ch::space::LEN)
        || (c.space_t.is_none() && scholar.between_phases && scholar.health * 2 < c.full_health && c.loosed);
    let live = live_part(&scholar).filter(|_| !in_launch);
    let previous = c.part();
    let in_eureka = previous.is_some_and(|p| p.mv == Move::Eureka);
    let rain_ended = previous.is_some_and(|p| p.mv == Move::AppleRain && p.striking)
        && live.is_none_or(|(mv, ..)| mv != Move::AppleRain);
    match live {
        // The rain is over and the golden apple is falling: EUREKA plays out
        // over the Rest the pattern leaves after the rain.
        None if rain_ended => {
            c.set_part(Some(Part { mv: Move::Eureka, striking: true, t: 0.0, tel_dur: 0.0, beat_t: 0.0 }));
            c.last_remaining = EUREKA_LEN;
            c.latch_from = vec![a(current[0]), a(current[1])];
            let side = if aim_now.x < giant_pos.x { -1.0 } else { 1.0 };
            let x = giant_pos.x + side * (giant_size.x * 0.5 + 50.0);
            let x = x.clamp(hall.left + 40.0, hall.right - 40.0);
            c.landing = [x, hall.floor - scholar_size.y * 0.5];
        }
        None if in_eureka && c.last_remaining > dt => {
            c.part_t += dt;
            c.part_beat_t += dt;
            c.last_remaining -= dt;
        }
        None => {
            if previous.is_some() {
                c.latch_from = vec![a(current[0]), a(current[1])];
                c.resting = 0.0;
                c.set_part(None);
            } else {
                c.resting += dt;
            }
        }
        Some((mv, striking, remaining)) => {
            let same = previous.is_some_and(|p| p.mv == mv && p.striking == striking)
                && remaining <= c.last_remaining + 1e-3;
            if same {
                c.part_t += dt;
                c.part_beat_t += dt;
            } else {
                let follows_telegraph = previous.is_some_and(|p| p.mv == mv && !p.striking && striking);
                match previous.filter(|_| follows_telegraph) {
                    Some(p) => {
                        c.set_part(Some(Part { mv, striking, t: 0.0, tel_dur: p.t + dt, beat_t: p.beat_t + dt }));
                    }
                    None => {
                        // A new beat: latch what it needs.
                        let lead = fist_of_code(c.next_lead).unwrap_or(Fist::Left);
                        c.next_lead = fist_code(Some(lead.other()));
                        c.set_latch(Latch {
                            aim: aim_now,
                            lead: Some(lead),
                            from: current,
                            curve_phase: ch::fluxions::phase_for(&stage, aim_now.x, lead.side()),
                        });
                        c.aims = vec![a(aim_now); 2];
                        if matches!(mv, Move::PortalPunch | Move::PortalKick) {
                            // The blow is chosen by where the player is: the
                            // limb, and the far line's place and angle.
                            let feet = scholar.target_body.map_or(aim_now.y, |b| b.position[1] + b.size[1] * 0.5);
                            let high = hall.floor - feet > blows::HIGH;
                            let near = (aim_now.x - giant_pos.x).abs() < giant_size.x * 0.5 + blows::NEAR;
                            let kick = mv == Move::PortalKick;
                            let exit = blows::choose(kick, high, near, Exit::of_code(c.last_exit), c.blow_count);
                            // The arm on the player's side of the giant (as it
                            // is drawn), or, soon after a blow, the other arm.
                            let combo = c.last_blow_end > 0.0 && clock - c.last_blow_end < blows::COMBO_GAP;
                            let drawn_side = (aim_now.x - giant_pos.x).signum() * turn;
                            let arm = blows::arm(drawn_side, Arm::of_code(c.last_arm), combo);
                            let blow = exit.blow(arm);
                            let side = blows::side(&stage, giant_pos, giant_size, blow, exit, aim_now);
                            let (far, way) = blows::far_end(&stage, giant_pos, giant_size, blow, exit, side, aim_now);
                            c.blow_exit = exit.code();
                            c.blow_arm = if kick { 0 } else { arm.code() };
                            c.floored = false;
                            c.blow_spent = false;
                            c.glinted = false;
                            c.blow_side = side;
                            c.last_exit = exit.code();
                            c.blow_count = c.blow_count.wrapping_add(1);
                            c.punch_far = a(far);
                            c.far_normal = a(way);
                            c.blow_landed = false;
                            c.blow_struck = 0;
                            c.freeze = 0.0;
                            c.lag = 0.0;
                            c.clashed_at = None;
                            // Not a punch the pattern starts after the clamp began.
                            if kick {
                                say(inv, &mut c, Bark::Kick, head)?;
                            } else if combo {
                                say(inv, &mut c, Bark::Combo, head)?;
                            } else if !c.loosed && c.clamp.is_none() {
                                say(inv, &mut c, Bark::Punch, head)?;
                            }
                            play(inv, SFX_WINDUP, giant_pos)?;
                            play(inv, SFX_PORTAL_OPEN, far)?;
                        }
                        c.set_part(Some(Part { mv, striking, t: 0.0, tel_dur: 0.0, beat_t: 0.0 }));
                    }
                }
                if striking {
                    // The strike's own onset.
                    match mv {
                        Move::Buck => {
                            buck(inv, scholar_pos, giant.back)?;
                            play(inv, SFX_SNORT, giant_pos)?;
                        }
                        Move::PortalPunch | Move::PortalKick if c.blow_exit != 0 => {
                            // The swing: heard at the far line, where it comes.
                            play(inv, SFX_WHOOSH, v(c.punch_far))?;
                        }
                        Move::Stomp => {
                            let feet = Vec2::new(giant_pos.x, hall.floor - WAVE_HALF.y);
                            for dir in [-1.0, 1.0] {
                                let Some(slot) = c.wave_x.iter().position(Option::is_none) else {
                                    break;
                                };
                                c.wave_x[slot] = Some(feet.x + dir * giant_size.x * 0.5);
                                c.wave_dir[slot] = dir;
                                c.wave_generation[slot] = c.wave_generation[slot].wrapping_add(1);
                            }
                            play(inv, SFX_STOMP, feet)?;
                            dust(inv, feet, 26, 320.0, [0.55, 0.45, 0.35, 1.0])?;
                        }
                        _ => {}
                    }
                }
            }
            c.last_remaining = remaining;
        }
    }

    // ── The giant's arms: on it, or in their lines ──
    if !c.loosed {
        if scholar.between_phases && c.clamp.is_none() {
            c.clamp = Some(0.0);
            play(inv, SFX_SNORT, giant_pos)?;
            say(inv, &mut c, Bark::Clamp, head)?;
        }
        // A move of the free fists, with no clamp before it (a fight that
        // starts late): the arms are in their lines now.
        let skipped = scholar.enraged || live.is_some_and(|(mv, ..)| mv.frees_the_fists());
        let through = c.clamp.is_some_and(|t| t + dt >= ch::arms::CLAMP_THROUGH);
        if skipped || through {
            c.loosed = true;
            // Each fist comes out of its line where it rests.
            c.latch_from = vec![a(stage.homes[0]), a(stage.homes[1])];
            c.last_pos = vec![None; 3];
            for fist in Fist::BOTH {
                spark(inv, stage.home(fist), GOLD, 16, 240.0)?;
            }
        }
    }
    if let Some(t) = c.clamp {
        if t < ch::arms::SNAP && t + dt >= ch::arms::SNAP {
            // The lines are tight on the arms.
            for fist in Fist::BOTH {
                spark(inv, shoulder(fist), GOLD, 18, 260.0)?;
                spark(inv, stage.home(fist) - Vec2::new(fist.side() * ch::arms::WRIST_LINE, 0.0), GOLD, 12, 200.0)?;
            }
            play(inv, SFX_STOMP, giant_pos)?;
            say(inv, &mut c, Bark::Clamped, head)?;
        }
        c.clamp = (t + dt < ch::arms::CLAMP).then_some(t + dt);
    }
    let arms_on = !c.loosed;
    // The blow of this beat: a punch while the arms are on the giant, or a
    // kick. Its pair is open from its tell to the end of its strike.
    let exit = Exit::of_code(c.blow_exit).filter(|_| {
        c.part().is_some_and(|p| p.mv == Move::PortalKick || (p.mv == Move::PortalPunch && arms_on))
    });
    let blow = exit.and(c.blow_kind());
    let blow_limb = blow.map(blows::limb);
    c.punch_open = exit.is_some();
    if let Some(blow_limb) = blow_limb.as_ref() {
        c.punch_near = a(from_giant(blow_limb.line));
        c.near_normal = [blow_limb.normal.x * turn, blow_limb.normal.y];
    }

    // ── The fists ──
    let part = c.part();
    let lead = fist_of_code(c.latch_lead).unwrap_or(Fist::Left);
    let cue = part.map(|p| Cue {
        mv: p.mv,
        striking: p.striking,
        t: p.t,
        dur: p.t + c.last_remaining,
        tel_dur: p.tel_dur,
        beat_t: p.beat_t,
        clock,
    });

    // The far line of a blow follows the player through the tell, then locks.
    if let (Some(exit), Some(_)) = (exit, cue.as_ref().filter(|cue| blows::tracking(cue))) {
        let (want, _) = blows::far_end(&stage, giant_pos, giant_size, blow.unwrap_or(Blow::Hammer), exit, c.blow_side, aim_now);
        let mut at = v(c.punch_far);
        // The line does not go through the giant: when the player is on its
        // other side, the line is there at once.
        if (want.x - giant_pos.x) * (at.x - giant_pos.x) < 0.0 {
            at = want;
        }
        let to = want - at;
        let step = blows::TRACK_SPEED * dt;
        c.punch_far = a(at + if to.length() <= step { to } else { to.normalize() * step });
    }
    // The pair eases open through the tell, and closed at the end.
    c.opening = cue
        .as_ref()
        .filter(|_| c.punch_open)
        .zip(c.blow_kind())
        .map_or(1.0, |(cue, blow)| blows::opening(cue, &blows::limb(blow)));
    // A blow that landed holds still.
    if c.freeze > 0.0 {
        c.freeze -= dt;
        c.lag += dt;
    }
    // The part of the strike the blow is in, and the part it was in last tick.
    // A limb that a blow met goes back into its line from there.
    let clashed_at = c.clashed_at;
    let blow_part = |t: f32| {
        cue.as_ref().filter(|cue| cue.striking).zip(blow_limb.as_ref()).map(|(cue, blow_limb)| {
            let now = t - c.lag;
            match clashed_at {
                Some(at) if now >= at => blows::Part::Recover((now - at).min(blow_limb.recover)),
                _ => blows::part(blow_limb, now, cue.dur - t),
            }
        })
    };
    let part_now = cue.as_ref().and_then(|cue| blow_part(cue.t));

    // ── The gnu's reflex: hurt its rider from its back and it throws you ──
    let hurt = c.last_health.is_some_and(|hp| scholar.health < hp);
    let hearts = c.last_health.map_or(0, |hp| hp - scholar.health);
    c.last_health = Some(scholar.health);
    c.reflex_cooldown = (c.reflex_cooldown - dt).max(0.0);
    let foe_on_back = giant.back.is_some_and(|[min_x, min_y, max_x, _]| {
        scholar.target_body.is_some_and(|body| {
            let feet = body.position[1] + body.size[1] * 0.5;
            (feet - min_y).abs() <= 4.0 && (min_x..=max_x).contains(&body.position[0])
        })
    });
    if foe_on_back && arms_on {
        say(inv, &mut c, Bark::Boarded, head)?;
    }
    // The fight on the back: the player is on it, or over it in a jump.
    let foe_over_back = giant.back.is_some_and(|[min_x, min_y, max_x, _]| {
        scholar.target_body.is_some_and(|body| {
            let feet = body.position[1] + body.size[1] * 0.5;
            (min_y - ch::arms::OVER_THE_BACK..=min_y + 4.0).contains(&feet) && (min_x..=max_x).contains(&body.position[0])
        })
    });
    // Where he is: his saddle (the giant carries him there each tick, so
    // `scholar_pos` is the saddle), or a place from it.
    let mut me = scholar_pos + c.off.map_or(Vec2::ZERO, v);
    let to_player = scholar.target_body.map(|body| v(body.position) - me);
    // The ends of the back, from the saddle: where he can stand.
    let ends = giant.back.map_or((0.0, 0.0), |[min_x, _, max_x, _]| (min_x - scholar_pos.x, max_x - scholar_pos.x));
    c.full_health = c.full_health.max(scholar.health);
    // With its arms on it the giant lets him take blows, and throws at the
    // third. Each blow launches him away from it.
    let struck = hurt && (foe_on_back || (arms_on && foe_over_back));
    let throws = if arms_on {
        if struck {
            c.back_blows += 1;
        }
        struck && c.back_blows >= ch::arms::BLOWS_BEFORE_BUCK
    } else {
        struck
    };
    if arms_on && struck {
        let away = if to_player.is_some_and(|to| to.x > 0.0) { -1.0 } else { 1.0 };
        let lost = 1.0 - scholar.health as f32 / c.full_health.max(1) as f32;
        c.off = Some(a(me - scholar_pos));
        c.air = a(ch::arms::launch(away, c.back_blows.max(1), hearts, lost));
        c.step_to(Footing::Launched);
        spark(inv, me, GOLD, 10, 220.0)?;
    }
    if throws && c.reflex.is_none() && c.reflex_cooldown <= 0.0 {
        c.reflex = Some(0.0);
        c.back_blows = 0;
        play(inv, SFX_SNORT, giant_pos)?;
    }
    // ── He is a body on the back: the launch, the landing, the way back ──
    c.footing_t += dt;
    match (c.footing(), c.off.map(v)) {
        (Footing::Launched, Some(at)) => {
            let vel = v(c.air) + Vec2::new(0.0, ch::arms::GRAVITY * dt);
            let mut to = at + vel * dt;
            // The walls of the hall, and its floor.
            to.x = to.x.clamp(hall.left + scholar_size.x * 0.5 - scholar_pos.x, hall.right - scholar_size.x * 0.5 - scholar_pos.x);
            let floor = hall.floor - scholar_size.y * 0.5 - scholar_pos.y;
            c.air = a(vel);
            let over_back = (ends.0..=ends.1).contains(&to.x);
            if vel.y > 0.0 && over_back && at.y <= 0.0 && to.y >= 0.0 {
                // He comes down on the back.
                to.y = 0.0;
                c.air = [0.0, 0.0];
                c.step_to(Footing::Land);
                spark(inv, scholar_pos + to + Vec2::new(0.0, scholar_size.y * 0.5), DUST, 8, 120.0)?;
            } else if vel.y > 0.0 && (to.y >= ch::arms::RECOVER_UNDER.min(floor) || to.y >= floor) {
                // The blow threw him off the back: he comes back to its
                // nearer end.
                to.y = to.y.min(floor);
                let near = if (to.x - ends.0).abs() < (to.x - ends.1).abs() { ends.0 + ch::arms::RECOVER_IN } else { ends.1 - ch::arms::RECOVER_IN };
                c.path_from = a(to);
                c.path_to = [near, 0.0];
                c.path_len = ch::arms::recover_time(to, Vec2::new(near, 0.0));
                c.air = [0.0, 0.0];
                c.step_to(Footing::Recover);
                play(inv, SFX_CHARGE, scholar_pos + to)?;
            }
            c.off = Some(a(to));
        }
        (Footing::Land, _) if c.footing_t >= ch::arms::LAND => c.step_to(Footing::Stand),
        (Footing::Recover, Some(_)) => {
            let u = c.footing_t / c.path_len.max(0.01);
            let at = ch::arms::recover(v(c.path_from), v(c.path_to), u);
            c.off = Some(a(at));
            if ticks % 3 == 0 {
                spark(inv, scholar_pos + at + Vec2::new(0.0, scholar_size.y * 0.5), GOLD, 2, 60.0)?;
            }
            if u >= 1.0 {
                c.step_to(Footing::Land);
            }
        }
        (Footing::Roll, Some(at)) => {
            c.off = Some([ch::arms::roll(c.path_from[0], c.path_to[0], c.footing_t), at.y]);
            if c.footing_t >= ch::arms::ROLL {
                c.step_to(Footing::Stand);
            }
        }
        (Footing::Home, Some(_)) => {
            c.off = Some(a(ch::arms::hop(v(c.path_from), Vec2::ZERO, c.footing_t)));
            if c.footing_t >= ch::arms::HOP {
                c.off = None;
                c.step_to(Footing::Stand);
            }
        }
        _ => {}
    }
    me = scholar_pos + c.off.map_or(Vec2::ZERO, v);
    let to_player = scholar.target_body.map(|body| v(body.position) - me);
    let on_his_feet = matches!(c.footing(), Footing::Stand | Footing::Walk);
    // The move the player plays, and whether he is near enough for it to
    // be a blow at him.
    let foe_swing = scholar.target_body.and_then(|body| body.swing);
    let foe_near = to_player.is_some_and(|to| to.x.abs() <= ch::arms::REACT_REACH && to.y.abs() <= ch::arms::REACT_REACH * 0.6);
    // ── The duel: with the player on the back, he fights ──
    let was_dueling = c.dueling;
    c.dueling = arms_on && foe_over_back && c.clamp.is_none();
    // He reads a blow that comes: with no move of his own, his book goes up
    // and stays up until the blow is over.
    let reads = c.dueling
        && on_his_feet
        && c.reflex.is_none()
        && c.duel_move == 0
        && foe_near
        && foe_swing.is_some_and(|s| s.since >= ch::arms::REACT && (s.live || s.hurts_in > 0.0));
    if !c.dueling || !on_his_feet || c.reflex.is_some() {
        // No player on the back, or he is in the air, or the giant throws:
        // no move.
        c.duel_move = 0;
        if c.footing() != Footing::Roll {
            c.duel_wait = c.duel_wait.max(if c.dueling { ch::arms::DUEL_STAGGER } else { ch::arms::DUEL_READY });
        }
        if c.footing() == Footing::Walk {
            c.step_to(Footing::Stand);
        }
        // The fight on the back is over: he goes to his saddle.
        let over = !arms_on || c.clamp.is_some() || c.footing_t >= ch::arms::HOME_AFTER;
        if !c.dueling && c.footing() == Footing::Stand && c.off.is_some() && over {
            c.path_from = c.off.unwrap_or_default();
            c.step_to(Footing::Home);
        }
    } else {
        if reads {
            // He does not start a move into a blow he reads.
            c.duel_wait = c.duel_wait.max(dt * 1.5);
        }
        c.duel_wait -= dt;
        let playing = (c.duel_move as usize).checked_sub(1);
        let since = playing.and_then(|i| ch::arms::DUEL_MOVES.get(i)).map_or(0.0, |(_, gap, _)| gap - c.duel_wait);
        // The player's blow is over and he is still in it: he strikes now,
        // if his own move and the time to punish it are over.
        let closed = playing.and_then(|i| ch::arms::DUEL_MOVES.get(i)).is_none_or(|(_, _, open)| since >= *open);
        let missed = foe_swing.is_some_and(|s| !s.live && s.hurts_in <= 0.0 && s.ends_in >= ch::arms::PUNISH_LEAST);
        if closed && missed && foe_near {
            c.duel_wait = 0.0;
        }
        // The jab's string: the player is still at his elbow.
        if let (Some(next), Some(to)) = (playing.and_then(ch::arms::string_next), to_player) {
            if since >= ch::arms::STRING_AT && to.x.abs() <= ch::arms::STRING_REACH && to.y >= -44.0 {
                c.duel_move = next as u32 + 1;
                c.duel_turn = c.duel_turn.wrapping_add(1);
                c.duel_wait = ch::arms::DUEL_MOVES[next].1;
            }
        }
        // The dash attack: he slides on, the way he ran.
        if playing == Some(ch::arms::DASH_ATTACK) && since < ch::arms::DASH_SLIDE + dt {
            let u = ch::ease(since / ch::arms::DASH_SLIDE);
            c.off = Some([c.path_from[0] + (c.path_to[0] - c.path_from[0]) * u, c.path_from[1]]);
        }
        if c.duel_wait <= 0.0 {
            let here = c.off.map_or(Vec2::ZERO, v);
            let turn = c.duel_turn;
            match to_player {
                // The player is at his elbow: he rolls through him.
                Some(to) if ch::arms::duel_rolls(to, turn) => {
                    let (low, high) = (ends.0 + ch::arms::EDGE, ends.1 - ch::arms::EDGE);
                    let past = here.x + to.x + to.x.signum() * ch::arms::ROLL_PAST;
                    // With no room behind the player, he rolls away from him.
                    let away = here.x - to.x.signum() * 2.0 * ch::arms::ROLL_PAST;
                    let goal = if (low..=high).contains(&past) { past } else { away.clamp(low, high) };
                    c.off = Some(a(here));
                    c.path_from = a(here);
                    c.path_to = [goal, 0.0];
                    c.duel_move = 0;
                    c.duel_turn = turn.wrapping_add(1);
                    c.duel_wait = ch::arms::ROLL_THEN;
                    c.step_to(Footing::Roll);
                }
                Some(to) => match ch::arms::duel_choice(to, turn) {
                    Some(choice) => {
                        c.duel_move = choice as u32 + 1;
                        c.duel_turn = turn.wrapping_add(1);
                        c.duel_wait = ch::arms::DUEL_MOVES[choice].1;
                        if c.footing() == Footing::Walk {
                            c.step_to(Footing::Stand);
                        }
                    }
                    // He ran, and the player is near: he strikes out of the run.
                    None if c.footing() == Footing::Walk
                        && c.air[0].abs() > ch::arms::WALK * dt * 1.5
                        && to.x.abs() <= ch::arms::DASH_ATTACK_FROM =>
                    {
                        c.duel_move = ch::arms::DASH_ATTACK as u32 + 1;
                        c.duel_turn = turn.wrapping_add(1);
                        c.duel_wait = ch::arms::DUEL_MOVES[ch::arms::DASH_ATTACK].1;
                        let to_x = here.x + to.x - to.x.signum() * ch::arms::DASH_SLIDE_TO;
                        c.path_from = a(here);
                        c.path_to = [to_x.clamp(ends.0 + ch::arms::EDGE, ends.1 - ch::arms::EDGE), here.y];
                        c.step_to(Footing::Stand);
                    }
                    // Out of his reach: he goes to the player.
                    None => {
                        c.duel_move = 0;
                        c.duel_wait = 0.0;
                        // A run that started is a run to its end.
                        let ran = c.footing() == Footing::Walk && c.air[0].abs() > ch::arms::WALK * dt * 1.5;
                        let speed = if ran || to.x.abs() > ch::arms::RUN_FROM { ch::arms::RUN } else { ch::arms::WALK };
                        let x = (here.x + to.x.signum() * speed * dt).clamp(ends.0 + ch::arms::EDGE, ends.1 - ch::arms::EDGE);
                        c.air = [x - here.x, 0.0];
                        c.off = Some([x, here.y]);
                        if c.footing() != Footing::Walk {
                            c.step_to(Footing::Walk);
                        }
                    }
                },
                None => {
                    c.duel_move = 0;
                    c.duel_wait = 0.15;
                }
            }
        }
        if c.footing() == Footing::Stand {
            // He stands in the fight: this is not the wait for the way home.
            c.footing_t = 0.0;
        }
    }
    // After his move and the time to punish it, his book is up until he
    // chooses again; and in the middle of a roll.
    let rolls_safe = c.footing() == Footing::Roll && (ch::arms::ROLL_SAFE.0..ch::arms::ROLL_SAFE.1).contains(&c.footing_t);
    c.guard = rolls_safe
        || reads
        || (c.dueling
            && on_his_feet
            && c.reflex.is_none()
            && (c.duel_move as usize).checked_sub(1).and_then(|i| ch::arms::DUEL_MOVES.get(i)).is_some_and(|(_, gap, open)| gap - c.duel_wait >= *open));
    inv.submit::<BossGuardPort>(BossGuard { guarded: c.guard })?;
    // The duel cut a punch short: the giant takes its arm back.
    if c.dueling && !was_dueling && previous.is_some_and(|p| matches!(p.mv, Move::PortalPunch | Move::PortalKick) && p.striking) {
        c.pull = Some(0.0);
    }
    if let Some(t) = c.pull {
        let recover = blows::limb(c.blow_kind().unwrap_or(Blow::Hammer)).recover;
        c.pull = (t + dt < recover).then_some(t + dt);
    }
    // Where he is when he is not in his saddle, and his speed there.
    let shoulder_pose = c.off.map(|off| scholar_pos + v(off));
    let shoulder_vel = match c.footing() {
        Footing::Launched => c.air,
        _ => [0.0, 0.0],
    };
    if let Some(t) = c.reflex {
        let next = t + dt;
        if t < REFLEX_TELL && next >= REFLEX_TELL {
            // The throw rides him where he is: on a shoulder, or between them.
            buck(inv, shoulder_pose.unwrap_or(scholar_pos), giant.back)?;
        }
        c.reflex = (next < REFLEX_LEN).then_some(next);
        if c.reflex.is_none() {
            c.reflex_cooldown = REFLEX_COOLDOWN;
        }
    }

    // ── Phase 3: the launch, and the fight in space ──
    let landed = hurt && c.reflex.is_none();
    let launch_t = space(inv, &mut c, &scholar, &giant, hall, dt, landed)?;

    // ── The giant walks between its blows, and looks round at its back ──
    // Anywhere on it (its back, the slope, a horn): it does not walk from
    // under him.
    let foe_on_giant = scholar.target_body.is_some_and(|body| {
        let feet = body.position[1] + body.size[1] * 0.5;
        (body.position[0] - giant_pos.x).abs() < giant_size.x * 0.5 + 20.0 && feet < hall.floor - 40.0
    });
    let free = cue.is_none()
        && c.reflex.is_none()
        && c.clamp.is_none()
        && !c.dueling
        && c.pull.is_none()
        && !foe_on_giant
        && launch_t.is_none()
        && arriving.is_none();
    c.walk_wait = (c.walk_wait - dt).max(0.0);
    if free && c.walk_to.is_none() && c.walk_wait <= 0.0 {
        c.walk_to = ch::stride::mark(giant_pos.x, giant_size.x * 0.5, aim_now.x, hall.left, hall.right);
        c.walk_t = 0.0;
    }
    if let Some(to) = c.walk_to {
        // It stops at its mark, when a beat begins, or when it cannot get there.
        if !free || (giant_pos.x - to).abs() <= ch::stride::ARRIVED || c.walk_t > ch::stride::STEP / ch::stride::SPEED * 2.0 {
            c.walk_to = None;
            c.walk_wait = ch::stride::PAUSE;
        }
    }
    match c.walk_to {
        Some(to) => {
            c.walk_t += dt;
            inv.submit::<MountWalkPort>(MountWalk { to_x: to, speed: ch::stride::SPEED })?;
        }
        None => inv.submit::<MountWalkPort>(MountWalk { to_x: giant_pos.x, speed: 0.0 })?,
    }
    c.look_t = if foe_on_back { c.look_t + dt } else { 0.0 };

    // ── What he and the gnu are drawn as ──
    let t = c.footing_t;
    match (c.clamp, c.footing()) {
        // He comes in and leaps onto the shoulders.
        _ if arriving.is_some() => draw(inv, DrawnPart::Body, Some(ch::arrival::scholar_rows(arriving.unwrap_or(0.0))))?,
        // He calls his power for the launch.
        _ if launch_t.is_some() => draw(inv, DrawnPart::Body, Some(ch::space::scholar_rows(launch_t.unwrap_or(0.0))))?,
        // A blow launched him: he is thrown, then he tumbles.
        (_, Footing::Launched) if t < ch::arms::LAUNCH_ROW => draw(inv, DrawnPart::Body, Some((&["launch", "hit_heavy", "hit"], t, false)))?,
        (_, Footing::Launched) => draw(inv, DrawnPart::Body, Some((&["fall", "tumble"], t - ch::arms::LAUNCH_ROW, true)))?,
        (_, Footing::Land) => draw(inv, DrawnPart::Body, Some((&["land", "brace"], t, false)))?,
        // He comes back to the back.
        (_, Footing::Recover) => draw(inv, DrawnPart::Body, Some((&["special_up", "brace"], t, false)))?,
        (_, Footing::Roll) => draw(inv, DrawnPart::Body, Some((&["roll", "spot_dodge", "brace"], t, false)))?,
        // He goes to his saddle.
        (_, Footing::Home) => draw(inv, DrawnPart::Body, Some((&["brace"], t, false)))?,
        // He calls the two lines that take the arms, and closes them.
        (Some(t), _) if t < ch::arms::SNAP => draw(inv, DrawnPart::Body, Some((&["invoke"], t, false)))?,
        (_, Footing::Walk) if c.air[0].abs() > ch::arms::WALK * dt * 1.5 => draw(inv, DrawnPart::Body, Some((&["run", "walk", "idle"], clock, true)))?,
        (_, Footing::Walk) => draw(inv, DrawnPart::Body, Some((&["walk", "idle"], clock, true)))?,
        // His book is up.
        _ if c.guard => draw(inv, DrawnPart::Body, Some((&["shield_hold", "brace"], clock, true)))?,
        // His stance in the fight.
        _ if c.dueling && cue.is_none() => draw(inv, DrawnPart::Body, Some((&["idle"], clock, true)))?,
        _ => draw(inv, DrawnPart::Body, ch::rows::scholar(cue.as_ref()))?,
    }
    let blow_row = blow.zip(cue.as_ref()).map(|(blow, cue)| blows::gnu(blow, arms_on, cue, part_now));
    match (c.reflex, c.clamp) {
        // He comes in: it looks round at him, and roars when he is up.
        _ if arriving.is_some() => draw(inv, DrawnPart::Mount, Some(ch::arrival::gnu_rows(arriving.unwrap_or(0.0), arms_on)))?,
        // The launch: it roars, then it floats.
        _ if launch_t.is_some() => draw(inv, DrawnPart::Mount, Some(ch::space::gnu_rows(launch_t.unwrap_or(0.0))))?,
        // The reflex rears and bucks, whatever his fists are doing.
        (Some(t), _) if arms_on => draw(inv, DrawnPart::Mount, Some((&["armed_buck", "buck"], t, false)))?,
        (Some(t), _) => draw(inv, DrawnPart::Mount, Some((&["buck"], t, false)))?,
        (None, Some(t)) if t < ch::arms::SNAP => draw(inv, DrawnPart::Mount, Some((&["clamp"], t, false)))?,
        // The lines closed on its arms: it flinches.
        (None, Some(t)) => draw(inv, DrawnPart::Mount, Some((&["hit"], t - ch::arms::SNAP, false)))?,
        // A blow through the pair: its tell, its strike, its hold, its way back.
        (None, None) if blow_row.is_some() => draw(inv, DrawnPart::Mount, blow_row)?,
        // It walks: forward the way its art faces, or back.
        (None, None) if c.walk_to.is_some() => {
            let forward = c.walk_to.is_some_and(|to| (to - giant_pos.x) * turn > 0.0);
            draw(inv, DrawnPart::Mount, Some((ch::stride::rows(forward, arms_on), c.walk_t, true)))?
        }
        // The player on its back: it looks round at him.
        (None, None) if c.look_t > 0.0 && cue.is_none() => {
            draw(inv, DrawnPart::Mount, Some((ch::stride::look_back(arms_on), c.look_t, false)))?
        }
        (None, None) if arms_on => match c.pull.filter(|_| cue.is_none()) {
            Some(t) => {
                let blow = c.blow_kind().unwrap_or(Blow::Hammer);
                draw(inv, DrawnPart::Mount, Some((blows::rows(blow, true)[3], t, false)))?
            }
            None => draw(inv, DrawnPart::Mount, Some(ch::arms::gnu(cue.as_ref(), clock)))?,
        },
        (None, None) => draw(inv, DrawnPart::Mount, ch::rows::gnu(cue.as_ref()))?,
    }
    let mut poses = [Pose::idle(Vec2::ZERO); 2];
    for fist in Fist::BOTH {
        let i = fist.index();
        if let Some(cue) = cue.as_ref() {
            if ch::tracking(cue, lead, fist) {
                let to = aim_now - v(c.aims[i]);
                let step = ch::demonstrate::TRACK_SPEED * dt;
                c.aims[i] = a(v(c.aims[i]) + if to.length() <= step { to } else { to.normalize() * step });
            }
        }
        let latch = Latch { aim: v(c.aims[i]), ..c.latch() };
        poses[i] = match cue.as_ref() {
            Some(cue) => ch::perform(&stage, &latch, fist, cue),
            None => {
                let home = ch::idle(&stage, fist, clock).pos;
                let ease = ch::ease(c.resting / 0.6);
                Pose::idle(latch.from[i] + (home - latch.from[i]) * ease)
            }
        };
    }

    // ── The limbs on the giant: its fists while its arms are on it, and its hoof ──
    // The giant's picture draws each whole, its arm or its leg too; the limb's
    // body is where its fist (its hoof) is, posed as if there were no portal:
    // past the near line while the limb is in it, and the pair carries the
    // body, its strike and the giant's picture of it to the far line
    // (`docs/planning/engine/portals-a-body-in-two-places.md`). A limb that
    // is not on its way through waits inside the giant. Its own sheet draws
    // nothing. It is no opening, but for a hammer's fist in the floor: that
    // is the punish window of phase 1, as a fist that sticks is in phase 2.
    let near = v(c.punch_near);
    let way = v(c.far_normal);
    // The tell's last pull: a star where the limb will come out.
    if let (Some(cue), Some(_)) = (cue.as_ref().filter(|cue| !cue.striking), blow) {
        if !c.glinted && blows::tell(cue) >= blows::HITCH_AT {
            c.glinted = true;
            effect(inv, FX_GLINT, v(c.punch_far), GLINT_SCALE, way)?;
        }
    }
    let mut quake_at = None;
    let inward = -v(c.near_normal);
    let mut on_giant: Vec<(usize, &str, f32)> =
        Fist::BOTH.into_iter().filter(|_| arms_on).map(|fist| (fist.index(), FIST_SLOTS[fist.index()], fist.side())).collect();
    on_giant.push((HOOF, blows::HOOF, 1.0));
    for (i, slot, facing) in on_giant {
        let Some(body) = giant.limbs.iter().find(|l| l.slot == slot) else {
            continue;
        };
        let driven = blow_limb.as_ref().filter(|l| l.slot == slot).zip(part_now);
        let depth = driven.and_then(|(l, part)| blows::depth(l, part));
        let pos = depth.map_or(giant_pos, |d| near + inward * d);
        let vel = match (depth, c.last_pos[i]) {
            (Some(_), Some(last)) => (pos - v(last)) / dt,
            _ => Vec2::ZERO,
        };
        inv.submit::<LimbPosePort>(LimbPose { slot: slot.into(), hold: Some(LimbHold { position: a(pos), velocity: a(vel), facing }) })?;
        draw(inv, DrawnPart::Limb(slot.into()), Some(ch::arms::NO_FIST))?;
        // A hammer's fist in the floor, held there: its first blow passes on
        // to him, as a stuck fist's does in phase 2.
        let stuck = driven.is_some_and(|(_, part)| matches!(part, blows::Part::Hold(_)))
            && c.floored
            && blow.is_some_and(Blow::is_hammer);
        // Just out of its far line, before it hurts, a blow of the player's
        // meets it (`ch::clash`).
        let meetable = c.clashed_at.is_none() && driven.is_some_and(|(l, part)| blows::meetable(l, part));
        let open = (stuck && !c.blow_spent) || meetable;
        // Where it is at the far end of the pair.
        let far_fist = v(c.punch_far) + way * depth.unwrap_or(0.0);
        if body.lost_health > 0 && meetable {
            // Equal and opposite: the limb goes back into its line, the
            // player is thrown the way it came, the giant rocks back, and the
            // force goes up the arm to its rider.
            let pass = !scholar.invulnerable;
            inv.submit::<LimbBlowPort>(LimbBlow { slot: slot.into(), pass })?;
            if let Some((l, part)) = driven {
                c.clashed_at = blows::strike_time(l, part);
                c.freeze = ch::clash::FREEZE;
                let kind = blow.unwrap_or(Blow::Hammer);
                let from_fist = scholar.target_body.map_or(Vec2::ZERO, |b| v(b.position) - far_fist);
                inv.submit::<FoeLaunchPort>(FoeLaunch { velocity: ch::clash::foe_launch(kind, way, from_fist), flinchless: true })?;
                let away = giant_pos.x - aim_now.x;
                inv.submit::<MountLaunchPort>(MountLaunch { velocity: ch::clash::giant_recoil(kind, away), flinchless: true })?;
                inv.submit::<CameraShakePort>(CameraShake { amplitude_px: ch::clash::SHAKE })?;
                effect(inv, FX_CLASH, far_fist, CLASH_SCALE, -way)?;
                spark(inv, far_fist, FLASH, 30, 640.0)?;
                burst(inv, far_fist, "shard", 12, 520.0, [1.0, 0.9, 0.6, 1.0])?;
                play(inv, SFX_LAND, far_fist)?;
                say(inv, &mut c, Bark::Clash, head)?;
            }
        } else if body.lost_health > 0 {
            let pass = open && !scholar.invulnerable;
            inv.submit::<LimbBlowPort>(LimbBlow { slot: slot.into(), pass })?;
            if pass {
                c.blow_spent = true;
                spark(inv, far_fist, GOLD, 18, 280.0)?;
            }
        }
        inv.submit::<LimbOpeningPort>(LimbOpening { slot: slot.into(), open })?;
        if open && ticks % 12 == 0 {
            // It glints while it is an opening.
            spark(inv, far_fist - Vec2::new(0.0, body.size[1] * 0.5), GOLD, 2, 60.0)?;
        }
        match driven.filter(|(l, part)| blows::hurts(l, *part)) {
            Some((l, _)) => {
                if !c.armed[i] {
                    c.armed[i] = true;
                    c.armed_at[i] = clock;
                    c.fist_generation[i] = c.fist_generation[i].wrapping_add(1);
                }
                // The limb carries its strike: where it is through the pair,
                // the strike hurts there.
                inv.submit::<LimbStrikePort>(LimbStrike {
                    slot: slot.into(),
                    generation: c.fist_generation[i],
                    half_extent: a(l.strike_half),
                    damage: l.damage,
                    knockback: l.knockback,
                })?;
            }
            None => c.armed[i] = false,
        }
        if let (Some((l, part)), Some(cue), Some(kind)) = (driven, cue.as_ref(), blow) {
            // The lines of speed as it comes out of the far line.
            if let blows::Part::Strike(t) = part {
                if blows::comes_through(l, t - dt, t) {
                    effect(inv, FX_STREAK, v(c.punch_far), STREAK_SCALE, way)?;
                }
            }
            if let Some(arm) = kind.arm() {
                // An arm's blow ends now (or later): the next blow soon after
                // is the other arm's.
                c.last_arm = arm.code();
                c.last_blow_end = clock;
            }
            quake_at = impact(inv, &mut c, l, kind, body, cue, dt, hall.floor, scholar.target_body.map(|b| v(b.position)))?;
        }
        if i < 2 {
            poses[i] = Pose::idle(pos);
            c.last_stuck[i] = false;
        }
        c.last_pos[i] = Some(a(pos));
    }
    // Hurt, he puts more into the hammer: it sends a shock along the floor
    // each way, as the giant's stomp does.
    let worn = scholar.health as f32 <= c.full_health.max(1) as f32 * QUAKE_UNDER;
    if let Some(at) = quake_at.filter(|_| worn) {
        for dir in [-1.0, 1.0] {
            let Some(slot) = c.wave_x.iter().position(Option::is_none) else {
                break;
            };
            c.wave_x[slot] = Some(at.x + dir * blows::limb(Blow::Hammer).strike_half.x);
            c.wave_dir[slot] = dir;
            c.wave_generation[slot] = c.wave_generation[slot].wrapping_add(1);
        }
    }

    // ── The fists that are bodies of their own ──
    for fist in Fist::BOTH.into_iter().filter(|_| !arms_on) {
        let i = fist.index();
        let pose = poses[i];
        let Some(body) = limb(fist) else {
            continue;
        };
        let slot = FIST_SLOTS[i].to_string();
        let vel = c.last_pos[i].map_or(Vec2::ZERO, |last| (pose.pos - v(last)) / dt);
        // Chirality by anatomy: the sheet draws the fist wrist-left,
        // knuckles-right. The right fist is drawn as authored and the left as
        // its mirror, whichever side of the gnu a move carries it to.
        inv.submit::<LimbPosePort>(LimbPose {
            slot: slot.clone(),
            hold: Some(LimbHold { position: a(pose.pos), velocity: a(vel), facing: fist.side() }),
        })?;
        draw(inv, fist_part(fist), Some(ch::rows::fist(&pose, clock)))?;

        // Each time it sticks, a fist is a fresh opening.
        let landed = pose.stuck && !(c.last_pos[i].is_some() && c.last_stuck[i]);
        if landed {
            c.spent[i] = false;
        }
        // Its first blow lands on him; then it is spent until it sticks again.
        if body.lost_health > 0 {
            let passes = !scholar.invulnerable && !c.spent[i];
            inv.submit::<LimbBlowPort>(LimbBlow { slot: slot.clone(), pass: passes })?;
            if passes {
                c.spent[i] = true;
            }
        }
        // Only a stuck fist with its blow still to pass on can be struck.
        inv.submit::<LimbOpeningPort>(LimbOpening { slot, open: pose.stuck && !c.spent[i] })?;

        // Its hit volume.
        if pose.harmful {
            if !c.armed[i] || clock - c.armed_at[i] >= REARM {
                c.armed[i] = true;
                c.armed_at[i] = clock;
                c.fist_generation[i] = c.fist_generation[i].wrapping_add(1);
            }
            inv.submit::<HeldDamageBoxPort>(HeldDamageBox {
                slot: FIST_BOX_SLOT + i as u32,
                generation: c.fist_generation[i],
                center: a(pose.pos),
                half_extent: a(v(body.size) * 0.5 * FIST_HIT_SCALE),
                damage: FIST_DAMAGE,
                knockback: FIST_KNOCKBACK,
                lifetime_s: REARM + 0.5,
            })?;
        } else {
            c.armed[i] = false;
        }

        // Impact.
        if landed {
            let at = Vec2::new(pose.pos.x, hall.floor);
            let heavy = pose.harmful;
            let (count, speed) = if heavy { (22, 300.0) } else { (8, 140.0) };
            dust(inv, at, count, speed, [0.6, 0.5, 0.4, 1.0])?;
            if heavy {
                play(inv, SFX_STOMP, at)?;
            }
        }
        // A stuck fist glints while it is an opening.
        if pose.stuck && !c.spent[i] && ticks % 12 == 0 {
            spark(inv, pose.pos - Vec2::new(0.0, body.size[1] * 0.5), GOLD, 2, 60.0)?;
        }
        c.last_pos[i] = Some(a(pose.pos));
        c.last_stuck[i] = pose.stuck;
    }

    // ── The lines that are a look: one at each shoulder, one behind each wrist ──
    let shoulders = c.loosed || c.clamp.is_some();
    // When he is dazed his spell has no hold: the lines come open, and flicker.
    let dazed = cue.as_ref().is_some_and(|cue| {
        cue.mv == Move::Eureka && cue.striking && ch::eureka::part(cue.t, cue.dur) == ch::eureka::Part::Dazed
    });
    let tight = ch::arms::tight(c.clamp) - if dazed { ch::arms::dazed_slack(clock) } else { 0.0 };
    for fist in Fist::BOTH {
        let i = fist.index();
        c.line_half[i] = ch::arms::SHOULDER_WIDE + (ch::arms::SHOULDER_TIGHT - ch::arms::SHOULDER_WIDE) * tight;
        c.line_half[2 + i] = ch::arms::WRIST_WIDE + (ch::arms::WRIST_TIGHT - ch::arms::WRIST_WIDE) * tight;
        c.lines[i] = shoulders.then(|| a(shoulder(fist)));
        // The wrist of a fist is on the side of it that is not its own side.
        let wrist = poses[i].pos - Vec2::new(fist.side() * ch::arms::WRIST_LINE, 0.0);
        c.lines[2 + i] = (c.loosed && limb(fist).is_some()).then(|| a(wrist));
    }

    // ── Telegraph ink ──
    if let (Some(cue), true) = (cue.as_ref(), ticks % 3 == 0) {
        ink(inv, &c, cue, lead, &stage, &poses, fist_size)?;
    }
    // The cradle's clack.
    if let Some(cue) = cue.as_ref() {
        if cue.mv == Move::Cradle && cue.striking && ch::cradle::clacks((cue.t - dt).max(0.0), cue.t, cue.dur) {
            let at = (poses[0].pos + poses[1].pos) * 0.5;
            spark(inv, at, GOLD, 14, 260.0)?;
            play(inv, SFX_STOMP, at)?;
        }
    }

    // ── His arrival: from the ledge by the top door to the saddle ──
    if let Some(t) = arriving {
        let half_h = scholar_size.y * 0.5;
        let ledge = Vec2::new(hall.left + ch::arrival::LEDGE_IN, hall.floor - ch::arrival::LEDGE_OVER_FLOOR - half_h);
        let saddle = scholar_pos;
        let before = t - dt;
        if before < ch::arrival::STAND && t >= ch::arrival::STAND * 0.2 && before < ch::arrival::STAND * 0.2 {
            inv.submit::<SpeechBubblePort>(SpeechBubble {
                at: a(ledge - Vec2::new(0.0, half_h + 30.0)),
                text: "You'll not catch me! GNU, ARISE!".to_string(),
            })?;
        }
        let landed_at = ch::arrival::STAND + ch::arrival::LEAP;
        if before < landed_at && t >= landed_at {
            effect(inv, FX_POWER, saddle, POWER_SCALE, Vec2::X)?;
            inv.submit::<CameraShakePort>(CameraShake { amplitude_px: 22.0 })?;
            play(inv, SFX_CHARGE, saddle)?;
            play(inv, SFX_SNORT, giant_pos)?;
            dust(inv, saddle + Vec2::new(0.0, half_h), 16, 220.0, [0.62, 0.52, 0.42, 1.0])?;
            inv.submit::<SpeechBubblePort>(SpeechBubble {
                at: a(saddle - Vec2::new(0.0, half_h + 30.0)),
                text: "Behold: the shoulders of a GIANT!".to_string(),
            })?;
        }
        match ch::arrival::place(t, ledge, saddle) {
            Some(at) => {
                inv.submit::<ConductedPosePort>(ConductedPose {
                    pose: Some(BodyPose { position: a(at), velocity: [0.0, 0.0] }),
                    side: None,
                })?;
                c.posed = true;
            }
            None if c.posed => {
                inv.submit::<ConductedPosePort>(ConductedPose { pose: None, side: None })?;
                c.posed = false;
            }
            None => {}
        }
    }
    // ── Eureka: the scholar himself leaves the saddle ──
    match cue.filter(|cue| cue.mv == Move::Eureka && cue.striking && arriving.is_none()) {
        Some(cue) => {
            let saddle = scholar_pos;
            let head = saddle - Vec2::new(0.0, scholar_size.y * 0.5);
            match ch::eureka::part(cue.t, cue.dur) {
                ch::eureka::Part::AppleFalling(f) => {
                    let apple = head - Vec2::new(0.0, 320.0 * (1.0 - f * f));
                    spark(inv, apple, GOLD, 3, 6.0)?;
                    if cue.t + dt >= ch::eureka::APPLE {
                        spark(inv, head, GOLD, 30, 280.0)?;
                        play(inv, SFX_SNORT, head)?;
                        inv.submit::<SpeechBubblePort>(SpeechBubble {
                            at: a(head - Vec2::new(0.0, 30.0)),
                            text: "Eureka!".to_string(),
                        })?;
                    }
                }
                ch::eureka::Part::Dazed => {
                    if ticks % 4 == 0 {
                        let top = v(c.landing) - Vec2::new(0.0, scholar_size.y * 0.5 + 12.0);
                        for k in 0..3 {
                            let angle = clock * 5.0 + k as f32 * std::f32::consts::TAU / 3.0;
                            spark(inv, top + Vec2::new(angle.cos() * 26.0, angle.sin() * 8.0), GOLD, 1, 4.0)?;
                        }
                    }
                }
                _ => {}
            }
            let pos = ch::eureka::scholar(saddle, v(c.landing), cue.t, cue.dur);
            inv.submit::<ConductedPosePort>(ConductedPose {
                pose: Some(BodyPose { position: a(pos), velocity: [0.0, 0.0] }),
                // His facing is the gnu's rider's, not the conductor's.
                side: None,
            })?;
            c.posed = true;
        }
        None => match shoulder_pose {
            // His arrival poses him.
            _ if arriving.is_some() => {}
            // He is on the giant's other shoulder, or on his way to one.
            Some(at) => {
                inv.submit::<ConductedPosePort>(ConductedPose {
                    pose: Some(BodyPose { position: a(at), velocity: shoulder_vel }),
                    side: None,
                })?;
                c.posed = true;
            }
            None if c.posed => {
                // He is back in the saddle: the gnu carries him again.
                inv.submit::<ConductedPosePort>(ConductedPose { pose: None, side: None })?;
                c.posed = false;
            }
            None => {}
        },
    }

    // ── The stomp's shocks roll out along the floor ──
    for slot in 0..WAVES {
        let Some(x) = c.wave_x[slot] else {
            continue;
        };
        let x = x + c.wave_dir[slot] * WAVE_SPEED * dt;
        if x < hall.left + WAVE_HALF.x || x > hall.right - WAVE_HALF.x {
            c.wave_x[slot] = None;
            continue;
        }
        c.wave_x[slot] = Some(x);
        let center = Vec2::new(x, hall.floor - WAVE_HALF.y);
        inv.submit::<HeldDamageBoxPort>(HeldDamageBox {
            slot: WAVE_BOX_SLOT + slot as u32,
            generation: c.wave_generation[slot],
            center: a(center),
            half_extent: a(WAVE_HALF),
            damage: WAVE_DAMAGE,
            knockback: WAVE_KNOCKBACK,
            lifetime_s: WAVE_LIFETIME_S,
        })?;
        if ticks % 2 == 0 {
            dust(inv, center + Vec2::new(0.0, WAVE_HALF.y * 0.5), 3, 120.0, [0.62, 0.52, 0.4, 1.0])?;
        }
    }
    hold_punch_pair(inv, &c)?;
    hold_look_lines(inv, &c)?;
    bond_fists(inv, &c, &giant)?;
    c.store(inv)
}

/// The trail of each fist is part of his spell: it joins the two lines the
/// arm is in, from the shoulder's line (after phase 1). A fist on its arm
/// has none: the arm joins it. A loose fist with no line has its trail from
/// the giant: no start is asked for.
fn bond_fists(inv: &mut Invocation<'_>, c: &Conductor, giant: &MountView) -> Result<(), Fault> {
    let lines = c.look_lines();
    for fist in Fist::BOTH {
        let i = fist.index();
        if !giant.limbs.iter().any(|l| l.slot == FIST_SLOTS[i]) {
            continue;
        }
        // A fist on its arm (phase 1, through the punch's pair or not) is
        // joined by the arm, which the giant draws: it has no trail.
        let start = if c.loosed {
            match lines[i] {
                Some((at, _)) => Some(a(at)),
                None => continue,
            }
        } else {
            None
        };
        inv.submit::<LimbBondPort>(LimbBond { slot: FIST_SLOTS[i].into(), start })?;
    }
    // The hoof is on its leg, which the giant draws: it has no trail.
    if giant.limbs.iter().any(|l| l.slot == blows::HOOF) {
        inv.submit::<LimbBondPort>(LimbBond { slot: blows::HOOF.into(), start: None })?;
    }
    Ok(())
}

/// The colour of each look line, in the order of `look_lines`. A line has
/// the colour of the end of the punch it looks like: a shoulder's is blue
/// (24), a wrist's is orange (25).
const LOOK_LINE_LOOKS: [u8; 4] = [24, 24, 25, 25];
/// The side of each look line that the room is on. The giant is between his
/// two shoulders' lines; a fist is out from its wrist's line, away from him.
const LOOK_LINE_NORMALS: [[f32; 2]; 4] = [[1.0, 0.0], [-1.0, 0.0], [-1.0, 0.0], [1.0, 0.0]];

/// The lines at his shoulders and behind his wrists are a look and no way
/// through: ask the portal domain to draw each one that the record has, in
/// the slot of its place in `look_lines`. Each is ON its arm's cut: a
/// shoulder's on the stump the giant's picture publishes, a wrist's on the
/// cut of its fist's picture. The presentation puts it there, as wide as the
/// arm, and over it.
fn hold_look_lines(inv: &mut Invocation<'_>, c: &Conductor) -> Result<(), Fault> {
    for (slot, placed) in c.look_lines().into_iter().enumerate() {
        let Some((at, half)) = placed else {
            continue;
        };
        inv.submit::<LightLinePort>(LightLine {
            slot: slot as u32,
            at: a(at),
            normal: LOOK_LINE_NORMALS[slot],
            length: half * 2.0,
            look: LOOK_LINE_LOOKS[slot],
            on_limb: Some(LineOnLimb { limb: FIST_SLOTS[slot % 2].to_owned(), on_its_body: slot >= 2 }),
        })?;
    }
    Ok(())
}

/// The pair of a blow is a real pair of portals: ask the portal domain to
/// keep it for this tick, where the record says its lines are, as open as the
/// record says. The giant's limb goes into the near line, and comes out of
/// the far line; a body that goes into the far line comes out of the near
/// line. Its ends are blue and orange (channels 24 and 25), a look only.
fn hold_punch_pair(inv: &mut Invocation<'_>, c: &Conductor) -> Result<(), Fault> {
    let (Some([near, far]), Some(blow)) = (c.punch_pair(), c.blow_kind()) else {
        return Ok(());
    };
    // A pair that is not open is not asked for: it is closed.
    if c.opening <= 0.0 {
        return Ok(());
    }
    let half = blows::limb(blow).half * c.opening.min(1.0);
    let end = |at: Vec2, normal: [f32; 2]| PortalEnd { at: a(at), normal, half_length: half, half_thickness: None };
    inv.submit::<HeldPortalPairPort>(HeldPortalPair {
        slot: PUNCH_PAIR_SLOT,
        ends: [end(near, c.near_normal), end(far, c.far_normal)],
        looks: [24, 25],
        closes_on_use: false,
    })
}

/// Phase 3 (`ch::space`): the launch in the beat before the enrage, and the
/// fight in space after it. Returns the seconds into the launch while it
/// plays.
///
/// The beat before the enrage is the encounter's `between_phases` with his
/// health under half (the beat before phase 2 is at two thirds of it).
fn space(
    inv: &mut Invocation<'_>,
    c: &mut Conductor,
    scholar: &BossConduct,
    giant: &MountView,
    hall: Hall,
    dt: f32,
    landed: bool,
) -> Result<Option<f32>, Fault> {
    use ch::space::{self as sp, Sea};
    let low = scholar.health * 2 < c.full_health;
    let launching = scholar.between_phases && low && c.loosed;
    // Out of the enrage (a fight that was put back) is out of space.
    if c.space_t.is_some() && !scholar.enraged && !launching {
        c.space_t = None;
        c.ascent = 0.0;
        return Ok(None);
    }
    let prev = match c.space_t {
        Some(t) => t,
        // A fight that is in its enrage with no launch before it goes
        // straight to space.
        None if scholar.enraged => sp::LEN,
        None if launching => -dt,
        None => return Ok(None),
    };
    let t = prev + dt;
    c.space_t = Some(t);
    let crossed = |at: f32| prev < at && t >= at;
    let giant_pos = v(giant.position);
    let giant_size = v(giant.size);
    let head = v(scholar.position) - Vec2::new(0.0, scholar.size[1] * 0.5);
    let feet = Vec2::new(giant_pos.x, hall.floor);
    c.ascent = sp::ascent(t);
    if crossed(sp::POWER) {
        effect(inv, FX_POWER, v(scholar.position), POWER_SCALE, Vec2::X)?;
        play(inv, SFX_CHARGE, head)?;
        inv.submit::<SpeechBubblePort>(SpeechBubble {
            at: a(head - Vec2::new(0.0, 30.0)),
            text: "Every body pulls on every body...".to_string(),
        })?;
    }
    // The well: a pair of portals, the floor's and the one over it, and a
    // well of gravity at the floor's. A body falls into the floor and out of
    // the portal over it, and into the floor again: the hall falls through
    // itself until the pair closes, and then it is in space.
    let [(floor_at, floor_normal), (over_at, over_normal)] = sp::well_portals(hall);
    let open = sp::well_open(t);
    if open > 0.0 {
        let end = |at: [f32; 2], normal: [f32; 2]| PortalEnd {
            at,
            normal,
            half_length: sp::PORTAL_HALF * open,
            half_thickness: None,
        };
        inv.submit::<HeldPortalPairPort>(HeldPortalPair {
            slot: WELL_PAIR_SLOT,
            ends: [end(floor_at, floor_normal), end(over_at, over_normal)],
            looks: [24, 25],
            closes_on_use: false,
        })?;
    }
    let pull = sp::well_pull(t);
    if pull > 0.0 {
        inv.submit::<HeldForcePort>(HeldForce {
            slot: WELL_FIELD,
            min: [hall.left - 200.0, hall.floor - 4000.0],
            max: [hall.right + 200.0, hall.floor + 64.0],
            lift: 0.0,
            force: [0.0, 0.0],
            well: Some(GravityWell { at: floor_at, pull, near: sp::WELL_NEAR }),
        })?;
    }
    if crossed(sp::LIFTOFF) {
        let size = FX_SIZE * LIFTOFF_SCALE;
        effect(inv, FX_LIFTOFF, feet - Vec2::new(0.0, size * 0.5), LIFTOFF_SCALE, Vec2::NEG_Y)?;
        inv.submit::<CameraShakePort>(CameraShake { amplitude_px: 30.0 })?;
        play(inv, SFX_STOMP, feet)?;
        play(inv, SFX_FLOOR, feet)?;
        for side in [-1.0, 1.0] {
            dust(inv, feet + Vec2::new(side * giant_size.x * 0.4, 0.0), 24, 420.0, [0.75, 0.62, 0.45, 1.0])?;
        }
        inv.submit::<SpeechBubblePort>(SpeechBubble {
            at: a(head - Vec2::new(0.0, 30.0)),
            text: "...and MINE pulls hardest!".to_string(),
        })?;
    }
    if crossed(sp::CLOSE) {
        inv.submit::<CameraShakePort>(CameraShake { amplitude_px: 16.0 })?;
        play(inv, SFX_WHOOSH, Vec2::from(over_at))?;
    }
    if crossed(sp::HELMETS) && !c.helmets {
        c.helmets = true;
        for who in HELMET_WEARERS {
            inv.submit::<WearPort>(Wear { who, gear: HELMET.into(), on: true })?;
        }
        spark(inv, head, [0.75, 0.9, 1.0, 1.0], 14, 200.0)?;
    }
    if crossed(sp::LEN) {
        inv.submit::<SpeechBubblePort>(SpeechBubble {
            at: a(head - Vec2::new(0.0, 30.0)),
            text: "In space, nothing stops you!".to_string(),
        })?;
    }
    // The field over the hall: the weight space takes, and the tide.
    let (lift, push) = sp::field(t);
    if lift > 0.0 || push != [0.0, 0.0] {
        hold_space_field(inv, hall, lift, push)?;
    }
    // A tide is told before it comes, and is seen as it pushes.
    match sp::sea(t) {
        Sea::Warning(tide) if matches!(sp::sea(prev), Sea::Calm) => {
            inv.submit::<SpeechBubblePort>(SpeechBubble { at: a(head - Vec2::new(0.0, 30.0)), text: tide.bark().to_string() })?;
            play(inv, SFX_WHOOSH, head)?;
        }
        Sea::Tide(tide, _) if c.ticks % 6 == 0 => {
            // Puffs in the hall that go the way the tide pushes.
            let way = Vec2::from(tide.way());
            let k = (c.ticks / 6) % 5;
            let across = hall.left + (hall.right - hall.left) * (0.1 + 0.2 * k as f32);
            let at = Vec2::new(across, hall.floor - 160.0 - 180.0 * ((c.ticks / 30) % 4) as f32);
            effect(inv, FX_PUFF, at, PUFF_SCALE, way)?;
        }
        _ => {}
    }
    // Every blow that lands on him pushes the player back: the third law.
    c.recoil_wait = (c.recoil_wait - dt).max(0.0);
    if landed && t >= sp::LEN && c.recoil_wait <= 0.0 {
        if let Some(body) = scholar.target_body {
            let from = v(scholar.position).x;
            inv.submit::<FoeLaunchPort>(FoeLaunch { velocity: sp::recoil(body.position[0], from), flinchless: true })?;
            effect(inv, FX_PUFF, v(body.position), PUFF_SCALE, Vec2::new(if body.position[0] < from { -1.0 } else { 1.0 }, 0.0))?;
            c.recoil_wait = sp::RECOIL_WAIT;
        }
    }
    Ok((t < sp::LEN).then_some(t.max(0.0)))
}

/// The field of space over the hall, from wall to wall and from the floor
/// up past the top of the room.
fn hold_space_field(inv: &mut Invocation<'_>, hall: Hall, lift: f32, push: [f32; 2]) -> Result<(), Fault> {
    inv.submit::<HeldForcePort>(HeldForce {
        slot: SPACE_FIELD,
        min: [hall.left - 200.0, hall.floor - 4000.0],
        max: [hall.right + 200.0, hall.floor + 64.0],
        lift,
        force: push,
        well: None,
    })
}

/// Beaten (`ch::finale`): he falls from its shoulder, and lets his fists go.
/// The giant gathers him to its chest and holds him, its head low; it roars;
/// an apple falls on his head, he wakes, and it sets him back on its
/// shoulder, where the two of them are at rest.
fn finale(
    inv: &mut Invocation<'_>,
    c: &mut Conductor,
    scholar: &BossConduct,
    giant: &MountView,
    hall: Hall,
    dt: f32,
) -> Result<(), Fault> {
    use ch::finale::{self as fin, Part as FinalePart};
    let giant_pos = v(giant.position);
    let giant_size = v(giant.size);
    let turn = if giant.unmirrored { 1.0 } else { -1.0 };
    let half_h = scholar.size[1] * 0.5;
    let prev = c.finale_t;
    let t = match prev {
        Some(t) => t + dt,
        None => {
            c.finale_from = scholar.position;
            0.0
        }
    };
    c.finale_t = Some(t);
    // The moment `at` of the end passes in this tick.
    let crossed = |at: f32| match prev {
        Some(p) => p < at && t >= at,
        None => at <= 0.0,
    };
    let floor_x = (giant_pos.x + turn * fin::FALLS).clamp(hall.left + 40.0, hall.right - 40.0);
    let floor = Vec2::new(floor_x, hall.floor - half_h);
    let cradle = giant_pos + Vec2::new(fin::CRADLE.x * turn, fin::CRADLE.y);
    // His soles on the saddle (its catalog `saddle`, from its middle).
    let saddle = giant_pos + Vec2::new(256.0 * turn, -232.05 - half_h);
    match fin::scholar(t, v(c.finale_from), floor, cradle, saddle) {
        Some(at) => {
            inv.submit::<ConductedPosePort>(ConductedPose {
                pose: Some(BodyPose { position: a(at), velocity: [0.0, 0.0] }),
                side: None,
            })?;
            c.posed = true;
        }
        None if c.posed => {
            inv.submit::<ConductedPosePort>(ConductedPose { pose: None, side: None })?;
            c.posed = false;
        }
        None => {}
    }
    draw(inv, DrawnPart::Body, Some(fin::scholar_rows(t)))?;
    draw(inv, DrawnPart::Mount, Some(fin::gnu_rows(t)))?;
    // Beaten in space, they come back down to the ground as he falls: the
    // sky and the weight come back by the time the giant holds him.
    if c.space_t.is_some() {
        let back = ch::ease(t / (fin::FALL + fin::GATHER));
        c.ascent = c.ascent.min(1.0 - back);
        let lift = ch::space::LIFT * (1.0 - back);
        if lift > 0.0 {
            hold_space_field(inv, hall, lift, [0.0, 0.0])?;
        }
    }
    inv.submit::<MountWalkPort>(MountWalk { to_x: giant_pos.x, speed: 0.0 })?;
    // His fists: he lets them go as he falls; then the giant draws them back
    // into its arms, which hold him.
    let falling = matches!(fin::part(t), FinalePart::Fall(_));
    for fist in Fist::BOTH {
        let i = fist.index();
        c.armed[i] = false;
        if !giant.limbs.iter().any(|l| l.slot == FIST_SLOTS[i]) {
            continue;
        }
        if falling {
            draw(inv, fist_part(fist), Some((&["hit", "rest"], 0.0, false)))?;
            inv.submit::<LimbPosePort>(LimbPose { slot: FIST_SLOTS[i].into(), hold: None })?;
        } else {
            let hold = LimbHold { position: giant.position, velocity: [0.0, 0.0], facing: fist.side() };
            inv.submit::<LimbPosePort>(LimbPose { slot: FIST_SLOTS[i].into(), hold: Some(hold) })?;
            draw(inv, fist_part(fist), Some(ch::arms::NO_FIST))?;
        }
    }
    // The hoof is on its leg: it stays in the giant, and is not drawn.
    if giant.limbs.iter().any(|l| l.slot == blows::HOOF) {
        let hold = LimbHold { position: giant.position, velocity: [0.0, 0.0], facing: 1.0 };
        inv.submit::<LimbPosePort>(LimbPose { slot: blows::HOOF.into(), hold: Some(hold) })?;
        draw(inv, DrawnPart::Limb(blows::HOOF.into()), Some(ch::arms::NO_FIST))?;
    }
    // The moments of it.
    let head_of = |at: Vec2| at - Vec2::new(0.0, half_h);
    let giant_head = giant_pos + Vec2::new(150.0 * turn, -giant_size.y * 0.5 - 180.0);
    if crossed(fin::FALL) {
        dust(inv, Vec2::new(floor.x, hall.floor), 14, 260.0, [0.62, 0.52, 0.42, 1.0])?;
        play(inv, SFX_FLOOR, floor)?;
    }
    let roar_at = fin::FALL + fin::GATHER + fin::GRIEVE;
    if crossed(fin::FALL + fin::GATHER + 0.4) {
        inv.submit::<SpeechBubblePort>(SpeechBubble { at: a(giant_head), text: "Mmrrh...".to_string() })?;
    }
    if crossed(roar_at + 0.3) {
        // Grief, and then rage: it roars, and the hall shakes with it.
        inv.submit::<CameraShakePort>(CameraShake { amplitude_px: 34.0 })?;
        play(inv, SFX_SNORT, giant_head)?;
        play(inv, SFX_STOMP, giant_pos)?;
        for side in [-1.0, 1.0] {
            let at = Vec2::new(giant_pos.x + side * giant_size.x * 0.55, hall.floor);
            let size = FX_SIZE * QUAKE_SCALE;
            effect(inv, FX_QUAKE, at - Vec2::new(0.0, size * QUAKE_FLOOR), QUAKE_SCALE, Vec2::X)?;
        }
        inv.submit::<SpeechBubblePort>(SpeechBubble { at: a(giant_head), text: "RRRAAAWWRRR!".to_string() })?;
    }
    let apple_at = roar_at + fin::ROAR;
    if crossed(apple_at) {
        // The apple falls on his head: the effect's head is its middle.
        effect(inv, FX_APPLE, head_of(cradle), APPLE_SCALE, Vec2::X)?;
    }
    if crossed(apple_at + APPLE_HITS) {
        spark(inv, head_of(cradle), GOLD, 24, 240.0)?;
        play(inv, SFX_SNORT, head_of(cradle))?;
        inv.submit::<SpeechBubblePort>(SpeechBubble { at: a(head_of(cradle) - Vec2::new(0.0, 30.0)), text: "...Eureka?".to_string() })?;
    }
    if crossed(apple_at + fin::APPLE) && c.helmets {
        // Back on the ground, the helmets come off.
        c.helmets = false;
        for who in HELMET_WEARERS {
            inv.submit::<WearPort>(Wear { who, gear: HELMET.into(), on: false })?;
        }
    }
    if crossed(apple_at + fin::APPLE + fin::WAKE * 0.6) {
        inv.submit::<SpeechBubblePort>(SpeechBubble {
            at: a(head_of(saddle) - Vec2::new(0.0, 30.0)),
            text: "I yield. A body at rest stays at rest.".to_string(),
        })?;
    }
    Ok(())
}

/// The telegraph's ink: what each move draws in the air before it strikes.
fn ink(
    inv: &mut Invocation<'_>,
    c: &Conductor,
    cue: &Cue,
    lead: Fist,
    stage: &Stage,
    poses: &[Pose; 2],
    fist_size: Vec2,
) -> Result<(), Fault> {
    let hall = stage.hall;
    match cue.mv {
        Move::PortalPunch | Move::PortalKick if !cue.striking && c.punch_open => {
            // A dotted line from the far line along the way the limb will
            // come, as far as it reaches: the colour of a warning when the
            // line has locked.
            let (far, way) = (v(c.punch_far), v(c.far_normal));
            let reach = c.blow_kind().map_or(0.0, |b| blows::limb(b).reach()) * 1.6;
            let colour = if blows::tracking(cue) { INK } else { WARNING };
            for k in 1..=6 {
                let at = far + way * (reach * k as f32 / 6.0);
                if at.y <= hall.floor {
                    spark(inv, at, colour, 1, 4.0)?;
                }
            }
        }
        Move::Demonstrate | Move::DemonstratePair => {
            // A plumb line from each hovering fist to where it will land.
            for fist in Fist::BOTH {
                let pose = poses[fist.index()];
                let aiming = cue.mv == Move::DemonstratePair || fist == lead;
                if !aiming || pose.stuck || pose.harmful {
                    continue;
                }
                let top = pose.pos.y + fist_size.y * 0.5;
                let locked = !ch::tracking(cue, lead, fist);
                for k in 1..=6 {
                    let y = top + (hall.floor - top) * k as f32 / 6.0;
                    spark(inv, Vec2::new(pose.pos.x, y), if locked { WARNING } else { INK }, 1, 4.0)?;
                }
            }
        }
        Move::Pendulum | Move::Cradle => {
            // The strings.
            for fist in Fist::BOTH {
                let swinging = cue.mv == Move::Cradle || fist == lead;
                if !swinging {
                    continue;
                }
                let pose = poses[fist.index()];
                let latch = Latch { aim: v(c.aims[fist.index()]), ..c.latch() };
                let pivot = match cue.mv {
                    Move::Pendulum => ch::pendulum::pivot(stage, &latch),
                    _ => Vec2::new(hall.center_x() + fist.side() * fist_size.x * 0.5, hall.floor - ch::cradle::PIVOT_HEIGHT),
                };
                for k in 0..8 {
                    let p = pivot + (pose.pos - pivot) * (k as f32 / 8.0);
                    spark(inv, p, INK, 1, 3.0)?;
                }
            }
        }
        Move::Fluxions | Move::FluxionsPair if !cue.striking => {
            // He writes the curve before he traces it.
            let drawn = ch::fluxions::drawn(cue.t, cue.dur);
            let mirrors: &[bool] = if cue.mv == Move::FluxionsPair { &[false, true] } else { &[false] };
            for &mirror in mirrors {
                let dots = (28.0 * drawn).ceil() as usize;
                for k in 0..=dots {
                    let u = drawn * k as f32 / dots.max(1) as f32;
                    let p = ch::fluxions::point(stage, &c.latch(), u, mirror);
                    spark(inv, p + Vec2::new(0.0, fist_size.y * 0.5), INK, 1, 3.0)?;
                }
                if drawn < 1.0 {
                    let head = ch::fluxions::point(stage, &c.latch(), drawn, mirror);
                    spark(inv, head + Vec2::new(0.0, fist_size.y * 0.5), GOLD, 3, 20.0)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}
