//! Observer-relative optical read models built from bounded worldlines.
//!
//! The authoritative simulation remains in one Minkowski coordinate chart.
//! This module asks a separate question for presentation and perception:
//! which source event lies on the current observer event's past light cone,
//! and what photon direction/frequency does the observer measure locally?

use std::collections::VecDeque;

use ambition_platformer2d_core::snapshot::{put_f32, put_str, put_u64, Reader, SnapshotState};
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::lifecycle::SessionRoot;
use ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith;
use ambition_relativity::{
    minkowski_doppler_measurement, observe_photon_direction, InvariantSpeed, MinkowskiEvent,
};
use bevy::ecs::schedule::InternedScheduleLabel;
use bevy::prelude::*;

use crate::telemetry::WorldlineTracked2d;
use crate::{
    ActiveSpacetime2d, ProperTimeElapsed, Relativity2dSet, SpacetimeCoordinateTime2d,
    WorldlineHistoryView2d, WorldlineSample2d,
};

/// One source whose past-light-cone image may be published to optical consumers.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct OpticalSource2d {
    pub label: String,
    pub rest_frequency: f64,
    pub rest_intensity: f32,
    pub apparent_radius: f32,
}

impl OpticalSource2d {
    pub fn new(
        label: impl Into<String>,
        rest_frequency: f64,
        rest_intensity: f32,
        apparent_radius: f32,
    ) -> Self {
        assert!(rest_frequency.is_finite() && rest_frequency > 0.0);
        assert!(rest_intensity.is_finite() && rest_intensity >= 0.0);
        assert!(apparent_radius.is_finite() && apparent_radius > 0.0);
        Self {
            label: label.into(),
            rest_frequency,
            rest_intensity,
            apparent_radius,
        }
    }
}

impl SnapshotState for OpticalSource2d {
    fn encode(&self, out: &mut Vec<u8>) {
        put_str(out, &self.label);
        put_u64(out, self.rest_frequency.to_bits());
        put_f32(out, self.rest_intensity);
        put_f32(out, self.apparent_radius);
    }

    fn decode(reader: &mut Reader<'_>) -> Option<Self> {
        let value = Self {
            label: reader.str()?.to_owned(),
            rest_frequency: f64::from_bits(reader.u64()?),
            rest_intensity: reader.f32()?,
            apparent_radius: reader.f32()?,
        };
        (value.rest_frequency.is_finite()
            && value.rest_frequency > 0.0
            && value.rest_intensity.is_finite()
            && value.rest_intensity >= 0.0
            && value.apparent_radius.is_finite()
            && value.apparent_radius > 0.0)
            .then_some(value)
    }
}

/// Mark a body as an optical observer without changing simulation authority.
///
///  any number of bodies may carry this, and each one gets its own image
/// in [`RelativisticOpticalView2d`]. The string is the authored label — it is
/// what the published rows are ORDERED by, so it is worth keeping unique, but
/// nothing keys on it: [`RelativisticOpticalView2d::for_observer`] takes the
/// entity.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct RelativisticObserver2d(pub String);

impl SnapshotState for RelativisticObserver2d {
    fn encode(&self, out: &mut Vec<u8>) {
        put_str(out, &self.0);
    }

