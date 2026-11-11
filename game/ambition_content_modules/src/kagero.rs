//! Kagero's visit: a ninja comes to your aid in the Inugami's final hall.
//!
//! Jon (2026-10-21): "I also want a ninja to come to your aid in the fight,
//! appear on screen and do a few hits on the boss and then fly off the screen
//! (using flyline maybe). It should not be the Shadow Oni Leader, a different
//! ninja. We can have some story play in with that."
//!
//! Then (2026-10-24): "The ninja is who dispels the flames and lets you fight
//! once you reach the end of the hall", and "The ninja uses a spell to dispel
//! the blue flames protecting the boss."
//!
//! Kagero hunts the spirits the regime makes. She followed the inugami into
//! Castle Godwin. The Inugami's conductor calls her once, when it has hunted
//! you into its final hall (`crate::inugami`): it summons her on your side,
//! beside you, and she holds [`VISIT`] from birth. This module performs her
//! visit, a tick at a time ([`visit_tick_of`]):
//!
//! 1. She comes out of her smoke (`blink_in`), and speaks.
//! 2. She casts (`cast`): her hands make the seal, and she throws her paper
//!    talismans (ofuda) at the head (the effect [`VOLLEY_FX`]). They fly to
//!    it and strike ([`SEAL_FX`]), and at [`DISPEL_AT_S`] its ward of blue
//!    fire breaks. The head's conductor keeps that time itself: it is held
//!    from her birth, breaks its ward then, and wears her talismans
//!    (`crate::inugami::SEALED`) from then on.
//! 3. Her line drops from the sky, she catches it and is reeled up and away
//!    (`hang`; the line is `ambition.feedback.body_line`), and she speaks.
//! 4. She is dismissed when she is high, or when her time is out.
//!
//! She does not cut it: the fight is yours.
//!
//! She comes once more, for its finish ([`GREAT_SEAL`], Jon, 2026-11-11): held
//! at the last of its strength, it bursts into its invincible fire again, and
//! the head's conductor calls her back. She casts her great seal
//! (`great_seal`), a ring of talismans round it, tells you it is weak and to
//! strike now, and leaves the blow to you. Its conductor keeps the time her
//! seal lands, [`GREAT_SEAL_AT_S`].
//!
//! "Off screen" is not a fact the simulation may read: peers with different
//! windows would desync. She leaves by height and by time.

