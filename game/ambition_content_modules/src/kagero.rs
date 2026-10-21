//! Kagero's visit: a ninja comes to your aid in the Inugami's final hall.
//!
//! Jon (2026-10-21): "I also want a ninja to come to your aid in the fight,
//! appear on screen and do a few hits on the boss and then fly off the screen
//! (using flyline maybe). It should not be the Shadow Oni Leader, a different
//! ninja. We can have some story play in with that."
//!
//! Kagero hunts the spirits the regime makes. She followed the inugami into
//! Castle Godwin. The Inugami's conductor calls her once
//! (`crate::inugami`): it summons her on your side, beside the head, and she
//! holds [`VISIT`] from birth. This module performs her visit, a tick at a
//! time ([`visit_tick_of`]):
//!
//! 1. She comes out of her smoke (`blink_in`), and speaks.
//! 2. She goes at the head (`dash`).
//! 3. She cuts three times, one string (`slash`, `slash_up`, `cleave`). Each
//!    cut is a strike of her own (`ambition.combat.body_strike`): the damage
//!    is real, and lands as yours does.
//! 4. Her line drops from the sky, she catches it and is reeled up and away
//!    (`hang`; the line is `ambition.feedback.body_line`), and she speaks.
//! 5. She is dismissed when she is high, or when her time is out.
//!
//! "Off screen" is not a fact the simulation may read: peers with different
//! windows would desync. She leaves by height and by time.

use ambition_boss_special_port::{DrawnRow, DrawnRowPort};
use ambition_combat_port::{
    BodyLine, BodyLinePort, BodySound, BodySoundPort, BodyStrike, BodyStrikePort, Burst, BurstPort, Dismiss, DismissPort,
    FoesPort, HoldCourse, HoldCoursePort, Place, SpeechBubble, SpeechBubblePort, StatusTickPort,
};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation,
    Limits, ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};
use bevy_math::Vec2;

/// Her character.
pub const KAGERO_ID: &str = "npc_ninja_kagero";

/// The status she holds from birth: the module that performs her visit.
pub const VISIT: &str = "ambition.kagero_visit";

/// Her body box, world px (half extents).
pub const HALF_SIZE: [f32; 2] = [14.0, 30.0];

/// The beats of her visit, s. Each row is as long as its frames on her sheet
/// (the renderer's `ninja_kagero`).
const ARRIVE_S: f32 = 6.0 * 0.062;
/// Her smoke, her sound and her first line come this long after her birth.
const CUE_AFTER_S: f32 = 0.05;
/// The longest she goes at it before she cuts where she is.
const DASH_MAX_S: f32 = 0.7;
const CUTS: [(&str, f32); 3] = [("slash", 8.0 * 0.040), ("slash_up", 8.0 * 0.040), ("cleave", 9.0 * 0.046)];
/// Where in its row each cut goes through (the frame the blade passes).
const CUT_AT_U: f32 = 0.45;
/// The line drops for this long before it reels her.
const CATCH_S: f32 = 0.18;
/// The longest she is reeled before she is dismissed.
const REEL_S: f32 = 1.6;

/// How fast she goes at it, px/s.
const DASH_SPEED: f32 = 1100.0;
/// She cuts from this far from the head's middle: the head is big.
const STANDOFF: f32 = 130.0;
/// How fast she keeps her place by it while she cuts, px/s at most.
const KEEP_SPEED: f32 = 320.0;
/// The least she goes toward it across while she cuts, px/s.
const DRIFT: f32 = 24.0;
/// Her cut: forward of her middle along her facing, half extents, damage.
const CUT_FORWARD: [f32; 2] = [52.0, -4.0];
const CUT_HALF: [f32; 2] = [62.0, 54.0];
const CUT_DAMAGE: [i32; 3] = [2, 2, 3];
const CUT_KNOCKBACK: f32 = 0.0;