    fn decode(reader: &mut Reader<'_>) -> Option<Self> {
        Some(Self(reader.str()?.to_owned()))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetardedSourceEvent2d {
    pub coordinate_time: f64,
    pub position: Vec2,
    pub coordinate_velocity: Vec2,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OpticalObserverObservation2d {
    pub entity: Entity,
    pub label: String,
    pub coordinate_time: f64,
    pub proper_time: Option<f64>,
    pub position: Vec2,
    pub coordinate_velocity: Vec2,
    pub invariant_speed: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OpticalSourceObservation2d {
    pub entity: Entity,
    pub label: String,
    pub emission_event: MinkowskiEvent,
    pub reception_event: MinkowskiEvent,
    pub retarded_position: Vec2,
    pub source_coordinate_velocity: Vec2,
    pub apparent_source_direction: Vec2,
    pub photon_propagation_direction: Vec2,
    pub apparent_range: f64,
    pub light_age: f64,
    pub rest_frequency: f64,
    pub observed_frequency: f64,
    pub doppler_factor: f64,
    pub beaming_factor: f64,
    pub apparent_radius: f32,
    pub rest_intensity: f32,
}

/// One observer's past-light-cone image of the world. Each observer owns an
/// independent view because observers may reconstruct different source events,
/// aberration, and Doppler shifts.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ObserverOpticalView2d {
    pub model_id: Option<&'static str>,
    ///  an `Option` even though a published row always has one: this is also
    /// the shape a consumer reads through [`RelativisticOpticalView2d`]'s
    /// `Deref`, where "there is no observer at all" must still have an answer.
    pub observer: Option<OpticalObserverObservation2d>,
    pub sources: Vec<OpticalSourceObservation2d>,
    pub history_start_time: Option<f64>,
    pub history_end_time: Option<f64>,
    pub missed_sources: usize,
}

/// What "the observer's view" is when no observer published one.
///
/// Byte-for-byte [`ObserverOpticalView2d::default`]; it exists as a `static`
/// because the `Deref` below must hand out a reference that outlives the call.
static NO_OBSERVER_OPTICAL_VIEW: ObserverOpticalView2d = ObserverOpticalView2d {
    model_id: None,
    observer: None,
    sources: Vec::new(),
    history_start_time: None,
    history_end_time: None,
    missed_sources: 0,
};

/// Presentation/perception optical state: one image per simulation observer.
/// Keyed by observer entity because this is rollback-derived simulation state;
/// mapping a presentation view to an observer belongs to the view layer.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct RelativisticOpticalView2d {
    /// Ordered deterministically by authored observer label, then entity bits.
    views: Vec<(Entity, ObserverOpticalView2d)>,
}

impl RelativisticOpticalView2d {
    pub fn is_empty(&self) -> bool {
        self.views.is_empty()
    }

    pub fn len(&self) -> usize {
        self.views.len()
    }

    /// Every observer's image, in the published deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (Entity, &ObserverOpticalView2d)> + '_ {
        self.views.iter().map(|(entity, view)| (*entity, view))
    }

    pub fn observers(&self) -> impl Iterator<Item = Entity> + '_ {
        self.views.iter().map(|(entity, _)| *entity)
    }

    /// The image belonging to one observer — the accessor a per-view consumer
    /// wants, because it names whose eyes it is drawing.
    pub fn for_observer(&self, observer: Entity) -> Option<&ObserverOpticalView2d> {
        self.views
            .iter()
            .find(|(entity, _)| *entity == observer)
            .map(|(_, view)| view)
    }

    /// The image belonging to the observer with this authored label.
    ///
    ///  a label is authored text, and two observers may carry the same one;
    /// this returns the first in published order. [`Self::for_observer`] is the
    /// unambiguous lookup.
    pub fn for_label(&self, label: &str) -> Option<&ObserverOpticalView2d> {
        self.views
            .iter()
            .find(|(_, view)| {
                view.observer
                    .as_ref()
                    .is_some_and(|observer| observer.label == label)
            })
            .map(|(_, view)| view)
    }

    /// The first observer's image in published order.
    ///
    ///  this is a single-observer answer, and it says so. It exists for
    /// consumers that genuinely have one observer (and for the `Deref` below);
    /// a consumer that must survive a second observer has to choose one, which
    /// is [`Self::for_observer`].
    pub fn primary(&self) -> Option<&ObserverOpticalView2d> {
        self.views.first().map(|(_, view)| view)
    }

    pub(crate) fn clear(&mut self) {
        self.views.clear();
    }

    pub(crate) fn push(&mut self, observer: Entity, view: ObserverOpticalView2d) {
        self.views.push((observer, view));
    }
}

/// Field access reads the FIRST observer's image.
///
///  this is what keeps the one-observer world byte-identical to the version of
/// this module that had exactly one image: `view.observer` and `view.sources`
/// mean today what they meant then, and with no observer at all they still read
/// `None` and empty. With two observers they read the first one's image instead
/// of blanking, which is the whole point — but a consumer that reaches through
/// this with two observers on the field is drawing one observer's sky for both,
/// and should be reading [`RelativisticOpticalView2d::for_observer`].
impl std::ops::Deref for RelativisticOpticalView2d {
    type Target = ObserverOpticalView2d;

