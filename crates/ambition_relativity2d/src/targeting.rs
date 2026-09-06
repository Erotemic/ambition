//! Causal targeting read models for constant-velocity compact targets.
//!
//! This module does not aim or fire for a controller.

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

/// One observer's causal targeting facts, rebuilt every simulation tick. Each
/// observer has an independent intercept solution and observer-frame direction.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ObserverTargetingView2d {
    pub model_id: Option<&'static str>,
    pub coordinate_time: f64,
    ///  an `Option` even though a published row always has one: this is also
    /// the shape a consumer reads through [`RelativisticTargetingView2d`]'s
    /// `Deref`, where "there is no observer at all" must still have an answer.
    pub observer_entity: Option<Entity>,
    pub observer_position: Vec2,
    pub observer_velocity: Vec2,
    pub targets: Vec<RelativisticTargetObservation2d>,
}

/// What "the observer's aim" is when no observer published any.
///
/// Byte-for-byte [`ObserverTargetingView2d::default`]; it exists as a `static`
/// because the `Deref` below must hand out a reference that outlives the call.
static NO_OBSERVER_TARGETING_VIEW: ObserverTargetingView2d = ObserverTargetingView2d {
    model_id: None,
    coordinate_time: 0.0,
    observer_entity: None,
    observer_position: Vec2::ZERO,
    observer_velocity: Vec2::ZERO,
    targets: Vec::new(),
};

/// Observer-relative targeting facts, one set per simulation observer entity.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct RelativisticTargetingView2d {
    /// Ordered by the same deterministic observer key as the optical view.
    views: Vec<(Entity, ObserverTargetingView2d)>,
}

impl RelativisticTargetingView2d {
    pub fn is_empty(&self) -> bool {
        self.views.is_empty()
    }

    pub fn len(&self) -> usize {
        self.views.len()
    }

    /// Every observer's aim, in the published deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (Entity, &ObserverTargetingView2d)> + '_ {
        self.views.iter().map(|(entity, view)| (*entity, view))
    }

    pub fn observers(&self) -> impl Iterator<Item = Entity> + '_ {
        self.views.iter().map(|(entity, _)| *entity)
    }

    /// The aim belonging to one observer — the accessor a per-view consumer
    /// wants, because it names whose eyes it is solving from.
    pub fn for_observer(&self, observer: Entity) -> Option<&ObserverTargetingView2d> {
        self.views
            .iter()
            .find(|(entity, _)| *entity == observer)
            .map(|(_, view)| view)
    }

    /// The first published observer, in the deterministic order above.
    pub fn primary(&self) -> Option<&ObserverTargetingView2d> {
        self.views.first().map(|(_, view)| view)
    }

    pub(crate) fn clear(&mut self) {
        self.views.clear();
    }

    pub(crate) fn push(&mut self, observer: Entity, view: ObserverTargetingView2d) {
        self.views.push((observer, view));
    }
}

/// Field access reads the FIRST observer's aim.
///
///  this is what keeps the one-observer world byte-identical to the version of
/// this module that had exactly one set: `view.targets` and `view.observer_position`
/// mean today what they meant then. With two observers they read the first
/// one's — but a consumer that reaches through this with two observers on the
/// field is aiming for both from one pair of eyes, and should be reading
/// [`RelativisticTargetingView2d::for_observer`].
impl std::ops::Deref for RelativisticTargetingView2d {
    type Target = ObserverTargetingView2d;

    fn deref(&self) -> &Self::Target {
        self.primary().unwrap_or(&NO_OBSERVER_TARGETING_VIEW)
    }
}