/// Where her line hangs from: up and away from the head.
const LINE_UP: f32 = 1400.0;
const LINE_AWAY: f32 = 300.0;
/// How fast she is reeled: from the first speed to the last over the reel.
const REEL_FROM: [f32; 2] = [260.0, -700.0];
const REEL_TO: [f32; 2] = [520.0, -1700.0];
/// She is out of the hall when her middle is this high in the room.
const GONE_Y: f32 = 40.0;

const SMOKE: [f32; 4] = [0.78, 0.79, 0.86, 1.0];
const STEEL: [f32; 4] = [0.92, 0.95, 1.0, 1.0];

const SFX_ARRIVE: &str = "vfx.ninja_shadow_oni_leader.blink_arrive";
const SFX_CUT: &str = "enemy.shadow_oni.slash";
const SFX_CUT_HIT: &str = "player.robot.slash.impact.flesh.deep";
const SFX_LINE: &str = "vfx.pirate_admiral.grapple_cast";
const SFX_CATCH: &str = "vfx.pirate_admiral.grapple_catch";

/// Every cue she plays: each must have a recipe, or it plays silence.
pub const SOUNDS: [&str; 5] = [SFX_ARRIVE, SFX_CUT, SFX_CUT_HIT, SFX_LINE, SFX_CATCH];

/// What she says when she comes, and when she goes. The ninjas call the
/// robot by its noise (`docs/brainstorms/llm_brainstorm.md`: "noisy one",
/// "ripple-foot").
pub const LINE_ARRIVE: &str = "Noisy one. Even the dead heard you.";
pub const LINE_LEAVE: &str = "It was a dog once. Set it free, ripple-foot.";

record! {
    /// Her visit, so far.
    pub struct Visit = SchemaKey::new(crate::PROVIDER, "kagero.visit", 1);
    /// Gameplay seconds since she came.
    1 age: f32,
    /// The beat she is in ([`Beat`]) and the seconds she has been in it.
    2 beat: u32,
    3 beat_t: f32,
    /// How many of her cuts have gone through.
    4 cut: u32,
    /// The way from the head to her line (-1 or 1), and her line's anchor.
    5 away: f32,
    6 line_x: f32,
    7 line_y: f32,
}

/// The beats of her visit, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Beat {
    Arrive,
    Dash,
    /// The cut of this index, `0..3`.
    Cut(u32),
    Catch,
    Reel,
}

impl Beat {
    fn code(self) -> u32 {
        match self {
            Beat::Arrive => 0,
            Beat::Dash => 1,
            Beat::Cut(k) => 2 + k,
            Beat::Catch => 5,
            Beat::Reel => 6,
        }
    }

    fn of_code(code: u32) -> Beat {
        match code {
            0 => Beat::Arrive,
            1 => Beat::Dash,
            2..=4 => Beat::Cut(code - 2),
            5 => Beat::Catch,
            _ => Beat::Reel,
        }
    }
}

impl Visit {
    pub fn beat_now(&self) -> Beat {
        Beat::of_code(self.beat)
    }

    fn enter(&mut self, beat: Beat) {
        self.beat = beat.code();
        self.beat_t = 0.0;
    }
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "kagero"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![Visit::schema()],
        entries: vec![EntryDescriptor {
            key: "visit".into(),
            phase: TECHNIQUE_EXECUTION,
            trigger: TriggerBinding { port: StatusTickPort::KEY, selector: VISIT.into() },
            reads: vec![FoesPort::KEY],
            writes: vec![Visit::KEY],
            requests: vec![
                HoldCoursePort::KEY,
                DrawnRowPort::KEY,
                BodyStrikePort::KEY,
                BodyLinePort::KEY,
                BodySoundPort::KEY,
                BurstPort::KEY,
                SpeechBubblePort::KEY,
                DismissPort::KEY,
            ],
            after: Vec::new(),
            limits: Limits { max_requests: 12 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(visit),
        }],
    }
}

