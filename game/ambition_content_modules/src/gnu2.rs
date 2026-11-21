//! GNU2's conductor: the giant's armored sequel, which GNU-ton calls in at
//! his enrage (GNUTON-GNU2-FINALE).
//!
//! GNU2 is a boss placement of GNU-ton's arena, held in a sealed cell under
//! its floor, out of the camera's zone. It sleeps until GNU-ton enrages (or is
//! beaten: `successors_wake: Enraged`). Awake, it opens a pair of portals
//! (`ambition.portal.held_pair`): one end in the cell's east wall facing
//! west, the other in the arena's west wall facing east, each taller than
//! it. It walks east into the one and comes out of the other still walking
//! east (`ambition.boss.walk`), and the pair closes once it is wholly out.
//! The game glues a pair by a rotation (`PortalTuning`'s `Rotation`), and two
//! ends that face opposite ways are the identity under it: up stays up. Two
//! ends facing the same way are a half turn, which brought GNU2 into the hall
//! on its back. A boss flies (no gravity), so it walks level, and each end is
//! centred where its body passes.
//!
//! In the arena it walks to the player between its moves, and performs the
//! moves its pattern names: `stomp` sends a shock along the floor each way
//! (jump it), and `buck` throws whoever stands on its back. It is drawn row by
//! row by name from its parts (`npc_giant_gnu2`).

use ambition_boss_special_port::{
    BossConduct, BossConductPort, BossWalkPort, DrawnRow, DrawnRowPort, MountWalk, RoomHall,
};
use ambition_combat_port::{
    BodySound, BodySoundPort, Burst, BurstPort, CameraShake, CameraShakePort, HeldDamageBox, HeldDamageBoxPort, Place,
    RidingHitbox, RidingHitboxPort, RidingKnockback,
};
use ambition_extension_sdk::{
    phases::BOSS_CONDUCT, record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};
use ambition_portal_port::{HeldPortalPair, HeldPortalPairPort, PortalEnd};

/// The behaviour id of the boss it conducts.
pub const GNU2_ID: &str = "gnu2";

/// The giant's stride (world units a second), as GNU-ton's giant walks.
pub const WALK_SPEED: f32 = 135.0;
/// Seconds of each rest it walks: it stands still for the rest's last part,
/// so a move starts from a stand.
const WALK_WINDOW: f32 = 1.1;
/// It does not walk for less than this.
const WALK_MIN: f32 = 24.0;
/// Its walk row is drawn while it moves faster than this.
const WALKING: f32 = 20.0;

/// The pair's slot, and how far each end reaches past its body's half height.
const ENTRY_PAIR_SLOT: u32 = 0;
const ENTRY_PAD: f32 = 40.0;
/// How far over the hall's floor its feet come out of the far end. Its
/// placement holds it as far over the cell's floor: it does not fall, and a
/// body that is not standing is stopped by a face its feet only touch, which
/// is where each floor meets the plane of the end it passes.
const EXIT_LIFT: f32 = 2.0;
/// How far past the hall's wall its back is when it has come through.
const CLEAR: f32 = 8.0;
/// Feet within this of the arena's floor: it is in the hall.
const ARRIVED: f32 = 48.0;

const WAVE_SPEED: f32 = 540.0;
const WAVE_HALF: [f32; 2] = [26.0, 20.0];
const WAVE_DAMAGE: i32 = 2;
const WAVE_KNOCKBACK: f32 = 1.3;
const WAVE_LIFETIME_S: f32 = 6.0;
const WAVE_BOX_SLOT: u32 = 0;
const WAVES: usize = 4;

const BUCK_THROW_SPEED: f32 = 1100.0;
const BUCK_THROW_DIR: [f32; 2] = [0.75, -0.66];
const BUCK_DAMAGE: i32 = 1;
/// The buck's reach over its back, and how long it throws.
const BUCK_REACH: f32 = 96.0;
const BUCK_WINDOW: f32 = 0.16;
/// The top of its back, below its box's top (its head and horns stand over).
const BACK_DROP: f32 = 0.22;

/// Seconds its hit row plays when it is hurt.
const HIT_ROW: f32 = 0.3;

const SFX_STOMP: &str = "boss.gnu_ton.stomp";
const SFX_SNORT: &str = "boss.gnu_ton.snort";
const SFX_PORTAL_OPEN: &str = "boss.gnu_ton.portal_open";
const DUST: [f32; 4] = [0.55, 0.45, 0.35, 1.0];

