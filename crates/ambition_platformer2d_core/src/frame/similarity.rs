//! Validated similarities for canonical material and portal charts.

use bevy_math::{DMat2, DVec2, Mat2};

use super::{MapConvention, PortalAperture};
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
    scale: f64,
    orthogonal: DMat2,
    translation: bevy_math::DVec2,
}

impl Sim2 {
    pub const IDENTITY: Self = Self {
        scale: 1.0,
        orthogonal: DMat2::IDENTITY,
        translation: bevy_math::DVec2::ZERO,
    };

    /// Both the map and its inverse must fit the coordinate representation.
    pub fn new(scale: f32, orthogonal: Mat2, translation: Vec2) -> Result<Self, NumericFault> {
        Self::from_wide_translation(f64::from(scale), orthogonal_wide(orthogonal)?, translation.as_dvec2())
    }

    fn from_wide_translation(scale: f64, orthogonal: DMat2, translation: bevy_math::DVec2) -> Result<Self, NumericFault> {
        if !scale.is_finite() || !orthogonal.is_finite() || !translation.is_finite() {
            return Err(NumericFault::NonFinite);
        }
        if scale <= 0.0 {
            return Err(NumericFault::InvalidScale);
        }
        let gram = orthogonal.transpose() * orthogonal;
        let error = (gram.x_axis - DVec2::X)
            .abs()
            .max((gram.y_axis - DVec2::Y).abs());
        if error.max_element() > 2.0e-5 {
            return Err(NumericFault::InvalidFrame);
        }
        narrow_positive(scale)?;
        let inverse_scale = narrow_positive(1.0 / scale)?;
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
        let scale = f64::from(exit.half_length) / f64::from(enter.half_length);
        let enter_normal = enter.frame.normal.as_dvec2().normalize();
        let exit_normal = exit.frame.normal.as_dvec2().normalize();
        let enter_tangent = DVec2::new(-enter_normal.y, enter_normal.x);
        let exit_tangent = DVec2::new(-exit_normal.y, exit_normal.x);
        let along = match convention { MapConvention::Reflection => 1.0, MapConvention::Rotation => -1.0 };
        let orthogonal = DMat2::from_cols(
            -exit_normal * enter_normal.x + exit_tangent * (along * enter_tangent.x),
            -exit_normal * enter_normal.y + exit_tangent * (along * enter_tangent.y),
        );
        let offset = rotate_wide(orthogonal, enter.frame.origin.as_dvec2()) * scale;
        let translation = exit.frame.origin.as_dvec2() - offset;
        Self::from_wide_translation(scale, orthogonal, translation)
    }

    pub(super) fn scale_wide(self) -> f64 { self.scale }

    pub(super) fn inverse_direction_wide(self, direction: DVec2) -> DVec2 {
        self.orthogonal.transpose() * direction
    }

    pub fn scale(self) -> f32 {
        self.scale as f32
    }

    pub fn admits_scale_exponent(self, exponent: u8) -> bool {
        let limit = 2.0_f64.powi(i32::from(exponent));
        self.scale >= 1.0 / limit && self.scale <= limit
    }

    pub fn admits_scale_product(self, next: Self, exponent: u8) -> bool {
        let product = self.scale * next.scale;
        let limit = 2.0_f64.powi(i32::from(exponent));
        product >= 1.0 / limit && product <= limit
    }

    pub fn orthogonal(self) -> Mat2 {
        Mat2::from_cols(self.orthogonal.x_axis.as_vec2(), self.orthogonal.y_axis.as_vec2())
    }

    pub fn translation(self) -> bevy_math::DVec2 {
        self.translation
    }

    /// The determinant sign belongs to orientation, independently of scale.
    pub fn reflected(self) -> bool {
        self.orthogonal.determinant() < 0.0
    }

    pub(super) fn point_wide(self, point: DVec2) -> DVec2 {
        self.orthogonal * point * self.scale + self.translation
    }

    pub(super) fn direction_wide(self, direction: DVec2) -> DVec2 {
        self.orthogonal * direction
    }

    pub(super) fn point_from_wide(self, point: DVec2) -> Result<Vec2, NumericFault> {
        if !point.is_finite() { return Err(NumericFault::NonFinite); }
        narrow_vec(self.point_wide(point))
    }

    pub fn point(self, point: Vec2) -> Result<Vec2, NumericFault> {
        self.point_from_wide(point.as_dvec2())
    }

