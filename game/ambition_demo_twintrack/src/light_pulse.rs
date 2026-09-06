//! TwinTrack light-pulse instrument for two observers.
//!
//! Rays are derived null worldlines, not integrated projectiles:
//! `x = emission_position + c * (t - emission_time) * direction`. Observer-frame
//! values come from Lorentz transforms, demonstrating invariant measured speed,
//! aberration, Doppler shift, and observer-dependent arrival coordinates for the
//! same event. The values are derived from rollback-authoritative world/time state
//! and require no independent simulation state.

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::lifecycle::SessionRoot;
use ambition_platformer2d::relativity::{
    lorentz_boost_event, observe_photon_direction, InvariantSpeed, MinkowskiEvent,
};
use ambition_platformer2d::relativity2d::SpacetimeCoordinateTime2d;
use bevy::prelude::*;

use crate::dual_observer::{beacon_midpoint, beacon_omega_position, BEACON_HALF_SEPARATION};
use crate::{LaboratoryTwin, TravelerTwin, INVARIANT_SPEED};

/// Laboratory-frame period between flares.
pub const PULSE_PERIOD_SECONDS: f64 = 2.4;

/// How long after emission a pane still draws the moving front.
///
/// shorter than the period on purpose. At `INVARIANT_SPEED` a front
/// leaves the drawn pane after about this long, and a dot pinned to the pane
/// edge would read as a pulse that stopped.
pub const PULSE_VISIBLE_SECONDS: f64 = 1.7;

/// The flare's rest frequency, in the "THz" the panes label their colour with.
/// Chosen at green so a viewer can see the chased ray fall out of the visible
/// band into the infrared and the head-on ray climb past violet.
pub const PULSE_REST_FREQUENCY_THZ: f64 = 540.0;

pub const SPEED_INVARIANCE_TOLERANCE: f64 = 1.0e-6;

/// Below this the two panes' apparent angles are reported as agreeing.
pub const ABERRATION_EPSILON_DEGREES: f32 = 0.5;

/// One ray of the flare, named by where it goes in the laboratory.
///
/// named for the laboratory, not for the traveler. "Chased" and
/// "head-on" are facts about an observer, and the traveler may fly either way
/// along the axis; naming the ray by its own frame-independent laboratory
/// direction keeps the pane honest when the traveler turns around.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PulseRay {
    /// Straight down the beacon axis toward Alpha (`-x`).
    TowardAlpha,
    /// Square across the beacon axis (`+y`). This is the aberration ray: it is
    /// the only one whose direction is not degenerate under a boost along the
    /// axis.
    Crosswise,
    /// Straight down the beacon axis toward Omega (`+x`).
    TowardOmega,
}

impl PulseRay {
    pub const ALL: [Self; 3] = [Self::TowardAlpha, Self::Crosswise, Self::TowardOmega];

    /// Unit propagation direction in laboratory coordinates.
    pub fn lab_direction(self) -> Vec2 {
        match self {
            Self::TowardAlpha => Vec2::new(-1.0, 0.0),
            Self::Crosswise => Vec2::new(0.0, 1.0),
            Self::TowardOmega => Vec2::new(1.0, 0.0),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::TowardAlpha => "TO ALPHA",
            Self::Crosswise => "CROSSWISE",
            Self::TowardOmega => "TO OMEGA",
        }
    }
}

/// Where the flare is fired from: the lab twin's own rest position, which is
/// also the beacons' midpoint.
pub fn pulse_emission_position() -> Vec2 {
    beacon_midpoint()
}

/// Laboratory coordinate time of flare `index`.
pub fn pulse_emission_time(index: u64) -> f64 {
    index as f64 * PULSE_PERIOD_SECONDS
}

/// The most recent flare emitted at or before `coordinate_time`.
pub fn latest_pulse_index(coordinate_time: f64) -> Option<u64> {
    if !coordinate_time.is_finite() || coordinate_time < 0.0 {
        return None;
    }
    Some((coordinate_time / PULSE_PERIOD_SECONDS).floor() as u64)
}