record! {
    /// The conductor's memory, on GNU2.
    pub struct Conductor = SchemaKey::new(crate::PROVIDER, "gnu2.conductor", 1);
    /// The arena's floor and walls (its predecessor's hall), once seen: the
    /// predecessor may be gone by the time it is called.
    1 arena_known: bool,
    2 arena_floor: f32,
    3 arena_left: f32,
    4 arena_right: f32,
    /// The pair's two ends, fixed when it was opened; and whether it is.
    5 pair_open: bool,
    6 near_at: [f32; 2],
    7 far_at: [f32; 2],
    8 half_length: f32,
    /// It has come into the arena.
    9 arrived: bool,
    /// The live move (its key's code, 0 for none), whether striking, and the
    /// time left in it last tick: a part that restarts shows as this jumping
    /// back up.
    10 part_move: u32,
    11 part_striking: bool,
    12 last_remaining: f32,
    /// Seconds into the live move, and since the last one ended.
    13 part_t: f32,
    14 since_move: f32,
    /// Seconds walked (the walk row's clock) and at rest (the rest row's).
    15 walk_t: f32,
    16 rest_t: f32,
    /// Its health last tick, and seconds of its hit row left.
    17 last_health: Option<i32>,
    18 hit_t: f32,
    /// The stomp's shocks: where each is (none when gone), which way, and the
    /// generation of its held box.
    19 wave_x: Vec<Option<f32>> [max 4],
    20 wave_dir: Vec<f32> [max 4],
    21 wave_generation: Vec<u32> [max 4],
    22 ticks: u32,
}

/// The moves its pattern names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Stomp,
    Buck,
}

impl Move {
    fn from_key(key: &str) -> Option<Self> {
        match key {
            "stomp" => Some(Self::Stomp),
            "buck" => Some(Self::Buck),
            _ => None,
        }
    }

    fn code(self) -> u32 {
        match self {
            Self::Stomp => 1,
            Self::Buck => 2,
        }
    }
}

impl Conductor {
    fn sized(mut self) -> Self {
        self.wave_x.resize(WAVES, None);
        self.wave_dir.resize(WAVES, 0.0);
        self.wave_generation.resize(WAVES, 0);
        self
    }

    /// The arena it fights in, once seen.
    pub fn arena(&self) -> Option<RoomHall> {
        self.arena_known.then_some(RoomHall { floor: self.arena_floor, left: self.arena_left, right: self.arena_right })
    }

    /// Whether it has come into the arena, for tests and inspectors.
    pub fn has_arrived(&self) -> bool {
        self.arrived
    }

    /// The pair's ends while it is open, for tests and inspectors.
    pub fn entry_pair(&self) -> Option<([f32; 2], [f32; 2])> {
        self.pair_open.then_some((self.near_at, self.far_at))
    }
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "gnu2"),
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
            trigger: TriggerBinding { port: BossConductPort::KEY, selector: GNU2_ID.into() },
            reads: Vec::new(),
            writes: vec![Conductor::KEY],
            requests: vec![
                BossWalkPort::KEY,
                DrawnRowPort::KEY,
                HeldPortalPairPort::KEY,
                HeldDamageBoxPort::KEY,
                RidingHitboxPort::KEY,
                CameraShakePort::KEY,
                BurstPort::KEY,
                BodySoundPort::KEY,
            ],
            after: Vec::new(),
            // Four shocks, their dust, a stomp's dust and sound, a row, a
            // walk, a pair.
            limits: Limits { max_requests: 32 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(conduct),
        }],
    }
}

/// The live move of its pattern: which, whether it strikes, and the time left.
fn live_part(b: &BossConduct) -> Option<(Move, bool, f32)> {
    if let Some(m) = b.active.as_ref().and_then(|m| Move::from_key(&m.key).map(|mv| (mv, m.remaining))) {
        return Some((m.0, true, m.1));
    }
    b.telegraph.as_ref().and_then(|m| Move::from_key(&m.key).map(|mv| (mv, false, m.remaining)))
}

fn walk(inv: &mut Invocation<'_>, to_x: f32, speed: f32) -> Result<(), Fault> {
    inv.submit::<BossWalkPort>(MountWalk { to_x, speed })
}

fn stand(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    walk(inv, 0.0, 0.0)
}

fn row(inv: &mut Invocation<'_>, name: &str, elapsed: f32, looping: bool) -> Result<(), Fault> {
    inv.submit::<DrawnRowPort>(DrawnRow::body(name, elapsed, looping))
}

fn dust(inv: &mut Invocation<'_>, at: [f32; 2], count: u32, speed: f32) -> Result<(), Fault> {
    inv.submit::<BurstPort>(Burst { at, count, speed, color: DUST, kind: "dust".into() })
}

fn play(inv: &mut Invocation<'_>, cue: &str, at: [f32; 2]) -> Result<(), Fault> {
    inv.submit::<BodySoundPort>(BodySound { cue: cue.into(), at: Place::World(at) })
}

/// An upright end whose line is centred on `at`, its room side `side`.
fn upright(at: [f32; 2], half_length: f32, side: f32) -> PortalEnd {
    PortalEnd { at, normal: [side, 0.0], half_length, half_thickness: None }
}

