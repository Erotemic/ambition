//! What GNU-ton's two fists do, as functions of time.
//!
//! GNU-ton does not fight with his fists; he LECTURES with them. Every move is a
//! demonstration of one of his laws, performed on the player, and every
//! demonstration contains its own refutation:
//!
//! * **Demonstration** (the slam): a fist hovers over you, then falls. A falling
//!   body stops where the floor is, and a fist in the floor is a fist you can hit.
//! * **Pendulum**: a fist on a string, swinging a full arc low across the hall.
//!   A pendulum is the most predictable thing in physics.
//! * **Fluxions** (his signature): he draws the curve in the air first, then a
//!   fist traces it. The drawing IS the telegraph; stand where the curve is high.
//! * **Newton's Cradle**: the fists hang side by side and trade one impulse back
//!   and forth, clack by clack. Only the swinging fist is dangerous.
//! * **Orbits**: the fists orbit him like planets. Slip through between them.
//!
//! Everything here is pure: a pose is a function of the beat, the time in it,
//! and a few facts latched when the beat began (where the player was, which
//! fist leads). The conductor reads the scholar's live move, latches, and
//! asks for the poses and hit volumes.
//!
//! Coordinates are world units, y down.

use bevy_math::Vec2;

/// Which fist. `Left` is the fist on the giant's left in world space (smaller x).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Fist {
    Left,
    Right,
}

impl Fist {
    pub const BOTH: [Fist; 2] = [Fist::Left, Fist::Right];

    pub fn other(self) -> Fist {
        match self {
            Fist::Left => Fist::Right,
            Fist::Right => Fist::Left,
        }
    }

    /// `-1` for the left fist, `+1` for the right: the side it lives on.
    pub fn side(self) -> f32 {
        match self {
            Fist::Left => -1.0,
            Fist::Right => 1.0,
        }
    }

    pub fn index(self) -> usize {
        match self {
            Fist::Left => 0,
            Fist::Right => 1,
        }
    }
}

/// A floor and the walls either side of it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hall {
    /// The floor's top surface.
    pub floor: f32,
    /// Inner faces of the side walls.
    pub left: f32,
    pub right: f32,
}

impl Hall {
    pub fn center_x(&self) -> f32 {
        (self.left + self.right) * 0.5
    }
}

/// Where the performers are this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stage {
    pub hall: Hall,
    /// Each fist's idle anchor beside the giant (index by [`Fist::index`]).
    pub homes: [Vec2; 2],
    /// The scholar's centre: the orbit's sun.
    pub scholar: Vec2,
    /// A fist's full size.
    pub fist: Vec2,
}

impl Stage {
    pub fn home(&self, fist: Fist) -> Vec2 {
        self.homes[fist.index()]
    }

    /// A fist's centre when it rests on the floor at `x`.
    pub fn on_floor(&self, x: f32) -> Vec2 {
        Vec2::new(x, self.hall.floor - self.fist.y * 0.5)
    }

    /// Clamp an x so a fist of this size stays inside the hall.
    pub fn clamp_x(&self, x: f32) -> f32 {
        let margin = self.fist.x * 0.5 + 8.0;
        x.clamp(self.hall.left + margin, self.hall.right - margin)
    }
}

/// Facts latched when a beat began. No `Default`: `from` is where the fists
/// stand, and a zero there is the world origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Latch {
    /// Where the player was at the telegraph's start (or its lock, for a
    /// tracking move, which keeps updating it until the lock).
    pub aim: Vec2,
    /// The fist that leads this move.
    pub lead: Option<Fist>,
    /// Each fist's position when the beat began, so every move eases in from
    /// wherever the fist was instead of snapping.
    pub from: [Vec2; 2],
    /// Fluxions: the curve's phase, chosen so a dip lands under the player.
    pub curve_phase: f32,
}

/// One fist's pose this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub pos: Vec2,
    /// Touching it hurts.
    pub harmful: bool,
    /// Buried in the floor (or limp): the punish window. Drawn and read as
    /// stuck; striking it is the point.
    pub stuck: bool,
}

impl Pose {
    pub fn idle(pos: Vec2) -> Self {
        Self {
            pos,
            harmful: false,
            stuck: false,
        }
    }
}

// ─── Easing ──────────────────────────────────────────────────────────────────

fn clamp01(t: f32) -> f32 {
    t.clamp(0.0, 1.0)
}

/// Smoothstep: eased at both ends.
pub fn ease(t: f32) -> f32 {
    let t = clamp01(t);
    t * t * (3.0 - 2.0 * t)
}

/// Accelerating (a falling body).
fn ease_in(t: f32) -> f32 {
    let t = clamp01(t);
    t * t
}

fn lerp(a: Vec2, b: Vec2, t: f32) -> Vec2 {
    a + (b - a) * t
}

/// Ease a fist from where the beat found it to where the move wants it.
fn arrive(from: Vec2, to: Vec2, t: f32, over: f32) -> Vec2 {
    if over <= 0.0 {
        return to;
    }
    lerp(from, to, ease(t / over))
}

// ─── Idle ────────────────────────────────────────────────────────────────────

/// A fist at rest beside the giant: a slow conductor's bob.
pub fn idle(stage: &Stage, fist: Fist, clock: f32) -> Pose {
    let bob = (clock * 1.7 + fist.index() as f32 * 1.3).sin() * 10.0;
    Pose::idle(stage.home(fist) + Vec2::new(0.0, bob))
}

// ─── Demonstration (the slam) ────────────────────────────────────────────────

pub mod demonstrate {
    use super::*;

    /// How high over the floor the fist hovers while it tracks a player on the
    /// floor…
    pub const HOVER: f32 = 330.0;
    /// …and how high over a player standing higher up (a perch, the gnu's back).
    /// It still falls all the way to the floor, through whatever they stood on.
    pub const OVER_HEAD: f32 = 260.0;
    /// Never above this much over the floor.
    pub const CEILING: f32 = 900.0;
    /// The last stretch of the telegraph is a LOCK: the fist stops following and
    /// trembles, so the player has a fixed thing to step out from under.
    pub const LOCK: f32 = 0.3;
    /// How fast the aim follows the player during the track.
    pub const TRACK_SPEED: f32 = 520.0;
    /// The fall.
    pub const FALL: f32 = 0.13;
    /// Buried: the punish window.
    pub const STUCK: f32 = 0.85;
    /// Harmful for this long after impact (the shock of landing), then it is
    /// only a fist in the floor.
    pub const IMPACT: f32 = 0.06;

    /// The hover point over an aim.
    pub fn hover(stage: &Stage, aim: Vec2) -> Vec2 {
        let y = (stage.hall.floor - HOVER).min(aim.y - OVER_HEAD).max(stage.hall.floor - CEILING);
        Vec2::new(stage.clamp_x(aim.x), y)
    }

    pub fn telegraph(stage: &Stage, latch: &Latch, fist: Fist, t: f32, dur: f32) -> Pose {
        let target = hover(stage, latch.aim);
        let pos = arrive(latch.from[fist.index()], target, t, 0.35);
        // The lock trembles: a fist about to fall.
        let locked = t > dur - LOCK;
        let tremble = if locked {
            Vec2::new((t * 90.0).sin() * 4.0, (t * 70.0).cos() * 2.0)
        } else {
            Vec2::ZERO
        };
        Pose::idle(pos + tremble)
    }

    /// `t` is time into the strike; `dur` its length.
    pub fn strike(stage: &Stage, latch: &Latch, fist: Fist, t: f32, dur: f32) -> Pose {
        let top = hover(stage, latch.aim);
        let bottom = stage.on_floor(top.x);
        if t < FALL {
            return Pose {
                pos: lerp(top, bottom, ease_in(t / FALL)),
                harmful: true,
                stuck: false,
            };
        }
        let buried = t - FALL;
        if buried < STUCK {
            return Pose {
                pos: bottom,
                harmful: buried < IMPACT,
                stuck: true,
            };
        }
        // Back home over what is left of the strike.
        let rise = (buried - STUCK) / (dur - FALL - STUCK).max(0.05);
        Pose::idle(lerp(bottom, stage.home(fist), ease(rise)))
    }

    /// Is this the tick the fist lands? (`prev_t` is last tick's strike time.)
    pub fn lands(prev_t: f32, t: f32) -> bool {
        prev_t < FALL && t >= FALL
    }
}

/// A pair demonstration: the second fist follows the first by this much.
pub const PAIR_STAGGER: f32 = 0.45;

// ─── Pendulum ────────────────────────────────────────────────────────────────

pub mod pendulum {
    use super::*;

    /// The pivot's height above the floor.
    pub const PIVOT_HEIGHT: f32 = 600.0;
    /// How far off the floor the fist's bottom passes at the lowest point: low
    /// enough that only a double jump clears it.
    pub const SKIM: f32 = 10.0;
    /// Release angle (radians from straight down).
    pub const AMPLITUDE: f32 = 1.05;

    /// The pivot hangs over where the player stood when he raised the fist, so
    /// the bottom of the arc is where they were. Clamped so the whole arc stays
    /// inside the hall.
    pub fn pivot(stage: &Stage, latch: &Latch) -> Vec2 {
        let reach = length(stage) * AMPLITUDE.sin() + stage.fist.x * 0.5 + 8.0;
        let (lo, hi) = (stage.hall.left + reach, stage.hall.right - reach);
        let x = if lo < hi { latch.aim.x.clamp(lo, hi) } else { stage.hall.center_x() };
        Vec2::new(x, stage.hall.floor - PIVOT_HEIGHT)
    }

    pub fn length(stage: &Stage) -> f32 {
        PIVOT_HEIGHT - SKIM - stage.fist.y * 0.5
    }

    /// The fist's centre at angle `theta` (0 = hanging straight down, positive
    /// swings toward +x).
    pub fn at(stage: &Stage, latch: &Latch, theta: f32) -> Vec2 {
        pivot(stage, latch) + Vec2::new(theta.sin(), theta.cos()) * length(stage)
    }

    /// The side it is released from: the lead fist's own side.
    fn release_side(latch: &Latch) -> f32 {
        latch.lead.map_or(-1.0, Fist::side)
    }

    pub fn telegraph(stage: &Stage, latch: &Latch, fist: Fist, t: f32, _dur: f32) -> Pose {
        let release = at(stage, latch, AMPLITUDE * release_side(latch));
        Pose::idle(arrive(latch.from[fist.index()], release, t, 0.6))
    }

    /// One full period (out and back) over the strike: two passes to clear.
    pub fn strike(stage: &Stage, latch: &Latch, _fist: Fist, t: f32, dur: f32) -> Pose {
        let side = release_side(latch);
        let theta = AMPLITUDE * side * (std::f32::consts::PI * 2.0 * t / dur.max(0.1)).cos();
        Pose {
            pos: at(stage, latch, theta),
            harmful: true,
            stuck: false,
        }
    }
}

// ─── Fluxions ────────────────────────────────────────────────────────────────

pub mod fluxions {
    use super::*;

    /// Lobes in one curve: the curve dips to the floor this many times + 1.
    pub const LOBES: f32 = 2.5;
    /// How high the curve rises between dips (fist centre above its floor pose).
    pub const RISE: f32 = 300.0;
    /// Margin from each wall where the curve starts and ends.
    pub const MARGIN: f32 = 90.0;
    /// How much of the telegraph the drawing takes (it then holds, complete).
    pub const DRAW: f32 = 0.8;

    /// The curve he writes: `u` runs 0..1 from the lead fist's side across the
    /// hall. `mirror` flips which side it starts from (the second fist of a
    /// pair traces the reflection).
    pub fn point(stage: &Stage, latch: &Latch, u: f32, mirror: bool) -> Vec2 {
        let start_side = latch.lead.map_or(-1.0, Fist::side) * if mirror { -1.0 } else { 1.0 };
        let left = stage.hall.left + MARGIN;
        let right = stage.hall.right - MARGIN;
        let x = if start_side < 0.0 {
            left + (right - left) * u
        } else {
            right - (right - left) * u
        };
        // Dips (`cos == 1` → floor) placed by the latched phase so one lands
        // where the player stood. `(1 - cos)/2` rises 0..1 between dips.
        let wave = (1.0 - (std::f32::consts::TAU * (LOBES * u + latch.curve_phase)).cos()) * 0.5;
        stage.on_floor(x) - Vec2::new(0.0, RISE * wave)
    }

    /// The phase that puts a dip under `aim` for a curve started from `side`.
    pub fn phase_for(stage: &Stage, aim_x: f32, side: f32) -> f32 {
        let left = stage.hall.left + MARGIN;
        let right = stage.hall.right - MARGIN;
        let u = ((aim_x - left) / (right - left)).clamp(0.0, 1.0);
        let u = if side < 0.0 { u } else { 1.0 - u };
        // cos(2π(LOBES u + φ)) = 1  ⇔  LOBES u + φ ∈ ℤ.
        (-(LOBES * u)).rem_euclid(1.0)
    }

    pub fn telegraph(stage: &Stage, latch: &Latch, fist: Fist, t: f32, _dur: f32, mirror: bool) -> Pose {
        Pose::idle(arrive(latch.from[fist.index()], point(stage, latch, 0.0, mirror), t, 0.5))
    }

    /// How much of the curve is drawn `t` into the telegraph.
    pub fn drawn(t: f32, dur: f32) -> f32 {
        clamp01(t / (dur * DRAW).max(0.05))
    }

    pub fn strike(stage: &Stage, latch: &Latch, _fist: Fist, t: f32, dur: f32, mirror: bool) -> Pose {
        // The trace takes most of the strike; the tail is the fist sliding off
        // the end, harmless.
        let trace = dur * 0.85;
        if t <= trace {
            Pose {
                pos: point(stage, latch, ease(t / trace), mirror),
                harmful: true,
                stuck: false,
            }
        } else {
            Pose::idle(point(stage, latch, 1.0, mirror))
        }
    }
}

// ─── Newton's Cradle ─────────────────────────────────────────────────────────

pub mod cradle {
    use super::*;

    /// Pivot height above the floor (the same rig as the pendulum).
    pub const PIVOT_HEIGHT: f32 = 560.0;
    pub const AMPLITUDE: f32 = 0.95;
    /// Half-swings in one strike: out-and-back is two.
    pub const HALF_SWINGS: f32 = 6.0;