/// Laboratory position of one ray's front.
///
/// this is the whole definition of the projectile. There is no stored
/// position and no integrator: the front is wherever the emission event plus
/// `c` times the elapsed coordinate time puts it. `None` before emission.
pub fn pulse_front_position(
    index: u64,
    ray: PulseRay,
    coordinate_time: f64,
    invariant_speed: InvariantSpeed,
) -> Option<Vec2> {
    let age = coordinate_time - pulse_emission_time(index);
    if !age.is_finite() || age < 0.0 {
        return None;
    }
    let travelled = (invariant_speed.get() * age) as f32;
    let front = pulse_emission_position() + ray.lab_direction() * travelled;
    front.is_finite().then_some(front)
}

/// One observer's measurement of one ray.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PulseRayMeasurement {
    pub ray: PulseRay,
    pub measured_speed: f64,
    /// `measured_speed / c`. Every observer gets 1.
    pub measured_speed_fraction: f64,
    /// Unit propagation direction in this observer's frame.
    pub apparent_direction: Vec2,
    /// [`Self::apparent_direction`]'s angle from `+x`, in degrees.
    pub apparent_angle_degrees: f32,
    /// Unsigned angle between the laboratory direction and this observer's —
    /// the aberration. Zero for an observer at rest.
    pub aberration_degrees: f32,
    /// `observed_frequency / emitted_frequency` for this observer. Below one is
    /// a redshift; above one is a blueshift.
    pub doppler_factor: f64,
    /// [`PULSE_REST_FREQUENCY_THZ`] scaled by the Doppler factor. The emitter is
    /// at rest in the laboratory, so its coordinate frequency *is* its rest
    /// frequency and this is one multiply.
    pub observed_frequency_thz: f64,
}

/// Everything one pane says about the current flare.
#[derive(Clone, Debug, PartialEq)]
pub struct PulseObserverReport {
    pub label: String,
    pub coordinate_time: f64,
    pub position: Vec2,
    pub velocity: Vec2,
    /// Speed as a fraction of the invariant speed.
    pub beta: f32,
    pub lorentz_factor: f32,
    pub pulse_index: u64,
    pub emission_coordinate_time: f64,
    /// Laboratory time since emission.
    pub age_seconds: f64,
    /// Laboratory radius of the flare's light cone right now, `c * age`.
    pub front_radius: f32,
    /// One measurement per [`PulseRay::ALL`], in that order.
    pub rays: [PulseRayMeasurement; 3],
    /// Laboratory time at which the toward-Omega ray reached the Omega beacon.
    /// Both observers agree this event happened and agree on this number,
    /// because it is a laboratory-chart fact.
    pub omega_arrival_coordinate_time: f64,
    /// The same arrival, timed on this observer's own clock relative to its
    /// own now. The two observers disagree about this and agree about whether
    /// it happened — which is what a light cone buys you.
    pub omega_arrival_frame_seconds: f64,
}

impl PulseObserverReport {
    /// an exhaustive match, not a search. `rays` is filled in
    /// [`PulseRay::ALL`]'s order; a new variant breaks this match at compile
    /// time, and `every_ray_lands_in_its_own_slot` pins the two orders together.
    pub fn ray(&self, ray: PulseRay) -> &PulseRayMeasurement {
        let index = match ray {
            PulseRay::TowardAlpha => 0,
            PulseRay::Crosswise => 1,
            PulseRay::TowardOmega => 2,
        };
        &self.rays[index]
    }

    /// Whether the pane should still draw a moving front for this flare.
    pub fn front_is_visible(&self) -> bool {
        (0.0..PULSE_VISIBLE_SECONDS).contains(&self.age_seconds)
    }
}

/// The light-pulse read model: what each of the two panes is showing.
///
/// not rollback state and deliberately not registered, for the same
/// reason as `TwinTrackDualObserverView`: every field is a pure function of
/// coordinate time and canonical kinematics.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct TwinTrackLightPulseView {
    pub coordinate_time: f64,
    pub laboratory: Option<PulseObserverReport>,
    pub traveler: Option<PulseObserverReport>,
}

impl TwinTrackLightPulseView {
    pub fn both(&self) -> Option<(&PulseObserverReport, &PulseObserverReport)> {
        Some((self.laboratory.as_ref()?, self.traveler.as_ref()?))
    }

