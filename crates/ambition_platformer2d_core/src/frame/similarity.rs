//! Validated similarities for canonical material and portal charts.

use bevy_math::Mat2;

use super::{map_vec_between, MapConvention, PortalAperture};
use crate::Vec2;

/// A numerical reason for refusing a geometric operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericFault {
    NonFinite,
    InvalidScale,
    InvalidFrame,
    Overflow,
    Underflow,
    Precision,
}

/// A finite, invertible map `scale * orthogonal * point + translation`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sim2 {
    scale: f32,
    orthogonal: Mat2,
    translation: bevy_math::DVec2,
}

impl Sim2 {
    pub const IDENTITY: Self = Self {
        scale: 1.0,
        orthogonal: Mat2::IDENTITY,
        translation: bevy_math::DVec2::ZERO,
    };

    /// Both the map and its inverse must fit the coordinate representation.
    pub fn new(scale: f32, orthogonal: Mat2, translation: Vec2) -> Result<Self, NumericFault> {
        Self::from_wide_translation(scale, orthogonal, translation.as_dvec2())
    }

    fn from_wide_translation(scale: f32, orthogonal: Mat2, translation: bevy_math::DVec2) -> Result<Self, NumericFault> {
        if !scale.is_finite() || !orthogonal.is_finite() || !translation.is_finite() {
            return Err(NumericFault::NonFinite);
        }
        if scale <= 0.0 {
            return Err(NumericFault::InvalidScale);
        }
        let gram = orthogonal.transpose() * orthogonal;
        let error = (gram.x_axis - Vec2::X)
            .abs()
            .max((gram.y_axis - Vec2::Y).abs());
        if error.max_element() > 2.0e-5 {
            return Err(NumericFault::InvalidFrame);
        }
        let inverse_scale = narrow_positive(1.0 / f64::from(scale))?;
        let inverse_translation = -rotate_wide(orthogonal.transpose(), translation);
        if !(inverse_translation * f64::from(inverse_scale)).is_finite() {
            return Err(NumericFault::Overflow);
        }
        Ok(Self {
            scale,
            orthogonal,
            translation,
        })
    }

    /// Each aperture keeps its own authored length.
    pub fn through(
        enter: &PortalAperture,
        exit: &PortalAperture,
        convention: MapConvention,
    ) -> Result<Self, NumericFault> {
        for aperture in [enter, exit] {
            let frame = aperture.frame;
            if !aperture.half_length.is_finite()
                || !frame.origin.is_finite()
                || !frame.normal.is_finite()
                || !frame.velocity.is_finite()
                || !frame.spin.is_finite()
                || !frame.pivot.is_finite()
            {
                return Err(NumericFault::NonFinite);
            }
            if aperture.half_length <= 0.0 {
                return Err(NumericFault::InvalidScale);
            }
            if (frame.normal.length_squared() - 1.0).abs() > 2.0e-5 {
                return Err(NumericFault::InvalidFrame);
            }
        }
        let scale = narrow_positive(f64::from(exit.half_length) / f64::from(enter.half_length))?;
        let orthogonal = Mat2::from_cols(
            map_vec_between(Vec2::X, enter.frame.normal, exit.frame.normal, convention),
            map_vec_between(Vec2::Y, enter.frame.normal, exit.frame.normal, convention),
        );
        let offset = rotate_wide(orthogonal, enter.frame.origin.as_dvec2()) * f64::from(scale);
        let translation = exit.frame.origin.as_dvec2() - offset;
        Self::from_wide_translation(scale, orthogonal, translation)
    }

    pub fn scale(self) -> f32 {
        self.scale
    }

    pub fn orthogonal(self) -> Mat2 {
        self.orthogonal
    }

    pub fn translation(self) -> bevy_math::DVec2 {
        self.translation
    }

    /// The determinant sign belongs to orientation, independently of scale.
    pub fn reflected(self) -> bool {
        self.orthogonal.determinant() < 0.0
    }

