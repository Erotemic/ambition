//! Dimension-independent special-relativity foundations.
//!
//! This crate deliberately owns no Bevy or platformer vocabulary. Minkowski
//! spacetime is the first exact model behind Ambition's spacetime-provider
//! boundary; a later curved provider can reuse the clock, interval, observer,
//! and regression vocabulary without treating one global inertial frame as an
//! engine law.

use core::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InvariantSpeed(f64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvariantSpeedError;

impl fmt::Display for InvariantSpeedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invariant speed must be finite and strictly positive")
    }
}

impl std::error::Error for InvariantSpeedError {}

impl InvariantSpeed {
    pub fn new(value: f64) -> Result<Self, InvariantSpeedError> {
        if value.is_finite() && value > 0.0 {
            Ok(Self(value))
        } else {
            Err(InvariantSpeedError)
        }
    }

    #[inline]
    pub fn get(self) -> f64 {
        self.0
    }

    #[inline]
    pub fn squared(self) -> f64 {
        self.0 * self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntervalKind {
    Timelike,
    Null,
    Spacelike,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProperTimeRate(f64);

impl ProperTimeRate {
    pub const ZERO: Self = Self(0.0);
    pub const ONE: Self = Self(1.0);

    #[inline]
    pub fn get(self) -> f64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClockRateResult {
    pub interval_kind: IntervalKind,
    pub beta_squared: f64,
    pub lorentz_factor: Option<f64>,
    pub proper_time_rate: ProperTimeRate,
}

/// Proper-time rate for a coordinate worldline in Minkowski spacetime.
///
/// `speed_squared` is the Euclidean spatial speed squared in one inertial
/// coordinate chart. Null and spacelike inputs are classified without taking
/// an invalid square root, so diagnostics never receive a NaN.
pub fn minkowski_clock_rate(
    speed_squared: f64,
    invariant_speed: InvariantSpeed,
) -> ClockRateResult {
    let beta_squared = speed_squared / invariant_speed.squared();
    if !speed_squared.is_finite() || speed_squared < 0.0 || beta_squared > 1.0 {
        return ClockRateResult {
            interval_kind: IntervalKind::Spacelike,
            beta_squared,
            lorentz_factor: None,
            proper_time_rate: ProperTimeRate::ZERO,
        };
    }
    if beta_squared == 1.0 {
        return ClockRateResult {
            interval_kind: IntervalKind::Null,
            beta_squared,
            lorentz_factor: None,
            proper_time_rate: ProperTimeRate::ZERO,
        };
    }
    let rate = (1.0 - beta_squared.max(0.0)).sqrt();
    ClockRateResult {
        interval_kind: IntervalKind::Timelike,
        beta_squared,
        lorentz_factor: Some(1.0 / rate),
        proper_time_rate: ProperTimeRate(rate),
    }
}

/// Rapidity corresponding to one signed collinear velocity.
///
/// ⚠ **`atanh`/`tanh` are libm, and libm is not bit-identical across
/// platforms.** Nothing in the simulation calls this today — the rollback path
/// runs through [`minkowski_clock_rate`], whose only transcendental is `sqrt`,
/// which IEEE-754 pins exactly. That is what makes proper time safe to snapshot
/// and rewind. If a future mechanic composes velocities inside the sim, this
/// becomes a determinism decision under ADR 0023 rather than a free function
/// call: two peers could disagree in the last bits and diverge.

pub fn rapidity_from_velocity(
    velocity: f64,
    invariant_speed: InvariantSpeed,
) -> Option<f64> {
    let beta = velocity / invariant_speed.get();
    (beta.is_finite() && beta.abs() < 1.0).then(|| beta.atanh())
}

/// Signed collinear velocity corresponding to a rapidity.
pub fn velocity_from_rapidity(rapidity: f64, invariant_speed: InvariantSpeed) -> f64 {
    invariant_speed.get() * rapidity.tanh()
}

/// Compose signed collinear velocities by adding rapidities.
pub fn compose_collinear_velocities(
    lhs: f64,
    rhs: f64,
    invariant_speed: InvariantSpeed,
) -> Option<f64> {
    let lhs = rapidity_from_velocity(lhs, invariant_speed)?;
    let rhs = rapidity_from_velocity(rhs, invariant_speed)?;
    Some(velocity_from_rapidity(lhs + rhs, invariant_speed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stationary_clock_matches_coordinate_time() {
        let c = InvariantSpeed::new(10.0).unwrap();
        assert_eq!(minkowski_clock_rate(0.0, c).proper_time_rate, ProperTimeRate::ONE);
    }

    #[test]
    fn point_nine_c_has_the_expected_rate() {
        let c = InvariantSpeed::new(10.0).unwrap();
        let result = minkowski_clock_rate(81.0, c);
        assert_eq!(result.interval_kind, IntervalKind::Timelike);
        assert!((result.proper_time_rate.get() - 0.435_889_894_354_067_3).abs() < 1.0e-14);
    }

    #[test]
    fn null_and_spacelike_inputs_are_nan_free() {
        let c = InvariantSpeed::new(10.0).unwrap();
        let null = minkowski_clock_rate(100.0, c);
        let spacelike = minkowski_clock_rate(121.0, c);
        assert_eq!(null.interval_kind, IntervalKind::Null);
        assert_eq!(spacelike.interval_kind, IntervalKind::Spacelike);
        assert_eq!(null.proper_time_rate, ProperTimeRate::ZERO);
        assert_eq!(spacelike.proper_time_rate, ProperTimeRate::ZERO);
    }

    #[test]
    fn impossible_negative_speed_square_is_rejected_without_nan() {
        let c = InvariantSpeed::new(10.0).unwrap();
        let result = minkowski_clock_rate(-1.0, c);
        assert_eq!(result.interval_kind, IntervalKind::Spacelike);
        assert_eq!(result.proper_time_rate, ProperTimeRate::ZERO);
        assert!(!result.proper_time_rate.get().is_nan());
    }

    #[test]
    fn rapidity_composition_stays_subluminal() {
        let c = InvariantSpeed::new(1.0).unwrap();
        let composed = compose_collinear_velocities(0.9, 0.9, c).unwrap();
        assert!(composed < 1.0);
        assert!((composed - (1.8 / 1.81)).abs() < 1.0e-12);
    }

    #[test]
    fn invariant_speed_is_validated() {
        assert!(InvariantSpeed::new(0.0).is_err());
        assert!(InvariantSpeed::new(f64::INFINITY).is_err());
        assert!(InvariantSpeed::new(f64::NAN).is_err());
    }
}