    fn pivot(stage: &Stage, fist: Fist) -> Vec2 {
        Vec2::new(
            stage.hall.center_x() + fist.side() * stage.fist.x * 0.5,
            stage.hall.floor - PIVOT_HEIGHT,
        )
    }

    fn length(stage: &Stage) -> f32 {
        PIVOT_HEIGHT - 12.0 - stage.fist.y * 0.5
    }

    fn at(stage: &Stage, fist: Fist, theta: f32) -> Vec2 {
        pivot(stage, fist) + Vec2::new(theta.sin(), theta.cos()) * length(stage)
    }

    /// The fist that starts drawn back: the lead.
    fn opener(latch: &Latch) -> Fist {
        latch.lead.unwrap_or(Fist::Left)
    }

    pub fn telegraph(stage: &Stage, latch: &Latch, fist: Fist, t: f32, _dur: f32) -> Pose {
        let target = if fist == opener(latch) {
            at(stage, fist, AMPLITUDE * fist.side())
        } else {
            at(stage, fist, 0.0)
        };
        Pose::idle(arrive(latch.from[fist.index()], target, t, 0.6))
    }

    /// Which half-swing `t` falls in, and how far through it.
    pub fn half_swing(t: f32, dur: f32) -> (u32, f32) {
        let per = dur / HALF_SWINGS;
        let n = (t / per).floor().max(0.0);
        (n as u32, (t - n * per) / per)
    }

    /// The fist that moves during half-swing `n`: the opener falls in (n = 0),
    /// the far fist flies out and back (1, 2), the opener out and back (3, 4)…
    pub fn mover(latch: &Latch, n: u32) -> Fist {
        let opener = opener(latch);
        // 0: opener in. 1,2: other out+in. 3,4: opener out+in. 5: other out…
        if n == 0 || ((n + 1) / 2) % 2 == 0 {
            opener
        } else {
            opener.other()
        }
    }

    pub fn strike(stage: &Stage, latch: &Latch, fist: Fist, t: f32, dur: f32) -> Pose {
        let (n, f) = half_swing(t, dur);
        if fist != mover(latch, n) {
            return Pose::idle(at(stage, fist, 0.0));
        }
        // Falling in (toward 0) on even-numbered legs from the opener's start and
        // on every "in" leg; flying out on the others. A leg is a quarter of a
        // pendulum period: angle = A·cos(πf/2) falling in, A·sin(πf/2) out.
        let out = n != 0 && n % 2 == 1;
        let theta = if out {
            AMPLITUDE * (std::f32::consts::FRAC_PI_2 * f).sin()
        } else {
            AMPLITUDE * (std::f32::consts::FRAC_PI_2 * f).cos()
        };
        Pose {
            pos: at(stage, fist, theta * fist.side()),
            harmful: true,
            stuck: false,
        }
    }

    /// Clack: the tick a falling-in leg ends at the bottom.
    pub fn clacks(prev_t: f32, t: f32, dur: f32) -> bool {
        let (prev_n, _) = half_swing(prev_t, dur);
        let (n, _) = half_swing(t, dur);
        n != prev_n && prev_n % 2 == 0 && (n as f32) < HALF_SWINGS
    }
}

// ─── Orbits ──────────────────────────────────────────────────────────────────

pub mod orbit {
    use super::*;

    /// The ellipse's semi-axes around the scholar.
    pub const SEMI_X: f32 = 330.0;
    pub const SEMI_Y: f32 = 210.0;
    /// Seconds per revolution.
    pub const PERIOD: f32 = 2.2;
    /// The orbit breathes: the radius swells and shrinks by this fraction.
    pub const BREATH: f32 = 0.18;

    pub fn at(stage: &Stage, fist: Fist, t: f32) -> Vec2 {
        let angle = std::f32::consts::TAU * t / PERIOD + fist.index() as f32 * std::f32::consts::PI;
        let breath = 1.0 + BREATH * (std::f32::consts::TAU * t / (PERIOD * 1.5)).sin();
        let centre = stage.scholar;
        let mut p = centre + Vec2::new(angle.cos() * SEMI_X, angle.sin() * SEMI_Y) * breath;
        // Never through the floor.
        p.y = p.y.min(stage.hall.floor - stage.fist.y * 0.5);
        p
    }

    pub fn telegraph(stage: &Stage, latch: &Latch, fist: Fist, t: f32, _dur: f32) -> Pose {
        Pose::idle(arrive(latch.from[fist.index()], at(stage, fist, 0.0), t, 0.6))
    }

    pub fn strike(stage: &Stage, _latch: &Latch, fist: Fist, t: f32, _dur: f32) -> Pose {
        Pose {
            pos: at(stage, fist, t),
            harmful: true,
            stuck: false,
        }
    }
}

// ─── Apple rain and Eureka ───────────────────────────────────────────────────

/// While apples fall he shakes the tree: both fists up, trembling. Harmless.
pub fn shake_the_tree(stage: &Stage, latch: &Latch, fist: Fist, t: f32) -> Pose {
    let up = stage.scholar + Vec2::new(fist.side() * 150.0, -170.0);
    let shake = Vec2::new((t * 40.0 + fist.index() as f32).sin() * 12.0, (t * 33.0).cos() * 6.0);
    Pose::idle(arrive(latch.from[fist.index()], up, t, 0.4) + shake)
}

/// What each body is DRAWN as through the fight: the named sheet rows the
/// conductor pins (`PinnedRow`). `None` is "whatever the body is doing".
pub mod rows {
    use super::*;

    /// Rows in preference order, seconds into the row, and whether it loops.
    pub type Pin = (&'static [&'static str], f32, bool);

    /// The stomp row's slam lands this far into it (0.7 of 8 frames at 70 ms):
    /// the telegraph plays the rear-up so the slam falls on the strike.
    pub const STOMP_SLAM_S: f32 = 0.392;

    /// The scholar: he points a fist down, conducts a swing, reads the rain
    /// down from his book, braces while the gnu bucks, and — eureka — tumbles,
    /// sits dazed and climbs back.
    pub fn scholar(cue: Option<&Cue>) -> Option<Pin> {
        let cue = cue?;
        Some(match cue.mv {
            Move::Demonstrate | Move::DemonstratePair | Move::PortalPunch | Move::PortalKick => {
                (&["point"], cue.beat_t, false)
            }
            Move::Pendulum | Move::Cradle | Move::Orbit | Move::Fluxions | Move::FluxionsPair => {
                (&["conduct"], cue.beat_t, true)
            }
            Move::AppleRain => (&["invoke"], cue.beat_t, false),
            Move::Buck | Move::Stomp => (&["brace"], cue.beat_t, true),
            Move::Eureka => match eureka::part(cue.t, cue.dur) {
                // He is reading when the apple lands.
                eureka::Part::AppleFalling(_) => return None,
                eureka::Part::Tumbling(_) => (&["tumble"], cue.t - eureka::APPLE, false),
                eureka::Part::Dazed => (&["dazed"], cue.t - eureka::APPLE - eureka::TUMBLE, true),
                eureka::Part::Climbing(_) => (&["climb"], cue.t, true),
            },
        })
    }

    /// The gnu: it bucks on the strike, and rears through a stomp's telegraph
    /// so its front hooves come down as the shock goes out.
    pub fn gnu(cue: Option<&Cue>) -> Option<Pin> {
        let cue = cue?;
        match (cue.mv, cue.striking) {
            (Move::Buck, true) => Some((&["buck"], cue.t, false)),
            (Move::Stomp, false) => Some((&["stomp"], STOMP_SLAM_S * (cue.t / cue.dur.max(1e-3)).min(1.0), false)),
            (Move::Stomp, true) => Some((&["stomp"], STOMP_SLAM_S + cue.t, false)),
            _ => None,
        }
    }

    /// A fist: driven into the floor and straining, coming down, or hovering.
    pub fn fist(pose: &Pose, clock: f32) -> Pin {
        let row: &'static [&'static str] = if pose.stuck {
            &["stuck"]
        } else if pose.harmful {
            &["fall"]
        } else {
            &["rest"]
        };
        (row, clock, true)
    }
}

/// Phase 1 and its end: the giant's arms are on it, then go into two portals.
/// Its blows through a pair are [`blows`].
///
/// The places are of the giant's DRAWING (`giant_art.py` of the sprite
/// renderer: `CLAMP_LINE_L`, `CLAMP_LINE_R`, `FIST_STUB_LINE`), in world
/// units from the middle of its body, typed here by hand. That is a debt: `docs/planning/engine/
/// the-art-says-where-a-body-stands.md`.
pub mod arms {
    use super::*;

    /// With its arms on it the giant lets him take blows from its back: at
    /// this blow it throws. Each blow launches HIM (`launch`).
    pub const BLOWS_BEFORE_BUCK: u32 = 3;
    /// The way back to his saddle when the fight on the back is over: this
    /// long, this high.
    pub const HOP: f32 = 0.42;
    pub const HOP_HEIGHT: f32 = 70.0;

    /// Where he is `t` seconds into a hop from `from` to `to`.
    pub fn hop(from: Vec2, to: Vec2, t: f32) -> Vec2 {
        let u = ease(t / HOP);
        from + (to - from) * u - Vec2::new(0.0, (std::f32::consts::PI * u).sin() * HOP_HEIGHT)
    }

    /// The back is a platform, and on it he is a fighter: he goes to the
    /// player, he rolls through him, a blow launches him, and he comes back
    /// when it throws him off. The numbers of that.
    ///
    /// He goes to a player that is out of his reach at `WALK`, and at `RUN`
    /// from farther than `RUN_FROM`. He comes no nearer than `EDGE` to an end
    /// of the back.
    pub const WALK: f32 = 120.0;
    pub const RUN: f32 = 270.0;
    pub const RUN_FROM: f32 = 200.0;
    pub const EDGE: f32 = 22.0;
    /// The player is in the fight while he is over the back, to this high
    /// over it: a jump does not end the fight.
    pub const OVER_THE_BACK: f32 = 230.0;
    /// With the player gone from the back for this long, he goes to his saddle.
    pub const HOME_AFTER: f32 = 1.1;

    /// A blow launches him away from it. The speed grows with each blow
    /// since the giant last threw (the first moves him a step along the
    /// back, the third throws him off it), with the hearts of the blow, and
    /// with the part of his health he lost (a fighter's percent; he has many
    /// hearts, so this is small in one stay on the back).
    pub const LAUNCH_BASE: f32 = 300.0;
    pub const LAUNCH_PER_BLOW: f32 = 150.0;
    pub const LAUNCH_PER_HEART: f32 = 90.0;
    pub const LAUNCH_GROWTH: f32 = 400.0;
    /// The part of the speed that is along the back, and that is up.
    pub const LAUNCH_ALONG: f32 = 0.66;
    pub const LAUNCH_UP: f32 = 1.05;
    /// What pulls him down in the air.
    pub const GRAVITY: f32 = 1500.0;
    /// He does not act for this long after he lands.
    pub const LAND: f32 = 0.22;
    /// For this long of a launch he is drawn in the row `launch`, then he tumbles.
    pub const LAUNCH_ROW: f32 = 0.3;

    /// His speed when a blow launches him to the side `away` (1 or -1): the
    /// `blow`th since the giant last threw (from 1), of `hearts`, with `lost`
    /// of his health gone (0 to 1).
    pub fn launch(away: f32, blow: u32, hearts: i32, lost: f32) -> Vec2 {
        let speed = LAUNCH_BASE
            + LAUNCH_PER_BLOW * blow.saturating_sub(1).min(4) as f32
            + LAUNCH_PER_HEART * (hearts - 1).clamp(0, 4) as f32
            + LAUNCH_GROWTH * lost.clamp(0.0, 1.0);
        Vec2::new(away * speed * LAUNCH_ALONG, -speed * LAUNCH_UP)
    }

    /// Thrown off the back, he comes back when he is this far under it: he
    /// rises (his `levity`) to a place this far in from the end of the back.
    /// He has no guard on the way: a blow there launches him again.
    pub const RECOVER_UNDER: f32 = 44.0;
    pub const RECOVER_IN: f32 = 48.0;
    pub const RECOVER_SPEED: f32 = 330.0;
    pub const RECOVER_OVER: f32 = 58.0;

    /// The seconds of the way back from `from` to `to`.
    pub fn recover_time(from: Vec2, to: Vec2) -> f32 {
        ((to - from).length() / RECOVER_SPEED).clamp(0.5, 1.5)
    }

    /// Where he is at `u` (0 to 1) of the way back: up and over the end of
    /// the back, and down onto it.
    pub fn recover(from: Vec2, to: Vec2, u: f32) -> Vec2 {
        let u = u.clamp(0.0, 1.0);
        // He rises first and goes in after: the end of the back is in his way.
        let along = ease(((u - 0.25) / 0.75).clamp(0.0, 1.0));
        let up = ease((u / 0.7).clamp(0.0, 1.0));
        let over = (std::f32::consts::PI * u).sin() * RECOVER_OVER;
        Vec2::new(from.x + (to.x - from.x) * along, from.y + (to.y - from.y) * up - over)
    }

    /// He reads the player's blow (the conductor sees the move the player
    /// plays). With no move of his own, and the player this near, his book
    /// goes up `REACT` seconds into a blow that has not struck yet: a quick
    /// blow is there before the book, a slow one is not. And when a blow of
    /// the player is over, with this much of it or more still to play, he
    /// strikes at once: a blow on his book, or at the air, is paid for.
    pub const REACT: f32 = 0.13;
    pub const REACT_REACH: f32 = 150.0;
    pub const PUNISH_LEAST: f32 = 0.05;

    /// The roll: with the player at his elbow he rolls THROUGH him, to this
    /// far on his other side, in this long. For the part `ROLL_SAFE` of it
    /// his book is up.
    pub const ROLL: f32 = 0.4;
    pub const ROLL_PAST: f32 = 58.0;
    pub const ROLL_SAFE: (f32, f32) = (0.04, 0.3);
    /// After a roll he chooses in this long: the roll is the way to the
    /// player's back.
    pub const ROLL_THEN: f32 = 0.08;

