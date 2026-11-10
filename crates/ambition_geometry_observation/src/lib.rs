//! Read-only observation of the geometry gameplay tests (GOBS-1,
//! `docs/planning/engine/geometry-observability-and-f1.md`).
//!
//! A crate that owns some geometry (the collision world, the combat read
//! model, ...) installs a provider with [`GeometryObserverAppExt::add_geometry_observer`]:
//! a one-shot system that reads the owner's own resolved geometry and answers
//! an [`ObservationQuery`] with [`GeometryObservation`]s. It must not compute
//! the geometry a second way.
//!
//! [`observe_all`] runs every installed provider. F1 draws what it returns and
//! headless inspection prints it ([`to_json`]); both go through it, so they
//! cannot disagree, and a new provider is in both without an edit to either.
//! Nothing runs unless someone asks.
//!
//! Coordinates are the room's: pixels, y down, as the simulation has them.

use std::borrow::Cow;

use bevy_app::App;
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemId;
use serde::Serialize;

/// What a caller asks a provider about.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ObservationQuery {
    /// The live room's root entity. `None`: the provider's own default (a
    /// provider may then answer for no room at all).
    pub room: Option<Entity>,
    /// The body whose view of the room is asked for (a gate open for it, a
    /// portal hole it opened). `None`: the room's own view.
    pub subject: Option<Entity>,
}

/// A shape, in the room's pixels (y down).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "shape", rename_all = "snake_case")]
pub enum ObservedShape {
    Aabb { min: [f32; 2], max: [f32; 2] },
    Segment { a: [f32; 2], b: [f32; 2] },
    Circle { center: [f32; 2], radius: f32 },
    Polygon { points: Vec<[f32; 2]> },
}

impl ObservedShape {
    pub fn aabb(min: [f32; 2], max: [f32; 2]) -> Self {
        Self::Aabb { min, max }
    }
}

/// Whether the geometry takes part now, and why not.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ObservedState {
    /// Gameplay tests it now.
    Active,
    /// It is there, but gameplay does not test it now; `reason` is the
    /// owner's own word for why (broken, removed, not open for this body).
    Inactive { reason: Cow<'static, str> },
}

/// One piece of geometry, as its owner holds it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GeometryObservation {
    /// Stable within the provider: an authored id or name.
    pub key: String,
    /// What gameplay does with it (`solid`, `one_way`, `hurt`, `hit`, a
    /// domain's own word). More than one is allowed.
    pub roles: Vec<Cow<'static, str>>,
    pub shape: ObservedShape,
    #[serde(flatten)]
    pub state: ObservedState,
}

/// A provider, as the catalog lists it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ObserverInfo {
    /// Namespaced: `<domain>.<view>`.
    pub name: &'static str,
    pub description: &'static str,
    /// It answers only for an [`ObservationQuery::subject`].
    pub needs_subject: bool,
}

type ObserverSystem = SystemId<In<ObservationQuery>, Vec<GeometryObservation>>;

/// The installed providers, in installation order.
#[derive(Resource, Default)]
pub struct GeometryObservers {
    entries: Vec<(ObserverInfo, ObserverSystem)>,
}

impl GeometryObservers {
    /// The catalog: every installed provider.
    pub fn catalog(&self) -> impl Iterator<Item = &ObserverInfo> {
        self.entries.iter().map(|(info, _)| info)
    }
}

pub trait GeometryObserverAppExt {
    /// Install `system` as the provider `info.name`. A second install under
    /// one name replaces the first.
    fn add_geometry_observer<M>(
        &mut self,
        info: ObserverInfo,
        system: impl IntoSystem<In<ObservationQuery>, Vec<GeometryObservation>, M> + 'static,
    ) -> &mut Self;
}

impl GeometryObserverAppExt for App {
    fn add_geometry_observer<M>(
        &mut self,
        info: ObserverInfo,
        system: impl IntoSystem<In<ObservationQuery>, Vec<GeometryObservation>, M> + 'static,
    ) -> &mut Self {
        let id = self.world_mut().register_system(system);
        let mut observers = self.world_mut().get_resource_or_insert_with(GeometryObservers::default);
        if let Some(slot) = observers.entries.iter_mut().find(|(known, _)| known.name == info.name) {
            *slot = (info, id);
        } else {
            observers.entries.push((info, id));
        }
        self
    }
}

/// What one provider answered.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ObservedLayer {
    pub provider: &'static str,
    #[serde(flatten)]
    pub answer: ObserverAnswer,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ObserverAnswer {
    Observed { geometry: Vec<GeometryObservation> },
    /// The provider answers only for a subject and none was given.
    NeedsSubject,
    /// The provider's system could not run (its resources are not in this app).
    Unavailable { reason: String },
}