/// What one tick of her visit asks for.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VisitTick {
    pub course: [f32; 2],
    /// Her row and how far into it she is (s).
    pub row: (&'static str, f32),
    pub strike: Option<i32>,
    /// Her line's anchor, while she holds it.
    pub line: Option<[f32; 2]>,
    pub sounds: Vec<&'static str>,
    pub says: Option<&'static str>,
    /// She is in her smoke: a puff of it.
    pub smoke: bool,
    pub dismissed: bool,
}

/// One tick of her visit: she is at `at`, the head (her nearest foe) at
/// `head`. `v` is her visit so far; it is advanced.
pub fn visit_tick_of(v: &mut Visit, at: Vec2, head: Option<Vec2>, dt: f32) -> VisitTick {
    let was = v.age;
    v.age += dt;
    v.beat_t += dt;
    // Her arrival is heard and seen a few ticks after her birth: on the tick
    // she is made she is in no live room yet, and what she shows there is
    // drawn nowhere.
    let first = was < CUE_AFTER_S && v.age >= CUE_AFTER_S;
    let mut out = VisitTick::default();
    let toward = head.map_or(Vec2::ZERO, |h| (h - at).normalize_or_zero());
    // Her place by the head while she cuts: on her side of it, a little
    // toward it across, so she faces it (a body faces the way it goes).
    let keep = |head: Vec2| {
        let to_head = if at.x < head.x { 1.0 } else { -1.0 };
        let goal = Vec2::new(head.x - to_head * STANDOFF, head.y + 10.0);
        let across = ((goal.x - at.x) * 8.0).clamp(-KEEP_SPEED, KEEP_SPEED);
        let across = if across * to_head < DRIFT { to_head * DRIFT } else { across };
        Vec2::new(across, ((goal.y - at.y) * 8.0).clamp(-KEEP_SPEED, KEEP_SPEED))
    };
    match v.beat_now() {
        Beat::Arrive => {
            if first {
                out.sounds.push(SFX_ARRIVE);
                out.says = Some(LINE_ARRIVE);
                out.smoke = true;
            }
            out.row = ("blink_in", v.beat_t);
            if v.beat_t >= ARRIVE_S {
                v.enter(Beat::Dash);
            }
        }
        Beat::Dash => {
            out.row = ("dash", v.beat_t);
            let near = head.is_none_or(|h| (h - at).length() <= STANDOFF + 20.0);
            if near || v.beat_t >= DASH_MAX_S {
                v.enter(Beat::Cut(0));
            } else {
                out.course = (toward * DASH_SPEED).into();
            }
        }
        Beat::Cut(k) => {
            let (row, len) = CUTS[k as usize];
            out.row = (row, v.beat_t);
            out.course = head.map_or(Vec2::ZERO, keep).into();
            if v.cut == k && v.beat_t >= CUT_AT_U * len {
                v.cut += 1;
                out.strike = Some(CUT_DAMAGE[k as usize]);
                out.sounds.push(SFX_CUT);
            }
            if v.beat_t >= len {
                if k + 1 < CUTS.len() as u32 {
                    v.enter(Beat::Cut(k + 1));
                } else {
                    v.away = head.map_or(1.0, |h| if at.x < h.x { -1.0 } else { 1.0 });
                    v.line_x = at.x + v.away * LINE_AWAY;
                    v.line_y = at.y - LINE_UP;
                    out.sounds.push(SFX_LINE);
                    out.says = Some(LINE_LEAVE);
                    v.enter(Beat::Catch);
                }
            }
        }
        Beat::Catch => {
            out.row = ("hang", v.beat_t);
            out.line = Some([v.line_x, v.line_y]);
            if v.beat_t >= CATCH_S {
                out.sounds.push(SFX_CATCH);
                v.enter(Beat::Reel);
            }
        }
        Beat::Reel => {
            let u = (v.beat_t / REEL_S).clamp(0.0, 1.0);
            let speed = Vec2::from(REEL_FROM).lerp(Vec2::from(REEL_TO), u);
            out.row = ("hang", v.beat_t + CATCH_S);
            out.line = Some([v.line_x, v.line_y]);
            out.course = [speed.x * v.away, speed.y];
            out.dismissed = at.y < GONE_Y || v.beat_t >= REEL_S;
        }
    }
    out
}

