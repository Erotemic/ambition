//! Declarative movement paths for moving platforms, spike balls, patrol
//! dummies, and scripted hazards.
//!
//! This is world/geometry vocabulary: a path is a list of `Vec2` waypoints
//! plus a speed and playback mode. It lives beside `World`/`Block`/
//! `SurfaceChain` in the engine because it carries spatial primitives and is
//! authored INPUT to the sim, not actor mind/behavior data.

use crate::Vec2;

/// Declarative movement path for moving platforms, spike balls, patrol dummies,
/// and later scripted boss hazards.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KinematicPath {
    pub points: Vec<Vec2>,
    pub speed: f32,
    pub mode: KinematicPathMode,
    pub start_offset_seconds: f32,
}

impl KinematicPath {
    pub fn line(a: Vec2, b: Vec2, speed: f32) -> Self {
        Self {
            points: vec![a, b],
            speed,
            mode: KinematicPathMode::PingPong,
            start_offset_seconds: 0.0,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.points.len() >= 2 && self.speed > 0.0
    }
}

/// Resolve an authored path reference through the shared lookup rule.
/// References are trimmed and an empty reference names nothing. Authored path
/// keys are not normalized here, so malformed path definitions remain visible to
/// validation rather than being repaired during lookup.
pub fn resolve_kinematic_path<'a>(
    paths: &'a [(String, KinematicPath)],
    reference: &str,
) -> Option<&'a KinematicPath> {
    let reference = reference.trim();
    if reference.is_empty() {
        return None;
    }
    paths
        .iter()
        .find(|(key, _)| key == reference)
        .map(|(_, path)| path)
}

/// Playback style for a kinematic path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum KinematicPathMode {
    Once,
    Loop,
    PingPong,
}

// ── PATH1: one prepared spatial path law ────────────────────────────────────

/// Why an authored path cannot be prepared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathFault {
    /// Fewer than two points.
    TooFewPoints,
    /// A point, the speed or the offset is not finite.
    NonFinite,
    /// The speed is not positive.
    Speed,
    /// The initial offset is negative.
    Offset,
    /// Every leg has zero length: nothing to travel along.
    ZeroLength,
}

impl std::fmt::Display for PathFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::TooFewPoints => "a path needs at least two points",
            Self::NonFinite => "a point, the speed or the start offset is not finite",
            Self::Speed => "the speed must be positive",
            Self::Offset => "the start offset must not be negative",
            Self::ZeroLength => "every leg has zero length, so there is nothing to travel along",
        })
    }
}

/// An authored path prepared once: finite points with zero-length legs
/// removed, ordered cumulative distance (with `Loop`'s closing leg), and a
/// positive total. The one evaluator every path consumer shares; a consumer
/// keeps only a [`PathPhase`] continuation and its own authority.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PreparedPath {
    points: Vec<Vec2>,
    /// `cumulative[i]` is the distance from `points[0]` to `points[i]` along
    /// the route; `Loop` adds one entry for the closing leg back to `points[0]`.
    cumulative: Vec<f64>,
    speed: f32,
    mode: KinematicPathMode,
    start_offset_seconds: f32,
}

/// Where a prescribed mover is along its prepared path: the distance travelled
/// within one cycle. `Once` holds `[0, total]`, `Loop` `[0, total)`, and
/// `PingPong` `[0, 2 * total)` (outward, then back). Its direction follows
/// from the phase, so a restore needs nothing else.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PathPhase(pub f64);

/// A requested travel along a prepared path, proposed without mutation: from
/// `from`, `travel` units of unwrapped distance. The owner samples or cuts it
/// and commits the continuation and pose it accepts together.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathInterval {
    pub from: PathPhase,
    pub travel: f64,
}

