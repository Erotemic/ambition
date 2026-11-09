//! Recorded portal travel and samples within a world tick.

use super::{MapConvention, NumericFault, PortalAperture, PortalFrame, Sim2};
use crate::Vec2;

/// Rotation from the published end pose at one tick time.
pub(crate) fn rotation_ending_at(turn: f64, time: f64) -> bevy_math::DMat2 {
    if time == 1.0 {
        bevy_math::DMat2::IDENTITY
    } else {
        bevy_math::DMat2::from_angle(turn * time) * bevy_math::DMat2::from_angle(-turn)
    }
}

/// Include both ends when the midpoint rounds onto an interval end.
pub(crate) fn interval_center(time: &std::ops::Range<f64>) -> (f64, f64) {
    let middle = time.start + (time.end - time.start) * 0.5;
    (middle, (middle - time.start).max(time.end - middle))
}

/// Bound the normal change, including angular argument rounding.
pub(crate) fn rotation_bound(turn: f64, time: f64, radius: f64) -> f64 {
    if radius == 0.0 || turn == 0.0 {
        return 0.0;
    }
    let travel = turn.abs() * radius;
    (travel + (turn.abs() * time.abs() + travel) * f64::EPSILON * 8.0).min(2.0)
}

/// A host pose before projection to storage coordinates.
#[derive(Clone, Copy, Debug)]
pub(super) struct WideAperture {
    pub origin: bevy_math::DVec2,
    pub normal: bevy_math::DVec2,
    pub pivot: bevy_math::DVec2,
    pub shift: bevy_math::DVec2,
    pub turn: f64,
    pub half: f64,
    pub phase_error: f64,
}

impl WideAperture {
    pub fn tangent(self) -> bevy_math::DVec2 {
        bevy_math::DVec2::new(-self.normal.y, self.normal.x)
    }
    pub fn origin_velocity(self) -> bevy_math::DVec2 {
        let arm = self.origin - self.pivot;
        self.shift + bevy_math::DVec2::new(-arm.y, arm.x) * self.turn
    }
    pub fn variation(self, half: f64) -> f64 {
        self.shift.length() * half + (self.origin - self.pivot).length() * self.angle_bound(half)
    }
    pub fn angle_bound(self, half: f64) -> f64 {
        if half == 0.0 || self.turn == 0.0 {
            return 0.0;
        }
        let travel = self.turn.abs() * half;
        (travel + self.phase_error + travel * f64::EPSILON * 8.0).min(2.0)
    }
}

/// The start pose and the host's published travel during one tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ApertureStep {
    pub origin: Vec2,
    pub normal: Vec2,
    pub displacement: Vec2,
    pub turn: f32,
}

impl ApertureStep {
    pub(super) fn sample_wide(
        self,
        end: PortalAperture,
        time: f64,
    ) -> Result<WideAperture, NumericFault> {
        self.validate(end, time)?;
        let rotation = rotation_ending_at(f64::from(self.turn), time);
        let pivot = end.frame.pivot.as_dvec2() - self.displacement.as_dvec2() * (1.0 - time);
        Ok(WideAperture {
            origin: pivot + rotation * (end.frame.origin.as_dvec2() - end.frame.pivot.as_dvec2()),
            normal: rotation * end.frame.normal.as_dvec2().normalize(),
            pivot,
            shift: self.displacement.as_dvec2(),
            turn: f64::from(self.turn),
            half: f64::from(end.half_length),
            phase_error: f64::from(self.turn).abs() * time.abs() * f64::EPSILON * 8.0,
        })
    }
    pub fn fixed(origin: Vec2, normal: Vec2) -> Self {
        Self {
            origin,
            normal,
            displacement: Vec2::ZERO,
            turn: 0.0,
        }
    }

    /// Recover the start pose from the host's end pose and published travel.
    /// A host wrap changes its location, but is not part of that travel.
    pub fn ending_at(end: PortalFrame, displacement: Vec2, turn: f32) -> Self {
        let rotation = bevy_math::DMat2::from_angle(-f64::from(turn));
        Self {
            origin: (end.pivot.as_dvec2() - displacement.as_dvec2()
                + rotation * (end.origin.as_dvec2() - end.pivot.as_dvec2()))
            .as_vec2(),
            normal: (rotation * end.normal.as_dvec2().normalize_or_zero()).as_vec2(),
            displacement,
            turn,
        }
    }

