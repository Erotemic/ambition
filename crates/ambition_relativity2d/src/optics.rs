//! Observer-relative optical read models built from bounded worldlines.
//!
//! The authoritative simulation remains in one Minkowski coordinate chart.
//! This module asks a separate question for presentation and perception:
//! which source event lies on the current observer event's past light cone,
//! and what photon direction/frequency does the observer measure locally?

use std::collections::VecDeque;

use ambition_platformer2d_core::snapshot::{put_f32, put_str, put_u64, Reader, SnapshotState};
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_runtime::rollback::AmbitionRollbackApp;
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

/// Select one body as an optical observer without changing simulation authority.
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

/// Presentation/perception-facing optical state for the active observer.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct RelativisticOpticalView2d {
    pub model_id: Option<&'static str>,
    pub observer: Option<OpticalObserverObservation2d>,
    pub sources: Vec<OpticalSourceObservation2d>,
    pub history_start_time: Option<f64>,
    pub history_end_time: Option<f64>,
    pub missed_sources: usize,
}

pub(crate) fn install_optics_systems(app: &mut App, sim: InternedScheduleLabel) {
    app.init_resource::<RelativisticOpticalView2d>();
    app.declare_rollback_derived_resource::<RelativisticOpticalView2d>(
        "ambition_relativity2d",
        "relativity.optical_view_2d",
        "observer past-light-cone view rebuilt from canonical bodies and bounded worldline telemetry",
    );
    app.rollback_component_canonical::<OpticalSource2d>(
        "ambition_relativity2d",
        "relativity.optical_source_2d",
    )
    .rollback_component_canonical::<RelativisticObserver2d>(
        "ambition_relativity2d",
        "relativity.optical_observer_2d",
    );

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
    // ⛔ **`&WorldlineTracked2d` IS the join, and it used to be a string.** The
    // source's own `label` was looked up in the history map, so an optical
    // source and the worldline it reads were two independently typed names: a
    // typo silently produced a missing source, and a DUPLICATE let one entity
    // borrow another's past light cone. Requiring the tracked component means a
    // source reads the history of the entity it IS — the relationship cannot be
    // omitted or mistyped, because it is not written down twice.
    //
    // ⚠ a source with no tracked worldline is now unqueryable rather than
    // "missed": it has no history to reconstruct from, which was always true.
    sources: Query<(Entity, &OpticalSource2d, &WorldlineTracked2d)>,
    worldlines: Res<WorldlineHistoryView2d>,
    mut view: ResMut<RelativisticOpticalView2d>,
) {
    *view = RelativisticOpticalView2d::default();
    let (Ok(spacetime), Ok(coordinate_time), Ok((observer_entity, observer, body, proper_time))) = (
        spacetime.single(),
        coordinate_time.single(),
        observers.single(),
    ) else {
        return;
    };
    let model = spacetime.model();
    let Some(invariant_speed) = model.minkowski_optics_invariant_speed() else {
        return;
    };
    let reception_time = coordinate_time.seconds;
    view.model_id = Some(model.model_id());
    view.observer = Some(OpticalObserverObservation2d {
        entity: observer_entity,
        label: observer.0.clone(),
        coordinate_time: reception_time,
        proper_time: proper_time.map(|clock| clock.seconds),
        position: body.pos,
        coordinate_velocity: body.vel,
        invariant_speed: invariant_speed.get(),
    });

    let mut source_rows: Vec<_> = sources.iter().collect();
    source_rows.sort_by(|(_, lhs, _), (_, rhs, _)| lhs.label.cmp(&rhs.label));
    for (entity, source, tracked) in source_rows {
        let Some(samples) = worldlines.tracks.get(&tracked.0) else {
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
}

fn clear_optical_view_without_live_spacetime(
    spacetime: Query<(), (With<ActiveSpacetime2d>, With<SessionRoot>)>,
    mut view: ResMut<RelativisticOpticalView2d>,
) {
    if spacetime.is_empty()
        && (view.model_id.is_some() || view.observer.is_some() || !view.sources.is_empty())
    {
        *view = RelativisticOpticalView2d::default();
    }
}

/// Solve the intersection of one sampled source worldline with an observer
/// event's past light cone.
///
/// Source motion is linearly interpolated within each sampled segment. A fixed
/// twelve-step bisection makes the answer deterministic and avoids an iterative
/// convergence threshold becoming part of rollback behavior.
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
    use super::*;

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
}