    /// Does he roll at his `turn`th choice, with the player at `to_player`?
    pub fn duel_rolls(to_player: Vec2, turn: u32) -> bool {
        to_player.x.abs() <= 64.0 && to_player.y >= -44.0 && turn % 5 == 4
    }

    /// Where he is `t` seconds into a roll along the back from `from` to `to`.
    pub fn roll(from: f32, to: f32, t: f32) -> f32 {
        from + (to - from) * ease(t / ROLL)
    }

    /// The duel. With the giant's arms on it and the player on its back, HE
    /// fights: moves of his own table (`movesets/gnu_ton.ron`, the table of
    /// the fighter `npc_gnu_ton`), which the game plays on his body
    /// (`ambition_content::bosses::gnu_ton::duel`). This says only WHICH move
    /// and WHEN; a move's times, reach and hurt are in the table.
    ///
    /// Each: the move's id in the table, the seconds until he chooses again,
    /// and for how many of them he is OPEN (the move, and the time to punish
    /// it). For the rest he has his book up: a blow on his guard does nothing.
    /// So the way to hurt him is the way to hurt a fighter: make him miss, and
    /// strike before the book comes up.
    pub const DUEL_MOVES: [(&str, f32, f32); 8] = [
        ("jab", 0.7, 0.46),
        ("tilt_forward", 0.9, 0.62),
        ("tilt_up", 0.9, 0.62),
        ("tilt_down", 0.85, 0.58),
        ("smash_forward", 1.55, 1.25),
        // The rest of the jab's string (`string_next`).
        ("jab_2", 0.7, 0.46),
        ("jab_3", 0.95, 0.66),
        // Out of his run (`DASH_ATTACK`).
        ("gnu_ton_dash_attack", 1.05, 0.76),
    ];
    /// The jab is a string of three. With the player still at his elbow
    /// this long after a jab started, the next of the string starts (when
    /// the jab is over: the game starts a move when the last is gone).
    pub const STRING_AT: f32 = 0.2;
    pub const STRING_REACH: f32 = 70.0;

    /// The move that follows the move at `choice` in a string.
    pub fn string_next(choice: usize) -> Option<usize> {
        match choice {
            0 => Some(5),
            5 => Some(6),
            _ => None,
        }
    }

    /// He runs at a player who is far, and from this far he strikes out of
    /// the run: the place of the dash attack in `DUEL_MOVES`. He slides on in
    /// it for this long, to this far from where the player was (the move's
    /// reach is short: it is his shoulder).
    pub const DASH_ATTACK: usize = 7;
    pub const DASH_ATTACK_FROM: f32 = 150.0;
    pub const DASH_SLIDE: f32 = 0.26;
    pub const DASH_SLIDE_TO: f32 = 40.0;
    /// The beat before his first move, when the player comes onto the back.
    pub const DUEL_READY: f32 = 0.35;
    /// After he lands from a launch, or the giant throws, he does not swing
    /// for this long.
    pub const DUEL_STAGGER: f32 = 0.28;

    /// The move he chooses (a place in `DUEL_MOVES`) for a player at
    /// `to_player` from him, at his `turn`th choice. `None`: out of reach.
    pub fn duel_choice(to_player: Vec2, turn: u32) -> Option<usize> {
        let reach = to_player.x.abs();
        if to_player.y < -44.0 && reach < 70.0 {
            // Over him: the flourish over his head.
            Some(2)
        } else if reach <= 64.0 {
            // At his elbow: the jab, and each third time the sweep.
            Some(if turn % 3 == 2 { 3 } else { 0 })
        } else if reach <= 124.0 {
            // At the quill's length: the thrust, and each fourth time the book.
            Some(if turn % 4 == 3 { 4 } else { 1 })
        } else {
            None
        }
    }

    /// How far the lines come open when he is dazed (0 is tight): his spell
    /// has no hold, and they flicker.
    pub fn dazed_slack(clock: f32) -> f32 {
        0.3 + 0.35 * (clock * 23.0).sin().abs()
    }

    /// The row `armed_rest`, and its copy: a pinned row does not loop.
    pub const REST_ROW: f32 = 0.88;

    /// The two lines the arms are in after phase 1 (left, right): each stands
    /// up, out from its shoulder, and the level fore arm goes through it.
    /// Each is at the middle of the arm's cut at rest (the renderer's
    /// `giant_art.limb_seams` of the row `rest`, 2026-10-24). The lines are
    /// drawn ON the cuts the giant's frames publish (`LineOnLimb`); this is
    /// where a line is while no cut is published.
    pub const SHOULDER_LINES: [Vec2; 2] = [Vec2::new(-89.0, -102.5), Vec2::new(389.4, -102.5)];
    /// Half the length of a shoulder's line: when it opens, and when it is
    /// tight on the arm. Tight, it is shorter than the arm is wide, so the
    /// line is as wide as the arm's cut.
    pub const SHOULDER_WIDE: f32 = 78.0;
    pub const SHOULDER_TIGHT: f32 = 20.0;
    /// The line behind the wrist of a fist that is a body of its own: the cut
    /// of its fore arm (`giant_art.fist_seams`).
    pub const WRIST_LINE: f32 = 97.5;
    pub const WRIST_WIDE: f32 = 72.0;
    pub const WRIST_TIGHT: f32 = 20.0;
    /// Where such a fist rests: this far out from the line its arm is in, and
    /// this far down (up when less than 0).
    pub const REST_OUT: Vec2 = Vec2::new(230.0, -29.6);

    /// Where the fist of a shoulder (0 left, 1 right) rests, from the middle
    /// of the giant, as the giant is drawn.
    pub fn rest(shoulder: usize) -> Vec2 {
        let out = if shoulder == 1 { REST_OUT.x } else { -REST_OUT.x };
        SHOULDER_LINES[shoulder] + Vec2::new(out, REST_OUT.y)
    }
    /// The clamp. The arms go out and into two wide lines (the row `clamp`);
    /// at `CLAMP_THROUGH` they are through, and the fists are bodies of their
    /// own from then. From `SQUEEZE` to `SNAP` the lines close on the arms. At
    /// `SNAP` they are tight, and the giant flinches (its row `hit`) for
    /// `FLINCH`. `CLAMP` is all of it; the encounter's beat between its phases
    /// (`transition_seconds`) is longer.
    pub const CLAMP_THROUGH: f32 = 0.45;
    pub const SQUEEZE: f32 = 0.85;
    pub const SNAP: f32 = 1.05;
    pub const FLINCH: f32 = 0.48;
    pub const CLAMP: f32 = SNAP + FLINCH;

    /// How tight the lines are on the arms `clamp` seconds into the clamp
    /// (`None`: it is over), from 0 (as wide as they open) to 1.
    pub fn tight(clamp: Option<f32>) -> f32 {
        clamp.map_or(1.0, |t| {
            let u = ((t - SQUEEZE) / (SNAP - SQUEEZE)).clamp(0.0, 1.0);
            u * u * u
        })
    }

    /// The giant with its arms on it at rest, and its buck. A blow's rows
    /// are [`super::blows::gnu`].
    pub fn gnu(cue: Option<&Cue>, clock: f32) -> rows::Pin {
        match cue.map(|cue| (cue.mv, cue.striking, cue)) {
            Some((Move::Buck, true, cue)) => (&["armed_buck", "buck"], cue.t, false),
            _ => {
                let turn = (clock / REST_ROW) as u32 % 2 == 0;
                let row: &'static [&'static str] = if turn { &["armed_rest"] } else { &["armed_rest_b", "armed_rest"] };
                (row, clock % REST_ROW, true)
            }
        }
    }

    /// A limb that is on the giant: the giant's picture has it.
    pub const NO_FIST: rows::Pin = (&["none"], 0.0, false);
}

/// The blows through a portal pair (Jon, 2026-10-23: "it should track where
/// the player is so gnuton spawns the portal it punches through where it
/// makes sense ... down diagonal, horizontal, or up ... sometimes its a kick
/// with a leg"; 2026-10-24: "it would be good if both arms could do it").
///
/// A blow is a limb of the giant that goes into the NEAR line of a real pair,
/// square to it, and comes out of the FAR line by the player. The near line is
/// fixed to the giant (where its drawing puts the limb); the far line is where
/// the player is, and at the angle that brings the limb at them. The limb's
/// body is posed in the giant's chart, past the near line as its picture is,
/// and the pair carries the body, its strike and the giant's picture of it to
/// the far line (`docs/planning/engine/portals-a-body-in-two-places.md`).
///
/// Each arm has a blow down (the HAMMER), one across (the right JAB, the left
/// HOOK out behind) and one up (the UPPERCUT); the near fore leg KICKS. Each
/// has a windup of its own, so it is read before it comes: the arm high over
/// the head is a hammer, the fist by the jaw a jab, the arm across the chest a
/// hook, the fist low by the knee an uppercut.
///
/// The places and the numbers of each limb are of the giant's DRAWING
/// (`giant_art.BLOWS` of the sprite renderer; `giant_art.blow_lines_world`
/// prints them), typed here by hand: debt 5 of
/// `docs/planning/engine/the-art-says-where-a-body-stands.md`.
pub mod blows {
    use super::*;

    /// World units of one design unit of the drawing (its frame is half the
    /// design, at 1.3 world units a pixel).
    pub const DESIGN: f32 = 0.65;
    /// The part of the row `*_strike` at which the blow is full.
    pub const STRIKE_FULL: f32 = 0.8;
    /// The part of the row `*_recover` at which the limb is out of its line.
    pub const RECOVER_OUT: f32 = 0.45;
    /// How far the end of a limb gives back toward its line in `*_hold`.
    pub const HOLD_GIVE: f32 = 14.0;
    /// The length of each row `*_windup` (12 frames of 60 ms). The tell plays
    /// it over its own length, so the blow is cocked at its end.
    pub const WINDUP_ROW: f32 = 0.72;
    /// The part of a windup row from which the limb draws back the last of
    /// the way (its hitch): the tell that the blow goes now.
    pub const HITCH_AT: f32 = 0.84;
    /// The part of a windup row at which the blow is set on its line: the
    /// pair is open by then.
    pub const SET_AT: f32 = 0.66;