    pub fn point(self, point: Vec2) -> Result<Vec2, NumericFault> {
        if !point.is_finite() {
            return Err(NumericFault::NonFinite);
        }
        narrow_vec(
            rotate_wide(self.orthogonal, point.as_dvec2()) * f64::from(self.scale)
                + self.translation,
        )
    }

    pub fn vector(self, vector: Vec2) -> Result<Vec2, NumericFault> {
        if !vector.is_finite() {
            return Err(NumericFault::NonFinite);
        }
        narrow_vec(rotate_wide(self.orthogonal, vector.as_dvec2()) * f64::from(self.scale))
    }

    /// Unit directions and normalized covectors use only the orthogonal part.
    pub fn direction(self, direction: Vec2) -> Vec2 {
        self.orthogonal * direction
    }

    pub fn length(self, length: f32) -> Result<f32, NumericFault> {
        if !length.is_finite() {
            return Err(NumericFault::NonFinite);
        }
        if length < 0.0 {
            return Err(NumericFault::InvalidScale);
        }
        if length == 0.0 {
            return Ok(0.0);
        }
        narrow_positive(f64::from(length) * f64::from(self.scale))
    }

    /// Apply this map first, then `next`.
    pub fn then(self, next: Self) -> Result<Self, NumericFault> {
        let scale = narrow_positive(f64::from(self.scale) * f64::from(next.scale))?;
        let translation = rotate_wide(next.orthogonal, self.translation) * f64::from(next.scale) + next.translation;
        Self::from_wide_translation(scale, next.orthogonal * self.orthogonal, translation)
    }

    pub fn inverse(self) -> Result<Self, NumericFault> {
        let scale = narrow_positive(1.0 / f64::from(self.scale))?;
        let orthogonal = self.orthogonal.transpose();
        let translation = -rotate_wide(orthogonal, self.translation) / f64::from(self.scale);
        Self::from_wide_translation(scale, orthogonal, translation)
    }

    /// Check the feature in the destination chart and its local float spacing.
    pub fn admits_resolution(self, at: Vec2, feature_length: f32) -> Result<(), NumericFault> {
        let mapped = self.point(at)?;
        let length = self.length(feature_length)?;
        let spacing = mapped.abs().max_element().max(1.0) * f32::EPSILON;
        if length <= spacing * 8.0 {
            return Err(NumericFault::Precision);
        }
        Ok(())
    }

    /// Velocity is relative to the moving surfaces at the corresponding points.
    pub fn velocity(
        self,
        enter: &super::PortalFrame,
        exit: &super::PortalFrame,
        point: Vec2,
        velocity: Vec2,
    ) -> Result<Vec2, NumericFault> {
        let out_point = self.point(point)?;
        let relative = self.vector(velocity - enter.velocity_at(point))?;
        narrow_vec(relative.as_dvec2() + exit.velocity_at(out_point).as_dvec2())
    }
}

fn rotate_wide(orthogonal: Mat2, vector: bevy_math::DVec2) -> bevy_math::DVec2 {
    orthogonal.x_axis.as_dvec2() * vector.x + orthogonal.y_axis.as_dvec2() * vector.y
}

fn narrow_vec(value: bevy_math::DVec2) -> Result<Vec2, NumericFault> {
    if !value.is_finite() {
        return Err(NumericFault::Overflow);
    }
    let narrowed = value.as_vec2();
    if !narrowed.is_finite() {
        return Err(NumericFault::Overflow);
    }
    if (value.x != 0.0 && narrowed.x == 0.0) || (value.y != 0.0 && narrowed.y == 0.0) {
        return Err(NumericFault::Underflow);
    }
    Ok(narrowed)
}