    fn deref(&self) -> &Self::Target {
        self.primary().unwrap_or(&NO_OBSERVER_OPTICAL_VIEW)
    }
}

pub(crate) fn register_rollback_state(registrar: &mut impl ambition_platformer2d_core::snapshot::RollbackRegistrar) {
    registrar.declare_rollback_derived_resource::<RelativisticOpticalView2d>(
        "ambition_relativity2d",
        "relativity.optical_view_2d",
        "observer past-light-cone view rebuilt from canonical bodies and bounded worldline telemetry",
    );
    registrar.rollback_component_canonical::<OpticalSource2d>(
        "ambition_relativity2d",
        "relativity.optical_source_2d",
    )
    .rollback_component_canonical::<RelativisticObserver2d>(
        "ambition_relativity2d",
        "relativity.optical_observer_2d",
    );
}

pub(crate) fn install_optics_systems(app: &mut App, sim: InternedScheduleLabel) {
    app.init_resource::<RelativisticOpticalView2d>();
    app.add_systems(
        sim,
        publish_optical_view
            .run_if(crate::spacetime_is_active)
            .in_set(Relativity2dSet::PublishOptics)
            .in_set(Platformer2dSimulationPhaseMonolith::FeatureViewSync)
            .after(Relativity2dSet::PublishView),
    )
    .add_systems(Update, clear_optical_view_without_live_spacetime);
}

fn publish_optical_view(
    spacetime: Query<&ActiveSpacetime2d, With<SessionRoot>>,
    coordinate_time: Query<&SpacetimeCoordinateTime2d, With<SessionRoot>>,
    observers: Query<(
        Entity,
        &RelativisticObserver2d,
        &BodyKinematics,
        Option<&ProperTimeElapsed>,
    )>,
    // Join optics to worldline history through the tracked component on the
    // same entity; a source without worldline tracking has no reconstructable
    // history and does not enter this query.
    sources: Query<(Entity, &OpticalSource2d, &WorldlineTracked2d)>,
    worldlines: Res<WorldlineHistoryView2d>,
    mut views: ResMut<RelativisticOpticalView2d>,
) {
    views.clear();
    let (Ok(spacetime), Ok(coordinate_time)) = (spacetime.single(), coordinate_time.single())
    else {
        return;
    };
    let model = spacetime.model();
    let Some(invariant_speed) = model.minkowski_optics_invariant_speed() else {
        return;
    };
    let reception_time = coordinate_time.seconds;

    let mut source_rows: Vec<_> = sources.iter().collect();
    source_rows.sort_by(|(_, lhs, _), (_, rhs, _)| lhs.label.cmp(&rhs.label));

    //  the published order is sorted, never Bevy query order. Two peers
    // spawn the same observers in the same authored order but need not hand
    // them to this query in the same order, and a rewind need not either; a
    // consumer that draws `views.iter()` in sequence would then differ between
    // peers. The authored label is the stable key; entity bits only break a
    // duplicate-label tie, which is an authoring ambiguity that affects the
    // ORDER of two rows and never the contents of either.
    let mut observer_rows: Vec<_> = observers.iter().collect();
    observer_rows.sort_by(|(left_entity, left, ..), (right_entity, right, ..)| {
        left.0
            .cmp(&right.0)
            .then_with(|| left_entity.to_bits().cmp(&right_entity.to_bits()))
    });

    for (observer_entity, observer, body, proper_time) in observer_rows {
        let view = observe_from(
            model.model_id(),
            invariant_speed,
            reception_time,
            (observer_entity, observer, body, proper_time),
            &source_rows,
            &worldlines,
        );
        views.push(observer_entity, view);
    }
}

