//! Two observers, one pair of events, two orderings.
//!
//! This is TwinTrack's relativity-of-simultaneity instrument. Two beacons sit
//! at rest in the laboratory frame, symmetric about the laboratory twin, and
//! flash together in laboratory coordinate time. Every reading below is a
//! pure function of that authored schedule and the canonical bodies, so it adds
//! no simulation authority, no stored state, and nothing that rollback has to
//! rewind.
//!
//! Each observer answers the same two questions about the same flash pair:
//!
//! 1. which flash's light reached me first — a light-delay fact, decided by
//!    the observer's distance from each beacon; and
//! 2. which flash happened first in my own frame — the relativistic fact,
//!    the flash pair's time separation after an exact Lorentz boost into the
//!    observer's instantaneous rest frame.
//!
//! The laboratory twin is at rest and equidistant, so it answers
//! `Simultaneous` to both — by construction, not by luck. A traveler moving
//! along the beacon axis answers with a definite order whose sign is the sign
//! of its velocity: fly toward Omega and Omega flashed first; turn around and
//! Alpha did. That disagreement is the exhibit.

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::lifecycle::SessionRoot;
use ambition_platformer2d::relativity::{lorentz_boost_event, InvariantSpeed, MinkowskiEvent};
use ambition_platformer2d::relativity2d::SpacetimeCoordinateTime2d;
use bevy::prelude::*;

use crate::{LaboratoryTwin, TravelerTwin, INVARIANT_SPEED, LAB_POS};

/// Half the laboratory-frame distance between the two beacons.
///
/// this length is the size of the lesson. The observer-frame time split
/// is `gamma * beta * 2 * HALF_SEPARATION / c`; shrink it and the two panes
/// stop visibly disagreeing at ordinary flight speeds.
pub const BEACON_HALF_SEPARATION: f32 = 520.0;

/// Laboratory-frame period between synchronized flashes.
pub const BEACON_FLASH_PERIOD_SECONDS: f64 = 3.0;

/// How long after a flash's light arrives the pane keeps its beacon lit.
pub const BEACON_FLASH_GLOW_SECONDS: f64 = 0.42;

/// Below this the two readings are reported as simultaneous rather than as a
/// microscopic ordering that no viewer could act on.
pub const SIMULTANEITY_EPSILON_SECONDS: f64 = 1.0e-6;

/// The blue beacon, on the `-x` side of the laboratory twin.
pub fn beacon_alpha_position() -> Vec2 {
    LAB_POS - Vec2::new(BEACON_HALF_SEPARATION, 0.0)
}

/// The amber beacon, on the `+x` side of the laboratory twin.
pub fn beacon_omega_position() -> Vec2 {
    LAB_POS + Vec2::new(BEACON_HALF_SEPARATION, 0.0)
}

/// The point both beacons are symmetric about; every pane is drawn around it.
pub fn beacon_midpoint() -> Vec2 {
    LAB_POS
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TwinTrackBeacon {
    Alpha,
    Omega,
}

impl TwinTrackBeacon {
    pub const ALL: [Self; 2] = [Self::Alpha, Self::Omega];

    pub fn position(self) -> Vec2 {
        match self {
            Self::Alpha => beacon_alpha_position(),
            Self::Omega => beacon_omega_position(),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Alpha => "ALPHA",
            Self::Omega => "OMEGA",
        }
    }
}

/// Which of the two flashes came first, for one observer and one question.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventOrdering {
    AlphaFirst,
    Simultaneous,
    OmegaFirst,
}

impl EventOrdering {
    /// `alpha` and `omega` are times on the same axis; earlier is first.
    fn from_times(alpha: f64, omega: f64) -> Self {
        let difference = omega - alpha;
        if !difference.is_finite() || difference.abs() <= SIMULTANEITY_EPSILON_SECONDS {
            Self::Simultaneous
        } else if difference > 0.0 {
            Self::AlphaFirst
        } else {
            Self::OmegaFirst
        }
    }

    pub fn caption(self) -> &'static str {
        match self {
            Self::AlphaFirst => "ALPHA FIRST",
            Self::Simultaneous => "SIMULTANEOUS",
            Self::OmegaFirst => "OMEGA FIRST",
        }
    }
}