    /// An arm of the giant. Its left arm is on the left of its drawing (the
    /// side of its rump); a giant that is turned has each on the other side.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Arm {
        Left,
        Right,
    }

    impl Arm {
        pub fn other(self) -> Arm {
            match self {
                Arm::Left => Arm::Right,
                Arm::Right => Arm::Left,
            }
        }

        /// A record holds an arm as this.
        pub fn code(self) -> u32 {
            match self {
                Arm::Left => 1,
                Arm::Right => 2,
            }
        }

        pub fn of_code(code: u32) -> Option<Arm> {
            match code {
                1 => Some(Arm::Left),
                2 => Some(Arm::Right),
                _ => None,
            }
        }

        /// Its fist's limb slot.
        pub fn slot(self) -> &'static str {
            match self {
                Arm::Left => FIST_LEFT,
                Arm::Right => FIST,
            }
        }
    }

    /// Which limb, and how it goes into its line.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Blow {
        /// The right arm, down from over the shoulder.
        Hammer,
        /// The right arm, straight out from the shoulder.
        Jab,
        /// The right arm, up in front of the chest.
        Upper,
        /// The left arm, down from over its shoulder, over the slope of the back.
        LeftHammer,
        /// The left arm, wound across the chest and thrown out behind, level.
        LeftHook,
        /// The left arm, up from the left of the chest.
        LeftUpper,
        /// The near fore leg, forward and low.
        Kick,
    }

    impl Blow {
        /// The arm that throws it; `None` for the kick.
        pub fn arm(self) -> Option<Arm> {
            match self {
                Blow::Hammer | Blow::Jab | Blow::Upper => Some(Arm::Right),
                Blow::LeftHammer | Blow::LeftHook | Blow::LeftUpper => Some(Arm::Left),
                Blow::Kick => None,
            }
        }

        /// A blow down into the floor: the hammer of either arm.
        pub fn is_hammer(self) -> bool {
            matches!(self, Blow::Hammer | Blow::LeftHammer)
        }
    }

    /// Where the limb comes out of the far line, and which way it goes.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Exit {
        /// Straight down onto the player: the fist lands on the floor.
        Down,
        /// Down at 45 degrees, away from the giant (or toward it when there is
        /// no room).
        Diagonal,
        /// Level, at the height of a player who is off the floor.
        Across,
        /// Straight up, from under a player who is off the floor.
        Up,
        /// The hoof level along the floor: a thing to jump.
        Sweep,
        /// The hoof down from over the player's head.
        Stomp,
    }

    impl Exit {
        /// A new exit goes at the END: a record holds an exit as its place here.
        pub const ALL: [Exit; 6] = [Exit::Down, Exit::Diagonal, Exit::Across, Exit::Up, Exit::Sweep, Exit::Stomp];

        pub fn code(self) -> u32 {
            Self::ALL.iter().position(|e| *e == self).unwrap_or(0) as u32 + 1
        }

        pub fn of_code(code: u32) -> Option<Exit> {
            code.checked_sub(1).and_then(|i| Self::ALL.get(i as usize).copied())
        }

        /// The blow of this exit thrown by `arm` (a kick's exit is the leg's).
        pub fn blow(self, arm: Arm) -> Blow {
            match (self, arm) {
                (Exit::Down | Exit::Diagonal, Arm::Right) => Blow::Hammer,
                (Exit::Down | Exit::Diagonal, Arm::Left) => Blow::LeftHammer,
                (Exit::Across, Arm::Right) => Blow::Jab,
                (Exit::Across, Arm::Left) => Blow::LeftHook,
                (Exit::Up, Arm::Right) => Blow::Upper,
                (Exit::Up, Arm::Left) => Blow::LeftUpper,
                (Exit::Sweep | Exit::Stomp, _) => Blow::Kick,
            }
        }

        /// A kick's exit: the hoof's, not an arm's.
        pub fn is_kick(self) -> bool {
            matches!(self, Exit::Sweep | Exit::Stomp)
        }

        /// The way the limb comes out of the far line (the far line's normal),
        /// for a blow that goes to the side `side` (1 or -1).
        pub fn way(self, side: f32) -> Vec2 {
            let s = if side < 0.0 { -1.0 } else { 1.0 };
            match self {
                Exit::Down | Exit::Stomp => Vec2::new(0.0, 1.0),
                Exit::Diagonal => Vec2::new(s, 1.0) * std::f32::consts::FRAC_1_SQRT_2,
                Exit::Across | Exit::Sweep => Vec2::new(s, 0.0),
                Exit::Up => Vec2::new(0.0, -1.0),
            }
        }
    }

    /// A limb's part of the drawing and its numbers.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Limb {
        /// The middle of its near line from the middle of the giant, as the
        /// giant is drawn (facing +x), and the line's normal: the side the
        /// limb comes from. The limb goes in against the normal.
        pub line: Vec2,
        pub normal: Vec2,
        /// Design units: how far the end of the limb (the wrist, the fetlock)
        /// is on the near side of the line when it is cocked, past it at the
        /// full blow, and on the near side when it is out again; how far the
        /// middle of the fist (the hoof) is past the wrist.
        pub back: f32,
        pub deep: f32,
        pub out: f32,
        pub past: f32,
        /// The lengths of its rows `*_strike`, `*_hold` and `*_recover`.
        pub strike: f32,
        pub hold: f32,
        pub recover: f32,
        /// The part of the windup from which the arm is out of the place of
        /// the near line's opening (the hanging arm is where a hammer's line
        /// is), so the pair opens from there; the part of the recover by
        /// which it is closed, before the arm goes home. The drawing's
        /// `open_from` and `close_at` (`giant_art.BLOWS`).
        pub open_from: f32,
        pub close_at: f32,
        /// Half the length of each line of its pair: as big as the limb needs.
        pub half: f32,
        /// Its limb slot on the giant's rig.
        pub slot: &'static str,
        /// What it deals, and the launch over the player's standard one.
        pub damage: i32,
        pub knockback: f32,
        /// The half size of its strike, in world units.
        pub strike_half: Vec2,
    }

    impl Limb {
        /// How far the middle of the fist (the hoof) is past the far line at
        /// the full blow, in world units.
        pub fn reach(&self) -> f32 {
            (self.deep + self.past) * DESIGN
        }
    }

    /// The fists, and the hoof of the near fore leg.
    pub const FIST: &str = "hand_right";
    pub const FIST_LEFT: &str = "hand_left";
    pub const HOOF: &str = "hoof_right";

    pub const fn limb(blow: Blow) -> Limb {
        let fist = Vec2::new(54.0, 46.0);
        // The hammer: down, the heaviest, the fastest through the line.
        // The places of the near lines are the drawing's: each comes from the
        // arm of its blow, so only the forearm and the fist go through it
        // (`giant_art._derive_lines`, printed by `blow_lines_world`).
        let hammer = Limb {
            // Down and forward: the line is by the shoulder.
            line: Vec2::new(394.56, -187.5),
            normal: Vec2::new(-0.643, -0.766),
            back: 260.0,
            deep: 60.0,
            out: 140.0,
            past: 56.0,
            strike: 0.24,
            hold: 0.36,
            recover: 0.5,
            open_from: 0.3,
            close_at: 0.82,
            half: 84.0,
            slot: FIST,
            damage: 3,
            knockback: 2.8,
            strike_half: fist,
        };
        let upper = Limb {
            line: Vec2::new(395.67, -307.22),
            normal: Vec2::new(0.0, 1.0),
            back: 230.0,
            deep: 60.0,
            out: 140.0,
            past: 56.0,
            strike: 0.26,
            hold: 0.24,
            recover: 0.4,
            open_from: 0.05,
            close_at: 0.82,
            half: 84.0,
            slot: FIST,
            damage: 2,
            knockback: 2.6,
            strike_half: fist,
        };
        match blow {
            Blow::Hammer => hammer,
            // The left shoulder is at the middle of the body: its lines are
            // over the body, and its blows go up into them (a line in the
            // body would take the giant through its own portal).
            Blow::LeftHammer => Limb {
                line: Vec2::new(-97.55, -272.92),
                normal: Vec2::new(0.174, 0.985),
                open_from: 0.26,
                slot: FIST_LEFT,
                ..hammer
            },
            // Out and a little down: the line is turned to the forearm's way.
            Blow::Jab => Limb {
                line: Vec2::new(364.49, -58.76),
                normal: Vec2::new(-0.966, -0.259),
                back: 210.0,
                deep: 60.0,
                out: 140.0,
                past: 56.0,
                strike: 0.2,
                hold: 0.24,
                recover: 0.32,
                open_from: 0.3,
                close_at: 0.82,
                half: 84.0,
                slot: FIST,
                damage: 2,
                knockback: 2.4,
                strike_half: fist,
            },
            Blow::LeftHook => Limb {
                line: Vec2::new(-103.07, -291.66),
                normal: Vec2::new(0.342, 0.94),
                back: 210.0,
                deep: 60.0,
                out: 140.0,
                past: 56.0,
                strike: 0.2,
                hold: 0.24,
                recover: 0.36,
                open_from: 0.05,
                close_at: 0.82,
                half: 84.0,
                slot: FIST_LEFT,
                damage: 2,
                knockback: 2.5,
                strike_half: fist,
            },
            Blow::Upper => upper,
            Blow::LeftUpper => Limb { line: Vec2::new(-96.58, -324.78), slot: FIST_LEFT, ..upper },
            Blow::Kick => Limb {
                line: Vec2::new(315.25, 116.35),
                normal: Vec2::new(-1.0, 0.0),
                back: 50.0,
                deep: 100.0,
                out: 50.0,
                past: 24.0,
                strike: 0.30,
                hold: 0.24,
                recover: 0.42,
                open_from: 0.0,
                close_at: 1.0,
                half: 50.0,
                slot: HOOF,
                damage: 2,
                knockback: 2.2,
                strike_half: Vec2::new(30.0, 36.0),
            },
        }
    }

    /// Its rows: the tell, the blow, the hold, the way back. A kick with the
    /// arms on the giant has its own rows (its arms are drawn whole).
    pub fn rows(blow: Blow, arms_on: bool) -> [&'static [&'static str]; 4] {
        match (blow, arms_on) {
            (Blow::Hammer, _) => [&["hammer_windup"], &["hammer_strike"], &["hammer_hold"], &["hammer_recover"]],
            (Blow::Jab, _) => [&["jab_windup"], &["jab_strike"], &["jab_hold"], &["jab_recover"]],
            (Blow::Upper, _) => [&["upper_windup"], &["upper_strike"], &["upper_hold"], &["upper_recover"]],
            (Blow::LeftHammer, _) => {
                [&["left_hammer_windup"], &["left_hammer_strike"], &["left_hammer_hold"], &["left_hammer_recover"]]
            }
            (Blow::LeftHook, _) => [&["left_hook_windup"], &["left_hook_strike"], &["left_hook_hold"], &["left_hook_recover"]],
            (Blow::LeftUpper, _) => {
                [&["left_upper_windup"], &["left_upper_strike"], &["left_upper_hold"], &["left_upper_recover"]]
            }
            (Blow::Kick, true) => {
                [&["armed_kick_windup"], &["armed_kick_strike"], &["armed_kick_hold"], &["armed_kick_recover"]]
            }
            (Blow::Kick, false) => [&["kick_windup"], &["kick_strike"], &["kick_hold"], &["kick_recover"]],
        }
    }

    /// A part of the strike of a blow.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum Part {
        /// The row `*_strike`, seconds into it.
        Strike(f32),
        /// The row `*_hold`, seconds into it (it holds its last frame after).
        Hold(f32),
        /// The row `*_recover`, seconds into it.
        Recover(f32),
    }

    /// The part of the strike `t` seconds into it (`t` less the impact's
    /// freeze), with `left` seconds of it left: the recover takes the end.
    pub fn part(limb: &Limb, t: f32, left: f32) -> Part {
        if left <= limb.recover && t >= limb.strike.min(left) {
            Part::Recover((limb.recover - left).max(0.0))
        } else if t < limb.strike {
            Part::Strike(t.max(0.0))
        } else {
            Part::Hold(t - limb.strike)
        }
    }

    /// How far the end of the limb is past its near line (design units; less
    /// than 0: on the giant's side). `None`: it is not on its way through.
    /// The drawing's `blow_wrist_depth`.
    pub fn wrist_depth(limb: &Limb, part: Part) -> Option<f32> {
        match part {
            Part::Strike(t) => {
                let u = (t / limb.strike / STRIKE_FULL).min(1.0);
                Some(-limb.back + (limb.deep + limb.back) * u * u * u)
            }
            Part::Hold(t) => {
                let phase = (t / limb.hold).min(1.0);
                Some(limb.deep - HOLD_GIVE * (std::f32::consts::TAU * phase).sin().abs())
            }
            Part::Recover(t) => {
                let phase = t / limb.recover;
                (phase < RECOVER_OUT).then(|| limb.deep + (-limb.out - limb.deep) * ease(phase / RECOVER_OUT))
            }
        }
    }

    /// How far the middle of the fist (the hoof) is past the near line, in
    /// world units; `None`: it is not on its way through.
    pub fn depth(limb: &Limb, part: Part) -> Option<f32> {
        wrist_depth(limb, part).map(|wrist| (wrist + limb.past) * DESIGN)
    }

    /// The limb hurts: from the middle of its blow through its hold.
    pub fn harmful(limb: &Limb, part: Part) -> bool {
        match part {
            Part::Strike(t) => t >= limb.strike * 0.4,
            Part::Hold(t) => t < limb.hold,
            Part::Recover(_) => false,
        }
    }

    /// The time of the strike the end of the limb crosses its near line: it
    /// comes out of the far line then (the inverse of `wrist_depth`'s
    /// strike).
    pub fn through_time(limb: &Limb) -> f32 {
        let u = (limb.back / (limb.deep + limb.back).max(1e-3)).cbrt();
        u * limb.strike * STRIKE_FULL
    }

    /// How far into its strike the blow is in `part` (its hold goes on from
    /// the strike); `None` on its way back.
    pub fn strike_time(limb: &Limb, part: Part) -> Option<f32> {
        match part {
            Part::Strike(t) => Some(t),
            Part::Hold(t) => Some(limb.strike + t),
            Part::Recover(_) => None,
        }
    }

    /// The limb is out of its far line and can be met (`clash`): from when it
    /// comes through, for the clash's window. It hurts after that.
    pub fn meetable(limb: &Limb, part: Part) -> bool {
        let through = through_time(limb);
        strike_time(limb, part).is_some_and(|t| (through..through + super::clash::WINDOW).contains(&t))
    }

    /// The limb hurts: from the middle of its blow through its hold, and not
    /// while it can still be met.
    pub fn hurts(limb: &Limb, part: Part) -> bool {
        harmful(limb, part) && strike_time(limb, part).is_some_and(|t| t >= through_time(limb) + super::clash::WINDOW)
    }

    /// The blow is full between these two times of the strike: it lands, or
    /// it misses.
    pub fn lands(limb: &Limb, before: f32, now: f32) -> bool {
        let at = limb.strike * STRIKE_FULL;
        before < at && now >= at
    }

    /// The end of the limb crosses its near line between these two times of
    /// the strike: it comes out of the far line now.
    pub fn comes_through(limb: &Limb, before: f32, now: f32) -> bool {
        let depth = |t: f32| wrist_depth(limb, Part::Strike(t.max(0.0))).unwrap_or(-1.0);
        depth(before) < 0.0 && depth(now) >= 0.0
    }

    /// The giant's row in a blow: its tell over the tell's length, then the
    /// parts of the strike.
    pub fn gnu(blow: Blow, arms_on: bool, cue: &Cue, part: Option<Part>) -> rows::Pin {
        let [windup, strike, hold, recover] = rows(blow, arms_on);
        match (part, cue.striking) {
            (Some(Part::Strike(t)), true) => (strike, t, false),
            (Some(Part::Hold(t)), true) => (hold, t, false),
            (Some(Part::Recover(t)), true) => (recover, t, false),
            _ => (windup, tell(cue) * WINDUP_ROW, false),
        }
    }

    /// How far through its tell a blow is (0 to 1): the windup row is played
    /// over the tell's length.
    pub fn tell(cue: &Cue) -> f32 {
        (cue.t / cue.dur.max(0.1)).min(1.0)
    }

    /// The player is this far over the floor (their feet): off the floor.
    pub const HIGH: f32 = 48.0;
    /// A player this near the giant's flank is near it.
    pub const NEAR: f32 = 220.0;
    /// A far line is no nearer the giant's body than this, past its half
    /// length.
    pub const CLEAR: f32 = 26.0;
    /// Through the tell the far line follows the player this fast (a player
    /// runs at 270), until the last `LOCK` of it: then it stays, and the
    /// player has a thing to step out from under, and a way that is open.
    pub const TRACK_SPEED: f32 = 360.0;
    pub const LOCK: f32 = 0.3;
    /// A blow whose tell starts this soon after the last one ended is the next
    /// of a combination: the other arm throws it (the one-two). The pattern
    /// rests less than this between the blows of a combination, and more
    /// after a blow alone.
    pub const COMBO_GAP: f32 = 0.5;

    /// The exit of a blow, chosen by where the player is. `kick`: the move is
    /// a kick. `high`: the player is off the floor. `near`: by the giant's
    /// flank. `last`: the exit of the blow before (no blow is the same as the
    /// one before). `count`: the blows he has thrown: the first punch is
    /// straight down, the one that teaches the way up, and each third blow
    /// takes the other choice.
    pub fn choose(kick: bool, high: bool, near: bool, last: Option<Exit>, count: u32) -> Exit {
        if !kick && count == 0 && !high {
            return Exit::Down;
        }
        let choices: &[Exit] = match (kick, high) {
            (false, false) if near => &[Exit::Diagonal, Exit::Down],
            (false, false) => &[Exit::Down, Exit::Diagonal],
            (false, true) => &[Exit::Across, Exit::Up, Exit::Diagonal],
            (true, false) => &[Exit::Sweep, Exit::Stomp],
            (true, true) => &[Exit::Stomp, Exit::Sweep],
        };
        let first = if count % 3 == 2 { 1 } else { 0 };
        (0..choices.len()).map(|k| choices[(first + k) % choices.len()]).find(|e| Some(*e) != last).unwrap_or(choices[0])
    }

    /// The arm of a blow. The arm on the player's side of the giant (`side`:
    /// where the player is, in the giant's drawing: -1 its left, 1 its
    /// right), unless the blow follows another in a combination: then the
    /// other arm than the last (`last`).
    pub fn arm(side: f32, last: Option<Arm>, combo: bool) -> Arm {
        match last.filter(|_| combo) {
            Some(last) => last.other(),
            None if side < 0.0 => Arm::Left,
            None => Arm::Right,
        }
    }

    /// The side a blow goes to: away from the giant, unless its far line
    /// would be in the giant there.
    pub fn side(stage: &Stage, giant: Vec2, giant_size: Vec2, blow: Blow, exit: Exit, target: Vec2) -> f32 {
        let away = if target.x < giant.x { -1.0 } else { 1.0 };
        let (at, _) = far_end_unclear(stage, blow, exit, away, target);
        if in_giant(giant, giant_size, blow, exit, at) {
            -away
        } else {
            away
        }
    }

    fn in_giant(giant: Vec2, giant_size: Vec2, blow: Blow, exit: Exit, at: Vec2) -> bool {
        let half = limb(blow).half;
        let way = exit.way(1.0);
        // How far the line goes along x from its middle.
        let reach_x = half * way.y.abs() + CLEAR;
        (at.x - giant.x).abs() < giant_size.x * 0.5 + reach_x && at.y > giant.y - giant_size.y * 0.5 - CLEAR
    }

    /// Where the far line is, before it is kept clear of the giant.
    fn far_end_unclear(stage: &Stage, blow: Blow, exit: Exit, side: f32, target: Vec2) -> (Vec2, Vec2) {
        let l = limb(blow);
        let way = exit.way(side);
        let on_floor = stage.hall.floor - stage.fist.y * 0.5;
        // Where the middle of the fist (the hoof) is at the full blow.
        let end = match exit {
            Exit::Down => Vec2::new(target.x, on_floor),
            Exit::Diagonal => Vec2::new(target.x, target.y.min(on_floor)),
            _ => target,
        };
        let mut at = end - way * l.reach();
        // In the hall: between its walls, over its floor.
        let along = Vec2::new(-way.y, way.x).abs() * l.half;
        at.x = at.x.clamp(stage.hall.left + along.x + 8.0, stage.hall.right - along.x - 8.0);
        at.y = at.y.min(stage.hall.floor - along.y - 6.0);
        (at, way)
    }

    /// The far line of a blow `blow` with exit `exit` to the side `side`, for
    /// a player at `target`: its middle and its normal (the way the limb comes
    /// out). It is in the hall, over its floor, and clear of the giant.
    pub fn far_end(stage: &Stage, giant: Vec2, giant_size: Vec2, blow: Blow, exit: Exit, side: f32, target: Vec2) -> (Vec2, Vec2) {
        let (mut at, way) = far_end_unclear(stage, blow, exit, side, target);
        if in_giant(giant, giant_size, blow, exit, at) {
            let half = limb(blow).half;
            let clear = giant_size.x * 0.5 + half * way.y.abs() + CLEAR + 1.0;
            let to = if at.x < giant.x { -1.0 } else { 1.0 };
            at.x = giant.x + to * clear;
        }
        (at, way)
    }

    /// How open the pair is (0 to 1 of its length; 0: there is no pair). It
    /// eases open in the tell from the moment the arm is out of the place of
    /// its opening (`Limb::open_from`) to the moment the blow is set
    /// (`SET_AT`). It eases closed in the way back from the moment the limb
    /// is out of it (`RECOVER_OUT`) to `Limb::close_at`, before the arm goes
    /// home through that place. It never snaps (Jon, 2026-10-09).
    pub fn opening(cue: &Cue, limb: &Limb) -> f32 {
        if !cue.striking {
            return ease((tell(cue) - limb.open_from) / (SET_AT - limb.open_from).max(0.05));
        }
        let back = (cue.t - (cue.dur - limb.recover)) / limb.recover.max(1e-3);
        ease((limb.close_at - back) / (limb.close_at - RECOVER_OUT).max(0.05))
    }

    /// Does the far line still follow the player?
    pub fn tracking(cue: &Cue) -> bool {
        matches!(cue.mv, Move::PortalPunch | Move::PortalKick) && !cue.striking && cue.t < cue.dur - LOCK
    }
}