use ambition_boss_special_port::{DrawnRow, DrawnRowPort};
use ambition_combat_port::{
    BodyLine, BodyLinePort, BodySound, BodySoundPort, Burst, BurstPort, Dismiss, DismissPort, Effect, EffectPort, FoesPort,
    HoldCourse, HoldCoursePort, Place, SpeechBubble, SpeechBubblePort, StatusTickPort,
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

/// The status she holds from birth when she comes back for its finish.
pub const GREAT_SEAL: &str = "ambition.kagero_great_seal";

/// Her body box, world px (half extents).
pub const HALF_SIZE: [f32; 2] = [14.0, 30.0];

/// The beats of her visit, s. Each row is as long as its frames on her sheet
/// (the renderer's `ninja_kagero`).
const ARRIVE_S: f32 = 6.0 * 0.062;
/// Her smoke, her sound and her first line come this long after her birth.
const CUE_AFTER_S: f32 = 0.05;
/// Her spell (`cast`, 16 x 75 ms): the seal, then she throws at
/// `THROW_AT_S` into it, and her talismans fly to the head for `FLY_S`.
const CAST_S: f32 = 16.0 * 0.075;
const THROW_AT_S: f32 = 0.6;
const FLY_S: f32 = 0.3;
/// The head's ward breaks this long after her birth: when her talismans
/// reach it.
pub const DISPEL_AT_S: f32 = ARRIVE_S + THROW_AT_S + FLY_S;

/// Her great seal (`great_seal`): she casts longer and throws later.
const GREAT_CAST_S: f32 = 16.0 * 0.09;
const GREAT_THROW_AT_S: f32 = 0.8;
const GREAT_FLY_S: f32 = 0.35;
/// Seconds from her birth to her great seal landing on it.
pub const GREAT_SEAL_AT_S: f32 = ARRIVE_S + GREAT_THROW_AT_S + GREAT_FLY_S;
/// The great seal is bigger than her first.
const GREAT_SEAL_SCALE: f32 = SEAL_SCALE * 1.8;
/// She keeps her place as she casts, turned to the head: this slow toward it.
const DRIFT: f32 = 12.0;
/// The line drops for this long before it reels her.
const CATCH_S: f32 = 0.18;
/// The longest she is reeled before she is dismissed.
const REEL_S: f32 = 1.6;
/// Her talismans in flight (`ninja_kagero_vfx`): five, along the effect's
/// +X, crossing `VOLLEY_SPAN` of its square. It is played midway to the head,
/// turned to it and scaled to the distance, and lasts `FLY_S`.
pub const VOLLEY_FX: &str = "ofuda_volley";
const VOLLEY_SPAN: f32 = 0.84;
/// The world size of an effect's square at scale 1 (`ambition_render::fx`).
const FX_WORLD: f32 = 56.0;
/// Her talismans strike the head: her seal opens on it, this big.
pub const SEAL_FX: &str = "ofuda_seal";
const SEAL_SCALE: f32 = 3.6;

/// Where her line hangs from: up and away from the head.
const LINE_UP: f32 = 1400.0;
const LINE_AWAY: f32 = 300.0;
/// How fast she is reeled: from the first speed to the last over the reel.
const REEL_FROM: [f32; 2] = [260.0, -700.0];
const REEL_TO: [f32; 2] = [520.0, -1700.0];
/// She is out of the hall when her middle is this high in the room.
const GONE_Y: f32 = 40.0;

const SMOKE: [f32; 4] = [0.78, 0.79, 0.86, 1.0];
/// Her talismans: white paper.
const PAPER: [f32; 4] = [0.98, 0.96, 0.88, 1.0];

const SFX_ARRIVE: &str = "vfx.ninja_shadow_oni_leader.blink_arrive";
const SFX_THROW: &str = "enemy.shadow_oni.slash";
const SFX_LINE: &str = "vfx.pirate_admiral.grapple_cast";
const SFX_CATCH: &str = "vfx.pirate_admiral.grapple_catch";

/// Every cue she plays: each must have a recipe, or it plays silence.
pub const SOUNDS: [&str; 4] = [SFX_ARRIVE, SFX_THROW, SFX_LINE, SFX_CATCH];

/// What she says when she comes, and when she goes. The ninjas call the
/// robot by its noise (`docs/brainstorms/llm_brainstorm.md`: "noisy one",
/// "ripple-foot").
pub const LINE_ARRIVE: &str = "Noisy one. Even the dead heard you.";
pub const LINE_CAST: &str = "Burn out, borrowed fire!";
pub const LINE_LEAVE: &str = "It was a dog once. Set it free, ripple-foot.";
/// What she says when she comes back for its finish, as she seals it, and
/// as she goes.
pub const LINE_RETURN: &str = "Borrowed fire again? Not for long.";
pub const LINE_WEAK: &str = "It is weak now! Strike, ripple-foot. Strike now!";
pub const LINE_FINISH: &str = "One blow. Make it yours.";

record! {
    /// Her visit, so far.
    pub struct Visit = SchemaKey::new(crate::PROVIDER, "kagero.visit", 3);
    /// Gameplay seconds since she came.
    1 age: f32,
    /// The beat she is in ([`Beat`]) and the seconds she has been in it.
    2 beat: u32,
    3 beat_t: f32,
    /// She has thrown her talismans.
    4 thrown: bool,
    /// The way from the head to her line (-1 or 1), and her line's anchor.
    5 away: f32,
    6 line_x: f32,
    7 line_y: f32,
    /// She came back for its finish: her great seal.
    8 great: bool,
}

/// The beats of her visit, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Beat {
    Arrive,
    Cast,
    Catch,
    Reel,
}

impl Beat {
    fn code(self) -> u32 {
        match self {
            Beat::Arrive => 0,
            Beat::Cast => 1,
            Beat::Catch => 2,
            Beat::Reel => 3,
        }
    }