/// One observer's reading of one beacon.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeaconReading {
    /// Laboratory-frame distance from the observer to the beacon.
    pub range: f32,
    /// The most recent flash of THIS beacon whose light has reached the
    /// observer. Drives the pane's lamp, not the ordering rows.
    pub lit_flash_index: u64,
    /// Laboratory time at which that flash's light reached the observer.
    pub lit_arrival_time: f64,
    /// Laboratory time at which the COMPARED flash's light reached the
    /// observer. This is the "light reached me" ordering axis.
    pub compared_arrival_time: f64,
    /// Negative: it lies in the observer's past. This is the "in my frame" ordering axis.
    pub compared_frame_time: f64,
    /// Where this observer measures the beacon to be, relative to the beacon
    /// midpoint, with its own length contraction applied.
    pub frame_offset: Vec2,
}

impl BeaconReading {
    /// Whether the pane should show this beacon's lamp lit right now.
    pub fn is_lit(&self, coordinate_time: f64) -> bool {
        let age = coordinate_time - self.lit_arrival_time;
        (0.0..BEACON_FLASH_GLOW_SECONDS).contains(&age)
    }
}

/// Everything one pane needs to say who it is watching and what that observer
/// concluded.
#[derive(Clone, Debug, PartialEq)]
pub struct ObserverOrderingReport {
    pub label: String,
    pub coordinate_time: f64,
    pub position: Vec2,
    pub velocity: Vec2,
    /// Speed as a fraction of the invariant speed.
    pub beta: f32,
    pub lorentz_factor: f32,
    /// The flash index both ordering rows are about.
    pub compared_flash_index: u64,
    /// Laboratory time at which that flash pair happened. Both beacons flash at
    /// this one coordinate time — they are simultaneous in the laboratory by
    /// construction, which is what makes a disagreement meaningful.
    pub compared_flash_coordinate_time: f64,
    pub alpha: BeaconReading,
    pub omega: BeaconReading,
    /// Order in which the compared flashes' light reached this observer.
    pub seen_order: EventOrdering,
    /// Order in which the compared flashes happened in this observer's frame.
    pub frame_order: EventOrdering,
    /// Where this observer measures ITSELF to be, relative to the beacon
    /// midpoint, in its own frame.
    pub frame_offset: Vec2,
}

impl ObserverOrderingReport {
    pub fn reading(&self, beacon: TwinTrackBeacon) -> &BeaconReading {
        match beacon {
            TwinTrackBeacon::Alpha => &self.alpha,
            TwinTrackBeacon::Omega => &self.omega,
        }
    }

    /// Signed separation of the two flashes on the "light reached me" axis;
    /// positive means Alpha's light arrived first.
    pub fn seen_split_seconds(&self) -> f64 {
        self.omega.compared_arrival_time - self.alpha.compared_arrival_time
    }

    /// Signed separation of the two flashes on the "in my frame" axis;
    /// positive means Alpha happened first.
    pub fn frame_split_seconds(&self) -> f64 {
        self.omega.compared_frame_time - self.alpha.compared_frame_time
    }
}

/// The dual-observer read model: what each of the two panes is showing.
///
/// not rollback state and deliberately not registered. Every field is
/// recomputed from `SpacetimeCoordinateTime2d` and canonical `BodyKinematics`
/// every frame; it stores no accumulator and no memo, so a rewound simulation
/// republishes the identical value on the next pass.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct TwinTrackDualObserverView {
    pub coordinate_time: f64,
    pub laboratory: Option<ObserverOrderingReport>,
    pub traveler: Option<ObserverOrderingReport>,
}

impl TwinTrackDualObserverView {
    pub fn both(&self) -> Option<(&ObserverOrderingReport, &ObserverOrderingReport)> {
        Some((self.laboratory.as_ref()?, self.traveler.as_ref()?))
    }

    /// The two observers disagree about which flash their light reached first.
    pub fn seen_orders_disagree(&self) -> bool {
        self.both()
            .is_some_and(|(lab, traveler)| lab.seen_order != traveler.seen_order)
    }

    /// The two observers disagree about which flash HAPPENED first. This is the
    /// relativity-of-simultaneity claim.
    pub fn frame_orders_disagree(&self) -> bool {
        self.both()
            .is_some_and(|(lab, traveler)| lab.frame_order != traveler.frame_order)
    }

    /// Both panes are reading the same flash pair. A comparison across two
    /// different flashes would not be a disagreement at all, so every ordering
    /// assertion should confirm this first.
    pub fn compares_the_same_flash_pair(&self) -> bool {
        self.both().is_some_and(|(lab, traveler)| {
            lab.compared_flash_index == traveler.compared_flash_index
        })
    }
}