/// Build one observer's image of every optical source.
///
///  the per-observer math was already per-observer. Every quantity below
/// is a function of THIS observer's event and velocity; nothing here reads a
/// "the observer" singleton. Lifting it out of the system is what lets N
/// observers each get their own image — and lets a test call it twice.
fn observe_from(
    model_id: &'static str,
    invariant_speed: InvariantSpeed,
    reception_time: f64,
    (observer_entity, observer, body, proper_time): (
        Entity,
        &RelativisticObserver2d,
        &BodyKinematics,
        Option<&ProperTimeElapsed>,
    ),
    source_rows: &[(Entity, &OpticalSource2d, &WorldlineTracked2d)],
    worldlines: &WorldlineHistoryView2d,
) -> ObserverOpticalView2d {
    let mut view = ObserverOpticalView2d {
        model_id: Some(model_id),
        observer: Some(OpticalObserverObservation2d {
            entity: observer_entity,
            label: observer.0.clone(),
            coordinate_time: reception_time,
            proper_time: proper_time.map(|clock| clock.seconds),
            position: body.pos,
            coordinate_velocity: body.vel,
            invariant_speed: invariant_speed.get(),
        }),
        ..Default::default()
    };

    for (entity, source, tracked) in source_rows.iter().copied() {
        let Some(samples) = worldlines.tracks.get(&tracked.track) else {
            view.missed_sources += 1;
            continue;
        };
        if let Some(front) = samples.front() {
            view.history_start_time = Some(
                view.history_start_time
                    .map_or(front.coordinate_time, |time| {
                        time.min(front.coordinate_time)
                    }),
            );
        }
        if let Some(back) = samples.back() {
            view.history_end_time = Some(
                view.history_end_time
                    .map_or(back.coordinate_time, |time| time.max(back.coordinate_time)),
            );
        }
        let Some(retarded) =
            solve_retarded_source_event(samples, body.pos, reception_time, invariant_speed)
        else {
            view.missed_sources += 1;
            continue;
        };
        let photon_delta = body.pos - retarded.position;
        let photon_direction = photon_delta.normalize_or_zero();
        if photon_direction == Vec2::ZERO {
            continue;
        }
        let photon3 = [
            f64::from(photon_direction.x),
            f64::from(photon_direction.y),
            0.0,
        ];
        let observer3 = [f64::from(body.vel.x), f64::from(body.vel.y), 0.0];
        let source3 = [
            f64::from(retarded.coordinate_velocity.x),
            f64::from(retarded.coordinate_velocity.y),
            0.0,
        ];
        let Some(local) = observe_photon_direction(photon3, observer3, invariant_speed) else {
            continue;
        };
        let Some(doppler) = minkowski_doppler_measurement(
            source.rest_frequency,
            photon3,
            source3,
            observer3,
            invariant_speed,
        ) else {
            continue;
        };
        let light_age = (reception_time - retarded.coordinate_time).max(0.0);
        view.sources.push(OpticalSourceObservation2d {
            entity,
            label: source.label.clone(),
            emission_event: MinkowskiEvent {
                coordinate_time: retarded.coordinate_time,
                position: [
                    f64::from(retarded.position.x),
                    f64::from(retarded.position.y),
                    0.0,
                ],
            },
            reception_event: MinkowskiEvent {
                coordinate_time: reception_time,
                position: [f64::from(body.pos.x), f64::from(body.pos.y), 0.0],
            },
            retarded_position: retarded.position,
            source_coordinate_velocity: retarded.coordinate_velocity,
            apparent_source_direction: Vec2::new(
                local.apparent_source_direction[0] as f32,
                local.apparent_source_direction[1] as f32,
            ),
            photon_propagation_direction: Vec2::new(
                local.propagation_direction[0] as f32,
                local.propagation_direction[1] as f32,
            ),
            apparent_range: invariant_speed.get() * light_age,
            light_age,
            rest_frequency: source.rest_frequency,
            observed_frequency: doppler.observed_frequency,
            doppler_factor: doppler.total_factor,
            beaming_factor: doppler.total_factor * doppler.total_factor * doppler.total_factor,
            apparent_radius: source.apparent_radius,
            rest_intensity: source.rest_intensity,
        });
    }
    view
}

fn clear_optical_view_without_live_spacetime(
    spacetime: Query<(), (With<ActiveSpacetime2d>, With<SessionRoot>)>,
    mut views: ResMut<RelativisticOpticalView2d>,
) {
    // the emptiness test guards CHANGE DETECTION, not correctness: taking `ResMut` mutably every
    // frame with no spacetime would mark a derived resource changed forever.
    if spacetime.is_empty() && !views.is_empty() {
        views.clear();
    }
}