impl PreparedPath {
    pub fn prepare(path: &KinematicPath) -> Result<Self, PathFault> {
        if path.points.len() < 2 {
            return Err(PathFault::TooFewPoints);
        }
        if !path.speed.is_finite()
            || !path.start_offset_seconds.is_finite()
            || path.points.iter().any(|point| !point.is_finite())
        {
            return Err(PathFault::NonFinite);
        }
        if path.speed <= 0.0 {
            return Err(PathFault::Speed);
        }
        if path.start_offset_seconds < 0.0 {
            return Err(PathFault::Offset);
        }
        // Drop each point that repeats the one before it: a zero-length leg
        // consumes no distance, which is what let two coincident waypoints
        // spin a looping cursor forever. Revisits that are not adjacent stay:
        // a self-crossing route keeps its ordered occurrences.
        let mut points: Vec<Vec2> = Vec::with_capacity(path.points.len());
        for point in &path.points {
            if points.last() != Some(point) {
                points.push(*point);
            }
        }
        if path.mode == KinematicPathMode::Loop {
            while points.len() > 1 && points.last() == points.first() {
                points.pop();
            }
        }
        if points.len() < 2 {
            return Err(PathFault::ZeroLength);
        }
        let mut cumulative = Vec::with_capacity(points.len() + 1);
        let mut total = 0.0_f64;
        cumulative.push(0.0);
        let closing = (path.mode == KinematicPathMode::Loop).then(|| (points[points.len() - 1], points[0]));
        for (a, b) in points.windows(2).map(|pair| (pair[0], pair[1])).chain(closing) {
            total += (b.as_dvec2() - a.as_dvec2()).length();
            cumulative.push(total);
        }
        if !(total.is_finite() && total > 0.0) {
            return Err(PathFault::ZeroLength);
        }
        Ok(Self {
            points,
            cumulative,
            speed: path.speed,
            mode: path.mode,
            start_offset_seconds: path.start_offset_seconds,
        })
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }
    pub fn mode(&self) -> KinematicPathMode {
        self.mode
    }
    /// The prepared waypoints (zero-length legs removed).
    pub fn points(&self) -> &[Vec2] {
        &self.points
    }
    /// Length of the route once through (with `Loop`'s closing leg).
    pub fn length(&self) -> f64 {
        *self.cumulative.last().expect("a prepared path has a leg")
    }
    /// Distance of one cycle of the mode: `PingPong` goes out and back.
    fn cycle(&self) -> f64 {
        match self.mode {
            KinematicPathMode::PingPong => 2.0 * self.length(),
            _ => self.length(),
        }
    }

    /// Reduce unwrapped distance to the phase it lands on. Work is constant
    /// in the number of laps: one remainder, never a walk lap by lap.
    fn reduce(&self, distance: f64) -> PathPhase {
        match self.mode {
            KinematicPathMode::Once => PathPhase(distance.clamp(0.0, self.length())),
            _ => {
                let phase = distance.rem_euclid(self.cycle());
                // `rem_euclid` can round up to the modulus itself.
                PathPhase(if phase >= self.cycle() { 0.0 } else { phase })
            }
        }
    }

    /// The phase a NEW prescribed mover starts at: its authored offset in
    /// seconds of travel, reduced under the mode. Initial placement, not travel
    /// before the object existed; a restore keeps its own phase and never
    /// applies the offset again.
    pub fn seed(&self) -> PathPhase {
        self.reduce(f64::from(self.start_offset_seconds) * f64::from(self.speed))
    }

    /// Propose `dt` seconds of travel from `from` without moving anything.
    /// `None` for a nonpositive or nonfinite `dt`.
    pub fn propose(&self, from: PathPhase, dt: f32) -> Option<PathInterval> {
        if !(dt.is_finite() && dt > 0.0) {
            return None;
        }
        let travel = f64::from(self.speed) * f64::from(dt);
        travel.is_finite().then_some(PathInterval { from, travel })
    }

    /// Where `phase` is on the route.
    pub fn position(&self, phase: PathPhase) -> Vec2 {
        let along = match self.mode {
            KinematicPathMode::PingPong if phase.0 > self.length() => self.cycle() - phase.0,
            _ => phase.0,
        }
        .clamp(0.0, self.length());
        // The leg whose span holds `along`: binary search, not a walk.
        let leg = match self.cumulative.binary_search_by(|d| d.total_cmp(&along)) {
            Ok(index) => index.min(self.cumulative.len() - 2),
            Err(index) => index.saturating_sub(1).min(self.cumulative.len() - 2),
        };
        let (a, b) = (self.points[leg], self.points[(leg + 1) % self.points.len()]);
        let span = self.cumulative[leg + 1] - self.cumulative[leg];
        let t = if span > 0.0 { (along - self.cumulative[leg]) / span } else { 0.0 };
        (a.as_dvec2() + (b.as_dvec2() - a.as_dvec2()) * t).as_vec2()
    }

