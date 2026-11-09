//! Recorded portal travel and samples within a world tick.

use super::{MapConvention, NumericFault, PortalAperture, PortalFrame, Sim2};
use crate::Vec2;

/// The start pose and the host's published travel during one tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ApertureStep {
    pub origin: Vec2,
    pub normal: Vec2,
    pub displacement: Vec2,
    pub turn: f32,
}

impl ApertureStep {
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
        let rotation = bevy_math::Mat2::from_angle(-turn);
        Self {
            origin: end.pivot - displacement + rotation * (end.origin - end.pivot),
            normal: rotation * end.normal,
            displacement,
            turn,
        }
    }

    /// Sample the host's translation and rotation. A full turn retains its
    /// winding even when the start and end normals are the same.
    pub fn sample(self, end: PortalAperture, time: f64) -> Result<PortalAperture, NumericFault> {
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
        let rotation = DMat2::from_angle(f64::from(self.turn) * time);
        let origin = pivot
            + self.displacement.as_dvec2() * time
            + rotation * (self.origin.as_dvec2() - pivot);
        let normal = rotation * self.normal.as_dvec2();
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
        if time == 1.0 {
            return Ok(end);
        }
        Ok(PortalAperture {
            frame: PortalFrame {
                origin: Sim2::IDENTITY.point_from_wide(origin)?,
                normal: Sim2::IDENTITY.vector_from_wide(normal)?,
                pivot: Sim2::IDENTITY
                    .point_from_wide(pivot + self.displacement.as_dvec2() * time)?,
                ..end.frame
            },
            half_length: end.half_length,
        })
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
        assert_eq!(step.sample(end, 0.0).unwrap().frame.origin, step.origin);
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