    /// Both panes are reading the same flare. A comparison across two different
    /// emission events would not be a comparison at all.
    pub fn compares_the_same_pulse(&self) -> bool {
        self.both()
            .is_some_and(|(lab, traveler)| lab.pulse_index == traveler.pulse_index)
    }

    /// The postulate. Both observers measured every ray at the invariant
    /// speed, however fast they are moving relative to each other.
    pub fn speed_is_invariant_for_both(&self) -> bool {
        self.both().is_some_and(|(lab, traveler)| {
            [lab, traveler].into_iter().all(|report| {
                report.rays.iter().all(|measurement| {
                    (measurement.measured_speed_fraction - 1.0).abs() <= SPEED_INVARIANCE_TOLERANCE
                })
            })
        })
    }

    /// The two observers put the crosswise ray at different apparent angles.
    pub fn directions_disagree(&self) -> bool {
        self.both().is_some_and(|(lab, traveler)| {
            let lab_angle = lab.ray(PulseRay::Crosswise).apparent_angle_degrees;
            let traveler_angle = traveler.ray(PulseRay::Crosswise).apparent_angle_degrees;
            (lab_angle - traveler_angle).abs() > ABERRATION_EPSILON_DEGREES
        })
    }

    /// The two observers measure different colours for the same ray.
    pub fn doppler_factors_disagree(&self) -> bool {
        self.both().is_some_and(|(lab, traveler)| {
            PulseRay::ALL.iter().any(|ray| {
                (lab.ray(*ray).doppler_factor - traveler.ray(*ray).doppler_factor).abs() > 1.0e-3
            })
        })
    }

    /// Largest aberration either observer reports, for a pane caption.
    pub fn peak_aberration_degrees(&self) -> f32 {
        self.both().map_or(0.0, |(lab, traveler)| {
            [lab, traveler]
                .into_iter()
                .flat_map(|report| report.rays.iter())
                .map(|measurement| measurement.aberration_degrees)
                .fold(0.0, f32::max)
        })
    }
}

/// Measure one ray in one observer's frame.
///
/// The measurement takes a null displacement along the ray's worldline — one
/// laboratory second of coordinate time and the `c * 1s` of coordinate distance
/// that goes with it — boosts it into the observer's frame, and divides. The
/// separation's size cancels, so this is genuinely `dx'/dt'` and not a
/// normalization dressed up as one.
///
/// the naive answer is `c - v` and it is wrong. For an observer flying at
/// `0.9c` alongside the toward-Omega ray the boost returns `dt' = 0.229 s` and
/// `dx' = 0.229 c*s`, not `dt' = 1 s` and `dx' = 0.1 c*s`. The observer's own
/// clock and ruler shrink by exactly the amount that keeps the ratio at `c`.
pub fn measure_pulse_ray(
    ray: PulseRay,
    observer_velocity: Vec2,
    invariant_speed: InvariantSpeed,
) -> Option<PulseRayMeasurement> {
    if !observer_velocity.is_finite() {
        return None;
    }
    let c = invariant_speed.get();
    let lab_direction = ray.lab_direction();
    let frame_velocity = [
        f64::from(observer_velocity.x),
        f64::from(observer_velocity.y),
        0.0,
    ];

    const SEPARATION_SECONDS: f64 = 1.0;
    let boosted = lorentz_boost_event(
        MinkowskiEvent {
            coordinate_time: SEPARATION_SECONDS,
            position: [
                c * SEPARATION_SECONDS * f64::from(lab_direction.x),
                c * SEPARATION_SECONDS * f64::from(lab_direction.y),
                0.0,
            ],
        },
        frame_velocity,
        invariant_speed,
    )?;
    let travelled = boosted.position[0].hypot(boosted.position[1]);
    if !travelled.is_finite()
        || travelled <= 0.0
        || !boosted.coordinate_time.is_finite()
        || boosted.coordinate_time <= 0.0
    {
        return None;
    }
    let measured_speed = travelled / boosted.coordinate_time;
    let apparent_direction = Vec2::new(
        (boosted.position[0] / travelled) as f32,
        (boosted.position[1] / travelled) as f32,
    );
    if !apparent_direction.is_finite() {
        return None;
    }

    // the Doppler factor comes from the engine's photon law rather than from
    // this boost, so the pane's angle and its colour are two independent routes
    // to the same aberration. The test below asserts they agree; if they ever
    // stop agreeing, one of the two is wrong and the exhibit says so instead of
    // hiding it behind a shared helper.
    let observation = observe_photon_direction(
        [f64::from(lab_direction.x), f64::from(lab_direction.y), 0.0],
        frame_velocity,
        invariant_speed,
    )?;
    let observed_frequency_thz = PULSE_REST_FREQUENCY_THZ * observation.doppler_factor;
    if !observed_frequency_thz.is_finite() {
        return None;
    }

    Some(PulseRayMeasurement {
        ray,
        measured_speed,
        measured_speed_fraction: measured_speed / c,
        apparent_direction,
        apparent_angle_degrees: apparent_direction.to_angle().to_degrees(),
        aberration_degrees: lab_direction
            .angle_to(apparent_direction)
            .abs()
            .to_degrees(),
        doppler_factor: observation.doppler_factor,
        observed_frequency_thz,
    })
}