/// The giant is a character of its own (round 3, Jon 2026-10-24: "the
/// mount should be able to move around the room ... should feel like a
/// character in its own right"). Between his blows it walks to where it
/// fights from; with the player on its back it looks round at him.
pub mod stride {
    /// World units a second it walks: its hooves stand still on the floor at
    /// this speed (`giant_art.STRIDE_SPEED`, its `walk` row).
    pub const SPEED: f32 = 135.4;
    /// The player this near its middle is under it: it backs off.
    pub const UNDER: f32 = 300.0;
    /// The player this far from its middle is out of reach: it comes on.
    pub const FAR: f32 = 640.0;
    /// How far one walk goes.
    pub const STEP: f32 = 260.0;
    /// How near a wall its flank stops.
    pub const MARGIN: f32 = 40.0;
    /// A walk shorter than this is not begun.
    pub const SHORTEST: f32 = 30.0;
    /// How near its mark a walk ends.
    pub const ARRIVED: f32 = 10.0;
    /// The seconds it stands after a walk before the next.
    pub const PAUSE: f32 = 1.2;

    /// Where it walks to (world x), from `giant_x` with half its width
    /// `half_w`, for a player at `player_x`, in a hall from `left` to
    /// `right`. `None`: it stands where it is.
    pub fn mark(giant_x: f32, half_w: f32, player_x: f32, left: f32, right: f32) -> Option<f32> {
        let d = player_x - giant_x;
        let toward = if d.abs() < 1.0 { 1.0 } else { d.signum() };
        let to = if d.abs() < UNDER {
            giant_x - toward * STEP
        } else if d.abs() > FAR {
            giant_x + toward * STEP
        } else {
            return None;
        };
        let (lo, hi) = (left + half_w + MARGIN, right - half_w - MARGIN);
        let to = if lo <= hi { to.clamp(lo, hi) } else { giant_x };
        ((to - giant_x).abs() >= SHORTEST).then_some(to)
    }

    /// The rows of its walk: forward (the way its art faces, +X) or back,
    /// with its arms on it or in their lines.
    pub fn rows(forward: bool, arms_on: bool) -> &'static [&'static str] {
        match (forward, arms_on) {
            (true, true) => &["armed_walk", "armed_rest"],
            (true, false) => &["walk", "rest"],
            (false, true) => &["armed_walk_back", "armed_rest"],
            (false, false) => &["walk_back", "rest"],
        }
    }

    /// The rows of its look round at what is on its back.
    pub fn look_back(arms_on: bool) -> &'static [&'static str] {
        if arms_on {
            &["armed_look_back", "armed_rest"]
        } else {
            &["look_back", "rest"]
        }
    }
}

/// Two blows that meet (Newton's third law: every action has an equal and
/// opposite reaction). A blow of the player's on the giant's limb just as it
/// comes out of the far line, before it hurts, stops the limb and throws both
/// back: the limb goes back into its line, the player is thrown the way the
/// limb came, and the giant rocks back. The force of the blow goes up the arm
/// to its rider: the blow passes to him. Each is thrown as hard as the blow
/// pushed (the same force on both) over its mass (the second law): the player
/// far, the giant a little.
pub mod clash {
    use super::blows::Blow;

    /// The seconds after the limb comes out of the far line that it can be
    /// met: it hurts after them.
    pub const WINDOW: f32 = 0.09;
    /// The masses of the two bodies (the giant's catalog `mass`; the player
    /// is the reference body).
    pub const PLAYER_MASS: f32 = 1.0;
    pub const GIANT_MASS: f32 = 8.0;
    /// The giant is held still this long after a clash, and the camera shakes
    /// this hard.
    pub const FREEZE: f32 = 0.14;
    pub const SHAKE: f32 = 24.0;

    /// The impulse each body takes (world units a second, at mass 1): the
    /// heavier blow pushes harder.
    pub fn impulse(blow: Blow) -> f32 {
        match blow {
            Blow::Hammer | Blow::LeftHammer => 1100.0,
            Blow::Upper | Blow::LeftUpper => 950.0,
            Blow::Kick => 900.0,
            Blow::Jab | Blow::LeftHook => 760.0,
        }
    }

    /// How fast a body of `mass` is thrown by the impulse of `blow`.
    pub fn speed(blow: Blow, mass: f32) -> f32 {
        impulse(blow) / mass.max(0.1)
    }

    /// The player's launch: along the way the limb came (`way`, from the far
    /// line), and up. A limb that came down throws him off it along the floor
    /// (the floor takes the rest): away from the fist, `from_fist` the way
    /// from it to him.
    pub fn foe_launch(blow: Blow, way: bevy_math::Vec2, from_fist: bevy_math::Vec2) -> [f32; 2] {
        let way = way.normalize_or_zero();
        let dir = if way.y > 0.5 {
            let side = if from_fist.x.abs() < 1.0 { 1.0 } else { from_fist.x.signum() };
            bevy_math::Vec2::new(side, -0.45).normalize()
        } else {
            (way + bevy_math::Vec2::new(0.0, -0.35)).normalize_or_zero()
        };
        let v = dir * speed(blow, PLAYER_MASS);
        [v.x, v.y]
    }

    /// The giant's: away from the player, on the floor.
    pub fn giant_recoil(blow: Blow, away_x: f32) -> [f32; 2] {
        [away_x.signum() * speed(blow, GIANT_MASS), -40.0]
    }
}

/// His arrival: when his hall wakes, he comes in by the door at the top of
/// it (he ran there from the duel in the room before, `prelude` in
/// `ambition_content`), leaps down onto his giant's shoulders, and calls
/// his power; the giant roars. It plays in the encounter's intro.
pub mod arrival {
    use bevy_math::Vec2;

    /// The seconds of each part: he stands on the ledge, he leaps, he calls
    /// his power. Within the encounter's intro (`intro_seconds`, 2.8).
    pub const STAND: f32 = 0.45;
    pub const LEAP: f32 = 1.05;
    pub const POWER: f32 = 1.1;
    pub const LEN: f32 = STAND + LEAP + POWER;
    /// The ledge by the hall's top door: this far over the floor, and this
    /// far from the left wall (`gnu_ton_arena_area.ron`, `shoulders()` in the
    /// Science world's `make_specs.py`).
    pub const LEDGE_OVER_FLOOR: f32 = 1040.0;
    pub const LEDGE_IN: f32 = 120.0;
    /// How high over the higher end the leap goes.
    pub const RISE: f32 = 140.0;

    /// Where he is at `t`: on the ledge, in the air on a curve that falls as a
    /// thrown body does, then in the saddle. `None` once he is in it.
    pub fn place(t: f32, ledge: Vec2, saddle: Vec2) -> Option<Vec2> {
        if t < STAND {
            return Some(ledge);
        }
        let u = (t - STAND) / LEAP;
        if u >= 1.0 {
            return None;
        }
        let top = ledge.y.min(saddle.y) - RISE;
        // A parabola through the ledge, the top and the saddle, with its top
        // where the time to fall from it is in proportion to the height.
        let up = (ledge.y - top).sqrt();
        let down = (saddle.y - top).sqrt();
        let apex = up / (up + down);
        let y = if u < apex {
            let k = 1.0 - u / apex;
            top + (ledge.y - top) * k * k
        } else {
            let k = (u - apex) / (1.0 - apex);
            top + (saddle.y - top) * k * k
        };
        Some(Vec2::new(ledge.x + (saddle.x - ledge.x) * u, y))
    }

    pub fn scholar_rows(t: f32) -> (&'static [&'static str], f32, bool) {
        if t < STAND {
            (&["brace", "idle"], t, false)
        } else if t < STAND + LEAP {
            (&["special_up", "launch", "fall"], t - STAND, false)
        } else {
            (&["invoke", "brace"], t - STAND - LEAP, false)
        }
    }

    pub fn gnu_rows(t: f32, arms_on: bool) -> (&'static [&'static str], f32, bool) {
        if t < STAND + LEAP {
            (if arms_on { &["armed_look_back", "look_back", "rest"] } else { &["look_back", "rest"] }, t, false)
        } else if arms_on {
            (&["armed_roar", "roar", "rest"], t - STAND - LEAP, false)
        } else {
            (&["roar", "rest"], t - STAND - LEAP, false)
        }
    }
}

/// Phase 3, "The System of the World": the giant blasts off with him into
/// space, and the fight goes on there under Newton's laws.
///
/// * The launch is the beat before the enrage (`enrage_seconds` of the
///   encounter): he powers up, the giant crouches and lifts off, the sky
///   goes to space, and both put on their helmets.
/// * In space each body keeps the most part of its speed: a `held_force`
///   over the hall takes `LIFT` of every body's weight (the first law: a
///   launched body drifts far).
/// * Tides of force push across the hall in turn: one force on every body,
///   so the player (mass 1) goes eight times as fast as the giant (mass 8)
///   (the second law).
/// * Each blow the player lands on him pushes the player back (the third
///   law).
pub mod space {
    /// The launch, in seconds from its start: his power, the liftoff, the
    /// helmets, the end of it. `enrage_seconds` in the encounter is `LEN`.
    pub const POWER: f32 = 0.0;
    pub const LIFTOFF: f32 = 1.0;
    pub const HELMETS: f32 = 1.9;
    pub const LEN: f32 = 3.6;
    /// From the liftoff, the seconds the sky takes to go to space.
    pub const ASCEND: f32 = 2.4;
    /// The giant's hop at the liftoff (world units per second, up).
    pub const HOP: f32 = 620.0;
    /// The part of each body's weight that space takes away, and the
    /// seconds it takes to come and to go.
    pub const LIFT: f32 = 0.6;
    pub const LIFT_EASE: f32 = 0.6;
    /// The tides: each `CYCLE` seconds a calm, a warning, then a tide.
    pub const CYCLE: f32 = 10.0;
    pub const CALM: f32 = 4.5;
    pub const WARN: f32 = 1.0;
    /// The push of a side tide on a body of mass 1.
    pub const SIDE_FORCE: f32 = 1200.0;
    /// An up tide takes nearly all weight away and lifts a light body: the
    /// giant (mass 8) stays down, the player floats up.
    pub const UP_LIFT: f32 = 0.95;
    pub const UP_FORCE: f32 = 260.0;
    /// The push back on the player when a blow of theirs lands on him, and
    /// the least seconds between two.
    pub const RECOIL: f32 = 380.0;
    pub const RECOIL_RISE: f32 = 0.35;
    pub const RECOIL_WAIT: f32 = 0.45;