    fn of_code(code: u32) -> Beat {
        match code {
            0 => Beat::Arrive,
            1 => Beat::Cast,
            2 => Beat::Catch,
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
        entries: [("visit", VISIT, visit as fn(&mut Invocation<'_>) -> Result<(), Fault>), ("great_seal", GREAT_SEAL, great_seal)]
            .into_iter()
            .map(|(key, hold, run)| EntryDescriptor {
            key: key.into(),
            phase: TECHNIQUE_EXECUTION,
            trigger: TriggerBinding { port: StatusTickPort::KEY, selector: hold.into() },
            reads: vec![FoesPort::KEY],
            writes: vec![Visit::KEY],
            requests: vec![
                HoldCoursePort::KEY,
                DrawnRowPort::KEY,
                BodyLinePort::KEY,
                BodySoundPort::KEY,
                BurstPort::KEY,
                SpeechBubblePort::KEY,
                DismissPort::KEY,
                EffectPort::KEY,
            ],
            after: Vec::new(),
            limits: Limits { max_requests: 16 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(run),
        })
            .collect(),
    }
}

/// What one tick of her visit asks for.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VisitTick {
    pub course: [f32; 2],
    /// Her row and how far into it she is (s).
    pub row: (&'static str, f32),
    /// She throws her talismans at the head: from where, to where.
    pub throw: Option<([f32; 2], [f32; 2])>,
    /// They have reached it: the ward breaks.
    pub ofuda_hit: bool,
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
    match v.beat_now() {
        Beat::Arrive => {
            if first {
                out.sounds.push(SFX_ARRIVE);
                out.says = Some(if v.great { LINE_RETURN } else { LINE_ARRIVE });
                out.smoke = true;
            }
            out.row = ("blink_in", v.beat_t);
            out.course = (toward * DRIFT).into();
            if v.beat_t >= ARRIVE_S {
                v.enter(Beat::Cast);
            }
        }
        Beat::Cast => {
            let (row, throw_at, fly, cast) =
                if v.great { ("great_seal", GREAT_THROW_AT_S, GREAT_FLY_S, GREAT_CAST_S) } else { ("cast", THROW_AT_S, FLY_S, CAST_S) };
            out.row = (row, v.beat_t);
            out.course = (toward * DRIFT).into();
            if !v.thrown && v.beat_t >= throw_at {
                v.thrown = true;
                out.sounds.push(SFX_THROW);
                out.says = Some(if v.great { LINE_WEAK } else { LINE_CAST });
                out.throw = head.map(|h| (at.into(), h.into()));
            }
            out.ofuda_hit = v.beat_t >= throw_at + fly && v.beat_t - dt < throw_at + fly;
            if v.beat_t >= cast {
                v.away = head.map_or(1.0, |h| if at.x < h.x { -1.0 } else { 1.0 });
                v.line_x = at.x + v.away * LINE_AWAY;
                v.line_y = at.y - LINE_UP;
                out.sounds.push(SFX_LINE);
                out.says = Some(if v.great { LINE_FINISH } else { LINE_LEAVE });
                v.enter(Beat::Catch);
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

/// Her talismans in flight from `from` to `to`: the volley played midway,
/// turned to the head (clockwise, +Y down) and as big as the distance.
pub fn volley(from: Vec2, to: Vec2) -> Effect {
    let way = to - from;
    Effect {
        at: Place::World(from.midpoint(to).into()),
        fx: VOLLEY_FX.into(),
        scale: way.length() / (FX_WORLD * VOLLEY_SPAN),
        angle: way.y.atan2(way.x),
    }
}

fn visit(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    perform(inv, false)
}

/// Her visit for its finish: her great seal.
fn great_seal(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    perform(inv, true)
}

fn perform(inv: &mut Invocation<'_>, great: bool) -> Result<(), Fault> {
    let at = Vec2::from(inv.trigger::<StatusTickPort>()?.position);
    let head = inv
        .observe::<FoesPort>()?
        .foes
        .iter()
        .map(|foe| Vec2::from(foe.position))
        .min_by(|a, b| a.distance_squared(at).total_cmp(&b.distance_squared(at)));
    let mut v = Visit::load(inv)?;
    v.great |= great;
    let tick = visit_tick_of(&mut v, at, head, inv.dt());
    v.store(inv)?;
    if tick.dismissed {
        inv.submit::<BodyLinePort>(BodyLine { anchor: None })?;
        return inv.submit::<DismissPort>(Dismiss);
    }
    inv.submit::<HoldCoursePort>(HoldCourse { velocity: Some(tick.course) })?;
    // Her great seal is drawn as her cast where its row is not published.
    let names = std::iter::once(tick.row.0).chain((tick.row.0 == "great_seal").then_some("cast")).map(str::to_string).collect();
    inv.submit::<DrawnRowPort>(DrawnRow { names, ..DrawnRow::body(tick.row.0, tick.row.1, false) })?;
    if let Some((from, to)) = tick.throw {
        inv.submit::<EffectPort>(volley(Vec2::from(from), Vec2::from(to)))?;
    }
    if let (true, Some(h)) = (tick.ofuda_hit, head) {
        let (scale, count) = if v.great { (GREAT_SEAL_SCALE, 40) } else { (SEAL_SCALE, 14) };
        inv.submit::<EffectPort>(Effect { at: Place::World(h.into()), fx: SEAL_FX.into(), scale, angle: 0.0 })?;
        inv.submit::<BurstPort>(Burst { at: h.into(), count, speed: 240.0, color: PAPER, kind: "streak".into() })?;
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

    /// She comes, casts her spell at the head (her talismans reach it at
    /// `DISPEL_AT_S`), takes her line, and is gone: the whole visit.
    #[test]
    fn her_visit_casts_once_and_she_leaves_up_her_line() {
        let head = Vec2::new(800.0, 500.0);
        let mut at = Vec2::new(700.0, 520.0);
        let mut v = Visit::default();
        let mut hits = Vec::new();
        let mut throws = Vec::new();
        let mut said = Vec::new();
        let mut gone_after = None;
        for k in 0..600 {
            let tick = visit_tick_of(&mut v, at, Some(head), DT);
            if tick.ofuda_hit {
                hits.push(v.age);
            }
            said.extend(tick.says);
            throws.extend(tick.throw);
            if tick.dismissed {
                gone_after = Some(k as f32 * DT);
                break;
            }
            at += Vec2::from(tick.course) * DT;
        }
        assert_eq!(hits.len(), 1, "her talismans reach it once: {hits:?}");
        assert!((hits[0] - DISPEL_AT_S).abs() < 2.0 * DT, "they reach it at {}, not {DISPEL_AT_S}", hits[0]);
        assert_eq!(said, vec![LINE_ARRIVE, LINE_CAST, LINE_LEAVE]);
        assert_eq!(throws.len(), 1, "she throws once: {throws:?}");
        assert_eq!(throws[0].1, <[f32; 2]>::from(head), "she threw at the head");
        let gone = gone_after.expect("she never left");
        assert!(gone < 4.0, "her visit took {gone} s");
        assert!(at.y < head.y - 400.0, "she did not go up her line: {at:?}");
    }

    /// Her volley spans the way to the head, midway, turned to it: its
    /// talismans leave her hand and strike the head, whichever side it is on.
    #[test]
    fn her_volley_runs_from_her_hand_to_the_head() {
        for (from, to) in [(Vec2::new(100.0, 500.0), Vec2::new(400.0, 460.0)), (Vec2::new(900.0, 500.0), Vec2::new(650.0, 520.0))] {
            let fx = volley(from, to);
            let Place::World(mid) = fx.at else { panic!("{:?}", fx.at) };
            let half = FX_WORLD * fx.scale * VOLLEY_SPAN / 2.0;
            let dir = Vec2::from_angle(fx.angle);
            assert!((Vec2::from(mid) - dir * half).distance(from) < 1e-3, "it starts off her hand: {fx:?}");
            assert!((Vec2::from(mid) + dir * half).distance(to) < 1e-3, "it ends off the head: {fx:?}");
        }
    }
}