/// The engine's photon-aberration answer for the same ray, exposed so a test
/// can cross-check [`measure_pulse_ray`]'s boost-derived direction against a
/// different derivation instead of against itself.
pub fn aberrated_direction(
    ray: PulseRay,
    observer_velocity: Vec2,
    invariant_speed: InvariantSpeed,
) -> Option<Vec2> {
    let lab_direction = ray.lab_direction();
    let observation = observe_photon_direction(
        [f64::from(lab_direction.x), f64::from(lab_direction.y), 0.0],
        [
            f64::from(observer_velocity.x),
            f64::from(observer_velocity.y),
            0.0,
        ],
        invariant_speed,
    )?;
    let direction = Vec2::new(
        observation.propagation_direction[0] as f32,
        observation.propagation_direction[1] as f32,
    );
    direction.is_finite().then_some(direction)
}

/// The pulse event one observer calls "now", and where it puts it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PulseFrameSample {
    /// Laboratory time of the pulse event this observer calls simultaneous with
    /// its own current event. It is not the laboratory's now: that is the
    /// relativity of simultaneity acting on the pulse itself.
    pub coordinate_time: f64,
    /// Offset from the observer to that event, in the observer's own frame.
    pub offset: Vec2,
    /// The observer-frame time of that event relative to the observer's now.
    /// Zero by construction — carried so a test can falsify the solve instead
    /// of trusting the algebra that produced it.
    pub frame_time: f64,
}

/// Where an observer says a ray's front is at one instant of its own time.
///
/// this is not `observer_frame_offset`. That function length-contracts a
/// rod at rest in the laboratory. A pulse front is not at rest, so contracting
/// its laboratory-now position would answer a different question and be wrong by
/// exactly the term that carries the lesson. What happens here instead: solve
/// for the event on the *pulse's* null worldline whose boosted time equals the
/// observer's now, then boost that event.
pub fn pulse_frame_sample(
    index: u64,
    ray: PulseRay,
    coordinate_time: f64,
    observer_position: Vec2,
    observer_velocity: Vec2,
    invariant_speed: InvariantSpeed,
) -> Option<PulseFrameSample> {
    let front = pulse_front_position(index, ray, coordinate_time, invariant_speed)?;
    if !observer_position.is_finite() || !observer_velocity.is_finite() {
        return None;
    }
    let c = invariant_speed.get();
    let direction = ray.lab_direction();
    let separation = front - observer_position;
    let (dx, dy) = (f64::from(separation.x), f64::from(separation.y));
    let (nx, ny) = (f64::from(direction.x), f64::from(direction.y));
    let (vx, vy) = (
        f64::from(observer_velocity.x),
        f64::from(observer_velocity.y),
    );

    // t'(a) = gamma * [ a * (1 - v.n/c) - v.d / c^2 ] for an event `a` seconds
    // of laboratory time along the ray from its current front. Setting it to
    // zero is one divide because a null worldline is linear in `a`.
    let denominator = 1.0 - (vx * nx + vy * ny) / c;
    if !denominator.is_finite() || denominator <= 0.0 {
        return None;
    }
    let along = ((vx * dx + vy * dy) / invariant_speed.squared()) / denominator;
    if !along.is_finite() {
        return None;
    }
    let boosted = lorentz_boost_event(
        MinkowskiEvent {
            coordinate_time: along,
            position: [dx + c * along * nx, dy + c * along * ny, 0.0],
        },
        [vx, vy, 0.0],
        invariant_speed,
    )?;
    let offset = Vec2::new(boosted.position[0] as f32, boosted.position[1] as f32);
    offset.is_finite().then_some(PulseFrameSample {
        coordinate_time: coordinate_time + along,
        offset,
        frame_time: boosted.coordinate_time,
    })
}