fn conduct(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let b: BossConduct = inv.trigger::<BossConductPort>()?.clone();
    let dt = inv.dt();
    let mut c = Conductor::load(inv)?.sized();
    c.ticks = c.ticks.wrapping_add(1);
    let [x, y] = b.position;
    let [w, h] = b.size;
    let feet = y + h * 0.5;

    // The arena is its predecessor's hall: seen while GNU-ton fights, kept.
    if let Some(hall) = b.predecessor.and_then(|p| p.hall) {
        c.arena_known = true;
        c.arena_floor = hall.floor;
        c.arena_left = hall.left;
        c.arena_right = hall.right;
    }
    // A GNU2 with no arena to come into fights where it stands.
    let arena = c.arena().or(b.hall);
    // Its feet at the arena's floor: it is in the hall's chart, whether or
    // not it is through.
    let in_hall = c.arena().is_some_and(|hall| (feet - hall.floor).abs() <= ARRIVED);
    if !c.arrived {
        // Wholly out of the far end, in the hall.
        c.arrived = match c.arena() {
            Some(hall) => in_hall && x - w * 0.5 > hall.left + CLEAR && x < hall.right,
            None => b.awake,
        };
    }

    if !b.alive {
        stand(inv)?;
        c.pair_open = false;
        inv.submit::<DrawnRowPort>(DrawnRow::body_as_engine())?;
        return c.store(inv);
    }

    // Hurt: its hit row plays over whatever it does.
    if c.last_health.is_some_and(|last| b.health < last) {
        c.hit_t = HIT_ROW;
    }
    c.last_health = Some(b.health);
    c.hit_t = (c.hit_t - dt).max(0.0);

    // ── Asleep in its cell ──
    if !b.awake {
        stand(inv)?;
        c.rest_t += dt;
        row(inv, "armed_rest", c.rest_t, true)?;
        return c.store(inv);
    }

    // ── Called: the pair, and the walk through it ──
    if !c.arrived {
        if let (false, Some(hall), Some(reach)) = (c.pair_open, c.arena(), b.reach) {
            // Each end is centred where its body passes and reaches past it
            // both ways: a body cut by the plane past an end snags on it (a
            // body passes an opening only whole). It comes out with its feet
            // `EXIT_LIFT` over the hall's floor. The near end is in the
            // cell's east wall, ahead of it; the far one in the hall's west wall.
            c.half_length = h * 0.5 + ENTRY_PAD;
            c.near_at = [x + w * 0.5 + reach[1], y];
            c.far_at = [hall.left, hall.floor - h * 0.5 - EXIT_LIFT];
            c.pair_open = true;
            play(inv, SFX_PORTAL_OPEN, c.far_at)?;
            play(inv, SFX_SNORT, b.position)?;
        }
        if c.pair_open {
            inv.submit::<HeldPortalPairPort>(HeldPortalPair {
                slot: ENTRY_PAIR_SLOT,
                ends: [
                    upright(c.near_at, c.half_length, -1.0),
                    upright(c.far_at, c.half_length, 1.0),
                ],
                looks: [24, 25],
                closes_on_use: false,
            })?;
        }
        // East into the cell's wall, and on east out of the hall's: the pair
        // maps east to east. Its middle past the near end is the tick before
        // the pair takes it: what it is told now it does on the far side.
        let through = in_hall || c.pair_open && x > c.near_at[0];
        let to = match c.arena() {
            Some(hall) if through => hall.left + w,
            _ => c.near_at[0] + w,
        };
        walk(inv, to, WALK_SPEED)?;
        c.walk_t += dt;
        row(inv, "armed_walk", c.walk_t, true)?;
        return c.store(inv);
    }
    // Wholly out of the far end: the pair closes behind it.
    c.pair_open = false;

    // ── In the arena ──
    let Some(hall) = arena else {
        stand(inv)?;
        row(inv, "armed_rest", c.rest_t, true)?;
        return c.store(inv);
    };
    let part = live_part(&b);
    match part {
        Some((mv, striking, remaining)) => {
            let restarted = mv.code() == c.part_move && remaining > c.last_remaining + 1e-3;
            let fresh = mv.code() != c.part_move || striking != c.part_striking || restarted;
            if mv.code() != c.part_move || restarted {
                c.part_t = 0.0;
            }
            c.part_move = mv.code();
            c.part_striking = striking;
            c.last_remaining = remaining;
            c.part_t += dt;
            c.since_move = 0.0;
            if fresh && striking {
                match mv {
                    Move::Stomp => stomp(inv, &mut c, x, w, hall.floor)?,
                    Move::Buck => buck(inv, &b)?,
                }
            }
        }
        None => {
            c.part_move = 0;
            c.part_striking = false;
            c.last_remaining = 0.0;
            c.since_move += dt;
        }
    }

    // Between its moves it walks to the player, and stands for the end of
    // its rest.
    let half = w * 0.5;
    let mark = b.target.map(|t| t[0].clamp(hall.left + half + 16.0, hall.right - half - 16.0));
    match mark {
        Some(mark) if part.is_none() && c.since_move < WALK_WINDOW && (mark - x).abs() > WALK_MIN => {
            walk(inv, mark, WALK_SPEED)?;
        }
        _ => stand(inv)?,
    }

    // ── The shocks roll out along the floor ──
    for slot in 0..WAVES {
        let Some(wx) = c.wave_x[slot] else {
            continue;
        };
        let wx = wx + c.wave_dir[slot] * WAVE_SPEED * dt;
        if wx < hall.left + WAVE_HALF[0] || wx > hall.right - WAVE_HALF[0] {
            c.wave_x[slot] = None;
            continue;
        }
        c.wave_x[slot] = Some(wx);
        let center = [wx, hall.floor - WAVE_HALF[1]];
        inv.submit::<HeldDamageBoxPort>(HeldDamageBox {
            slot: WAVE_BOX_SLOT + slot as u32,
            generation: c.wave_generation[slot],
            center,
            half_extent: WAVE_HALF,
            damage: WAVE_DAMAGE,
            knockback: WAVE_KNOCKBACK,
            lifetime_s: WAVE_LIFETIME_S,
        })?;
        if c.ticks % 2 == 0 {
            dust(inv, [wx, hall.floor - WAVE_HALF[1] * 0.5], 3, 120.0)?;
        }
    }

    // ── What it is drawn as ──
    let moving = b.velocity[0].abs() > WALKING;
    if moving {
        c.walk_t += dt;
    } else {
        c.walk_t = 0.0;
    }
    if c.hit_t > 0.0 && part.is_none() {
        row(inv, "armed_hit", HIT_ROW - c.hit_t, false)?;
    } else {
        match part {
            Some((Move::Stomp, ..)) => row(inv, "armed_stomp", c.part_t, false)?,
            Some((Move::Buck, ..)) => row(inv, "armed_buck", c.part_t, false)?,
            None if moving => row(inv, "armed_walk", c.walk_t, true)?,
            None => {
                c.rest_t += dt;
                row(inv, "armed_rest", c.rest_t, true)?;
            }
        }
    }
    c.store(inv)
}