    /// Which way a tide pushes.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Tide {
        Right,
        Left,
        Up,
    }

    impl Tide {
        /// The tide of cycle `k`: right, left, up, again.
        pub fn of_cycle(k: u32) -> Self {
            match k % 3 {
                0 => Tide::Right,
                1 => Tide::Left,
                _ => Tide::Up,
            }
        }

        /// The way it pushes (+Y down).
        pub fn way(self) -> [f32; 2] {
            match self {
                Tide::Right => [1.0, 0.0],
                Tide::Left => [-1.0, 0.0],
                Tide::Up => [0.0, -1.0],
            }
        }

        /// What he says as it comes.
        pub fn bark(self) -> &'static str {
            match self {
                Tide::Right | Tide::Left => "The tide turns!",
                Tide::Up => "Gravity... is optional!",
            }
        }
    }

    /// The sea of space at `t` seconds after the launch: the tide that
    /// pushes now (with how far into it, 0..1), or the tide that is coming
    /// (a warning).
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum Sea {
        Calm,
        Warning(Tide),
        Tide(Tide, f32),
    }

    pub fn sea(t: f32) -> Sea {
        if t < LEN {
            return Sea::Calm;
        }
        let into = t - LEN;
        let k = (into / CYCLE).floor() as u32;
        let at = into - k as f32 * CYCLE;
        let tide = Tide::of_cycle(k);
        if at < CALM {
            Sea::Calm
        } else if at < CALM + WARN {
            Sea::Warning(tide)
        } else {
            Sea::Tide(tide, (at - CALM - WARN) / (CYCLE - CALM - WARN))
        }
    }

    /// The field over the hall at `t` seconds after the launch: the part of
    /// weight taken away, and the push on a body of mass 1. Nothing before
    /// the liftoff.
    pub fn field(t: f32) -> (f32, [f32; 2]) {
        if t < LIFTOFF {
            return (0.0, [0.0, 0.0]);
        }
        let lift = LIFT * super::ease((t - LIFTOFF) / LIFT_EASE);
        match sea(t) {
            Sea::Tide(tide, into) => {
                // It comes up and goes down: no snap at its ends.
                let swell = (into * std::f32::consts::PI).sin().clamp(0.0, 1.0).sqrt();
                match tide {
                    Tide::Up => (lift + (UP_LIFT - lift) * swell, [0.0, -UP_FORCE * swell]),
                    side => {
                        let way = side.way();
                        (lift, [way[0] * SIDE_FORCE * swell, 0.0])
                    }
                }
            }
            _ => (lift, [0.0, 0.0]),
        }
    }

    /// How far the sky is in space (0 on the ground, 1 in space) at `t`
    /// seconds after the launch.
    pub fn ascent(t: f32) -> f32 {
        super::ease((t - LIFTOFF) / ASCEND)
    }

    /// The push back on a player who lands a blow on him: away from `from`
    /// (his place), and a little up.
    pub fn recoil(player_x: f32, from_x: f32) -> [f32; 2] {
        let away = if player_x < from_x { -1.0 } else { 1.0 };
        [away * RECOIL, -RECOIL * RECOIL_RISE]
    }

    /// What he is drawn as in the launch: he calls his power, then he rides.
    pub fn scholar_rows(t: f32) -> (&'static [&'static str], f32, bool) {
        if t < LIFTOFF {
            (&["invoke", "brace"], t, false)
        } else {
            (&["brace", "idle"], t - LIFTOFF, true)
        }
    }

    /// What the giant is drawn as in the launch: it roars as he powers it,
    /// then it floats with its legs tucked.
    pub fn gnu_rows(t: f32) -> (&'static [&'static str], f32, bool) {
        if t < LIFTOFF {
            (&["roar", "hit"], t, false)
        } else {
            (&["float", "idle"], t - LIFTOFF, true)
        }
    }
}

/// The end of the fight (round 3, Jon 2026-10-24: "if you defeat the
/// scholar, it needs to have an appropriate reaction. The fight sort of ends
/// anticlimactically."). Beaten, the scholar falls from its shoulder. The
/// giant gathers him up and holds him, its head low; then it rears and roars.
/// An apple falls on his head (as one fell on Newton's), and he wakes; the
/// giant puts him back on its shoulder, and the two of them are at rest:
/// every body stays in its state of rest.
pub mod finale {
    use bevy_math::Vec2;

    /// The seconds of each part.
    pub const FALL: f32 = 0.8;
    pub const GATHER: f32 = 1.1;
    pub const GRIEVE: f32 = 2.6;
    pub const ROAR: f32 = 1.7;
    pub const APPLE: f32 = 1.0;
    pub const WAKE: f32 = 1.6;
    /// The whole of it.
    pub const LEN: f32 = FALL + GATHER + GRIEVE + ROAR + APPLE + WAKE;
    /// Where the giant holds him, from its middle (on the side its art
    /// faces): in front of its chest, where its arms meet (`giant_art`'s
    /// cradle).
    pub const CRADLE: Vec2 = Vec2::new(190.0, -46.0);
    /// How far in front of it he falls.
    pub const FALLS: f32 = 300.0;

    /// A part of the end, and the seconds into it.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum Part {
        Fall(f32),
        Gather(f32),
        Grieve(f32),
        Roar(f32),
        Apple(f32),
        Wake(f32),
        Rest(f32),
    }

    pub fn part(t: f32) -> Part {
        let mut at = t;
        for (len, make) in [
            (FALL, Part::Fall as fn(f32) -> Part),
            (GATHER, Part::Gather),
            (GRIEVE, Part::Grieve),
            (ROAR, Part::Roar),
            (APPLE, Part::Apple),
            (WAKE, Part::Wake),
        ] {
            if at < len {
                return make(at);
            }
            at -= len;
        }
        Part::Rest(at)
    }

    fn ease(u: f32) -> f32 {
        let u = u.clamp(0.0, 1.0);
        u * u * (3.0 - 2.0 * u)
    }

    /// Where he is: from where he was beaten he falls in front of it; it
    /// lifts him to its chest; and back to its shoulder (`saddle`) when he
    /// wakes. `None`: on its shoulder, the giant carries him.
    pub fn scholar(t: f32, from: Vec2, floor: Vec2, cradle: Vec2, saddle: Vec2) -> Option<Vec2> {
        match part(t) {
            Part::Fall(u) => {
                let u = u / FALL;
                // An arc: up a little, and down onto the floor.
                let line = from.lerp(floor, u);
                Some(line - Vec2::new(0.0, 80.0 * 4.0 * u * (1.0 - u)))
            }
            Part::Gather(u) => Some(floor.lerp(cradle, ease(u / GATHER))),
            Part::Grieve(_) | Part::Roar(_) | Part::Apple(_) => Some(cradle),
            Part::Wake(u) => {
                let u = ease(u / WAKE);
                Some(cradle.lerp(saddle, u) - Vec2::new(0.0, 60.0 * 4.0 * u * (1.0 - u)))
            }
            Part::Rest(_) => None,
        }
    }

    /// His rows: thrown, lying (in its arms too), dazed by the apple, up.
    pub fn scholar_rows(t: f32) -> super::rows::Pin {
        match part(t) {
            Part::Fall(u) => (&["tumble", "hit_heavy", "death"], u, false),
            Part::Gather(_) | Part::Grieve(_) | Part::Roar(_) => (&["death"], 10.0, false),
            Part::Apple(u) => (&["dazed", "death"], u, true),
            Part::Wake(u) => (&["climb", "dazed"], u, false),
            Part::Rest(u) => (&["rest", "idle"], u, true),
        }
    }

    /// The giant's rows: it flinches, holds him, roars, and is at rest.
    pub fn gnu_rows(t: f32) -> super::rows::Pin {
        match part(t) {
            Part::Fall(u) => (&["armed_hit", "hit"], u, false),
            Part::Gather(u) => (&["grieve", "armed_rest"], u, false),
            Part::Grieve(u) => (&["grieve", "armed_rest"], GATHER + u, false),
            Part::Roar(u) => (&["armed_roar", "roar"], u, false),
            Part::Apple(u) => (&["grieve", "armed_rest"], GATHER + GRIEVE + u, false),
            Part::Wake(u) | Part::Rest(u) => (&["armed_rest"], u, true),
        }
    }
}

pub mod eureka {
    use super::*;

    /// The golden apple's fall onto his head.
    pub const APPLE: f32 = 0.55;
    /// Then he tumbles off the giant…
    pub const TUMBLE: f32 = 0.55;
    /// …sits dazed on the floor…
    pub const DAZED: f32 = 1.9;
    // …and climbs back for whatever is left.

    /// How far he is through each part of it.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum Part {
        AppleFalling(f32),
        Tumbling(f32),
        Dazed,
        Climbing(f32),
    }

    pub fn part(t: f32, dur: f32) -> Part {
        if t < APPLE {
            Part::AppleFalling(t / APPLE)
        } else if t < APPLE + TUMBLE {
            Part::Tumbling((t - APPLE) / TUMBLE)
        } else if t < APPLE + TUMBLE + DAZED {
            Part::Dazed
        } else {
            let climb = (dur - APPLE - TUMBLE - DAZED).max(0.05);
            Part::Climbing((t - APPLE - TUMBLE - DAZED) / climb)
        }
    }

    /// Where the scholar is: his saddle, or on the floor in front of the giant.
    /// `landing` is where he lands.
    pub fn scholar(saddle: Vec2, landing: Vec2, t: f32, dur: f32) -> Vec2 {
        match part(t, dur) {
            Part::AppleFalling(_) => saddle,
            Part::Tumbling(f) => {
                // A hop up and over, then down.
                let flat = lerp(saddle, landing, ease_in(f));
                flat - Vec2::new(0.0, (std::f32::consts::PI * f).sin() * 70.0)
            }
            Part::Dazed => landing,
            Part::Climbing(f) => lerp(landing, saddle, ease(f)),
        }
    }

    /// The fists go limp while he is dazed: they drop where they are.
    pub fn fist(stage: &Stage, latch: &Latch, fist: Fist, t: f32, dur: f32) -> Pose {
        let from = latch.from[fist.index()];
        let floor = stage.on_floor(stage.clamp_x(from.x));
        match part(t, dur) {
            Part::AppleFalling(_) => Pose::idle(from),
            Part::Tumbling(f) => Pose {
                pos: lerp(from, floor, ease_in(f)),
                harmful: false,
                stuck: false,
            },
            Part::Dazed => Pose {
                pos: floor,
                harmful: false,
                stuck: true,
            },
            Part::Climbing(f) => Pose::idle(lerp(floor, stage.home(fist), ease(f))),
        }
    }
}

// ─── The repertoire ──────────────────────────────────────────────────────────

/// Every beat he knows, by the `Special` key the boss profile names it with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Demonstrate,
    DemonstratePair,
    Pendulum,
    Fluxions,
    FluxionsPair,
    Cradle,
    Orbit,
    AppleRain,
    /// Not a scripted move: the conductor performs it over the Rest after an
    /// apple rain, so the pattern sees an honest punish window.
    Eureka,
    /// The gnu's own moves: the fists brace at home while the giant bucks or
    /// stomps (the conductor performs those on the giant).
    Buck,
    Stomp,
    /// Phase 1, his arms on him: the right fist goes into a portal by the
    /// giant, and comes out of the pair's far line, by the player
    /// ([`blows`]).
    PortalPunch,
    /// The giant's near fore leg kicks into a portal by it, and the hoof
    /// comes out of the pair's far line, by the player: with its arms on it
    /// or in their lines ([`blows`]).
    PortalKick,
}

impl Move {
    /// A new move goes at the END: a record holds a move as its place here.
    pub const ALL: [Move; 13] = [
        Move::Demonstrate,
        Move::DemonstratePair,
        Move::Pendulum,
        Move::Fluxions,
        Move::FluxionsPair,
        Move::Cradle,
        Move::Orbit,
        Move::AppleRain,
        Move::Eureka,
        Move::Buck,
        Move::Stomp,
        Move::PortalPunch,
        Move::PortalKick,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Move::Demonstrate => "demonstrate",
            Move::DemonstratePair => "demonstrate_pair",
            Move::Pendulum => "pendulum",
            Move::Fluxions => "fluxions",
            Move::FluxionsPair => "fluxions_pair",
            Move::Cradle => "cradle",
            Move::Orbit => "orbit",
            Move::AppleRain => "apple_rain",
            Move::Eureka => "eureka",
            Move::Buck => "buck",
            Move::Stomp => "stomp",
            Move::PortalPunch => "portal_punch",
            Move::PortalKick => "portal_kick",
        }
    }

    pub fn from_key(key: &str) -> Option<Move> {
        Move::ALL.into_iter().find(|mv| mv.key() == key)
    }

    /// The fists perform it as bodies of their own: the giant's arms are in
    /// their portals. The giant's own moves, the punch of phase 1 and its
    /// kick are not.
    pub fn frees_the_fists(self) -> bool {
        !matches!(self, Move::Buck | Move::Stomp | Move::PortalPunch | Move::PortalKick)
    }
}

/// Where in a beat the performance is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cue {
    pub mv: Move,
    /// In the strike (else the telegraph).
    pub striking: bool,
    /// Time into this part, and its length.
    pub t: f32,
    pub dur: f32,
    /// The telegraph's length, once the strike has begun (a pair's follower
    /// keeps telegraphing into the strike).
    pub tel_dur: f32,
    /// Time since the beat began, across telegraph and strike.
    pub beat_t: f32,
    /// A free-running clock, for idling.
    pub clock: f32,
}

/// A pair's second fist falls this long after the first.
fn follower_fall(cue: &Cue) -> Option<(f32, f32)> {
    // (time into the follower's own strike, its length), once it is falling.
    let t = cue.t - PAIR_STAGGER;
    (cue.striking && t >= 0.0).then(|| (t, (cue.dur - PAIR_STAGGER).max(0.1)))
}