/// Build one observer's report, or `None` before the first flare is emitted.
pub fn observe_light_pulse(
    label: &str,
    coordinate_time: f64,
    position: Vec2,
    velocity: Vec2,
    invariant_speed: InvariantSpeed,
) -> Option<PulseObserverReport> {
    if !coordinate_time.is_finite() || !position.is_finite() || !velocity.is_finite() {
        return None;
    }
    let c = invariant_speed.get();
    let beta = (f64::from(velocity.length()) / c) as f32;
    if !(0.0..1.0).contains(&beta) {
        return None;
    }
    let lorentz_factor = (1.0 - f64::from(beta) * f64::from(beta)).sqrt().recip() as f32;
    if !lorentz_factor.is_finite() {
        return None;
    }

    let index = latest_pulse_index(coordinate_time)?;
    let emission_coordinate_time = pulse_emission_time(index);
    let age_seconds = coordinate_time - emission_coordinate_time;

    let mut rays = [None; 3];
    for (slot, ray) in rays.iter_mut().zip(PulseRay::ALL) {
        *slot = measure_pulse_ray(ray, velocity, invariant_speed);
    }
    let [Some(toward_alpha), Some(crosswise), Some(toward_omega)] = rays else {
        return None;
    };

    // The toward-Omega ray meets a beacon sitting at rest a fixed laboratory
    // distance away: one light crossing time after emission, always.
    let omega_arrival_coordinate_time =
        emission_coordinate_time + f64::from(BEACON_HALF_SEPARATION) / c;
    let arrival_offset = beacon_omega_position() - position;
    let arrival = lorentz_boost_event(
        MinkowskiEvent {
            coordinate_time: omega_arrival_coordinate_time - coordinate_time,
            position: [
                f64::from(arrival_offset.x),
                f64::from(arrival_offset.y),
                0.0,
            ],
        },
        [f64::from(velocity.x), f64::from(velocity.y), 0.0],
        invariant_speed,
    )?;

    Some(PulseObserverReport {
        label: label.to_owned(),
        coordinate_time,
        position,
        velocity,
        beta,
        lorentz_factor,
        pulse_index: index,
        emission_coordinate_time,
        age_seconds,
        front_radius: (c * age_seconds) as f32,
        rays: [toward_alpha, crosswise, toward_omega],
        omega_arrival_coordinate_time,
        omega_arrival_frame_seconds: arrival.coordinate_time,
    })
}