/// Solve the intersection of one sampled source worldline with an observer
/// event's past light cone.
///
/// Source motion is linearly interpolated within each sampled segment.
pub fn solve_retarded_source_event(
    samples: &VecDeque<WorldlineSample2d>,
    observer_position: Vec2,
    reception_time: f64,
    invariant_speed: InvariantSpeed,
) -> Option<RetardedSourceEvent2d> {
    if samples.is_empty() || !observer_position.is_finite() || !reception_time.is_finite() {
        return None;
    }
    if let Some(latest) = samples.back() {
        if (latest.coordinate_time - reception_time).abs() <= f64::EPSILON
            && latest.position.distance_squared(observer_position) <= f32::EPSILON
        {
            return Some(RetardedSourceEvent2d {
                coordinate_time: reception_time,
                position: latest.position,
                coordinate_velocity: latest.coordinate_velocity,
            });
        }
    }

    for newer_index in (1..samples.len()).rev() {
        let older = samples.get(newer_index - 1)?;
        let newer = samples.get(newer_index)?;
        if older.coordinate_time > reception_time {
            continue;
        }
        let older_residual = light_cone_residual(
            older.coordinate_time,
            older.position,
            observer_position,
            reception_time,
            invariant_speed,
        );
        let newer_residual = light_cone_residual(
            newer.coordinate_time.min(reception_time),
            interpolate_position(older, newer, newer.coordinate_time.min(reception_time)),
            observer_position,
            reception_time,
            invariant_speed,
        );
        if older_residual < 0.0 || newer_residual > 0.0 {
            continue;
        }

        let mut low = older.coordinate_time;
        let mut high = newer.coordinate_time.min(reception_time);
        for _ in 0..12 {
            let middle = 0.5 * (low + high);
            let position = interpolate_position(older, newer, middle);
            let residual = light_cone_residual(
                middle,
                position,
                observer_position,
                reception_time,
                invariant_speed,
            );
            if residual >= 0.0 {
                low = middle;
            } else {
                high = middle;
            }
        }
        let coordinate_time = 0.5 * (low + high);
        return Some(RetardedSourceEvent2d {
            coordinate_time,
            position: interpolate_position(older, newer, coordinate_time),
            coordinate_velocity: interpolate_velocity(older, newer, coordinate_time),
        });
    }
    None
}

fn light_cone_residual(
    emission_time: f64,
    source_position: Vec2,
    observer_position: Vec2,
    reception_time: f64,
    invariant_speed: InvariantSpeed,
) -> f64 {
    invariant_speed.get() * (reception_time - emission_time)
        - f64::from(source_position.distance(observer_position))
}

fn interpolation_fraction(
    older: &WorldlineSample2d,
    newer: &WorldlineSample2d,
    coordinate_time: f64,
) -> f32 {
    let duration = newer.coordinate_time - older.coordinate_time;
    if duration <= f64::EPSILON {
        0.0
    } else {
        ((coordinate_time - older.coordinate_time) / duration).clamp(0.0, 1.0) as f32
    }
}

fn interpolate_position(
    older: &WorldlineSample2d,
    newer: &WorldlineSample2d,
    coordinate_time: f64,
) -> Vec2 {
    older.position.lerp(
        newer.position,
        interpolation_fraction(older, newer, coordinate_time),
    )
}

fn interpolate_velocity(
    older: &WorldlineSample2d,
    newer: &WorldlineSample2d,
    coordinate_time: f64,
) -> Vec2 {
    older.coordinate_velocity.lerp(
        newer.coordinate_velocity,
        interpolation_fraction(older, newer, coordinate_time),
    )
}

#[cfg(test)]
mod tests {
    use ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId;

    use super::*;
    use crate::telemetry::WorldlineTrackId;

    fn sample(time: f64, x: f32, velocity: f32) -> WorldlineSample2d {
        WorldlineSample2d {
            sim_tick: (time * 60.0) as u64,
            coordinate_time: time,
            proper_time: Some(time),
            position: Vec2::new(x, 0.0),
            coordinate_velocity: Vec2::new(velocity, 0.0),
        }
    }

