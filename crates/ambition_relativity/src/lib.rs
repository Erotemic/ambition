//! Dimension-independent special-relativity foundations.
//!
//! This crate deliberately owns no Bevy or platformer vocabulary.

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

/// One event in a Minkowski coordinate chart.
///
/// Position has three spatial coordinates even when a consumer uses only a
/// plane. Keeping the local SR kernel 3-vector-shaped prevents the 2D adapter
/// from becoming the engine's eventual physics model.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MinkowskiEvent {
    pub coordinate_time: f64,
    pub position: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MinkowskiInterval {
    /// Signed interval squared with the `(+---)` convention.
    pub squared: f64,
    pub kind: IntervalKind,
}

/// Classify one event displacement by its invariant Minkowski interval.
///
/// The input is interpreted as a displacement (`Δt`, `Δx`) even though it uses
/// the same coordinate carrier as an absolute event. The tolerance scales with
/// the temporal and spatial terms so an analytically null path remains null
/// under ordinary floating-point roundoff.
pub fn minkowski_interval(
    displacement: MinkowskiEvent,
    invariant_speed: InvariantSpeed,
) -> Option<MinkowskiInterval> {
    if !displacement.coordinate_time.is_finite() || !finite3(displacement.position) {
        return None;
    }
    let temporal =
        invariant_speed.squared() * displacement.coordinate_time * displacement.coordinate_time;
    let spatial = dot3(displacement.position, displacement.position);
    let squared = temporal - spatial;
    if !squared.is_finite() {
        return None;
    }
    let tolerance = 64.0 * f64::EPSILON * temporal.abs().max(spatial.abs()).max(1.0);
    let kind = if squared.abs() <= tolerance {
        IntervalKind::Null
    } else if squared > 0.0 {
        IntervalKind::Timelike
    } else {
        IntervalKind::Spacelike
    };
    Some(MinkowskiInterval { squared, kind })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DopplerMeasurement {
    /// Frequency in the selected Minkowski coordinate chart.
    pub coordinate_frequency: f64,
    pub observed_frequency: f64,
    /// `observed_frequency / emitted_proper_frequency`.
    pub total_factor: f64,
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
    if !speed_squared.is_finite() || speed_squared < 0.0 {
        return ClockRateResult {
            interval_kind: IntervalKind::Spacelike,
            beta_squared: f64::INFINITY,
            lorentz_factor: None,
            proper_time_rate: ProperTimeRate::ZERO,
        };
    }
    let beta_squared = speed_squared / invariant_speed.squared();
    if beta_squared > 1.0 {
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

/// Transform an event into an inertial frame moving with `frame_velocity`.
///
/// This is an exact Lorentz boost in a flat chart. It transforms coordinates;
/// it does not claim to be an optical view of the event.
pub fn lorentz_boost_event(
    event: MinkowskiEvent,
    frame_velocity: [f64; 3],
    invariant_speed: InvariantSpeed,
) -> Option<MinkowskiEvent> {
    if !event.coordinate_time.is_finite() || !finite3(event.position) || !finite3(frame_velocity) {
        return None;
    }
    let speed_squared = dot3(frame_velocity, frame_velocity);
    if speed_squared == 0.0 {
        return Some(event);
    }
    let rate = minkowski_clock_rate(speed_squared, invariant_speed);
    let gamma = rate.lorentz_factor?;
    let velocity_dot_position = dot3(frame_velocity, event.position);
    let time = gamma * (event.coordinate_time - velocity_dot_position / invariant_speed.squared());
    let parallel_scale =
        (gamma - 1.0) * velocity_dot_position / speed_squared - gamma * event.coordinate_time;
    let position = add3(event.position, scale3(frame_velocity, parallel_scale));
    (time.is_finite() && finite3(position)).then_some(MinkowskiEvent {
        coordinate_time: time,
        position,
    })
}

/// Coordinate-chart frequency of a null signal emitted with a source-local
/// proper frequency.
///
/// The direction points from emission toward propagation and is normalized by
/// this function. The relation is the local four-vector measurement
/// `frequency = -u·k`, expressed in one Minkowski chart.
pub fn coordinate_frequency_from_emitter(
    emitted_proper_frequency: f64,
    photon_direction: [f64; 3],
    emitter_velocity: [f64; 3],
    invariant_speed: InvariantSpeed,
) -> Option<f64> {
    if !emitted_proper_frequency.is_finite() || emitted_proper_frequency <= 0.0 {
        return None;
    }
    let direction = normalize3(photon_direction)?;
    let gamma = minkowski_clock_rate(dot3(emitter_velocity, emitter_velocity), invariant_speed)
        .lorentz_factor?;
    let denominator = gamma * (1.0 - dot3(direction, emitter_velocity) / invariant_speed.get());
    if !denominator.is_finite() || denominator <= 0.0 {
        return None;
    }
    let frequency = emitted_proper_frequency / denominator;
    (frequency.is_finite() && frequency > 0.0).then_some(frequency)
}

pub fn observed_frequency_from_coordinate(
    coordinate_frequency: f64,
    photon_direction: [f64; 3],
    receiver_velocity: [f64; 3],
    invariant_speed: InvariantSpeed,
) -> Option<f64> {
    if !coordinate_frequency.is_finite() || coordinate_frequency <= 0.0 {
        return None;
    }
    let direction = normalize3(photon_direction)?;
    let gamma = minkowski_clock_rate(dot3(receiver_velocity, receiver_velocity), invariant_speed)
        .lorentz_factor?;
    let frequency = coordinate_frequency
        * gamma
        * (1.0 - dot3(direction, receiver_velocity) / invariant_speed.get());
    (frequency.is_finite() && frequency > 0.0).then_some(frequency)
}

/// Complete emitter-to-receiver Doppler measurement in one Minkowski chart.
pub fn minkowski_doppler_measurement(
    emitted_proper_frequency: f64,
    photon_direction: [f64; 3],
    emitter_velocity: [f64; 3],
    receiver_velocity: [f64; 3],
    invariant_speed: InvariantSpeed,
) -> Option<DopplerMeasurement> {
    let coordinate_frequency = coordinate_frequency_from_emitter(
        emitted_proper_frequency,
        photon_direction,
        emitter_velocity,
        invariant_speed,
    )?;
    let observed_frequency = observed_frequency_from_coordinate(
        coordinate_frequency,
        photon_direction,
        receiver_velocity,
        invariant_speed,
    )?;
    Some(DopplerMeasurement {
        coordinate_frequency,
        observed_frequency,
        total_factor: observed_frequency / emitted_proper_frequency,
    })
}

/// Photon direction and spectral measurement in one observer's local inertial frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhotonObservation {
    /// Unit photon propagation direction in the observer frame.
    pub propagation_direction: [f64; 3],
    /// Direction from the observer toward the apparent source.
    pub apparent_source_direction: [f64; 3],
    /// `observed_frequency / coordinate_frequency` for this observer.
    pub doppler_factor: f64,
    /// A documented point-source beaming proxy, `doppler_factor^3`.
    pub beaming_factor: f64,
}

/// Transform a coordinate-chart photon direction into a moving observer's
/// local inertial frame.
///
/// This is the velocity-addition law evaluated for a null ray. The returned
/// vector is normalized and always represents photon propagation; the apparent
/// source direction is its negation.
pub fn observe_photon_direction(
    photon_direction: [f64; 3],
    observer_velocity: [f64; 3],
    invariant_speed: InvariantSpeed,
) -> Option<PhotonObservation> {
    let direction = normalize3(photon_direction)?;
    if !finite3(observer_velocity) {
        return None;
    }
    let speed_squared = dot3(observer_velocity, observer_velocity);
    if speed_squared == 0.0 {
        return Some(PhotonObservation {
            propagation_direction: direction,
            apparent_source_direction: scale3(direction, -1.0),
            doppler_factor: 1.0,
            beaming_factor: 1.0,
        });
    }
    let gamma = minkowski_clock_rate(speed_squared, invariant_speed).lorentz_factor?;
    let c = invariant_speed.get();
    let beta_dot_n = dot3(direction, observer_velocity) / c;
    let denominator = 1.0 - beta_dot_n;
    if !denominator.is_finite() || denominator <= 0.0 {
        return None;
    }

    let speed = speed_squared.sqrt();
    let boost_direction = scale3(observer_velocity, 1.0 / speed);
    let parallel_magnitude = dot3(direction, boost_direction);
    let parallel = scale3(boost_direction, parallel_magnitude);
    let transverse = add3(direction, scale3(parallel, -1.0));
    let transformed_parallel = scale3(
        boost_direction,
        (parallel_magnitude - speed / c) / denominator,
    );
    let transformed_transverse = scale3(transverse, 1.0 / (gamma * denominator));
    let propagation_direction = normalize3(add3(transformed_parallel, transformed_transverse))?;
    let doppler_factor = gamma * denominator;
    let beaming_factor = doppler_factor * doppler_factor * doppler_factor;
    (doppler_factor.is_finite() && doppler_factor > 0.0 && beaming_factor.is_finite()).then_some(
        PhotonObservation {
            propagation_direction,
            apparent_source_direction: scale3(propagation_direction, -1.0),
            doppler_factor,
            beaming_factor,
        },
    )
}

/// Transform a photon propagation direction from one observer's local inertial
/// frame back into the selected Minkowski coordinate chart.
///
/// This is the inverse of [`observe_photon_direction`] for the same observer
/// velocity. It lets controller-local optical aim become an authoritative chart
/// direction without treating the laboratory axes as the observer's own frame.
pub fn coordinate_photon_direction_from_observer(
    observer_local_direction: [f64; 3],
    observer_velocity: [f64; 3],
    invariant_speed: InvariantSpeed,
) -> Option<[f64; 3]> {
    observe_photon_direction(
        observer_local_direction,
        scale3(observer_velocity, -1.0),
        invariant_speed,
    )
    .map(|observation| observation.propagation_direction)
}

/// Convert a spatial proper velocity `w = gamma * v` into coordinate velocity.
///
/// This algebraic form uses only `sqrt`, stays strictly subluminal for every
/// finite input, and is therefore suitable for a future deterministic
/// relativistic movement authority without relying on `tanh`.
pub fn coordinate_velocity_from_proper_velocity(
    proper_velocity: [f64; 3],
    invariant_speed: InvariantSpeed,
) -> Option<[f64; 3]> {
    if !finite3(proper_velocity) {
        return None;
    }
    let denominator =
        (1.0 + dot3(proper_velocity, proper_velocity) / invariant_speed.squared()).sqrt();
    (denominator.is_finite() && denominator > 0.0)
        .then_some(scale3(proper_velocity, 1.0 / denominator))
}

/// Convert a timelike coordinate velocity into spatial proper velocity.
pub fn proper_velocity_from_coordinate_velocity(
    coordinate_velocity: [f64; 3],
    invariant_speed: InvariantSpeed,
) -> Option<[f64; 3]> {
    if !finite3(coordinate_velocity) {
        return None;
    }
    let gamma = minkowski_clock_rate(
        dot3(coordinate_velocity, coordinate_velocity),
        invariant_speed,
    )
    .lorentz_factor?;
    Some(scale3(coordinate_velocity, gamma))
}

/// Earliest future intersection between a null ray emitted now and a target
/// continuing at constant coordinate velocity in one Minkowski chart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NullIntercept {
    pub coordinate_time_to_intercept: f64,
    pub emission_direction: [f64; 3],
    pub intercept_position: [f64; 3],
}

/// Solve `|relative_position + target_velocity * t| = c * t`.
///
/// The emitter is the origin at coordinate time zero. The target is assumed to
/// continue at constant coordinate velocity. The earliest finite non-negative
/// root is returned; spacelike targets and degenerate coincident events return
/// `None` rather than producing an invalid direction.
pub fn solve_null_intercept_constant_velocity(
    relative_position: [f64; 3],
    target_velocity: [f64; 3],
    invariant_speed: InvariantSpeed,
) -> Option<NullIntercept> {
    if !finite3(relative_position) || !finite3(target_velocity) {
        return None;
    }
    let distance_squared = dot3(relative_position, relative_position);
    if !distance_squared.is_finite() || distance_squared <= 0.0 {
        return None;
    }
    let target_speed_squared = dot3(target_velocity, target_velocity);
    if target_speed_squared >= invariant_speed.squared() {
        return None;
    }

    let a = target_speed_squared - invariant_speed.squared();
    let b = 2.0 * dot3(relative_position, target_velocity);
    let discriminant = b * b - 4.0 * a * distance_squared;
    if !discriminant.is_finite() || discriminant < 0.0 {
        return None;
    }
    let root = discriminant.sqrt();
    let denominator = 2.0 * a;
    let mut time = f64::INFINITY;
    for candidate in [(-b - root) / denominator, (-b + root) / denominator] {
        if candidate.is_finite() && candidate > 0.0 && candidate < time {
            time = candidate;
        }
    }
    if !time.is_finite() {
        return None;
    }

    let intercept_position = add3(relative_position, scale3(target_velocity, time));
    let emission_direction = normalize3(intercept_position)?;
    let light_distance = invariant_speed.get() * time;
    let intercept_distance = dot3(intercept_position, intercept_position).sqrt();
    let tolerance = 128.0 * f64::EPSILON * light_distance.max(intercept_distance).max(1.0);
    if (light_distance - intercept_distance).abs() > tolerance {
        return None;
    }
    Some(NullIntercept {
        coordinate_time_to_intercept: time,
        emission_direction,
        intercept_position,
    })
}

/// Rapidity corresponding to one signed collinear velocity.
///
/// `atanh`/`tanh` are libm, and libm is not bit-identical across
/// platforms. Nothing in the simulation calls this today — the rollback path
/// runs through [`minkowski_clock_rate`], whose only transcendental is `sqrt`,
/// which IEEE-754 pins exactly. If a future mechanic composes velocities inside
/// the sim, this becomes a determinism decision under ADR 0023 rather than a
/// free function call.
pub fn rapidity_from_velocity(velocity: f64, invariant_speed: InvariantSpeed) -> Option<f64> {
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

fn finite3(value: [f64; 3]) -> bool {
    value.into_iter().all(f64::is_finite)
}

fn dot3(lhs: [f64; 3], rhs: [f64; 3]) -> f64 {
    lhs[0] * rhs[0] + lhs[1] * rhs[1] + lhs[2] * rhs[2]
}

fn add3(lhs: [f64; 3], rhs: [f64; 3]) -> [f64; 3] {
    [lhs[0] + rhs[0], lhs[1] + rhs[1], lhs[2] + rhs[2]]
}

fn scale3(value: [f64; 3], scale: f64) -> [f64; 3] {
    [value[0] * scale, value[1] * scale, value[2] * scale]
}

fn normalize3(value: [f64; 3]) -> Option<[f64; 3]> {
    if !finite3(value) {
        return None;
    }
    let length_squared = dot3(value, value);
    if !length_squared.is_finite() || length_squared <= 0.0 {
        return None;
    }
    let inverse_length = 1.0 / length_squared.sqrt();
    Some(scale3(value, inverse_length))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stationary_clock_matches_coordinate_time() {
        let c = InvariantSpeed::new(10.0).unwrap();
        assert_eq!(
            minkowski_clock_rate(0.0, c).proper_time_rate,
            ProperTimeRate::ONE
        );
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
    fn impossible_speed_squares_are_rejected_without_nan() {
        let c = InvariantSpeed::new(10.0).unwrap();
        for speed_squared in [-1.0, f64::INFINITY, f64::NAN] {
            let result = minkowski_clock_rate(speed_squared, c);
            assert_eq!(result.interval_kind, IntervalKind::Spacelike);
            assert_eq!(result.proper_time_rate, ProperTimeRate::ZERO);
            assert!(!result.beta_squared.is_nan());
            assert!(!result.proper_time_rate.get().is_nan());
        }
    }

    #[test]
    fn causal_interval_classifies_timelike_null_and_spacelike_displacements() {
        let c = InvariantSpeed::new(10.0).unwrap();
        let timelike = minkowski_interval(
            MinkowskiEvent {
                coordinate_time: 2.0,
                position: [10.0, 0.0, 0.0],
            },
            c,
        )
        .unwrap();
        let null = minkowski_interval(
            MinkowskiEvent {
                coordinate_time: 2.0,
                position: [20.0, 0.0, 0.0],
            },
            c,
        )
        .unwrap();
        let spacelike = minkowski_interval(
            MinkowskiEvent {
                coordinate_time: 2.0,
                position: [30.0, 0.0, 0.0],
            },
            c,
        )
        .unwrap();
        assert_eq!(timelike.kind, IntervalKind::Timelike);
        assert_eq!(null.kind, IntervalKind::Null);
        assert_eq!(spacelike.kind, IntervalKind::Spacelike);
    }

    #[test]
    fn lorentz_boost_preserves_the_minkowski_interval() {
        let c = InvariantSpeed::new(10.0).unwrap();
        let displacement = MinkowskiEvent {
            coordinate_time: 3.0,
            position: [12.0, 4.0, 0.0],
        };
        let boosted = lorentz_boost_event(displacement, [6.0, 0.0, 0.0], c).unwrap();
        let before = minkowski_interval(displacement, c).unwrap();
        let after = minkowski_interval(boosted, c).unwrap();
        assert_eq!(before.kind, after.kind);
        assert!((before.squared - after.squared).abs() < 1.0e-10);
    }

    #[test]
    fn observer_aberration_and_doppler_are_finite_and_directional() {
        let c = InvariantSpeed::new(1.0).unwrap();
        // Photon travels from a forward source toward an observer moving toward it.
        let observed = observe_photon_direction([-1.0, 0.0, 0.0], [0.8, 0.0, 0.0], c).unwrap();
        assert!(observed.doppler_factor > 1.0);
        assert!(observed.beaming_factor > observed.doppler_factor);
        assert!((observed.propagation_direction[0] + 1.0).abs() < 1.0e-12);
        assert!((observed.apparent_source_direction[0] - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn observer_local_photon_direction_round_trips_to_the_chart() {
        let c = InvariantSpeed::new(1.0).unwrap();
        let chart = normalize3([0.6, 0.8, 0.0]).unwrap();
        let velocity = [0.72, 0.0, 0.0];
        let local = observe_photon_direction(chart, velocity, c)
            .unwrap()
            .propagation_direction;
        let recovered = coordinate_photon_direction_from_observer(local, velocity, c).unwrap();
        for axis in 0..3 {
            assert!((chart[axis] - recovered[axis]).abs() < 1.0e-12);
        }
    }

    #[test]
    fn proper_velocity_round_trip_is_subluminal_and_reversible() {
        let c = InvariantSpeed::new(10.0).unwrap();
        let proper = [90.0, 12.0, 0.0];
        let coordinate = coordinate_velocity_from_proper_velocity(proper, c).unwrap();
        assert!(dot3(coordinate, coordinate) < c.squared());
        let recovered = proper_velocity_from_coordinate_velocity(coordinate, c).unwrap();
        for axis in 0..3 {
            assert!((proper[axis] - recovered[axis]).abs() < 1.0e-10);
        }
    }

    #[test]
    fn null_intercept_leads_a_moving_target() {
        let c = InvariantSpeed::new(10.0).unwrap();
        let solution =
            solve_null_intercept_constant_velocity([30.0, 0.0, 0.0], [0.0, 6.0, 0.0], c).unwrap();
        assert!(solution.coordinate_time_to_intercept > 3.0);
        assert!(solution.emission_direction[1] > 0.0);
        let light_distance = c.get() * solution.coordinate_time_to_intercept;
        let target_distance = dot3(solution.intercept_position, solution.intercept_position).sqrt();
        assert!((light_distance - target_distance).abs() < 1.0e-10);
    }

    #[test]
    fn null_intercept_rejects_spacelike_targets() {
        let c = InvariantSpeed::new(10.0).unwrap();
        assert!(
            solve_null_intercept_constant_velocity([30.0, 0.0, 0.0], [10.0, 0.0, 0.0], c,)
                .is_none()
        );
    }

    #[test]
    fn rapidity_composition_stays_subluminal() {
        let c = InvariantSpeed::new(1.0).unwrap();
        let composed = compose_collinear_velocities(0.9, 0.9, c).unwrap();
        assert!(composed < 1.0);
        assert!((composed - (1.8 / 1.81)).abs() < 1.0e-12);
    }

    #[test]
    fn boost_preserves_the_origin_event_and_transforms_a_distant_event() {
        let c = InvariantSpeed::new(10.0).unwrap();
        let origin = lorentz_boost_event(MinkowskiEvent::default(), [6.0, 0.0, 0.0], c).unwrap();
        assert_eq!(origin, MinkowskiEvent::default());

        let transformed = lorentz_boost_event(
            MinkowskiEvent {
                coordinate_time: 2.0,
                position: [10.0, 0.0, 0.0],
            },
            [6.0, 0.0, 0.0],
            c,
        )
        .unwrap();
        assert!((transformed.coordinate_time - 1.75).abs() < 1.0e-12);
        assert!((transformed.position[0] + 2.5).abs() < 1.0e-12);
    }

    #[test]
    fn forward_emission_at_point_nine_c_is_blue_shifted_in_the_lab() {
        let c = InvariantSpeed::new(10.0).unwrap();
        let measured = minkowski_doppler_measurement(
            100.0,
            [1.0, 0.0, 0.0],
            [9.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            c,
        )
        .unwrap();
        assert!((measured.observed_frequency - 435.889_894_354_067_3).abs() < 1.0e-10);
    }

    #[test]
    fn co_moving_emitter_and_receiver_measure_the_source_frequency() {
        let c = InvariantSpeed::new(10.0).unwrap();
        let measured = minkowski_doppler_measurement(
            100.0,
            [1.0, 0.0, 0.0],
            [9.0, 0.0, 0.0],
            [9.0, 0.0, 0.0],
            c,
        )
        .unwrap();
        assert!((measured.observed_frequency - 100.0).abs() < 1.0e-10);
    }

    #[test]
    fn invariant_speed_is_validated() {
        assert!(InvariantSpeed::new(0.0).is_err());
        assert!(InvariantSpeed::new(f64::INFINITY).is_err());
        assert!(InvariantSpeed::new(f64::NAN).is_err());
    }
}