pub(crate) fn publish_light_pulse_view(
    coordinate_time: Query<&SpacetimeCoordinateTime2d, With<SessionRoot>>,
    laboratory: Query<&ae::BodyKinematics, With<LaboratoryTwin>>,
    traveler: Query<&ae::BodyKinematics, With<TravelerTwin>>,
    mut view: ResMut<TwinTrackLightPulseView>,
) {
    let mut next = TwinTrackLightPulseView::default();
    let (Ok(coordinate_time), Ok(c)) = (
        coordinate_time.single(),
        InvariantSpeed::new(f64::from(INVARIANT_SPEED)),
    ) else {
        if *view != next {
            *view = next;
        }
        return;
    };
    next.coordinate_time = coordinate_time.seconds;
    if let Ok(body) = laboratory.single() {
        next.laboratory =
            observe_light_pulse("LAB TWIN", coordinate_time.seconds, body.pos, body.vel, c);
    }
    if let Ok(body) = traveler.single() {
        next.traveler =
            observe_light_pulse("TRAVELER", coordinate_time.seconds, body.pos, body.vel, c);
    }
    if *view != next {
        *view = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c() -> InvariantSpeed {
        InvariantSpeed::new(f64::from(INVARIANT_SPEED)).unwrap()
    }

    /// The demo's terminal speed, and the speed the exhibit is authored for.
    fn nine_tenths() -> Vec2 {
        Vec2::new(0.9 * INVARIANT_SPEED, 0.0)
    }

    /// Late enough that a flare has been emitted and is still in flight.
    const SETTLED: f64 = 24.6;

    #[test]
    fn the_front_position_is_derived_from_the_emission_event_alone() {
        let index = latest_pulse_index(SETTLED).unwrap();
        let emitted = pulse_emission_time(index);
        let age = SETTLED - emitted;
        // a zero floor: an age of zero would make every distance assertion
        // below pass by saying nothing.
        assert!(
            age > 0.25,
            "the sampled flare should be in flight, age {age}"
        );
        for ray in PulseRay::ALL {
            let front = pulse_front_position(index, ray, SETTLED, c()).unwrap();
            let travelled = f64::from(front.distance(pulse_emission_position()));
            assert!(
                (travelled - c().get() * age).abs() < 1.0e-2,
                "{ray:?} front travelled {travelled} but c*age is {}",
                c().get() * age,
            );
        }
        assert!(pulse_front_position(index, PulseRay::Crosswise, emitted - 0.5, c()).is_none());
    }

    #[test]
    fn every_ray_lands_in_its_own_slot() {
        // `rays` is built by zipping `PulseRay::ALL` and read back by an
        // exhaustive match on the variant. If those two orders ever drift, the
        // panes would silently label the head-on ray with the chased ray's
        // numbers, and every other test here would still pass.
        let report =
            observe_light_pulse("traveler", SETTLED, beacon_midpoint(), nine_tenths(), c())
                .unwrap();
        for ray in PulseRay::ALL {
            assert_eq!(report.ray(ray).ray, ray);
        }
        assert_eq!(report.rays.len(), PulseRay::ALL.len());
    }

    #[test]
    fn both_observers_measure_the_pulse_at_the_invariant_speed() {
        let lab = observe_light_pulse("lab", SETTLED, beacon_midpoint(), Vec2::ZERO, c()).unwrap();
        let traveler =
            observe_light_pulse("traveler", SETTLED, beacon_midpoint(), nine_tenths(), c())
                .unwrap();
        // The premise: these two are moving at 0.9c relative to each other.
        assert!(
            (f64::from(traveler.beta) - 0.9).abs() < 1.0e-4,
            "traveler beta was {}",
            traveler.beta,
        );
        assert!(lab.beta < 1.0e-6);

        for report in [&lab, &traveler] {
            for measurement in &report.rays {
                assert!(
                    (measurement.measured_speed_fraction - 1.0).abs() <= SPEED_INVARIANCE_TOLERANCE,
                    "{} measured {:?} at {} c",
                    report.label,
                    measurement.ray,
                    measurement.measured_speed_fraction,
                );
            }
        }

        // the falsifier. A fast projectile would give the traveler `c - v`
        // for the ray it chases; this asserts the exhibit is NOT that.
        let galilean = c().get() - f64::from(nine_tenths().x);
        let chased = traveler.ray(PulseRay::TowardOmega).measured_speed;
        assert!(
            (chased - galilean).abs() > 0.5 * c().get(),
            "the chased ray measured {chased}, which is suspiciously close to the \
             velocity-addition answer {galilean}",
        );
    }

    #[test]
    fn the_two_observers_disagree_about_the_crosswise_rays_direction() {
        let lab = observe_light_pulse("lab", SETTLED, beacon_midpoint(), Vec2::ZERO, c()).unwrap();
        let traveler =
            observe_light_pulse("traveler", SETTLED, beacon_midpoint(), nine_tenths(), c())
                .unwrap();

        // The observer at rest sees it leave at exactly the laboratory angle.
        let at_rest = lab.ray(PulseRay::Crosswise);
        assert!((at_rest.apparent_angle_degrees - 90.0).abs() < 1.0e-3);
        assert!(at_rest.aberration_degrees < 1.0e-3);

        // The traveler sees the same ray swept far around toward its own tail.
        let moving = traveler.ray(PulseRay::Crosswise);
        assert!(
            moving.aberration_degrees > 45.0,
            "the crosswise ray should aberrate hard at 0.9c, got {} deg",
            moving.aberration_degrees,
        );
        // sin(theta') = 1/gamma for a laboratory-transverse ray, so the swept
        // angle is the closed form and not a plausible-looking number.
        let expected = (1.0_f32 / traveler.lorentz_factor).asin().to_degrees();
        assert!(
            (moving.apparent_angle_degrees - (180.0 - expected)).abs() < 1.0e-2,
            "apparent angle {} deg should be 180 - asin(1/gamma) = {} deg",
            moving.apparent_angle_degrees,
            180.0 - expected,
        );

        // ...and the AXIAL rays do not aberrate at all, so this is a direction
        // effect rather than a global rotation of the traveler's pane.
        for ray in [PulseRay::TowardAlpha, PulseRay::TowardOmega] {
            assert!(
                traveler.ray(ray).aberration_degrees < 1.0e-3,
                "{ray:?} is collinear with the boost and must not aberrate",
            );
        }
    }

    #[test]
    fn the_chased_ray_is_redshifted_and_the_head_on_ray_is_blueshifted() {
        let traveler =
            observe_light_pulse("traveler", SETTLED, beacon_midpoint(), nine_tenths(), c())
                .unwrap();
        let chased = traveler.ray(PulseRay::TowardOmega).doppler_factor;
        let head_on = traveler.ray(PulseRay::TowardAlpha).doppler_factor;
        assert!(
            chased < 0.5,
            "the chased ray should be redshifted, got {chased}"
        );
        assert!(
            head_on > 2.0,
            "the head-on ray should be blueshifted, got {head_on}",
        );
        // gamma(1-beta) * gamma(1+beta) = 1, exactly, for any beta.
        assert!(
            (chased * head_on - 1.0).abs() < 1.0e-9,
            "the two axial Doppler factors must be reciprocals, got {chased} and {head_on}",
        );
        // The transverse ray's factor is gamma itself — the pure time-dilation
        // shift, with no line-of-sight component in it at all.
        let crosswise = traveler.ray(PulseRay::Crosswise).doppler_factor;
        assert!(
            (crosswise - f64::from(traveler.lorentz_factor)).abs() < 1.0e-6,
            "the crosswise Doppler factor {crosswise} should be gamma {}",
            traveler.lorentz_factor,
        );

        // The observer at rest measures the emitter's own colour on every ray.
        let lab = observe_light_pulse("lab", SETTLED, beacon_midpoint(), Vec2::ZERO, c()).unwrap();
        for measurement in &lab.rays {
            assert!((measurement.doppler_factor - 1.0).abs() < 1.0e-9);
            assert!((measurement.observed_frequency_thz - PULSE_REST_FREQUENCY_THZ).abs() < 1.0e-6,);
        }
    }

    #[test]
    fn the_apparent_direction_matches_the_engines_photon_aberration_law() {
        // Two derivations that share no code: a boost of the ray's null
        // displacement, and the engine's velocity-addition law for a null ray.
        for velocity in [
            Vec2::ZERO,
            nine_tenths(),
            -nine_tenths(),
            Vec2::new(0.0, -0.6 * INVARIANT_SPEED),
        ] {
            for ray in PulseRay::ALL {
                let measured = measure_pulse_ray(ray, velocity, c()).unwrap();
                let engine = aberrated_direction(ray, velocity, c()).unwrap();
                assert!(
                    measured.apparent_direction.distance(engine) < 1.0e-5,
                    "{ray:?} at {velocity:?}: boost said {:?}, the photon law said {engine:?}",
                    measured.apparent_direction,
                );
            }
        }
    }

    #[test]
    fn an_observers_own_now_puts_the_front_somewhere_the_laboratory_does_not() {
        let index = latest_pulse_index(SETTLED).unwrap();
        let ray = PulseRay::TowardOmega;
        let observer = beacon_midpoint() + Vec2::new(0.0, 240.0);

        let at_rest = pulse_frame_sample(index, ray, SETTLED, observer, Vec2::ZERO, c()).unwrap();
        let moving = pulse_frame_sample(index, ray, SETTLED, observer, nine_tenths(), c()).unwrap();

        // The solve is checked, not trusted: both samples must land on the
        // observer's own simultaneity slice.
        for sample in [at_rest, moving] {
            assert!(
                sample.frame_time.abs() < 1.0e-6,
                "the solved event was {} s off the observer's now",
                sample.frame_time,
            );
        }
        // An observer at rest calls the laboratory's now its own.
        assert!((at_rest.coordinate_time - SETTLED).abs() < 1.0e-9);
        // A moving one does not, and that is the relativity of simultaneity
        // reaching the pulse itself rather than a pair of beacons.
        assert!(
            (moving.coordinate_time - SETTLED).abs() > 1.0e-3,
            "a 0.9c observer should not call the laboratory's now its own",
        );
        assert!(at_rest.offset.distance(moving.offset) > 1.0);
    }

    #[test]
    fn both_observers_agree_the_ray_reached_omega_and_disagree_about_when() {
        let lab = observe_light_pulse("lab", SETTLED, beacon_midpoint(), Vec2::ZERO, c()).unwrap();
        let traveler =
            observe_light_pulse("traveler", SETTLED, beacon_midpoint(), nine_tenths(), c())
                .unwrap();

        // One arrival event, one laboratory time: they never disagree about
        // whether the light got there.
        assert_eq!(
            lab.omega_arrival_coordinate_time,
            traveler.omega_arrival_coordinate_time,
        );
        assert!(
            (lab.omega_arrival_coordinate_time
                - lab.emission_coordinate_time
                - f64::from(BEACON_HALF_SEPARATION) / c().get())
            .abs()
                < 1.0e-9,
        );
        assert!(lab.omega_arrival_frame_seconds.is_finite());
        assert!(traveler.omega_arrival_frame_seconds.is_finite());
        // ...but they put it at different times on their own clocks.
        assert!(
            (lab.omega_arrival_frame_seconds - traveler.omega_arrival_frame_seconds).abs() > 0.1,
            "the two observers timed the same arrival at {} s and {} s",
            lab.omega_arrival_frame_seconds,
            traveler.omega_arrival_frame_seconds,
        );
    }

    #[test]
    fn the_view_reports_invariance_and_disagreement_only_when_both_panes_exist() {
        let empty = TwinTrackLightPulseView::default();
        // a view with no observers must not report a green postulate.
        assert!(!empty.speed_is_invariant_for_both());
        assert!(!empty.directions_disagree());
        assert!(!empty.doppler_factors_disagree());
        assert!(!empty.compares_the_same_pulse());

        let view = TwinTrackLightPulseView {
            coordinate_time: SETTLED,
            laboratory: observe_light_pulse("lab", SETTLED, beacon_midpoint(), Vec2::ZERO, c()),
            traveler: observe_light_pulse(
                "traveler",
                SETTLED,
                beacon_midpoint(),
                nine_tenths(),
                c(),
            ),
        };
        assert!(view.compares_the_same_pulse());
        assert!(view.speed_is_invariant_for_both());
        assert!(view.directions_disagree());
        assert!(view.doppler_factors_disagree());
        assert!(view.peak_aberration_degrees() > 45.0);
    }

    #[test]
    fn no_pulse_exists_before_the_first_emission() {
        assert!(latest_pulse_index(-0.5).is_none());
        assert!(observe_light_pulse("lab", -0.5, beacon_midpoint(), Vec2::ZERO, c()).is_none());
        assert_eq!(latest_pulse_index(0.0), Some(0));
    }

    #[test]
    fn a_superluminal_observer_has_no_report_rather_than_a_nonsense_one() {
        let faster_than_light = Vec2::new(1.4 * INVARIANT_SPEED, 0.0);
        assert!(
            observe_light_pulse("bad", SETTLED, beacon_midpoint(), faster_than_light, c())
                .is_none()
        );
        assert!(measure_pulse_ray(PulseRay::Crosswise, faster_than_light, c()).is_none());
    }
}