    #[test]
    fn static_source_is_seen_one_light_travel_time_in_the_past() {
        let c = InvariantSpeed::new(10.0).unwrap();
        let samples = VecDeque::from([
            sample(0.0, 100.0, 0.0),
            sample(5.0, 100.0, 0.0),
            sample(10.0, 100.0, 0.0),
            sample(15.0, 100.0, 0.0),
            sample(20.0, 100.0, 0.0),
        ]);
        let event = solve_retarded_source_event(&samples, Vec2::ZERO, 20.0, c).unwrap();
        assert!((event.coordinate_time - 10.0).abs() < 5.0e-3);
        assert!((event.position.x - 100.0).abs() < 1.0e-4);
    }

    #[test]
    fn approaching_source_is_observed_at_a_consistent_null_event() {
        let c = InvariantSpeed::new(10.0).unwrap();
        let samples = VecDeque::from([
            sample(0.0, 100.0, -2.0),
            sample(5.0, 90.0, -2.0),
            sample(10.0, 80.0, -2.0),
            sample(15.0, 70.0, -2.0),
            sample(20.0, 60.0, -2.0),
        ]);
        let event = solve_retarded_source_event(&samples, Vec2::ZERO, 20.0, c).unwrap();
        let light_distance = c.get() * (20.0 - event.coordinate_time);
        assert!((light_distance - f64::from(event.position.length())).abs() < 2.0e-2);
    }

    /// The smallest world that can tell a per-observer image from a shared one.
    ///
    ///  a FIXTURE, not a claim about how many observers a session should have.
    /// One beacon drifting from x=100 toward x=60 at 2 units/s, an invariant
    /// speed of 10, and reception at t=20 — chosen so the two observers' answers
    /// are separable by hand:
    ///
    /// - an observer at the origin: `10·(20−t) = 100−2t`  emission at t=12.5,
    ///   x=75, light age 7.5;
    /// - an observer at x=160: `10·(20−t) = 60+2t`  emission at t≈11.667,
    ///   x≈76.667, light age ≈8.333.
    ///
    /// The beacon is on OPPOSITE sides of the two, so even the sign of the
    /// apparent direction disagrees.
    const OBSERVER_ALPHA: (&str, Vec2, Vec2) = ("alpha", Vec2::new(0.0, 0.0), Vec2::new(5.0, 0.0));
    const OBSERVER_BETA: (&str, Vec2, Vec2) = ("beta", Vec2::new(160.0, 0.0), Vec2::new(0.0, 6.0));

    fn beacon_history() -> WorldlineHistoryView2d {
        let mut history = WorldlineHistoryView2d::default();
        history.tracks.insert(
            WorldlineTrackId("beacon".to_owned()),
            VecDeque::from([
                sample(0.0, 100.0, -2.0),
                sample(5.0, 90.0, -2.0),
                sample(10.0, 80.0, -2.0),
                sample(15.0, 70.0, -2.0),
                sample(20.0, 60.0, -2.0),
            ]),
        );
        history
    }

    /// Spawn one session, one beacon, and the given observers IN THE GIVEN
    /// ORDER, then publish once.
    fn publish_with(observers: &[(&str, Vec2, Vec2)]) -> (App, Vec<Entity>) {
        let mut app = App::new();
        app.init_resource::<RelativisticOpticalView2d>();
        app.insert_resource(beacon_history());
        app.add_systems(Update, publish_optical_view);
        app.world_mut().spawn((
            SessionRoot(SessionScopeId(0)),
            ActiveSpacetime2d::minkowski(10.0).expect("10 units/s is a valid invariant speed"),
            SpacetimeCoordinateTime2d {
                seconds: 20.0,
                epoch: 0,
            },
        ));
        app.world_mut().spawn((
            OpticalSource2d::new("Beacon", 100.0, 1.0, 4.0),
            WorldlineTracked2d::new("beacon").with_label("Beacon"),
        ));
        let mut spawned = Vec::new();
        for (label, position, velocity) in observers {
            spawned.push(
                app.world_mut()
                    .spawn((
                        RelativisticObserver2d((*label).to_owned()),
                        BodyKinematics {
                            pos: *position,
                            vel: *velocity,
                            ..Default::default()
                        },
                    ))
                    .id(),
            );
        }
        app.update();
        (app, spawned)
    }