fn visit(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let at = Vec2::from(inv.trigger::<StatusTickPort>()?.position);
    let head = inv
        .observe::<FoesPort>()?
        .foes
        .iter()
        .map(|foe| Vec2::from(foe.position))
        .min_by(|a, b| a.distance_squared(at).total_cmp(&b.distance_squared(at)));
    let mut v = Visit::load(inv)?;
    let tick = visit_tick_of(&mut v, at, head, inv.dt());
    v.store(inv)?;
    if tick.dismissed {
        inv.submit::<BodyLinePort>(BodyLine { anchor: None })?;
        return inv.submit::<DismissPort>(Dismiss);
    }
    inv.submit::<HoldCoursePort>(HoldCourse { velocity: Some(tick.course) })?;
    inv.submit::<DrawnRowPort>(DrawnRow::body(tick.row.0, tick.row.1, false))?;
    if let Some(damage) = tick.strike {
        inv.submit::<BodyStrikePort>(BodyStrike {
            forward: CUT_FORWARD,
            half_extent: CUT_HALF,
            damage,
            knockback: CUT_KNOCKBACK,
            lifetime_s: 0.08,
            hit_sound: Some(SFX_CUT_HIT.into()),
        })?;
        inv.submit::<BurstPort>(Burst { at: at.into(), count: 10, speed: 260.0, color: STEEL, kind: "streak".into() })?;
    }
    inv.submit::<BodyLinePort>(BodyLine { anchor: tick.line })?;
    for cue in tick.sounds {
        inv.submit::<BodySoundPort>(BodySound { cue: cue.into(), at: Place::World(at.into()) })?;
    }
    if tick.smoke {
        inv.submit::<BurstPort>(Burst { at: at.into(), count: 26, speed: 180.0, color: SMOKE, kind: "dust".into() })?;
    }
    if let Some(text) = tick.says {
        inv.submit::<SpeechBubblePort>(SpeechBubble { at: [at.x, at.y - 56.0], text: text.into() })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    /// She comes, goes at the head, cuts three times, takes her line, and
    /// is gone: the whole visit, against a head that keeps still.
    #[test]
    fn her_visit_cuts_three_times_and_she_leaves_up_her_line() {
        let head = Vec2::new(800.0, 500.0);
        let mut at = Vec2::new(400.0, 520.0);
        let mut v = Visit::default();
        let mut damage = Vec::new();
        let mut said = Vec::new();
        let mut gone_after = None;
        for k in 0..600 {
            let tick = visit_tick_of(&mut v, at, Some(head), DT);
            damage.extend(tick.strike);
            said.extend(tick.says);
            if tick.dismissed {
                gone_after = Some(k as f32 * DT);
                break;
            }
            at += Vec2::from(tick.course) * DT;
        }
        assert_eq!(damage, CUT_DAMAGE.to_vec(), "her cuts");
        assert_eq!(said, vec![LINE_ARRIVE, LINE_LEAVE]);
        let gone = gone_after.expect("she never left");
        assert!(gone < 4.0, "her visit took {gone} s");
        assert!(at.y < head.y - 400.0, "she did not go up her line: {at:?}");
    }

    /// She cuts from her side of the head, close enough for her cut to
    /// reach it.
    #[test]
    fn she_cuts_close_to_the_head() {
        let head = Vec2::new(800.0, 500.0);
        let mut at = Vec2::new(1300.0, 480.0);
        let mut v = Visit::default();
        for _ in 0..600 {
            let tick = visit_tick_of(&mut v, at, Some(head), DT);
            if tick.strike.is_some() {
                assert!(at.x > head.x, "she crossed the head");
                assert!((at - head).length() < STANDOFF + 40.0, "she cut from {at:?}");
                return;
            }
            at += Vec2::from(tick.course) * DT;
        }
        panic!("she never cut");
    }
}
