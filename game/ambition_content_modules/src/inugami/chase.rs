//! The run: it flies straight at you through anything between, at your pace.
//!
//! Jon, 2026-11-19: it got stuck following the route and the rubber band did
//! not feel like running from it. It is a spirit in the chase: no wall, floor
//! or ceiling holds it, so it homes on you in a straight line. Its speed is
//! your measured pace corrected toward keeping `gap` from you, so a runner at
//! full tilt holds it off just behind, and one who stops is caught.

use bevy_math::Vec2;

/// The least it closes on a runner who has stopped, so stopping is caught.
const MIN_SPEED: f32 = 240.0;
pub(super) const MAX_SPEED: f32 = 1100.0;
/// How hard it corrects toward its gap: px/s per px off it.
const GAP_RATE: f32 = 1.8;
/// How fast its sense of your pace follows your actual pace (1/s).
const PACE_RATE: f32 = 8.0;

pub(super) struct Pursuit<'a> {
    pub last_feet: &'a mut Option<[f32; 2]>,
    pub pace: &'a mut f32,
    pub forward: &'a mut [f32; 2],
}

pub(super) struct Step {
    pub position: Vec2,
    pub distance: f32,
    pub forward: Vec2,
}

impl Pursuit<'_> {
    /// One tick toward `goal` (the runner's middle) from `at`, keeping `gap`
    /// from it, held in `bounds` (the room's air). `feet` measures the pace.
    pub fn advance(&mut self, at: Vec2, goal: Vec2, feet: Vec2, gap: f32, dt: f32, bounds: impl Fn(Vec2) -> Vec2) -> Step {
        let delta = self.last_feet.map_or(Vec2::ZERO, |last| feet - Vec2::from(last));
        let observed_pace = (delta.length() / dt.max(1e-4)).min(MAX_SPEED);
        *self.pace += (observed_pace - *self.pace) * (1.0 - (-PACE_RATE * dt).exp());
        if delta.length_squared() > 0.25 {
            *self.forward = delta.normalize().into();
        }
        *self.last_feet = Some(feet.into());
        let distance = at.distance(goal);
        let speed = (*self.pace + (distance - gap) * GAP_RATE).clamp(MIN_SPEED, MAX_SPEED);
        // It stops at its gap rather than sitting on you: the lunge is what bites.
        let travel = (speed * dt).min((distance - gap).max(0.0));
        Step {
            position: bounds(at.move_towards(goal, travel)),
            distance,
            forward: Vec2::from(*self.forward),
        }
    }
}

/// Horizontal catches lift a body over a sill; a fall continues down the shaft.
pub(super) fn escape_launch(forward: Vec2) -> Vec2 {
    if forward.x.abs() > 0.2 {
        Vec2::new(forward.x.signum(), -0.5).normalize()
    } else {
        Vec2::new(0.0, if forward.y < 0.0 { -1.0 } else { 1.0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(head: Vec2, feet: Vec2, gap: f32, state: &mut (Option<[f32; 2]>, f32, [f32; 2])) -> Step {
        Pursuit { last_feet: &mut state.0, pace: &mut state.1, forward: &mut state.2 }
            .advance(head, feet - Vec2::Y * 24.0, feet, gap, 1.0 / 60.0, |p| p)
    }

    #[test]
    fn a_runner_at_full_tilt_holds_it_off_and_one_who_stops_is_caught() {
        for direction in [-1.0, 1.0] {
            let mut state = (None, 0.0, [direction, 0.0]);
            let mut head = Vec2::new(-direction * 900.0, -300.0);
            let mut feet = Vec2::ZERO;
            let gap = 300.0;
            for tick in 0..900 {
                let running = tick < 600;
                feet.x += if running { direction * 480.0 / 60.0 } else { 0.0 };
                let next = step(head, feet, gap, &mut state);
                assert!(next.position.distance(head) <= MAX_SPEED / 60.0 + 1e-3);
                head = next.position;
                if (300..600).contains(&tick) {
                    // On your heels and no nearer: it chases, and you are running away.
                    let d = head.distance(feet - Vec2::Y * 24.0);
                    assert!((gap - 1.0..gap + 120.0).contains(&d), "tick {tick}: {d}");
                }
            }
            assert!(head.distance(feet - Vec2::Y * 24.0) <= gap + 1.0, "a stopped runner is caught up with");
            assert!(escape_launch(Vec2::from(state.2)).dot(Vec2::X * direction) > 0.0);
        }
    }

    #[test]
    fn nothing_between_holds_it() {
        // Straight through: the step is the straight line, whatever lies on it.
        let mut state = (None, 0.0, [1.0, 0.0]);
        let head = Vec2::new(0.0, 0.0);
        let next = step(head, Vec2::new(1000.0, 800.0), 200.0, &mut state);
        let along = (next.position - head).normalize();
        let want = (Vec2::new(1000.0, 776.0) - head).normalize();
        assert!(along.dot(want) > 0.999);
    }
}