pub(crate) fn register_rollback_state(
    registrar: &mut impl ambition_platformer2d_core::snapshot::RollbackRegistrar,
) {
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
    observers: Query<(Entity, &RelativisticObserver2d, &BodyKinematics)>,
    targets: Query<(Entity, &RelativisticTarget2d, &BodyKinematics)>,
    optical: Res<RelativisticOpticalView2d>,
    mut views: ResMut<RelativisticTargetingView2d>,
) {
    views.clear();
    let (Ok(spacetime), Ok(time)) = (spacetime.single(), coordinate_time.single()) else {
        return;
    };
    let model = spacetime.model();
    let Some(invariant_speed) = model.minkowski_optics_invariant_speed() else {
        return;
    };

    let mut rows: Vec<_> = targets.iter().collect();
    rows.sort_by(|(_, lhs, _), (_, rhs, _)| lhs.0.cmp(&rhs.0));

    //  the SAME ordering rule the optical view publishes under — authored
    // label, entity bits only to break a duplicate-label tie — so the two
    // resources iterate in the same order and a per-pane consumer can pair them
    // without a second sort that could disagree.
    let mut observer_rows: Vec<_> = observers.iter().collect();
    observer_rows.sort_by(|(left_entity, left, _), (right_entity, right, _)| {
        left.0
            .cmp(&right.0)
            .then_with(|| left_entity.to_bits().cmp(&right_entity.to_bits()))
    });

    for (observer_entity, _, observer_body) in observer_rows {
        let view = solve_for_observer(
            model.model_id(),
            invariant_speed,
            time.seconds,
            (observer_entity, observer_body),
            &rows,
            &optical,
        );
        views.push(observer_entity, view);
    }
}