/// Run every installed provider for `query`. Providers whose name `wanted`
/// rejects are not run.
pub fn observe_all(world: &mut World, query: ObservationQuery, wanted: impl Fn(&str) -> bool) -> Vec<ObservedLayer> {
    let Some(observers) = world.get_resource::<GeometryObservers>() else {
        return Vec::new();
    };
    let entries: Vec<(ObserverInfo, ObserverSystem)> = observers
        .entries
        .iter()
        .filter(|(info, _)| wanted(info.name))
        .cloned()
        .collect();
    entries
        .into_iter()
        .map(|(info, id)| {
            let answer = if info.needs_subject && query.subject.is_none() {
                ObserverAnswer::NeedsSubject
            } else {
                match world.run_system_with(id, query) {
                    Ok(mut geometry) => {
                        geometry.sort_by(|a, b| a.key.cmp(&b.key));
                        ObserverAnswer::Observed { geometry }
                    }
                    Err(error) => ObserverAnswer::Unavailable {
                        reason: error.to_string(),
                    },
                }
            };
            ObservedLayer {
                provider: info.name,
                answer,
            }
        })
        .collect()
}

/// The headless form: stable order, room pixels, y down.
pub fn to_json(layers: &[ObservedLayer]) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "schema": "ambition.geometry_observation.v1",
        "frame": "room pixels, y down",
        "layers": layers,
    }))
    .expect("observations serialize")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Resource)]
    struct Walls(Vec<(String, [f32; 2], [f32; 2], bool)>);

    fn walls(_: In<ObservationQuery>, walls: Res<Walls>) -> Vec<GeometryObservation> {
        walls
            .0
            .iter()
            .map(|(key, min, max, on)| GeometryObservation {
                key: key.clone(),
                roles: vec!["solid".into()],
                shape: ObservedShape::aabb(*min, *max),
                state: if *on {
                    ObservedState::Active
                } else {
                    ObservedState::Inactive { reason: "broken".into() }
                },
            })
            .collect()
    }

    fn mine(query: In<ObservationQuery>) -> Vec<GeometryObservation> {
        vec![GeometryObservation {
            key: format!("{:?}", query.subject),
            roles: vec!["sensor".into()],
            shape: ObservedShape::Circle { center: [0.0, 0.0], radius: 4.0 },
            state: ObservedState::Active,
        }]
    }

    #[test]
    fn a_provider_answers_in_order_and_says_why_a_wall_is_off() {
        let mut app = App::new();
        app.insert_resource(Walls(vec![
            ("b".into(), [0.0, 0.0], [1.0, 1.0], false),
            ("a".into(), [1.0, 1.0], [2.0, 2.0], true),
        ]))
        .add_geometry_observer(
            ObserverInfo { name: "test.walls", description: "walls", needs_subject: false },
            walls,
        );
        let layers = observe_all(app.world_mut(), ObservationQuery::default(), |_| true);
        let ObserverAnswer::Observed { geometry } = &layers[0].answer else {
            panic!("observed: {layers:?}")
        };
        assert_eq!(geometry.iter().map(|g| g.key.as_str()).collect::<Vec<_>>(), ["a", "b"]);
        assert_eq!(geometry[1].state, ObservedState::Inactive { reason: "broken".into() });
        assert!(to_json(&layers).contains("\"reason\": \"broken\""));
    }

    #[test]
    fn a_subject_provider_is_not_run_without_a_subject() {
        let mut app = App::new();
        app.add_geometry_observer(ObserverInfo { name: "test.mine", description: "", needs_subject: true }, mine);
        let layers = observe_all(app.world_mut(), ObservationQuery::default(), |_| true);
        assert_eq!(layers[0].answer, ObserverAnswer::NeedsSubject);
        let subject = app.world_mut().spawn_empty().id();
        let layers = observe_all(app.world_mut(), ObservationQuery { room: None, subject: Some(subject) }, |_| true);
        assert!(matches!(&layers[0].answer, ObserverAnswer::Observed { geometry } if geometry.len() == 1));
    }

    #[test]
    fn a_provider_whose_resources_are_missing_is_unavailable_not_empty() {
        let mut app = App::new();
        app.add_geometry_observer(ObserverInfo { name: "test.walls", description: "", needs_subject: false }, walls);
        let layers = observe_all(app.world_mut(), ObservationQuery::default(), |_| true);
        assert!(matches!(layers[0].answer, ObserverAnswer::Unavailable { .. }), "{layers:?}");
    }
}