    /// Playback direction at `phase`: `-1` on a ping-pong's way back, else `+1`.
    pub fn direction(&self, phase: PathPhase) -> i32 {
        if self.mode == KinematicPathMode::PingPong && phase.0 > self.length() { -1 } else { 1 }
    }
}

impl PathInterval {
    /// The phase this interval ends at on `path`.
    pub fn end(&self, path: &PreparedPath) -> PathPhase {
        self.at(path, 1.0)
    }
    /// The phase after the fraction `t` (`0..=1`) of this interval: a cut keeps
    /// the interval's own mapping, so a prefix is never re-paced.
    pub fn at(&self, path: &PreparedPath, t: f64) -> PathPhase {
        path.reduce(self.from.0 + self.travel * t.clamp(0.0, 1.0))
    }
}

#[cfg(test)]
mod reference_resolution_tests {
    use super::*;

    fn table() -> Vec<(String, KinematicPath)> {
        vec![
            // A room built in Rust rather than LDtk can carry a path whose
            // authored id is blank — LDtk's converter falls back to the entity
            // name and then the iid, and nothing outside it does.
            (
                String::new(),
                KinematicPath::line(Vec2::ZERO, Vec2::ZERO, 1.0),
            ),
            (
                "lab_patrol".to_string(),
                KinematicPath::line(Vec2::new(0.0, 0.0), Vec2::new(64.0, 0.0), 40.0),
            ),
        ]
    }

    /// A padded authored reference names the same path a clean one does, and a
    /// blank one names nothing. The whole point of the rule living in one
    /// place: a leading space in an NPC's `patrol_path_id` used to disable that
    /// patrol silently, because a miss here degrades to "no motion" rather than
    /// to an error anyone sees.
    #[test]
    fn an_authored_reference_is_trimmed_and_a_blank_one_names_nothing() {
        let paths = table();
        let clean = resolve_kinematic_path(&paths, "lab_patrol");
        assert!(clean.is_some(), "precondition: the clean spelling resolves");
        assert_eq!(
            resolve_kinematic_path(&paths, "  lab_patrol "),
            clean,
            "padding around an authored reference must not change what it names"
        );
        assert!(
            resolve_kinematic_path(&paths, "   ").is_none(),
            "a whitespace-only reference names nothing — it must NOT fall through \
             onto a path whose own id happens to be blank"
        );
    }
}

#[cfg(test)]
mod prepared_path_tests {
    use super::*;

    fn path(points: &[(f32, f32)], mode: KinematicPathMode, offset: f32) -> KinematicPath {
        KinematicPath { points: points.iter().map(|&(x, y)| Vec2::new(x, y)).collect(), speed: 10.0, mode,
            start_offset_seconds: offset }
    }

    /// Two coincident waypoints used to spin a looping cursor forever: a
    /// zero-length leg consumed no distance. Prepared, the repeat is gone and
    /// a step returns.
    #[test]
    fn coincident_waypoints_cannot_stall_a_loop() {
        for mode in [KinematicPathMode::Loop, KinematicPathMode::PingPong] {
            let prepared = PreparedPath::prepare(&path(&[(0.0, 0.0), (0.0, 0.0), (10.0, 0.0)], mode, 0.0)).unwrap();
            assert_eq!(prepared.points().len(), 2, "{mode:?}");
            let step = prepared.propose(prepared.seed(), 0.5).unwrap();
            assert_eq!(prepared.position(step.end(&prepared)), Vec2::new(5.0, 0.0), "{mode:?}");
        }
        assert_eq!(PreparedPath::prepare(&path(&[(3.0, 3.0), (3.0, 3.0)], KinematicPathMode::Loop, 0.0)),
            Err(PathFault::ZeroLength));
        assert_eq!(PreparedPath::prepare(&path(&[(0.0, 0.0)], KinematicPathMode::Once, 0.0)),
            Err(PathFault::TooFewPoints));
        assert_eq!(PreparedPath::prepare(&KinematicPath { speed: 0.0, ..path(&[(0.0, 0.0), (1.0, 0.0)], KinematicPathMode::Once, 0.0) }),
            Err(PathFault::Speed));
        assert_eq!(PreparedPath::prepare(&path(&[(0.0, 0.0), (f32::NAN, 0.0)], KinematicPathMode::Once, 0.0)),
            Err(PathFault::NonFinite));
    }