/// The stomp: a shock out along the floor each way from its feet.
fn stomp(inv: &mut Invocation<'_>, c: &mut Conductor, x: f32, w: f32, floor: f32) -> Result<(), Fault> {
    for dir in [-1.0, 1.0] {
        let Some(slot) = c.wave_x.iter().position(Option::is_none) else {
            break;
        };
        c.wave_x[slot] = Some(x + dir * w * 0.5);
        c.wave_dir[slot] = dir;
        c.wave_generation[slot] = c.wave_generation[slot].wrapping_add(1);
    }
    play(inv, SFX_STOMP, [x, floor])?;
    dust(inv, [x, floor], 26, 320.0)?;
    inv.submit::<CameraShakePort>(CameraShake { amplitude_px: 18.0 })
}

/// The buck: a throw over its back. It rides GNU2, so a body on its back is
/// thrown away from the middle of it.
fn buck(inv: &mut Invocation<'_>, b: &BossConduct) -> Result<(), Fault> {
    let [w, h] = b.size;
    let back = -h * 0.5 + h * BACK_DROP;
    play(inv, SFX_SNORT, b.position)?;
    inv.submit::<RidingHitboxPort>(RidingHitbox {
        offset: [0.0, back - BUCK_REACH * 0.5],
        half_extent: [w * 0.5, BUCK_REACH * 0.5],
        circle_radius: None,
        damage: BUCK_DAMAGE,
        knockback: RidingKnockback::LaunchSpeed { base: BUCK_THROW_SPEED, growth: None },
        launch_dir: Some(ambition_combat_port::LaunchDirection::AwayFromSource(BUCK_THROW_DIR)),
        lifetime_s: BUCK_WINDOW,
        name: "gnu2_buck_throw".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn its_moves_are_the_patterns_keys() {
        assert_eq!(Move::from_key("stomp"), Some(Move::Stomp));
        assert_eq!(Move::from_key("buck"), Some(Move::Buck));
        assert_eq!(Move::from_key("portal_punch"), None);
    }

    #[test]
    fn an_upright_end_is_centred_where_it_is_asked() {
        let end = upright([100.0, 628.0], 272.0, 1.0);
        assert_eq!(end.at, [100.0, 628.0]);
        assert_eq!(end.normal, [1.0, 0.0]);
    }
}
