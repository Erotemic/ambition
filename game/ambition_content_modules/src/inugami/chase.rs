//! Follow the target's trail with a bounded speed and a continuous distance correction.

use bevy_math::Vec2;

use super::{RUN_GAP, SCENT_MAX, SCENT_REACH, SCENT_STEP};

const MIN_SPEED: f32 = 90.0;
const MAX_SPEED: f32 = 1100.0;
const GAP_RATE: f32 = 1.8;
const PACE_RATE: f32 = 8.0;

pub(super) struct Trail<'a> {
    pub points: &'a mut Vec<[f32; 2]>,
    pub last_feet: &'a mut Option<[f32; 2]>,
    pub pace: &'a mut f32,
    pub forward: &'a mut [f32; 2],
}

pub(super) struct Step {
    pub position: Vec2,
    pub distance: f32,
    pub forward: Vec2,
}

impl Trail<'_> {
    pub fn advance(&mut self, at: Vec2, feet: Vec2, clearance: f32, dt: f32) -> Step {
        let delta = self.last_feet.map_or(Vec2::ZERO, |last| feet - Vec2::from(last));
        let observed_pace = (delta.length() / dt.max(1e-4)).min(MAX_SPEED);
        *self.pace += (observed_pace - *self.pace) * (1.0 - (-PACE_RATE * dt).exp());
        if delta.length_squared() > 0.25 {
            *self.forward = delta.normalize().into();
        }
        *self.last_feet = Some(feet.into());
        if self.points.last().is_none_or(|p| Vec2::from(*p).distance(feet) >= SCENT_STEP) {
            if self.points.len() >= SCENT_MAX as usize {
                // Remove the least bend, so a long straight does not discard the next corner.
                let least = self.points.windows(3).enumerate().min_by(|(_, a), (_, b)| {
                    bend(a).total_cmp(&bend(b))
                }).map_or(1, |(i, _)| i + 1);
                self.points.remove(least);
            }
            self.points.push(feet.into());
        }
        let over = |p: Vec2| p - Vec2::Y * clearance;
        while self.points.first().is_some_and(|p| over(Vec2::from(*p)).distance(at) < SCENT_REACH) {
            self.points.remove(0);
        }
        let goal = self.points.first().map_or(over(feet), |p| over(Vec2::from(*p)));
        let mut distance = 0.0;
        let mut previous = at;
        for p in self.points.iter().map(|p| over(Vec2::from(*p))).chain([over(feet)]) {
            distance += previous.distance(p);
            previous = p;
        }
        let speed = (*self.pace + (distance - RUN_GAP) * GAP_RATE).clamp(MIN_SPEED, MAX_SPEED);
        let step = goal - at;
        Step {
            position: at + step.normalize_or_zero() * step.length().min(speed * dt),
            distance,
            forward: Vec2::from(*self.forward),
        }
    }
}

fn bend(points: &[[f32; 2]]) -> f32 {
    let (a, b, c) = (Vec2::from(points[0]), Vec2::from(points[1]), Vec2::from(points[2]));
    a.distance(b) + b.distance(c) - a.distance(c)
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

    #[test]
    fn pursuit_closes_a_large_gap_and_catches_a_stopped_body_in_both_directions() {
        for direction in [-1.0, 1.0] {
            let mut points = vec![];
            let mut last = None;
            let mut pace = 0.0;
            let mut forward = [direction, 0.0];
            let mut head = Vec2::new(-direction * 700.0, -200.0);
            let mut feet = Vec2::ZERO;
            for tick in 0..600 {
                let speed = if tick < 240 { 420.0 } else { 0.0 };
                feet.x += direction * speed / 60.0;
                let step = Trail { points: &mut points, last_feet: &mut last, pace: &mut pace, forward: &mut forward }
                    .advance(head, feet, 200.0, 1.0 / 60.0);
                assert!(step.position.distance(head) <= MAX_SPEED / 60.0 + 1e-3);
                head = step.position;
                if tick == 239 {
                    assert!(head.distance(feet - Vec2::Y * 200.0) < 400.0);
                }
            }
            assert!(head.distance(feet - Vec2::Y * 200.0) < SCENT_REACH);
            assert!(escape_launch(Vec2::from(forward)).dot(Vec2::X * direction) > 0.0);
        }
    }
}