    /// `Loop` closes its circuit; `PingPong` reverses and reports its way back;
    /// `Once` holds at its end.
    #[test]
    fn each_mode_closes_reverses_or_holds() {
        let square = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)];
        let looped = PreparedPath::prepare(&path(&square, KinematicPathMode::Loop, 0.0)).unwrap();
        let total = 20.0 + 200.0_f64.sqrt();
        assert!((looped.length() - total).abs() < 1e-9);
        assert_eq!(looped.position(PathPhase(total - 1e-9)).round(), Vec2::ZERO, "the closing leg returns to the start");
        let pong = PreparedPath::prepare(&path(&square, KinematicPathMode::PingPong, 0.0)).unwrap();
        assert_eq!(pong.position(PathPhase(25.0)), Vec2::new(10.0, 5.0), "15 back from the far end");
        assert_eq!(pong.direction(PathPhase(25.0)), -1);
        assert_eq!(pong.direction(PathPhase(5.0)), 1);
        let once = PreparedPath::prepare(&path(&square, KinematicPathMode::Once, 0.0)).unwrap();
        let far = once.propose(once.seed(), 100.0).unwrap().end(&once);
        assert_eq!(once.position(far), Vec2::new(10.0, 10.0));
    }

    /// A nonzero offset is where a new mover starts, reduced under its mode.
    #[test]
    fn a_start_offset_seeds_the_phase() {
        let line = [(0.0, 0.0), (10.0, 0.0)];
        // 10 u/s for 1.5 s: 15 along a 10-unit ping-pong is 5 on the way back.
        let pong = PreparedPath::prepare(&path(&line, KinematicPathMode::PingPong, 1.5)).unwrap();
        assert_eq!(pong.position(pong.seed()), Vec2::new(5.0, 0.0));
        assert_eq!(pong.direction(pong.seed()), -1);
        let once = PreparedPath::prepare(&path(&line, KinematicPathMode::Once, 7.0)).unwrap();
        assert_eq!(once.position(once.seed()), Vec2::new(10.0, 0.0));
        assert_eq!(PreparedPath::prepare(&path(&line, KinematicPathMode::Once, -1.0)), Err(PathFault::Offset));
    }

    /// A step across many laps costs one reduction and lands where the same
    /// travel in small steps lands; a cut keeps the interval's mapping.
    #[test]
    fn a_step_across_many_laps_lands_where_small_steps_do() {
        let looped = PreparedPath::prepare(&path(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
            KinematicPathMode::Loop, 0.0)).unwrap();
        // 4001 steps of 2.5 s at 10 u/s: 100025 units, 2500 laps of 40 and 25 more.
        let big = looped.propose(PathPhase(3.0), 10_002.5).unwrap().end(&looped);
        let mut small = PathPhase(3.0);
        for _ in 0..4001 { small = looped.propose(small, 2.5).unwrap().end(&looped); }
        assert!((big.0 - small.0).abs() < 1e-6, "{big:?} vs {small:?}");
        assert!((looped.position(big) - Vec2::new(2.0, 10.0)).length() < 1e-4, "{:?}", looped.position(big));
        let step = looped.propose(PathPhase(3.0), 1.0).unwrap();
        assert_eq!(step.at(&looped, 0.5), PathPhase(8.0));
        assert!(looped.propose(PathPhase(0.0), 0.0).is_none() && looped.propose(PathPhase(0.0), f32::NAN).is_none());
    }
}