fn narrow_positive(value: f64) -> Result<f32, NumericFault> {
    let narrowed = value as f32;
    if !value.is_finite() || !narrowed.is_finite() {
        Err(NumericFault::Overflow)
    } else if narrowed <= 0.0 {
        Err(NumericFault::Underflow)
    } else {
        Ok(narrowed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::PortalFrame;

    fn aperture(at: Vec2, normal: Vec2, half_length: f32) -> PortalAperture {
        PortalAperture {
            frame: PortalFrame::fixed(at, normal),
            half_length,
        }
    }

    #[test]
    fn unequal_apertures_are_inverse_maps_with_both_parities() {
        let a = aperture(Vec2::new(2000.0, 800.0), Vec2::NEG_Y, 40.0);
        let b = aperture(Vec2::new(2600.0, 600.0), Vec2::from_angle(0.37), 20.0);
        for convention in [MapConvention::Reflection, MapConvention::Rotation] {
            let forward = Sim2::through(&a, &b, convention).unwrap();
            let reverse = Sim2::through(&b, &a, convention).unwrap();
            assert_eq!(forward.scale(), 0.5);
            assert_eq!(reverse.scale(), 2.0);
            assert_eq!(
                forward.reflected(),
                matches!(convention, MapConvention::Reflection)
            );
            for point in [a.frame.origin, a.frame.origin + Vec2::new(7.0, -19.0)] {
                let there = forward.point(point).unwrap();
                assert!(reverse.point(there).unwrap().distance(point) < 0.002);
                assert!(
                    forward
                        .inverse()
                        .unwrap()
                        .point(there)
                        .unwrap()
                        .distance(point)
                        < 0.002
                );
            }
        }
    }

    #[test]
    fn composition_maps_translation_and_restores_parity_after_two_reflections() {
        let mirror = Mat2::from_cols(Vec2::NEG_X, Vec2::Y);
        let a = Sim2::new(2.0, mirror, Vec2::new(9.0, 4.0)).unwrap();
        let b = Sim2::new(0.5, mirror, Vec2::new(-3.0, 7.0)).unwrap();
        let together = a.then(b).unwrap();
        let point = Vec2::new(13.0, -8.0);
        assert!(!together.reflected());
        assert_eq!(together.scale(), 1.0);
        assert_eq!(
            together.point(point).unwrap(),
            b.point(a.point(point).unwrap()).unwrap()
        );
    }

    #[test]
    fn invalid_maps_and_lost_features_have_distinct_reasons() {
        assert_eq!(
            Sim2::new(f32::NAN, Mat2::IDENTITY, Vec2::ZERO),
            Err(NumericFault::NonFinite)
        );
        assert_eq!(
            Sim2::new(0.0, Mat2::IDENTITY, Vec2::ZERO),
            Err(NumericFault::InvalidScale)
        );
        assert_eq!(
            Sim2::new(f32::from_bits(1), Mat2::IDENTITY, Vec2::ZERO),
            Err(NumericFault::Overflow)
        );
        assert_eq!(
            Sim2::IDENTITY.admits_resolution(Vec2::splat(1.0e20), 1.0),
            Err(NumericFault::Precision)
        );
        let large = Sim2::new(1.0e20, Mat2::IDENTITY, Vec2::ZERO).unwrap();
        assert_eq!(large.then(large), Err(NumericFault::Overflow));
        let small = Sim2::new(1.0e-20, Mat2::IDENTITY, Vec2::ZERO).unwrap();
        assert_eq!(small.then(small), Err(NumericFault::Overflow));
        assert_eq!(
            small.length(f32::MIN_POSITIVE),
            Err(NumericFault::Underflow)
        );
    }

    #[test]
    fn moving_surface_velocity_uses_the_scaled_contact_point() {
        let mut a = aperture(Vec2::ZERO, Vec2::NEG_Y, 40.0);
        let mut b = aperture(Vec2::new(100.0, -100.0), Vec2::X, 20.0);
        a.frame.velocity = Vec2::new(4.0, 7.0);
        b.frame.spin = 2.0;
        b.frame.pivot = b.frame.origin;
        let map = Sim2::through(&a, &b, MapConvention::Rotation).unwrap();
        let point = Vec2::new(6.0, -2.0);
        let velocity = a.frame.velocity_at(point) + Vec2::new(8.0, 12.0);
        let expected = b.frame.velocity_at(map.point(point).unwrap())
            + map.vector(velocity - a.frame.velocity_at(point)).unwrap();
        assert!(
            map.velocity(&a.frame, &b.frame, point, velocity)
                .unwrap()
                .distance(expected)
                < 1.0e-5
        );
    }
}
