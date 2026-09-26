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
//! fist leads). The conductor (`conductor.rs`) owns the ECS side: it reads the
//! scholar's live move, latches, and writes the poses and hit volumes.
//!
//! Coordinates are world units, y down.

use ambition_platformer2d_core::Vec2;

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

/// The hall a fight happens in, measured once from the room.
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

    pub fn width(&self) -> f32 {
        self.right - self.left
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
            Move::Demonstrate | Move::DemonstratePair => (&["point"], cue.beat_t, false),
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
}

impl Move {
    pub const ALL: [Move; 11] = [
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
        }
    }

    pub fn from_key(key: &str) -> Option<Move> {
        Move::ALL.into_iter().find(|mv| mv.key() == key)
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

    /// Every row a pin can name is on the sheet that body draws with, at every
    /// quality tier. A pin naming a row the sheet lacks draws the body's slot
    /// row instead — silently — so a renamed or misspelled row would put the
    /// scholar back in his resting pose with nothing red. The sheet keys come
    /// from where the game resolves them (the boss sprite table, the character
    /// catalog), not a copy here.
    #[test]
    fn every_row_a_pin_names_is_on_the_sheet_that_draws_it() {
        use ambition_sprite_sheet::boss::boss_ron_target;
        use ambition_sprite_sheet::character::sheets::{available_sheet_keys, record_for_sheet_key};
        use std::collections::BTreeSet;

        let mut scholar = BTreeSet::new();
        let mut gnu = BTreeSet::new();
        let mut fist = BTreeSet::new();
        let s = stage();
        let l = latch(900.0);
        for mv in Move::ALL {
            for striking in [false, true] {
                let dur = 3.4;
                for step in 0..=68 {
                    let c = cue(mv, striking, dur * step as f32 / 68.0, dur);
                    scholar.extend(rows::scholar(Some(&c)).into_iter().flat_map(|p| p.0.iter().copied()));
                    gnu.extend(rows::gnu(Some(&c)).into_iter().flat_map(|p| p.0.iter().copied()));
                    for f in [Fist::Left, Fist::Right] {
                        fist.extend(rows::fist(&perform(&s, &l, f, &c), 0.0).0.iter().copied());
                    }
                }
            }
        }
        // The sample reaches every pose each body is choreographed into.
        for (body, named, expected) in [
            ("scholar", &scholar, &["point", "conduct", "invoke", "brace", "tumble", "dazed", "climb"][..]),
            ("gnu", &gnu, &["buck", "stomp"][..]),
            ("fist", &fist, &["rest", "fall", "stuck"][..]),
        ] {
            assert_eq!(named, &expected.iter().copied().collect::<BTreeSet<_>>(), "{body}");
        }

        let catalog = crate::character_catalog::load_catalog();
        let sheet_of_character = |id: &str| {
            boss_ron_target(&catalog.get(id).expect(id).spritesheet).expect(id).to_string()
        };
        let rider = crate::bosses::boss_sprite_filenames()[crate::bosses::gnu_ton::conductor::GNU_TON_ID].clone();
        let bodies = [
            (boss_ron_target(&rider).expect("rider sheet").to_string(), &scholar),
            (sheet_of_character("npc_giant_gnu"), &gnu),
            (sheet_of_character("npc_giant_gnu_hands"), &fist),
        ];
        for (key, named) in bodies {
            let tiers: Vec<&str> = available_sheet_keys()
                .into_iter()
                .filter(|k| *k == key || k.strip_prefix(key.as_str()).is_some_and(|rest| rest.starts_with('.')))
                .collect();
            assert!(tiers.len() >= 2, "`{key}` should ship with its quality tiers: {tiers:?}");
            for tier in tiers {
                let record = record_for_sheet_key(tier).expect("a listed key has a record");
                for row in named.iter() {
                    assert!(
                        record.first_bound_row([*row]).is_some(),
                        "`{tier}` has no `{row}` row; the pin would silently draw the slot row"
                    );
                }
            }
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