    /// Sample the host's translation and rotation. A full turn retains its
    /// winding even when the start and end normals are the same.
    pub fn sample(self, end: PortalAperture, time: f64) -> Result<PortalAperture, NumericFault> {
        let pose = self.sample_wide(end, time)?;
        if time == 1.0 {
            return Ok(end);
        }
        Ok(PortalAperture {
            frame: PortalFrame {
                origin: Sim2::IDENTITY.point_from_wide(pose.origin)?,
                normal: Sim2::IDENTITY.vector_from_wide(pose.normal)?,
                pivot: Sim2::IDENTITY.point_from_wide(pose.pivot)?,
                ..end.frame
            },
            half_length: end.half_length,
        })
    }
    fn validate(self, end: PortalAperture, time: f64) -> Result<(), NumericFault> {
        use bevy_math::DMat2;
        if !time.is_finite()
            || !self.origin.is_finite()
            || !self.normal.is_finite()
            || !self.displacement.is_finite()
            || !self.turn.is_finite()
        {
            return Err(NumericFault::NonFinite);
        }
        if !(0.0..=1.0).contains(&time) {
            return Err(NumericFault::InvalidFrame);
        }
        Sim2::through(&end, &end, MapConvention::Reflection)?;
        if (self.normal.as_dvec2().length_squared() - 1.0).abs() > 2.0e-5 {
            return Err(NumericFault::InvalidFrame);
        }
        let pivot = end.frame.pivot.as_dvec2() - self.displacement.as_dvec2();
        let expected = pivot
            + self.displacement.as_dvec2()
            + DMat2::from_angle(f64::from(self.turn)) * (self.origin.as_dvec2() - pivot);
        let expected_normal = DMat2::from_angle(f64::from(self.turn)) * self.normal.as_dvec2();
        // Rotation mixes the errors of both coordinates.
        let magnitude = pivot.abs().max_element()
            + self.displacement.as_dvec2().abs().max_element()
            + (self.origin.as_dvec2() - pivot).length()
            + end.frame.origin.as_dvec2().abs().max_element();
        let error = magnitude * f64::from(f32::EPSILON) * 8.0;
        if (expected - end.frame.origin.as_dvec2()).abs().max_element() > error
            || expected_normal.distance(end.frame.normal.as_dvec2())
                > f64::from(f32::EPSILON) * 16.0
        {
            return Err(NumericFault::Precision);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_aperture_step_follows_its_host_arc_and_translation() {
        let step = ApertureStep {
            origin: Vec2::new(10.0, 0.0),
            normal: Vec2::X,
            displacement: Vec2::new(5.0, 3.0),
            turn: std::f32::consts::FRAC_PI_2,
        };
        let end = PortalAperture {
            frame: PortalFrame {
                origin: Vec2::new(5.0, 13.0),
                normal: Vec2::Y,
                velocity: Vec2::new(5.0, 3.0),
                spin: step.turn,
                pivot: Vec2::new(5.0, 3.0),
            },
            half_length: 40.0,
        };
        let middle = step.sample(end, 0.5).unwrap();
        let expected = Vec2::new(2.5, 1.5) + Vec2::splat(10.0 / std::f32::consts::SQRT_2);
        assert!(middle.frame.origin.distance(expected) < 1.0e-5);
        assert!(
            middle
                .frame
                .origin
                .distance((step.origin + end.frame.origin) * 0.5)
                > 2.0
        );
        assert!(
            middle
                .frame
                .normal
                .distance(Vec2::splat(1.0 / std::f32::consts::SQRT_2))
                < 1.0e-6
        );
        assert_eq!(middle.frame.pivot, Vec2::new(2.5, 1.5));
        assert_eq!(middle.frame.velocity, end.frame.velocity);
        assert!(
            step.sample(end, 0.0)
                .unwrap()
                .frame
                .origin
                .distance(step.origin)
                < 1.0e-5
        );
        assert_eq!(step.sample(end, 1.0).unwrap(), end);
    }

    #[test]
    fn a_full_turn_is_not_replaced_with_the_shortest_normal_arc() {
        let step = ApertureStep {
            origin: Vec2::new(10.0, 0.0),
            normal: Vec2::X,
            displacement: Vec2::ZERO,
            turn: std::f32::consts::TAU * 2.0,
        };
        let end = PortalAperture {
            frame: PortalFrame {
                pivot: Vec2::ZERO,
                ..PortalFrame::fixed(step.origin, step.normal)
            },
            half_length: 10.0,
        };
        let quarter = step.sample(end, 0.125).unwrap();
        assert!(quarter.frame.origin.distance(Vec2::new(0.0, 10.0)) < 1.0e-5);
        assert!(quarter.frame.normal.distance(Vec2::Y) < 1.0e-6);
    }

    #[test]
    fn an_inconsistent_aperture_history_is_a_typed_refusal() {
        let mut step = ApertureStep::fixed(Vec2::ZERO, Vec2::Y);
        step.displacement = Vec2::X;
        let end = PortalAperture {
            frame: PortalFrame::fixed(Vec2::ZERO, Vec2::Y),
            half_length: 10.0,
        };
        assert_eq!(step.sample(end, 0.5), Err(NumericFault::Precision));
        step.displacement = Vec2::ZERO;
        assert_eq!(step.sample(end, f64::NAN), Err(NumericFault::NonFinite));
        assert_eq!(step.sample(end, 1.1), Err(NumericFault::InvalidFrame));
    }
}