/// Laboratory coordinate time of flash `index`.
pub fn flash_coordinate_time(index: u64) -> f64 {
    index as f64 * BEACON_FLASH_PERIOD_SECONDS
}

/// The most recent flash of a beacon `range` away whose light has reached an
/// observer at coordinate time `now`, or `None` before the first one arrives.
fn latest_arrived_flash(now: f64, range: f32, invariant_speed: f64) -> Option<u64> {
    let light_delay = f64::from(range) / invariant_speed;
    let elapsed = now - light_delay;
    if !elapsed.is_finite() || elapsed < 0.0 {
        return None;
    }
    Some((elapsed / BEACON_FLASH_PERIOD_SECONDS).floor() as u64)
}

/// One observer's own-frame picture of a laboratory-frame displacement.
///
/// this is length contraction, not a boosted event. Boosting the two
/// laboratory-simultaneous beacon events would give `gamma` times the
/// separation, because those two events are NOT simultaneous in the observer's
/// frame — that number is a pair of events, not a length. What a pane draws is
/// the separation the observer MEASURES at one instant of its own time, which
/// for a rod at rest in the laboratory is the laboratory length divided by
/// `gamma` along the direction of motion.
pub fn observer_frame_offset(delta: Vec2, velocity: Vec2, lorentz_factor: f32) -> Vec2 {
    let speed = velocity.length();
    if speed <= f32::EPSILON || !lorentz_factor.is_finite() || lorentz_factor <= 0.0 {
        return delta;
    }
    let direction = velocity / speed;
    let parallel = direction * delta.dot(direction);
    let transverse = delta - parallel;
    parallel / lorentz_factor + transverse
}