    pub fn vector(self, vector: Vec2) -> Result<Vec2, NumericFault> {
        if !vector.is_finite() {
            return Err(NumericFault::NonFinite);
        }
        narrow_vec(rotate_wide(self.orthogonal, vector.as_dvec2()) * self.scale)
    }

    /// Unit directions and normalized covectors use only the orthogonal part.
    pub fn direction(self, direction: Vec2) -> Vec2 {
        (self.orthogonal * direction.as_dvec2()).as_vec2()
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
        narrow_positive(f64::from(length) * self.scale)
    }

    /// Apply this map first, then `next`.
    pub fn then(self, next: Self) -> Result<Self, NumericFault> {
        let scale = self.scale * next.scale;
        let translation = rotate_wide(next.orthogonal, self.translation) * next.scale + next.translation;
        Self::from_wide_translation(scale, next.orthogonal * self.orthogonal, translation)
    }

    pub fn inverse(self) -> Result<Self, NumericFault> {
        let scale = 1.0 / self.scale;
        let orthogonal = self.orthogonal.transpose();
        let translation = -rotate_wide(orthogonal, self.translation) / self.scale;
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
        if !point.is_finite() || !velocity.is_finite() { return Err(NumericFault::NonFinite); }
        let point = point.as_dvec2();
        let out_point = self.point_wide(point);
        narrow_vec(out_point)?;
        let surface_velocity = |frame: &super::PortalFrame, at: DVec2| -> Result<DVec2, NumericFault> {
            if !frame.velocity.is_finite() || !frame.spin.is_finite() || !frame.pivot.is_finite() {
                return Err(NumericFault::NonFinite);
            }
            let arm = at - frame.pivot.as_dvec2();
            Ok(frame.velocity.as_dvec2() + f64::from(frame.spin) * DVec2::new(-arm.y, arm.x))
        };
        let relative = velocity.as_dvec2() - surface_velocity(enter, point)?;
        narrow_vec(self.orthogonal * relative * self.scale + surface_velocity(exit, out_point)?)
    }
}

fn rotate_wide(orthogonal: DMat2, vector: DVec2) -> DVec2 {
    orthogonal * vector
}

fn orthogonal_wide(matrix: Mat2) -> Result<DMat2, NumericFault> {
    if !matrix.is_finite() { return Err(NumericFault::NonFinite); }
    let matrix = DMat2::from_cols(matrix.x_axis.as_dvec2(), matrix.y_axis.as_dvec2());
    let gram = matrix.transpose() * matrix;
    if (gram.x_axis - DVec2::X).abs().max((gram.y_axis - DVec2::Y).abs()).max_element() > 2.0e-5 {
        return Err(NumericFault::InvalidFrame);
    }
    let x = matrix.x_axis.normalize();
    let parity = matrix.determinant().signum();
    Ok(DMat2::from_cols(x, DVec2::new(-x.y, x.x) * parity))
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

pub(crate) fn scale_quantity(value: f32, scale: f32) -> Result<f32, NumericFault> {
    if !value.is_finite() { return Err(NumericFault::NonFinite); }
    narrow_vec(DVec2::new(f64::from(value) * f64::from(scale), 0.0)).map(|value| value.x)
}

pub(crate) fn scale_bound(value: f32, scale: f32) -> Result<f32, NumericFault> {
    if value == f32::INFINITY { return Ok(value); }
    scale_quantity(value, scale)
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
    fn moving_frame_velocity_keeps_finite_cancellation() {
        let frame = PortalFrame { velocity: Vec2::new(f32::MAX, 0.0), ..PortalFrame::fixed(Vec2::ZERO, Vec2::Y) };
        assert_eq!(Sim2::IDENTITY.velocity(&frame, &frame, Vec2::ZERO, Vec2::new(-f32::MAX, 0.0)).unwrap(), Vec2::new(-f32::MAX, 0.0));
    }

    #[test]
    fn repeated_turns_keep_an_invertible_orthogonal_frame() {
        let turn = Sim2::new(1.0, Mat2::from_angle(0.37), Vec2::ZERO).unwrap();
        let mut composed = Sim2::IDENTITY;
        for _ in 0..100_000 { composed = composed.then(turn).unwrap(); }
        let point = Vec2::new(7.0, 13.0);
        assert!((composed.vector(point).unwrap().length() - point.length()).abs() < 0.00001);
        assert!((composed.then(composed.inverse().unwrap()).unwrap().point(point).unwrap() - point).length() < 0.00001);
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