/// Every target's null intercept, solved from ONE observer's position and
/// velocity and aberrated into that observer's frame.
fn solve_for_observer(
    model_id: &'static str,
    invariant_speed: ambition_relativity::InvariantSpeed,
    coordinate_time: f64,
    (observer_entity, observer_body): (Entity, &BodyKinematics),
    rows: &[(Entity, &RelativisticTarget2d, &BodyKinematics)],
    optical: &RelativisticOpticalView2d,
) -> ObserverTargetingView2d {
    let time = SpacetimeCoordinateTime2d {
        seconds: coordinate_time,
        ..Default::default()
    };
    //  name whose sky this joins. The optical view holds one image per
    // observer; joining "the" sources would pair this observer's intercept
    // solutions with a different observer's light-delayed images.
    let optical_sources = optical
        .for_observer(observer_entity)
        .map(|view| view.sources.as_slice())
        .unwrap_or_default();

    let mut view = ObserverTargetingView2d {
        model_id: Some(model_id),
        coordinate_time: time.seconds,
        observer_entity: Some(observer_entity),
        observer_position: observer_body.pos,
        observer_velocity: observer_body.vel,
        targets: Vec::new(),
    };

    for (entity, target, body) in rows.iter().copied() {
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
    view
}

fn clear_targeting_view_without_live_spacetime(
    spacetime: Query<(), (With<ActiveSpacetime2d>, With<SessionRoot>)>,
    mut views: ResMut<RelativisticTargetingView2d>,
) {
    if spacetime.is_empty() && !views.is_empty() {
        *views = RelativisticTargetingView2d::default();
    }
}

#[cfg(test)]
mod tests {
    use ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId;

    use super::*;

    /// The smallest world that can tell a per-observer aim from a shared one.
    ///
    /// One target standing still at x=100, an invariant speed of 10, and two
    /// observers on OPPOSITE sides of it. A null intercept is solved from the
    /// observer's own position, so the two must disagree about the direction to
    /// shoot — by a sign, which no rounding can hide.
    const OBSERVER_ALPHA: (&str, Vec2) = ("alpha", Vec2::new(0.0, 0.0));
    const OBSERVER_BETA: (&str, Vec2) = ("beta", Vec2::new(200.0, 0.0));

    /// Spawn one session, one target, and the given observers IN THE GIVEN
    /// ORDER, then publish once.
    fn publish_with(observers: &[(&str, Vec2)]) -> (App, Vec<Entity>) {
        let mut app = App::new();
        app.init_resource::<RelativisticTargetingView2d>();
        app.init_resource::<RelativisticOpticalView2d>();
        app.add_systems(Update, publish_targeting_view);
        app.world_mut().spawn((
            SessionRoot(SessionScopeId(0)),
            ActiveSpacetime2d::minkowski(10.0).expect("10 units/s is a valid invariant speed"),
            SpacetimeCoordinateTime2d {
                seconds: 20.0,
                epoch: 0,
            },
        ));
        app.world_mut().spawn((
            RelativisticTarget2d("quarry".to_owned()),
            BodyKinematics {
                pos: Vec2::new(100.0, 0.0),
                ..Default::default()
            },
        ));
        let mut spawned = Vec::new();
        for (label, position) in observers {
            spawned.push(
                app.world_mut()
                    .spawn((
                        RelativisticObserver2d((*label).to_owned()),
                        BodyKinematics {
                            pos: *position,
                            ..Default::default()
                        },
                    ))
                    .id(),
            );
        }
        app.update();
        (app, spawned)
    }

    fn quarry(view: &ObserverTargetingView2d) -> &RelativisticTargetObservation2d {
        view.targets
            .iter()
            .find(|target| target.label == "quarry")
            .expect("the target is reachable from both observers")
    }

    /// Each observer retains an independent targeting view.
    #[test]
    fn two_observers_aim_in_opposite_directions_instead_of_blanking() {
        let (app, spawned) = publish_with(&[OBSERVER_ALPHA, OBSERVER_BETA]);
        let views = app.world().resource::<RelativisticTargetingView2d>();
        assert_eq!(views.len(), 2, "two observers, two aims");

        let alpha = quarry(views.for_observer(spawned[0]).expect("alpha aims"));
        let beta = quarry(views.for_observer(spawned[1]).expect("beta aims"));

        assert!(
            alpha.emission_direction.x > 0.9,
            "an observer at the origin shoots RIGHT at a target at x=100, not {}",
            alpha.emission_direction,
        );
        assert!(
            beta.emission_direction.x < -0.9,
            "an observer at x=200 shoots LEFT at the same target, not {}",
            beta.emission_direction,
        );
    }

    /// The one-observer world reads exactly as it did before the split, through
    /// the same field access, and a second observer does not perturb the first
    /// one's numbers.
    #[test]
    fn one_observer_reads_exactly_as_it_did_before() {
        let (alone_app, alone_spawned) = publish_with(&[OBSERVER_ALPHA]);
        let alone = alone_app.world().resource::<RelativisticTargetingView2d>();
        assert_eq!(alone.len(), 1);

        // Field access through `Deref` is the pre-split reading.
        assert_eq!(alone.model_id, Some("minkowski"));
        assert_eq!(alone.observer_entity, Some(alone_spawned[0]));
        assert_eq!(alone.observer_position, Vec2::ZERO);
        assert_eq!(alone.coordinate_time, 20.0);
        assert_eq!(alone.targets.len(), 1);
        assert_eq!(
            alone.primary(),
            alone.for_observer(alone_spawned[0]),
            "with one observer the primary IS that observer",
        );

        let (pair_app, pair_spawned) = publish_with(&[OBSERVER_ALPHA, OBSERVER_BETA]);
        let pair = pair_app.world().resource::<RelativisticTargetingView2d>();
        let alone_quarry = quarry(alone.primary().expect("one observer publishes one row"));
        let pair_quarry = quarry(
            pair.for_observer(pair_spawned[0])
                .expect("alpha is still published"),
        );
        assert_eq!(
            (
                alone_quarry.time_to_intercept,
                alone_quarry.emission_direction,
            ),
            (
                pair_quarry.time_to_intercept,
                pair_quarry.emission_direction
            ),
            "alpha's own intercept cannot depend on whether beta exists",
        );
    }

    ///  the falsifier for the sort: the SAME world spawned in the OPPOSITE
    /// order must publish the same rows in the same order, and in the SAME order
    /// the optical view publishes — a per-pane consumer pairs the two.
    #[test]
    fn rows_are_published_in_label_order_not_spawn_order() {
        let (forward_app, forward_spawned) = publish_with(&[OBSERVER_ALPHA, OBSERVER_BETA]);
        let (backward_app, backward_spawned) = publish_with(&[OBSERVER_BETA, OBSERVER_ALPHA]);

        let forward: Vec<Entity> = forward_app
            .world()
            .resource::<RelativisticTargetingView2d>()
            .observers()
            .collect();
        let backward: Vec<Entity> = backward_app
            .world()
            .resource::<RelativisticTargetingView2d>()
            .observers()
            .collect();

        // alpha before beta, whichever order they were spawned in.
        assert_eq!(forward, vec![forward_spawned[0], forward_spawned[1]]);
        assert_eq!(backward, vec![backward_spawned[1], backward_spawned[0]]);
    }
}