    fn beacon_image(view: &ObserverOpticalView2d) -> &OpticalSourceObservation2d {
        view.sources
            .iter()
            .find(|source| source.label == "Beacon")
            .expect("the beacon intersects both observers' retained past light cones")
    }

    /// Every number an observer's own light cone decides, as one comparable value.
    fn beacon_numbers(view: &ObserverOpticalView2d) -> (f64, f32, f64, f64, f64) {
        let beacon = beacon_image(view);
        (
            beacon.emission_event.coordinate_time,
            beacon.retarded_position.x,
            beacon.light_age,
            beacon.apparent_range,
            beacon.observed_frequency,
        )
    }

    #[test]
    fn two_observers_publish_two_different_retarded_images() {
        let (app, spawned) = publish_with(&[OBSERVER_ALPHA, OBSERVER_BETA]);
        let views = app.world().resource::<RelativisticOpticalView2d>();
        assert_eq!(views.len(), 2, "two observers, two images");

        let alpha = views
            .for_observer(spawned[0])
            .expect("alpha has its own image");
        let beta = views
            .for_observer(spawned[1])
            .expect("beta has its own image");

        // The retarded EVENT differs: the two observers are not reading the
        // same photon, because they are not at the same place.
        let alpha_beacon = beacon_image(alpha);
        let beta_beacon = beacon_image(beta);
        assert!(
            (alpha_beacon.emission_event.coordinate_time - 12.5).abs() < 5.0e-3,
            "alpha at the origin sees the beacon as it was at t=12.5, not {}",
            alpha_beacon.emission_event.coordinate_time,
        );
        assert!((alpha_beacon.retarded_position.x - 75.0).abs() < 5.0e-3);
        assert!((alpha_beacon.light_age - 7.5).abs() < 5.0e-3);
        assert!(
            (beta_beacon.emission_event.coordinate_time - 35.0 / 3.0).abs() < 5.0e-3,
            "beta at x=160 sees an EARLIER beacon event, not {}",
            beta_beacon.emission_event.coordinate_time,
        );
        assert!((beta_beacon.retarded_position.x - 230.0 / 3.0).abs() < 5.0e-3);
        assert!((beta_beacon.light_age - 25.0 / 3.0).abs() < 5.0e-3);
        assert_ne!(
            alpha_beacon.emission_event, beta_beacon.emission_event,
            "the two images must not be the same image published twice",
        );

        // The ABERRATION differs: the beacon is on opposite sides, and only
        // beta's velocity has a component across its own line of sight.
        assert!(
            alpha_beacon.apparent_source_direction.x > 0.0
                && beta_beacon.apparent_source_direction.x < 0.0,
            "the beacon is ahead of alpha and behind beta",
        );
        assert!(
            alpha_beacon.apparent_source_direction.y.abs() < 1.0e-3,
            "alpha moves along its own line of sight, so nothing can tilt it: {}",
            alpha_beacon.apparent_source_direction.y,
        );
        assert!(
            beta_beacon.apparent_source_direction.y.abs() > 0.05,
            "beta moves across its line of sight at 0.6c and must see the beacon \
             displaced, not at {}",
            beta_beacon.apparent_source_direction.y,
        );

        // The DOPPLER differs: one is closing on the beacon, the other is not.
        assert!(
            alpha_beacon.observed_frequency.is_finite() && alpha_beacon.observed_frequency > 0.0
        );
        assert!(beta_beacon.observed_frequency.is_finite() && beta_beacon.observed_frequency > 0.0);
        assert!(
            (alpha_beacon.observed_frequency - beta_beacon.observed_frequency).abs() > 5.0,
            "two observers of one 100 Hz beacon measured {} and {}",
            alpha_beacon.observed_frequency,
            beta_beacon.observed_frequency,
        );

        // And each row knows whose it is.
        assert_eq!(
            alpha
                .observer
                .as_ref()
                .map(|observer| observer.label.as_str()),
            Some("alpha"),
        );
        assert_eq!(
            beta.observer
                .as_ref()
                .map(|observer| observer.label.as_str()),
            Some("beta"),
        );
        assert_eq!(
            alpha.observer.as_ref().map(|observer| observer.entity),
            Some(spawned[0])
        );
        assert_eq!(
            beta.observer.as_ref().map(|observer| observer.entity),
            Some(spawned[1])
        );
    }

