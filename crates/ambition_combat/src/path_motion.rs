//! `PathMotion`: a prescribed mover's continuation along a prepared path,
//! used by moving hazards (and carried, still unconsumed, by actor routes).
//!
//! The path law itself is core's [`PreparedPath`]: geometry is prepared once,
//! a new mover is seeded from the authored offset, and each step proposes an
//! interval before the owner commits its end. This type keeps only the phase.
//! Re-exported via `pub use path_motion::*`.

use super::*;
use ambition_platformer2d_core::{KinematicPath, PathFault, PathPhase, PreparedPath};

#[derive(Clone, Debug)]
pub struct PathMotion {
    path: PreparedPath,
    phase: PathPhase,
}

impl PathMotion {
    /// Prepare `path` and seed a new mover at its authored offset.
    pub fn new(path: &KinematicPath) -> Result<Self, PathFault> {
        let path = PreparedPath::prepare(path)?;
        let phase = path.seed();
        Ok(Self { path, phase })
    }

    /// The mutable half of this component, for `SnapshotState` (netcode.md N3.1).
    /// The prepared path is authored content and never changes; a rollback
    /// rewinds only the phase.
    pub fn phase(&self) -> PathPhase {
        self.phase
    }

    /// Rewind the phase.
    pub fn set_phase(&mut self, phase: PathPhase) {
        self.phase = phase;
    }

    /// Where the mover is now: its seed for a new one.
    pub fn position(&self) -> ae::Vec2 {
        self.path.position(self.phase)
    }

    /// Playback direction, `-1` on a ping-pong's way back.
    pub fn direction(&self) -> i32 {
        self.path.direction(self.phase)
    }

    /// Travel `dt` seconds of the path and return the new position. The
    /// prescribed mover accepts its whole proposal, so the phase and pose
    /// commit together.
    pub fn step(&mut self, dt: f32) -> ae::Vec2 {
        if let Some(interval) = self.path.propose(self.phase, dt) {
            self.phase = interval.end(&self.path);
        }
        self.position()
    }
}

#[cfg(test)]
mod path_motion_tests {
    use super::*;
    use ambition_platformer2d_core::KinematicPathMode;

    fn path(points: Vec<ae::Vec2>, mode: KinematicPathMode) -> KinematicPath {
        KinematicPath { points, speed: 1.0, mode, start_offset_seconds: 0.0 }
    }

    #[test]
    fn a_step_moves_toward_the_next_waypoint() {
        let mut m = PathMotion::new(&path(vec![ae::Vec2::ZERO, ae::Vec2::new(10.0, 0.0)], KinematicPathMode::Once)).unwrap();
        assert_eq!(m.position(), ae::Vec2::ZERO, "a new mover starts at its seed");
        let p = m.step(1.0);
        assert!((p.x - 1.0).abs() < 1e-4 && p.y.abs() < 1e-4, "{p:?}");
        assert_eq!(m.step(0.0), p, "no time, no travel");
    }

    #[test]
    fn a_degenerate_path_is_refused_not_frozen() {
        assert_eq!(PathMotion::new(&path(vec![ae::Vec2::ZERO], KinematicPathMode::Once)).err(), Some(PathFault::TooFewPoints));
    }

    /// A two-waypoint `Loop` circulates: out along the first leg and back along
    /// the closing one, every frame returning.
    #[test]
    fn a_two_point_looping_path_circulates_instead_of_spinning() {
        let a = ae::Vec2::new(0.0, 0.0);
        let b = ae::Vec2::new(300.0, 0.0);
        let mut motion = PathMotion::new(&KinematicPath {
            points: vec![a, b], speed: 600.0, mode: KinematicPathMode::Loop, start_offset_seconds: 0.0,
        }).unwrap();
        let dt = 1.0 / 60.0;
        let step = 600.0 * dt;
        let (mut reached_b, mut back_near_a) = (false, false);
        for _ in 0..120 {
            let pos = motion.step(dt);
            if (pos - b).length() <= step { reached_b = true; }
            if reached_b && (pos - a).length() <= step { back_near_a = true; }
        }
        assert!(reached_b && back_near_a, "{:?}", motion.position());
    }
}