/// Does `fist`'s aim still follow the player? A demonstration tracks until its
/// lock; nothing else re-aims after the telegraph began.
pub fn tracking(cue: &Cue, lead: Fist, fist: Fist) -> bool {
    let lock = demonstrate::LOCK;
    match cue.mv {
        Move::Demonstrate => fist == lead && !cue.striking && cue.t < cue.dur - lock,
        Move::DemonstratePair if fist == lead => !cue.striking && cue.t < cue.dur - lock,
        // The follower hovers through the lead's fall and tracks the player's
        // dodge until its own lock.
        Move::DemonstratePair => !cue.striking || cue.t < PAIR_STAGGER - lock,
        _ => false,
    }
}

/// The resting pose a fist eases back to when it has nothing to do this beat.
fn resting(stage: &Stage, latch: &Latch, fist: Fist, cue: &Cue) -> Pose {
    let home = idle(stage, fist, cue.clock).pos;
    Pose::idle(arrive(latch.from[fist.index()], home, cue.beat_t, 0.5))
}

/// One fist's pose for this cue. `latch.aim` is THIS fist's aim.
pub fn perform(stage: &Stage, latch: &Latch, fist: Fist, cue: &Cue) -> Pose {
    let lead = latch.lead.unwrap_or(Fist::Left);
    let leads = fist == lead;
    let (t, dur) = (cue.t, cue.dur);
    match cue.mv {
        Move::Demonstrate | Move::DemonstratePair if leads || cue.mv == Move::DemonstratePair => {
            if leads {
                if cue.striking {
                    demonstrate::strike(stage, latch, fist, t, dur)
                } else {
                    demonstrate::telegraph(stage, latch, fist, t, dur)
                }
            } else if let Some((ft, fdur)) = follower_fall(cue) {
                demonstrate::strike(stage, latch, fist, ft, fdur)
            } else {
                // Still hovering: its telegraph runs on into the lead's strike.
                let (ht, hdur) = if cue.striking {
                    (cue.tel_dur + t, cue.tel_dur + PAIR_STAGGER)
                } else {
                    (t, dur + PAIR_STAGGER)
                };
                demonstrate::telegraph(stage, latch, fist, ht, hdur)
            }
        }
        Move::Pendulum if leads => {
            if cue.striking {
                pendulum::strike(stage, latch, fist, t, dur)
            } else {
                pendulum::telegraph(stage, latch, fist, t, dur)
            }
        }
        Move::Fluxions | Move::FluxionsPair if leads || cue.mv == Move::FluxionsPair => {
            let mirror = !leads;
            if cue.striking {
                fluxions::strike(stage, latch, fist, t, dur, mirror)
            } else {
                fluxions::telegraph(stage, latch, fist, t, dur, mirror)
            }
        }
        Move::Cradle => {
            if cue.striking {
                cradle::strike(stage, latch, fist, t, dur)
            } else {
                cradle::telegraph(stage, latch, fist, t, dur)
            }
        }
        Move::Orbit => {
            if cue.striking {
                orbit::strike(stage, latch, fist, t, dur)
            } else {
                orbit::telegraph(stage, latch, fist, t, dur)
            }
        }
        Move::AppleRain => shake_the_tree(stage, latch, fist, cue.beat_t),
        Move::Eureka if cue.striking => eureka::fist(stage, latch, fist, t, dur),
        _ => resting(stage, latch, fist, cue),
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn stage() -> Stage {
        Stage {
            hall: Hall {
                floor: 1248.0,
                left: 16.0,
                right: 1776.0,
            },
            homes: [Vec2::new(600.0, 1000.0), Vec2::new(1200.0, 1000.0)],
            scholar: Vec2::new(850.0, 970.0),
            fist: Vec2::new(130.0, 100.0),
        }
    }

    fn latch(aim_x: f32) -> Latch {
        Latch {
            aim: Vec2::new(aim_x, 1225.0),
            lead: Some(Fist::Left),
            from: stage().homes,
            curve_phase: 0.0,
        }
    }

    #[test]
    fn a_demonstration_lands_on_the_floor_under_its_aim_and_stays_buried() {
        let s = stage();
        let l = latch(400.0);
        let landed = demonstrate::strike(&s, &l, Fist::Left, demonstrate::FALL + 0.01, 1.5);
        assert_eq!(landed.pos, s.on_floor(400.0), "it lands where it was aimed");
        assert!(landed.stuck && landed.harmful, "the impact itself hurts");
        let buried = demonstrate::strike(&s, &l, Fist::Left, demonstrate::FALL + 0.4, 1.5);
        assert!(buried.stuck && !buried.harmful, "a buried fist is a punish window");
        let home = demonstrate::strike(&s, &l, Fist::Left, 1.5, 1.5);
        assert_eq!(home.pos, s.home(Fist::Left), "and it goes home by the end");
    }

    #[test]
    fn a_demonstration_hovers_over_a_player_on_a_perch_and_falls_through_to_the_floor() {
        let s = stage();
        let mut l = latch(400.0);
        l.aim.y = 700.0;
        let top = demonstrate::strike(&s, &l, Fist::Left, 0.0, 1.5);
        assert!(top.pos.y < l.aim.y - 200.0, "it hovers over their head, not below their perch");
        let landed = demonstrate::strike(&s, &l, Fist::Left, demonstrate::FALL + 0.01, 1.5);
        assert_eq!(landed.pos, s.on_floor(400.0), "and lands on the floor under them");
    }

    #[test]
    fn a_pendulum_skims_the_floor_at_the_bottom_and_starts_high() {
        let s = stage();
        let bottom = pendulum::at(&s, &latch(0.0), 0.0);
        let fist_bottom = bottom.y + s.fist.y * 0.5;
        assert!(
            (s.hall.floor - fist_bottom - pendulum::SKIM).abs() < 0.5,
            "the lowest point skims the floor: {fist_bottom} vs floor {}",
            s.hall.floor
        );
        let start = pendulum::strike(&s, &latch(0.0), Fist::Left, 0.0, 2.4);
        assert!(start.pos.y < bottom.y - 200.0, "released from high up the arc");
        assert!(start.pos.x < bottom.x, "on the lead fist's side");
        let middle = pendulum::strike(&s, &latch(0.0), Fist::Left, 0.6, 2.4);
        assert!((middle.pos - bottom).length() < 1.0, "a quarter through, it is at the bottom");
    }

    #[test]
    fn the_fluxion_curve_dips_to_the_floor_where_the_player_stood() {
        let s = stage();
        for aim in [300.0, 900.0, 1500.0] {
            let mut l = latch(aim);
            l.curve_phase = fluxions::phase_for(&s, aim, -1.0);
            // Find the curve's x nearest the aim and check it is at a dip.
            let samples = 2000;
            let (dip, _) = (0..=samples)
                .map(|i| fluxions::point(&s, &l, i as f32 / samples as f32, false))
                .map(|p| (p, (p.x - aim).abs()))
                .fold((Vec2::ZERO, f32::MAX), |best, c| if c.1 < best.1 { c } else { best });
            assert!(
                (dip.y - s.on_floor(aim).y).abs() < 3.0,
                "aim {aim}: the curve passes the aim at floor level, got y {}",
                dip.y
            );
        }
    }

    #[test]
    fn the_fluxion_curve_leaves_high_ground_to_stand_under() {
        let s = stage();
        let l = latch(900.0);
        let highest = (0..=200)
            .map(|i| fluxions::point(&s, &l, i as f32 / 200.0, false).y)
            .fold(f32::MAX, f32::min);
        let fist_bottom = highest + s.fist.y * 0.5;
        assert!(
            s.hall.floor - fist_bottom > 150.0,
            "between dips a standing player is safe under the curve: bottom {fist_bottom}"
        );
    }

    #[test]
    fn the_cradle_alternates_which_fist_swings_and_clacks_between() {
        let l = latch(0.0);
        let movers: Vec<Fist> = (0..6).map(|n| cradle::mover(&l, n)).collect();
        assert_eq!(
            movers,
            [Fist::Left, Fist::Right, Fist::Right, Fist::Left, Fist::Left, Fist::Right],
            "opener falls in, the far fist flies out and back, then the opener…"
        );
        let dur = 3.3;
        let clacks = (1..=330)
            .filter(|i| cradle::clacks((*i - 1) as f32 * 0.01, *i as f32 * 0.01, dur))
            .count();
        assert_eq!(clacks, 3, "a clack each time a fist falls in to the bottom");
    }

    #[test]
    fn eureka_drops_him_to_the_floor_and_brings_him_back() {
        let saddle = Vec2::new(850.0, 970.0);
        let landing = Vec2::new(700.0, 1200.0);
        let dur = 3.4;
        assert_eq!(eureka::scholar(saddle, landing, 0.1, dur), saddle);
        assert_eq!(eureka::scholar(saddle, landing, 1.5, dur), landing, "dazed on the floor");
        assert!((eureka::scholar(saddle, landing, dur, dur) - saddle).length() < 0.5);
    }

    fn cue(mv: Move, striking: bool, t: f32, dur: f32) -> Cue {
        Cue {
            mv,
            striking,
            t,
            dur,
            tel_dur: 1.0,
            beat_t: if striking { 1.0 + t } else { t },
            clock: 0.0,
        }
    }

    #[test]
    fn every_move_has_a_key_that_names_it_back() {
        for mv in Move::ALL {
            assert_eq!(Move::from_key(mv.key()), Some(mv));
        }
        assert_eq!(Move::from_key("hand_slam"), None, "the old moves are gone");
    }

    #[test]
    fn a_pair_demonstration_falls_one_fist_then_the_other() {
        let s = stage();
        let l = latch(900.0);
        // The lead has landed; the follower is still up.
        let t = demonstrate::FALL + 0.05;
        let lead = perform(&s, &l, Fist::Left, &cue(Move::DemonstratePair, true, t, 2.0));
        let follower = perform(&s, &l, Fist::Right, &cue(Move::DemonstratePair, true, t, 2.0));
        assert!(lead.stuck, "the lead is in the floor");
        assert!(!follower.stuck && !follower.harmful, "the follower still hovers");
        let later = PAIR_STAGGER + demonstrate::FALL + 0.05;
        let follower = perform(&s, &l, Fist::Right, &cue(Move::DemonstratePair, true, later, 2.0));
        assert!(follower.stuck, "then it lands too");
        // A single demonstration leaves the other fist resting.
        let other = perform(&s, &l, Fist::Right, &cue(Move::Demonstrate, true, t, 1.5));
        assert!(!other.harmful && !other.stuck);
    }

    /// The giant of the arena, as the tests' stage has it: its middle and size.
    const GIANT: (Vec2, Vec2) = (Vec2::new(896.0, 1016.0), Vec2::new(630.0, 464.0));

    /// Each exit brings the limb at the player: at the full blow the middle of
    /// the fist (the hoof) is where the player is (on the floor under them for
    /// a blow down), the far line is clear of the giant, and over the floor.
    #[test]
    fn a_blows_far_line_brings_the_limb_at_the_player() {
        let s = stage();
        let (giant, size) = GIANT;
        for (exit, arm) in blows::Exit::ALL.into_iter().flat_map(|e| [(e, blows::Arm::Left), (e, blows::Arm::Right)]) {
            let blow = exit.blow(arm);
            for target in [Vec2::new(250.0, 1200.0), Vec2::new(1550.0, 1000.0), Vec2::new(1450.0, 700.0)] {
                let side = blows::side(&s, giant, size, blow, exit, target);
                let (at, way) = blows::far_end(&s, giant, size, blow, exit, side, target);
                let limb = blows::limb(blow);
                let fist = at + way * limb.reach();
                let want = match exit {
                    blows::Exit::Down => Vec2::new(target.x, s.on_floor(target.x).y),
                    blows::Exit::Diagonal => Vec2::new(target.x, target.y.min(s.on_floor(target.x).y)),
                    _ => target,
                };
                assert!((way.length() - 1.0).abs() < 1e-4, "{exit:?}: the line's normal is a unit");
                assert!(at.y < s.hall.floor, "{exit:?} at {target}: the far line {at} is over the floor");
                let clear = (at.x - giant.x).abs() >= size.x * 0.5 || at.y < giant.y - size.y * 0.5;
                assert!(clear, "{exit:?} at {target}: the far line {at} is in the giant");
                // Where nothing moved the line, the limb comes to the player.
                if (fist - want).length() > 1.0 {
                    assert!(
                        matches!(exit, blows::Exit::Up | blows::Exit::Sweep) || (at.x - giant.x).abs() < size.x,
                        "{exit:?} at {target}: the limb comes to {fist}, not to {want}"
                    );
                }
            }
        }
    }

    /// Each exit of an arm is a blow of that arm, and the arm of a blow is the
    /// one on the player's side, unless the blow is the next of a combination:
    /// then it is the other arm (the one-two).
    #[test]
    fn a_blow_is_thrown_by_the_arm_on_the_players_side_and_a_combination_alternates() {
        use blows::{Arm, Exit};
        for exit in [Exit::Down, Exit::Diagonal, Exit::Across, Exit::Up] {
            for arm in [Arm::Left, Arm::Right] {
                assert_eq!(exit.blow(arm).arm(), Some(arm), "{exit:?} of the {arm:?} arm");
                assert_eq!(blows::limb(exit.blow(arm)).slot, arm.slot());
            }
        }
        assert!(Exit::Down.blow(Arm::Left).is_hammer() && Exit::Diagonal.blow(Arm::Right).is_hammer());
        assert_eq!(blows::arm(-1.0, None, false), Arm::Left);
        assert_eq!(blows::arm(1.0, Some(Arm::Right), false), Arm::Right, "no combination: the player's side");
        assert_eq!(blows::arm(1.0, Some(Arm::Right), true), Arm::Left, "the next of a combination: the other arm");
        assert_eq!(blows::arm(-1.0, Some(Arm::Left), true), Arm::Right);
    }

    /// A blow comes out of its far line exactly once in its strike, and before
    /// it is full.
    #[test]
    fn a_blow_comes_through_its_line_once_before_it_is_full() {
        for blow in [blows::Blow::Hammer, blows::Blow::LeftHook, blows::Blow::LeftUpper, blows::Blow::Kick] {
            let l = blows::limb(blow);
            let dt = 1.0 / 60.0;
            let ticks: Vec<f32> = (0..=(l.strike / dt) as usize + 1).map(|k| k as f32 * dt).collect();
            let through: Vec<f32> = ticks.windows(2).filter(|w| blows::comes_through(&l, w[0], w[1])).map(|w| w[1]).collect();
            assert_eq!(through.len(), 1, "{blow:?}: {through:?}");
            assert!(through[0] <= l.strike * blows::STRIKE_FULL + dt, "{blow:?}: through at {}, full at {}", through[0], l.strike * blows::STRIKE_FULL);
        }
    }

    /// A limb that comes out of its far line can be met for the clash's
    /// window, at the time its line is crossed, and hurts only after it.
    #[test]
    fn a_limb_out_of_its_line_can_be_met_before_it_hurts() {
        for blow in [blows::Blow::Hammer, blows::Blow::Jab, blows::Blow::LeftUpper, blows::Blow::Kick] {
            let l = blows::limb(blow);
            let through = blows::through_time(&l);
            let dt = 1.0 / 60.0;
            let crossed = (1..200).map(|k| k as f32 * dt).find(|t| blows::comes_through(&l, t - dt, *t)).unwrap();
            assert!((crossed - through).abs() <= dt, "{blow:?}: crossed at {crossed}, through at {through}");
            let at = |t: f32| if t < l.strike { blows::Part::Strike(t) } else { blows::Part::Hold(t - l.strike) };
            assert!(!blows::meetable(&l, at(through - dt)), "{blow:?}: met before it is through");
            assert!(blows::meetable(&l, at(through + 0.01)), "{blow:?}: not met as it comes through");
            assert!(!blows::hurts(&l, at(through + 0.01)), "{blow:?}: it hurts while it can be met");
            assert!(blows::hurts(&l, at(through + clash::WINDOW + 0.01)), "{blow:?}: it does not hurt after");
            assert!(!blows::meetable(&l, at(through + clash::WINDOW + 0.01)));
        }
    }

    /// The same impulse throws the light player far and the heavy giant a
    /// little; the hammer pushes hardest.
    #[test]
    fn a_clash_throws_each_body_by_its_mass() {
        // A hammer down throws him off along the floor (and up); a jab
        // across throws him the way it came.
        let hammer = clash::foe_launch(blows::Blow::Hammer, Vec2::new(0.0, 1.0), Vec2::new(-30.0, 0.0));
        let jab = clash::foe_launch(blows::Blow::Jab, Vec2::new(-1.0, 0.0), Vec2::new(-30.0, 0.0));
        assert!(hammer[0] < -500.0 && hammer[1] < 0.0 && jab[0] < -500.0, "{hammer:?} {jab:?}");
        let giant = clash::giant_recoil(blows::Blow::Hammer, 1.0);
        assert!(giant[0] > 0.0 && giant[0] * clash::GIANT_MASS <= clash::impulse(blows::Blow::Hammer) + 1.0);
        assert!(clash::speed(blows::Blow::Hammer, 1.0) > clash::speed(blows::Blow::Jab, 1.0));
    }

    /// Between blows the giant comes on at a player far from it and backs off
    /// one under it, and stays in its hall.
    #[test]
    fn the_giant_walks_to_where_it_fights_from() {
        let (left, right, half) = (0.0, 1800.0, 315.0);
        assert_eq!(stride::mark(900.0, half, 1000.0, left, right), Some(900.0 - stride::STEP));
        assert_eq!(stride::mark(900.0, half, 1700.0, left, right), Some(900.0 + stride::STEP));
        assert_eq!(stride::mark(900.0, half, 1350.0, left, right), None);
        // Against a wall it goes no farther.
        let to = stride::mark(400.0, half, 450.0, left, right);
        assert!(to.is_none_or(|x| x >= left + half + stride::MARGIN), "{to:?}");
    }

    /// He stands on the ledge, leaps over the ends of his way and lands on
    /// the saddle, and is in it when the leap is over.
    #[test]
    fn he_leaps_from_the_ledge_onto_the_shoulders() {
        use super::arrival as ar;
        let (ledge, saddle) = (Vec2::new(130.0, 160.0), Vec2::new(900.0, 640.0));
        assert_eq!(ar::place(0.1, ledge, saddle), Some(ledge));
        let mut highest = f32::MAX;
        let mut last = ledge;
        for k in 0..=100 {
            let t = ar::STAND + ar::LEAP * k as f32 / 100.0 - 1e-4;
            let Some(at) = ar::place(t, ledge, saddle) else { break };
            assert!((at - last).length() < 40.0, "a jump in the leap at {t}: {last} to {at}");
            highest = highest.min(at.y);
            last = at;
        }
        assert!(highest < ledge.y - ar::RISE * 0.9, "the leap goes up first: {highest}");
        assert!((last - saddle).length() < 30.0, "it lands on the saddle: {last}");
        assert_eq!(ar::place(ar::STAND + ar::LEAP + 0.01, ledge, saddle), None);
        assert!(ar::LEN < 2.8, "the arrival is in the intro");
    }

    /// In space: no field before the liftoff; then most of the weight goes
    /// and stays gone; a tide comes after a warning, swells from nothing and
    /// goes back to nothing, and the tides go right, left, then up.
    #[test]
    fn in_space_weight_goes_and_the_tides_come_after_a_warning() {
        use super::space::{self as sp, Sea, Tide};
        assert_eq!(sp::field(sp::LIFTOFF - 0.01), (0.0, [0.0, 0.0]));
        assert_eq!(sp::ascent(sp::LIFTOFF), 0.0);
        assert_eq!(sp::ascent(sp::LIFTOFF + sp::ASCEND), 1.0);
        let (lift, push) = sp::field(sp::LEN + 1.0);
        assert!((lift - sp::LIFT).abs() < 1e-5 && push == [0.0, 0.0], "{lift} {push:?}");
        let first = sp::LEN + sp::CALM;
        assert_eq!(sp::sea(first - 0.01), Sea::Calm);
        assert_eq!(sp::sea(first + 0.01), Sea::Warning(Tide::Right));
        let tide_len = sp::CYCLE - sp::CALM - sp::WARN;
        let start = first + sp::WARN;
        let near = |t: f32| sp::field(t).1[0].abs();
        assert!(near(start + 0.01) < sp::SIDE_FORCE * 0.2, "a tide snaps on: {}", near(start + 0.01));
        assert!(near(start + tide_len * 0.5) > sp::SIDE_FORCE * 0.99);
        assert!(near(start + tide_len - 0.01) < sp::SIDE_FORCE * 0.2, "a tide snaps off");
        assert!(sp::field(start + tide_len * 0.5).1[0] > 0.0, "the first tide goes right");
        assert!(sp::field(start + sp::CYCLE + tide_len * 0.5).1[0] < 0.0, "the second goes left");
        let (up_lift, up) = sp::field(start + 2.0 * sp::CYCLE + tide_len * 0.5);
        assert!(up[1] < 0.0 && up_lift > sp::LIFT, "the third goes up: {up_lift} {up:?}");
        // The up tide lifts the player (mass 1) and not the giant (mass 8).
        let g = 2250.0;
        assert!(-up[1] > (1.0 - up_lift) * g, "the player stays down in an up tide");
        assert!(-up[1] / 8.0 < (1.0 - up_lift) * g, "the giant floats up in an up tide");
        // A blow that lands pushes the player away from him, and up.
        assert!(sp::recoil(100.0, 400.0)[0] < 0.0 && sp::recoil(700.0, 400.0)[0] > 0.0);
        assert!(sp::recoil(100.0, 400.0)[1] < 0.0);
    }

    /// The end: he falls in front of it, it holds him, and he is back on its
    /// shoulder at the end; each part follows the one before.
    #[test]
    fn beaten_he_falls_is_held_and_wakes_on_its_shoulder() {
        let (from, floor, cradle, saddle) =
            (Vec2::new(0.0, -300.0), Vec2::new(300.0, 0.0), Vec2::new(190.0, -46.0), Vec2::new(256.0, -260.0));
        assert_eq!(finale::scholar(0.0, from, floor, cradle, saddle), Some(from));
        let landed = finale::scholar(finale::FALL - 1e-3, from, floor, cradle, saddle).unwrap();
        assert!(landed.distance(floor) < 4.0, "{landed:?}");
        assert_eq!(finale::scholar(finale::FALL + finale::GATHER + 0.5, from, floor, cradle, saddle), Some(cradle));
        assert_eq!(finale::scholar(finale::LEN + 0.1, from, floor, cradle, saddle), None);
        let mut last = finale::part(0.0);
        for k in 1..200 {
            let now = finale::part(k as f32 * finale::LEN / 150.0);
            let order = |p: finale::Part| match p {
                finale::Part::Fall(_) => 0,
                finale::Part::Gather(_) => 1,
                finale::Part::Grieve(_) => 2,
                finale::Part::Roar(_) => 3,
                finale::Part::Apple(_) => 4,
                finale::Part::Wake(_) => 5,
                finale::Part::Rest(_) => 6,
            };
            assert!(order(now) >= order(last));
            last = now;
        }
    }

    /// No blow is the one before it, and the first punch is straight down
    /// (the one that shows the way up).
    #[test]
    fn no_blow_is_the_one_before() {
        assert_eq!(blows::choose(false, false, false, None, 0), blows::Exit::Down);
        let mut last = None;
        let mut seen = std::collections::BTreeSet::new();
        for count in 0..60u32 {
            let (kick, high, near) = (count % 4 == 3, count % 5 < 2, count % 7 < 3);
            let exit = blows::choose(kick, high, near, last, count);
            assert_ne!(Some(exit), last, "blow {count} is the one before it");
            assert_eq!(kick, exit.blow(blows::Arm::Left) == blows::Blow::Kick, "a kick is the leg, a punch the arm");
            seen.insert(exit.code());
            last = Some(exit);
        }
        assert_eq!(seen.len(), blows::Exit::ALL.len(), "every exit is thrown");
    }

    /// The limb goes into its line from the giant's side, is past it at the
    /// full blow by its `deep`, gives in its hold, and is out of the line
    /// again early in its way back.
    #[test]
    fn a_blows_limb_goes_through_its_line_and_comes_back_out() {
        for blow in [
            blows::Blow::Hammer,
            blows::Blow::Jab,
            blows::Blow::Upper,
            blows::Blow::LeftHammer,
            blows::Blow::LeftHook,
            blows::Blow::LeftUpper,
            blows::Blow::Kick,
        ] {
            let l = blows::limb(blow);
            let at = |part| blows::wrist_depth(&l, part);
            assert_eq!(at(blows::Part::Strike(0.0)), Some(-l.back), "{blow:?}: cocked on the giant's side");
            let full = at(blows::Part::Strike(l.strike * blows::STRIKE_FULL)).unwrap();
            assert!((full - l.deep).abs() < 1e-3, "{blow:?}: full at its depth, {full}");
            assert!(at(blows::Part::Hold(l.hold * 0.25)).unwrap() < l.deep, "{blow:?}: it gives in its hold");
            assert!(at(blows::Part::Recover(l.recover * 0.5)).is_none(), "{blow:?}: out of the line on the way back");
            assert!(blows::lands(&l, l.strike * 0.79, l.strike * 0.81));
            // The strike of a pattern: its parts in order.
            let dur = 1.2;
            assert_eq!(blows::part(&l, 0.1, dur - 0.1), blows::Part::Strike(0.1));
            assert!(matches!(blows::part(&l, l.strike + 0.1, dur - l.strike - 0.1), blows::Part::Hold(_)));
            assert!(matches!(blows::part(&l, dur - 0.1, 0.1), blows::Part::Recover(_)));
        }
    }

    /// The pair eases open through the tell and closed in the way back: it
    /// never snaps (Jon 2026-10-09). It is closed while the arm is where its
    /// opening is: at the start of the tell, and when it goes home.
    #[test]
    fn a_blows_pair_eases_open_and_closed() {
        for blow in [blows::Blow::Hammer, blows::Blow::Jab, blows::Blow::Upper, blows::Blow::LeftHammer, blows::Blow::LeftHook, blows::Blow::LeftUpper, blows::Blow::Kick] {
            let limb = blows::limb(blow);
            let tell = |t| blows::opening(&cue(Move::PortalPunch, false, t, 1.0), &limb);
            let dur = limb.strike + limb.hold + limb.recover;
            let strike = |t| blows::opening(&cue(Move::PortalPunch, true, t, dur), &limb);
            assert_eq!(tell(limb.open_from * 0.99), 0.0, "{blow:?}: open while the arm is in its place");
            assert!((tell(blows::SET_AT) - 1.0).abs() < 1e-4, "{blow:?}: not open when the blow is set");
            let mut before = 0.0;
            for k in 0..=120 {
                let now = tell(k as f32 / 120.0);
                assert!(now >= before && now - before < 0.12, "{blow:?}: a step of the opening at {k}: {before} to {now}");
                before = now;
            }
            assert!((strike(limb.strike + limb.hold) - 1.0).abs() < 1e-4, "{blow:?}: it closes before the limb is out");
            let closed = dur - limb.recover * (1.0 - limb.close_at);
            assert_eq!(strike(closed + 1e-3), 0.0, "{blow:?}: open when the arm goes home");
            let mut before = 1.0;
            for k in 0..=120 {
                let now = strike(dur * k as f32 / 120.0);
                assert!(now <= before + 1e-6 && before - now < 0.25, "{blow:?}: a step of the closing at {k}: {before} to {now}");
                before = now;
            }
        }
    }

    #[test]
    fn only_a_demonstration_tracks_and_only_until_its_lock() {
        let c = cue(Move::Demonstrate, false, 0.2, 1.0);
        assert!(tracking(&c, Fist::Left, Fist::Left));
        assert!(!tracking(&c, Fist::Left, Fist::Right), "the other fist is not aiming");
        let locked = cue(Move::Demonstrate, false, 0.8, 1.0);
        assert!(!tracking(&locked, Fist::Left, Fist::Left), "locked: step out from under it");
        let follower = cue(Move::DemonstratePair, true, 0.05, 2.0);
        assert!(tracking(&follower, Fist::Left, Fist::Right), "the follower tracks your dodge");
        assert!(!tracking(&cue(Move::Pendulum, false, 0.1, 0.9), Fist::Left, Fist::Left));
    }
}