    /// Adding a second observer must not clear the primary observer's view.
    #[test]
    fn a_second_observer_no_longer_blanks_the_first() {
        let (app, spawned) = publish_with(&[OBSERVER_ALPHA, OBSERVER_BETA]);
        let views = app.world().resource::<RelativisticOpticalView2d>();
        assert!(
            views.observer.is_some() && !views.sources.is_empty(),
            "a second observer must add an image, never erase one",
        );
        assert_eq!(
            views.observer.as_ref().map(|observer| observer.entity),
            Some(spawned[0]),
        );
    }

    /// Adding another observer does not change an existing observer's optical result.
    #[test]
    fn one_observer_reads_exactly_as_it_did_before() {
        let (alone_app, alone_spawned) = publish_with(&[OBSERVER_ALPHA]);
        let alone = alone_app.world().resource::<RelativisticOpticalView2d>();
        assert_eq!(alone.len(), 1);

        // `Deref` exposes the primary observer view.
        assert_eq!(alone.model_id, Some("minkowski"));
        assert_eq!(
            alone.observer.as_ref().map(|observer| observer.entity),
            Some(alone_spawned[0]),
        );
        assert_eq!(alone.sources.len(), 1);
        assert_eq!(alone.missed_sources, 0);
        assert_eq!(alone.history_start_time, Some(0.0));
        assert_eq!(alone.history_end_time, Some(20.0));
        assert_eq!(
            alone.primary(),
            alone.for_observer(alone_spawned[0]),
            "with one observer the primary IS that observer",
        );
        assert_eq!(alone.for_label("alpha"), alone.primary());

        let (pair_app, pair_spawned) = publish_with(&[OBSERVER_ALPHA, OBSERVER_BETA]);
        let pair = pair_app.world().resource::<RelativisticOpticalView2d>();
        assert_eq!(
            beacon_numbers(alone.primary().expect("one observer publishes one row")),
            beacon_numbers(
                pair.for_observer(pair_spawned[0])
                    .expect("alpha is still published"),
            ),
            "alpha's own light cone cannot depend on whether beta exists",
        );
    }

    /// Published row order is deterministic and independent of Bevy spawn/query order.
    #[test]
    fn rows_are_published_in_label_order_not_spawn_order() {
        let (forward_app, forward_spawned) = publish_with(&[OBSERVER_ALPHA, OBSERVER_BETA]);
        let (reverse_app, reverse_spawned) = publish_with(&[OBSERVER_BETA, OBSERVER_ALPHA]);
        let forward = forward_app.world().resource::<RelativisticOpticalView2d>();
        let reverse = reverse_app.world().resource::<RelativisticOpticalView2d>();

        let labels = |views: &RelativisticOpticalView2d| -> Vec<String> {
            views
                .iter()
                .map(|(_, view)| {
                    view.observer
                        .as_ref()
                        .expect("a published row always names its observer")
                        .label
                        .clone()
                })
                .collect()
        };
        assert_eq!(labels(forward), vec!["alpha".to_owned(), "beta".to_owned()]);
        assert_eq!(labels(reverse), labels(forward));

        // Each observer keeps its own values after sorting.
        assert_eq!(
            beacon_numbers(
                forward
                    .for_observer(forward_spawned[0])
                    .expect("forward alpha"),
            ),
            beacon_numbers(
                reverse
                    .for_observer(reverse_spawned[1])
                    .expect("reverse alpha"),
            ),
        );
        assert_eq!(
            reverse.primary(),
            reverse.for_observer(reverse_spawned[1]),
            "the first row is alpha even though beta was spawned first",
        );
    }

    #[test]
    fn no_observer_publishes_no_rows_and_still_reads_blank() {
        let (app, _) = publish_with(&[]);
        let views = app.world().resource::<RelativisticOpticalView2d>();
        assert!(views.is_empty());
        assert_eq!(views.len(), 0);
        assert!(views.primary().is_none());
        assert_eq!(views.model_id, None);
        assert!(views.observer.is_none());
        assert!(views.sources.is_empty());
        assert_eq!(views.missed_sources, 0);
    }
}
