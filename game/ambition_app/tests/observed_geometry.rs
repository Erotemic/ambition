//! GEOMETRY-OBSERVABILITY GOBS-1: F1 and headless inspection read one
//! observation.
//!
//! A provider installed by a test, a crate F1 has never heard of, is in the
//! catalog, in F1's data feed and in the headless answer, with the same
//! geometry in both. The real providers (the collision views and the combat
//! read model) answer for the shown room. With the layer off, F1 asks for
//! nothing.

use ambition_app::{AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::dev_tools::{DeveloperRuntimeState, dev_tools::DeveloperTools};
use ambition_platformer2d::geometry_observation::{
    GeometryObservation, GeometryObserverAppExt as _, GeometryObservers, ObservationQuery, ObservedShape, ObservedState,
    ObserverAnswer, ObserverInfo, observe_all, to_json,
};
use ambition_platformer2d::platformer::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::render::rendering::observed_geometry::{ObservedGeometryFrame, collect_observed_geometry};
use bevy::prelude::{Entity, In, With};

use crate::common::fixed_60hz_room_options;

const ROOM: &str = "basement_breakables";

fn third_party(query: In<ObservationQuery>) -> Vec<GeometryObservation> {
    vec![GeometryObservation {
        key: format!("sensor for {:?}", query.subject.is_some()),
        roles: vec!["sensor".into()],
        shape: ObservedShape::Circle { center: [100.0, 100.0], radius: 12.0 },
        state: ObservedState::Inactive { reason: "a test's own reason".into() },
    }]
}

fn layer_on(sim: &mut Platformer2dSimHarness, on: bool) {
    let world = sim.world_mut();
    world.get_resource_or_insert_with(DeveloperRuntimeState::default).debug = on;
    let mut tools = world.get_resource_or_insert_with(DeveloperTools::default);
    tools.gizmos_enabled = true;
    tools.show_observed_geometry = on;
}

#[test]
fn f1_and_headless_inspection_read_one_observation() {
    let mut sim = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(ROOM)).expect("the room boots");
    sim.app_mut().add_geometry_observer(
        ObserverInfo { name: "test.third_party", description: "a test's provider", needs_subject: false },
        third_party,
    );
    for _ in 0..5 {
        sim.app_mut().update();
    }
    let world = sim.world_mut();
    let catalog: Vec<&str> = world.resource::<GeometryObservers>().catalog().map(|info| info.name).collect();
    for name in ["collision.authored", "collision.composed", "collision.for_subject", "combat.volumes", "test.third_party"] {
        assert!(catalog.contains(&name), "{name} is in the catalog: {catalog:?}");
    }

    layer_on(&mut sim, true);
    collect_observed_geometry(sim.world_mut());
    let world = sim.world_mut();
    let (player, room) = world
        .query_filtered::<(Entity, &InRoomInstance), With<PrimaryPlayer>>()
        .single(world)
        .map(|(player, room)| (player, room.0))
        .expect("one player");
    let root = world
        .query_filtered::<(Entity, &LiveRoomInstance), With<RoomInstanceRoot>>()
        .iter(world)
        .find(|(_, live)| **live == room)
        .map(|(root, _)| root)
        .expect("the player's room has a root");
    let f1 = world.resource::<ObservedGeometryFrame>().in_room(room).to_vec();
    assert!(!f1.is_empty(), "F1's feed has the shown room: {:?}", world.resource::<ObservedGeometryFrame>());
    let headless = observe_all(world, ObservationQuery { room: Some(root), subject: Some(player) }, |_| true);
    assert_eq!(f1, headless, "F1 draws what headless inspection prints");

    let answer = |name: &str| {
        let layer = headless.iter().find(|layer| layer.provider == name).expect("answered");
        match &layer.answer {
            ObserverAnswer::Observed { geometry } => geometry.clone(),
            other => panic!("{name}: {other:?}"),
        }
    };
    let ours = answer("test.third_party");
    assert_eq!(ours.len(), 1);
    assert_eq!(ours[0].state, ObservedState::Inactive { reason: "a test's own reason".into() }, "inactive stays inactive");
    let walls = answer("collision.composed");
    assert!(walls.iter().any(|wall| wall.key.ends_with("respawning breakable platform")), "the composed walls hold the breakable");
    assert!(answer("collision.authored").len() < walls.len(), "the composed walls hold more than the authored blocks");
    assert!(!answer("combat.volumes").is_empty(), "the player has hurt volumes");
    assert!(to_json(&headless).contains("\"provider\": \"test.third_party\""));

    // GOBS-1: F1 draws one observer at a time. The collector published the
    // installed names; the menu's cycle steps from all to the first one.
    let first = {
        let mut tools = sim.world_mut().resource_mut::<DeveloperTools>();
        assert_eq!(tools.observed_geometry_providers, catalog_names(&headless), "the cycle steps through the installed observers");
        tools.cycle_observed_geometry_provider(1);
        tools.observed_geometry_only.clone().expect("one step from all selects one observer")
    };
    collect_observed_geometry(sim.world_mut());
    let drawn: Vec<String> =
        sim.world().resource::<ObservedGeometryFrame>().in_room(room).iter().map(|layer| layer.provider.to_string()).collect();
    assert_eq!(drawn, vec![first.clone()], "F1 drew other observers than the selected one");
    sim.world_mut().resource_mut::<DeveloperTools>().cycle_observed_geometry_provider(-1);
    collect_observed_geometry(sim.world_mut());
    assert_eq!(
        sim.world().resource::<ObservedGeometryFrame>().in_room(room).len(),
        headless.len(),
        "a step back from the first observer draws them all again"
    );

    layer_on(&mut sim, false);
    collect_observed_geometry(sim.world_mut());
    assert!(sim.world().resource::<ObservedGeometryFrame>().rooms.is_empty(), "with the layer off, nothing is asked");
}

/// The providers a full observation answered, in its order.
fn catalog_names(layers: &[ambition_platformer2d::geometry_observation::ObservedLayer]) -> Vec<String> {
    layers.iter().map(|layer| layer.provider.to_string()).collect()
}
