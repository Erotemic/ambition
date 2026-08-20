//! Causal targeting read models for constant-velocity compact targets.
//!
//! This module does not aim or fire for a controller. It publishes the exact
//! Minkowski null-intercept direction from the current observer event alongside
//! the target's retarded optical direction, so game rules and instruments can
//! expose the difference between "where the light came from" and "where a new
//! light signal must be sent".

use ambition_platformer2d_core::snapshot::{put_str, Reader, SnapshotState};
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::lifecycle::SessionRoot;
use ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith;
use ambition_relativity::{
    observe_photon_direction, solve_null_intercept_constant_velocity, MinkowskiEvent,
};
use bevy::ecs::schedule::InternedScheduleLabel;
use bevy::prelude::*;

use crate::{
    ActiveSpacetime2d, RelativisticObserver2d, RelativisticOpticalView2d, Relativity2dSet,
    SpacetimeCoordinateTime2d,
};

/// Opt one compact body into causal intercept publication.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct RelativisticTarget2d(pub String);

impl SnapshotState for RelativisticTarget2d {
    fn encode(&self, out: &mut Vec<u8>) {
        put_str(out, &self.0);
    }

    fn decode(reader: &mut Reader<'_>) -> Option<Self> {
        Some(Self(reader.str()?.to_owned()))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RelativisticTargetObservation2d {
    pub entity: Entity,
    pub label: String,
    pub coordinate_position: Vec2,
    pub coordinate_velocity: Vec2,
    pub intercept_event: MinkowskiEvent,
    pub time_to_intercept: f64,
    pub emission_direction: Vec2,
    pub observer_local_emission_direction: Vec2,
    pub apparent_source_direction: Option<Vec2>,
    pub retarded_position: Option<Vec2>,
    pub optical_light_age: Option<f64>,
    pub apparent_to_intercept_angle: Option<f32>,
}

/// Observer-relative causal targeting facts rebuilt every simulation tick.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct RelativisticTargetingView2d {
    pub model_id: Option<&'static str>,
    pub coordinate_time: f64,
    pub observer_entity: Option<Entity>,
    pub observer_position: Vec2,
    pub observer_velocity: Vec2,
    pub targets: Vec<RelativisticTargetObservation2d>,
}

pub(crate) fn register_rollback_state(registrar: &mut impl ambition_platformer2d_core::snapshot::RollbackRegistrar) {
    registrar.declare_rollback_derived_resource::<RelativisticTargetingView2d>(
        "ambition_relativity2d",
        "relativity.targeting_view_2d",
        "constant-velocity null-intercept facts rebuilt from canonical bodies and the optical view",
    );
    registrar.rollback_component_canonical::<RelativisticTarget2d>(
        "ambition_relativity2d",
        "relativity.target_marker_2d",
    );
}

pub(crate) fn install_targeting_systems(app: &mut App, sim: InternedScheduleLabel) {
    app.init_resource::<RelativisticTargetingView2d>();
    app.add_systems(
        sim,
        publish_targeting_view
            .run_if(crate::spacetime_is_active)
            .in_set(Relativity2dSet::PublishTargeting)
            .in_set(Platformer2dSimulationPhaseMonolith::FeatureViewSync)
            .after(Relativity2dSet::PublishOptics),
    )
    .add_systems(Update, clear_targeting_view_without_live_spacetime);
}

fn publish_targeting_view(
    spacetime: Query<&ActiveSpacetime2d, With<SessionRoot>>,
    coordinate_time: Query<&SpacetimeCoordinateTime2d, With<SessionRoot>>,
    observers: Query<(Entity, &BodyKinematics), With<RelativisticObserver2d>>,
    targets: Query<(Entity, &RelativisticTarget2d, &BodyKinematics)>,
    optical: Res<RelativisticOpticalView2d>,
    mut view: ResMut<RelativisticTargetingView2d>,
) {
    *view = RelativisticTargetingView2d::default();
    let (Ok(spacetime), Ok(time), Ok((observer_entity, observer_body))) = (
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

    // ⭐ **name whose sky this joins.** The optical view holds one image per
    // observer; joining "the" sources would have silently paired this
    // observer's intercept solutions with a different observer's light-delayed
    // images the moment a second observer existed. With one observer this is
    // that observer's row, which is exactly what the old `sources` field was.
    //
    // ⚠ targeting itself is still single-observer (`observers.single()` above);
    // this is the join being correct in advance, not that gap being closed.
    let optical_sources = optical
        .for_observer(observer_entity)
        .map(|view| view.sources.as_slice())
        .unwrap_or_default();

    view.model_id = Some(model.model_id());
    view.coordinate_time = time.seconds;
    view.observer_entity = Some(observer_entity);
    view.observer_position = observer_body.pos;
    view.observer_velocity = observer_body.vel;

    let mut rows: Vec<_> = targets.iter().collect();
    rows.sort_by(|(_, lhs, _), (_, rhs, _)| lhs.0.cmp(&rhs.0));
    for (entity, target, body) in rows {
        let relative = body.pos - observer_body.pos;
        let Some(solution) = solve_null_intercept_constant_velocity(
            [f64::from(relative.x), f64::from(relative.y), 0.0],
            [f64::from(body.vel.x), f64::from(body.vel.y), 0.0],
            invariant_speed,
        ) else {
            continue;
        };
        let emission_direction = Vec2::new(
            solution.emission_direction[0] as f32,
            solution.emission_direction[1] as f32,
        )
        .normalize_or_zero();
        if emission_direction == Vec2::ZERO {
            continue;
        }
        let Some(observer_local_emission_direction) = observe_photon_direction(
            solution.emission_direction,
            [
                f64::from(observer_body.vel.x),
                f64::from(observer_body.vel.y),
                0.0,
            ],
            invariant_speed,
        )
        .map(|observation| {
            Vec2::new(
                observation.propagation_direction[0] as f32,
                observation.propagation_direction[1] as f32,
            )
            .normalize_or_zero()
        })
        .filter(|direction| *direction != Vec2::ZERO) else {
            continue;
        };
        let optical_source = optical_sources
            .iter()
            .find(|source| source.entity == entity);
        let apparent_source_direction = optical_source
            .map(|source| source.apparent_source_direction.normalize_or_zero())
            .filter(|direction| *direction != Vec2::ZERO);
        let apparent_to_intercept_angle = apparent_source_direction.map(|apparent| {
            apparent
                .dot(observer_local_emission_direction)
                .clamp(-1.0, 1.0)
                .acos()
        });
        view.targets.push(RelativisticTargetObservation2d {
            entity,
            label: target.0.clone(),
            coordinate_position: body.pos,
            coordinate_velocity: body.vel,
            intercept_event: MinkowskiEvent {
                coordinate_time: time.seconds + solution.coordinate_time_to_intercept,
                position: [
                    f64::from(observer_body.pos.x) + solution.intercept_position[0],
                    f64::from(observer_body.pos.y) + solution.intercept_position[1],
                    solution.intercept_position[2],
                ],
            },
            time_to_intercept: solution.coordinate_time_to_intercept,
            emission_direction,
            observer_local_emission_direction,
            apparent_source_direction,
            retarded_position: optical_source.map(|source| source.retarded_position),
            optical_light_age: optical_source.map(|source| source.light_age),
            apparent_to_intercept_angle,
        });
    }
}

fn clear_targeting_view_without_live_spacetime(
    spacetime: Query<(), (With<ActiveSpacetime2d>, With<SessionRoot>)>,
    mut view: ResMut<RelativisticTargetingView2d>,
) {
    if spacetime.is_empty()
        && (view.model_id.is_some() || view.observer_entity.is_some() || !view.targets.is_empty())
    {
        *view = RelativisticTargetingView2d::default();
    }
}