/// Build one observer's report, or `None` before both beacons have been seen.
pub fn observe_beacon_pair(
    label: &str,
    now: f64,
    position: Vec2,
    velocity: Vec2,
    invariant_speed: InvariantSpeed,
) -> Option<ObserverOrderingReport> {
    if !now.is_finite() || !position.is_finite() || !velocity.is_finite() {
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

    let alpha_range = position.distance(beacon_alpha_position());
    let omega_range = position.distance(beacon_omega_position());
    let alpha_lit = latest_arrived_flash(now, alpha_range, c)?;
    let omega_lit = latest_arrived_flash(now, omega_range, c)?;
    // the two beacons' newest ARRIVED flashes need not carry the same index
    // when the range difference straddles a period boundary. Ordering is only
    // meaningful about ONE pair of events, so both rows compare the newest
    // index both beacons have delivered.
    let compared = alpha_lit.min(omega_lit);
    let compared_time = flash_coordinate_time(compared);

    let reading = |range: f32, lit_index: u64, beacon_position: Vec2| -> Option<BeaconReading> {
        let compared_arrival_time = compared_time + f64::from(range) / c;
        // The boost is homogeneous, so feeding it the displacement from the
        // observer's current event gives that event's own-frame time measured
        // from the observer's now — small, signed, and directly comparable.
        let displacement = beacon_position - position;
        let boosted = lorentz_boost_event(
            MinkowskiEvent {
                coordinate_time: compared_time - now,
                position: [f64::from(displacement.x), f64::from(displacement.y), 0.0],
            },
            [f64::from(velocity.x), f64::from(velocity.y), 0.0],
            invariant_speed,
        )?;
        Some(BeaconReading {
            range,
            lit_flash_index: lit_index,
            lit_arrival_time: flash_coordinate_time(lit_index) + f64::from(range) / c,
            compared_arrival_time,
            compared_frame_time: boosted.coordinate_time,
            frame_offset: observer_frame_offset(
                beacon_position - beacon_midpoint(),
                velocity,
                lorentz_factor,
            ),
        })
    };

    let alpha = reading(alpha_range, alpha_lit, beacon_alpha_position())?;
    let omega = reading(omega_range, omega_lit, beacon_omega_position())?;

    Some(ObserverOrderingReport {
        label: label.to_owned(),
        coordinate_time: now,
        position,
        velocity,
        beta,
        lorentz_factor,
        compared_flash_index: compared,
        compared_flash_coordinate_time: compared_time,
        seen_order: EventOrdering::from_times(
            alpha.compared_arrival_time,
            omega.compared_arrival_time,
        ),
        frame_order: EventOrdering::from_times(alpha.compared_frame_time, omega.compared_frame_time),
        alpha,
        omega,
        frame_offset: observer_frame_offset(
            position - beacon_midpoint(),
            velocity,
            lorentz_factor,
        ),
    })
}

pub(crate) fn publish_dual_observer_view(
    coordinate_time: Query<&SpacetimeCoordinateTime2d, With<SessionRoot>>,
    laboratory: Query<&ae::BodyKinematics, With<LaboratoryTwin>>,
    traveler: Query<&ae::BodyKinematics, With<TravelerTwin>>,
    mut view: ResMut<TwinTrackDualObserverView>,
) {
    let mut next = TwinTrackDualObserverView::default();
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
        next.laboratory = observe_beacon_pair(
            "LAB TWIN",
            coordinate_time.seconds,
            body.pos,
            body.vel,
            c,
        );
    }
    if let Ok(body) = traveler.single() {
        next.traveler = observe_beacon_pair(
            "TRAVELER",
            coordinate_time.seconds,
            body.pos,
            body.vel,
            c,
        );
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

    /// A long-enough coordinate time that both beacons have delivered a flash
    /// to an observer anywhere near the plaza.
    const SETTLED: f64 = 40.0;

    #[test]
    fn an_equidistant_observer_at_rest_calls_the_flashes_simultaneous() {
        let report =
            observe_beacon_pair("lab", SETTLED, beacon_midpoint(), Vec2::ZERO, c()).unwrap();
        assert_eq!(report.seen_order, EventOrdering::Simultaneous);
        assert_eq!(report.frame_order, EventOrdering::Simultaneous);
    }

    #[test]
    fn a_moving_observer_orders_the_same_flash_pair_by_its_direction() {
        let toward_omega =
            observe_beacon_pair("t", SETTLED, beacon_midpoint(), Vec2::new(480.0, 0.0), c())
                .unwrap();
        let toward_alpha =
            observe_beacon_pair("t", SETTLED, beacon_midpoint(), Vec2::new(-480.0, 0.0), c())
                .unwrap();
        // Moving toward a flash puts it EARLIER in your own frame.
        assert_eq!(toward_omega.frame_order, EventOrdering::OmegaFirst);
        assert_eq!(toward_alpha.frame_order, EventOrdering::AlphaFirst);
        // ...while the light-delay answer is unchanged: the observer is still
        // equidistant, so this is not a repackaged distance effect.
        assert_eq!(toward_omega.seen_order, EventOrdering::Simultaneous);
        assert_eq!(toward_alpha.seen_order, EventOrdering::Simultaneous);
        assert_eq!(
            toward_omega.compared_flash_index,
            toward_alpha.compared_flash_index
        );
    }

    #[test]
    fn the_frame_split_matches_the_closed_form_relativity_of_simultaneity() {
        let velocity = Vec2::new(0.8 * INVARIANT_SPEED, 0.0);
        let report =
            observe_beacon_pair("t", SETTLED, beacon_midpoint(), velocity, c()).unwrap();
        let beta = f64::from(velocity.x) / c().get();
        let gamma = (1.0 - beta * beta).sqrt().recip();
        let separation = f64::from(2.0 * BEACON_HALF_SEPARATION);
        // dt' = gamma * (dt - v.dx/c^2), with dt = 0 in the laboratory frame.
        let expected = -gamma * beta * separation / c().get();
        assert!(
            (report.frame_split_seconds() - expected).abs() < 1.0e-6,
            "frame split {} should match {expected}",
            report.frame_split_seconds(),
        );
    }

    #[test]
    fn length_contraction_squeezes_the_beacon_axis_only_along_the_motion() {
        let gamma = 2.0;
        let along = observer_frame_offset(Vec2::new(100.0, 0.0), Vec2::new(300.0, 0.0), gamma);
        let across = observer_frame_offset(Vec2::new(0.0, 100.0), Vec2::new(300.0, 0.0), gamma);
        assert!((along.x - 50.0).abs() < 1.0e-4);
        assert!((across.y - 100.0).abs() < 1.0e-4);
    }

    #[test]
    fn no_report_exists_before_the_first_flash_has_had_time_to_arrive() {
        assert!(observe_beacon_pair("lab", 0.0, beacon_midpoint(), Vec2::ZERO, c()).is_none());
    }
}
