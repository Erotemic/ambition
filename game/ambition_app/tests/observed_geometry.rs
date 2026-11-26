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
    ObserverAnswer, ObserverInfo, observe_all,
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
    tools.observed_geometry_preset.clear();
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
    // GOBS-5: each domain that makes gameplay geometry ships its observer, so
    // one cannot be dropped from the shipped composition unnoticed.
    for name in [
        "collision.authored",
        "collision.composed",
        "collision.for_subject",
        "body.sweep",
        "body.material",
        "combat.projectiles",
        "world.regions",
        "combat.volumes",
        "interaction.reach",
        "navigation.graph",
        "portal.apertures",
        "test.third_party",
    ] {
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
    let view = world.query_filtered::<Entity, With<ambition_platformer2d::sim_view::LocalView>>()
        .single(world).expect("one view");
    let f1 = world.resource::<ObservedGeometryFrame>().in_view(view).unwrap().report.as_ref().unwrap().layers.clone();
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
    let report = ambition_platformer2d::sim_view::geometry_inspection::inspect_geometry(
        world, ObservationQuery { room: Some(root), subject: Some(player) }, |_| true).unwrap();
    assert_eq!(report.layers, headless);
    assert!(report.to_json().contains("\"provider\": \"test.third_party\""));
    assert!(report.context.sim_tick.is_some());
    assert_eq!(report.context.room_instance, room.ordinal());
    assert_eq!(report.context.room, ROOM);

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
        sim.world().resource::<ObservedGeometryFrame>().in_view(view).unwrap().report.as_ref().unwrap().layers.iter().map(|layer| layer.provider.to_string()).collect();
    assert_eq!(drawn, vec![first.clone()], "F1 drew other observers than the selected one");
    sim.world_mut().resource_mut::<DeveloperTools>().cycle_observed_geometry_provider(-1);
    collect_observed_geometry(sim.world_mut());
    assert_eq!(
        sim.world().resource::<ObservedGeometryFrame>().in_view(view).unwrap().report.as_ref().unwrap().layers.len(),
        headless.len(),
        "a step back from the first observer draws them all again"
    );

    layer_on(&mut sim, false);
    collect_observed_geometry(sim.world_mut());
    assert!(sim.world().resource::<ObservedGeometryFrame>().views.is_empty(), "with the layer off, nothing is asked");
}

/// The providers a full observation answered, in its order.
fn catalog_names(layers: &[ambition_platformer2d::geometry_observation::ObservedLayer]) -> Vec<String> {
    layers.iter().map(|layer| layer.provider.to_string()).collect()
}

/// Two live rooms retain separate subjects and authority. Two views of one
/// room can also ask about different bodies without sharing a report.
#[test]
fn a_two_room_geometry_lab_keeps_each_views_subject_and_refuses_a_foreign_body() {
    use ambition_platformer2d::{characters::control::PlayerSlot, sim_view::{LocalView, ResolvedViewSubject}};
    use ambition_platformer2d::sim_view::geometry_inspection::{geometry_subjects, inspect_geometry};
    let (mut sim, held) = crate::two_players_two_live_rooms::alice_leaves_bob_in(
        "portal_lab", "central_hub_complex", Some(PlayerSlot(1)), crate::common::walk_through_the_door_to);
    sim.step_n(crate::common::base(), 2);
    layer_on(&mut sim, true);
    collect_observed_geometry(sim.world_mut());
    let world = sim.world_mut();
    let reports = &world.resource::<ObservedGeometryFrame>().views;
    assert_eq!(reports.len(), 2, "the rooms have separate views: {reports:?}");
    for shown in reports {
        let report = shown.report.as_ref().expect("each view has a valid context");
        assert_eq!(report.context.room_instance, shown.room.ordinal());
        assert!(report.context.subject.is_some(), "each view has its own subject: {report:?}");
        assert!(report.layers.iter().all(|layer| matches!(layer.answer, ObserverAnswer::Observed { .. })), "{report:?}");
    }
    let subjects = geometry_subjects(world);
    let alice = world.query_filtered::<Entity, With<PrimaryPlayer>>().single(world).unwrap();
    let bob = world.query::<(Entity, &ambition_platformer2d::combat::components::FeatureId)>()
        .iter(world).find(|(_, id)| id.0 == "ow1_bob").map(|(entity, _)| entity).expect("Bob stayed in the portal lab");
    let roots: Vec<_> = world.query_filtered::<(Entity, &LiveRoomInstance), With<RoomInstanceRoot>>()
        .iter(world).map(|(root, live)| (root, *live)).collect();
    let portal_root = roots.iter().find(|(_, room)| *room == held).unwrap().0;
    let wrong = inspect_geometry(world, ObservationQuery { room: Some(portal_root), subject: Some(alice) }, |_| true);
    assert!(wrong.unwrap_err().contains("not in this live room"));
    let portal = inspect_geometry(world, ObservationQuery { room: Some(portal_root), subject: Some(bob) }, |_| true).unwrap();
    let layer = portal.layers.iter().find(|layer| layer.provider == "portal.apertures").unwrap();
    assert!(matches!(&layer.answer, ObserverAnswer::Observed { geometry } if !geometry.is_empty()), "{layer:?}");
    let headless = portal.to_json();
    let bob_view = world.query_filtered::<(Entity, &ResolvedViewSubject), With<LocalView>>()
        .iter(world).find(|(_, subject)| subject.0 == Some(bob)).map(|(view, _)| view).unwrap();
    assert_eq!(world.resource::<ObservedGeometryFrame>().in_view(bob_view).unwrap().report.as_ref().unwrap().to_json(), headless);

    let kin = *world.get::<ambition_platformer2d::engine_core::BodyKinematics>(bob).unwrap();
    let other = world.spawn((kin, InRoomInstance(held),
        ambition_platformer2d::platformer::sim_id::SimId::singleton("geometry_lab", "other"))).id();
    let camera = world.get::<ambition_platformer2d::sim_view::camera_snapshot::ResolvedCameraSnapshot>(bob_view).unwrap().clone();
    let other_view = world.spawn((LocalView, camera, ResolvedViewSubject(Some(other)))).id();
    collect_observed_geometry(world);
    let feed = world.resource::<ObservedGeometryFrame>();
    let bob_subject = &feed.in_view(bob_view).unwrap().report.as_ref().unwrap().context.subject;
    let other_subject = &feed.in_view(other_view).unwrap().report.as_ref().unwrap().context.subject;
    assert_ne!(bob_subject, other_subject, "two views in one room keep their own query contexts");
    world.despawn(other_view);
    world.despawn(other);

    world.resource_mut::<DeveloperTools>().observed_geometry_subject = ambition_platformer2d::dev_tools::dev_tools::GeometrySubjectSelection::Room;
    collect_observed_geometry(world);
    let feed = world.resource::<ObservedGeometryFrame>();
    assert!(feed.views.iter().all(|shown| shown.report.as_ref().unwrap().context.subject.is_none()));
    assert!(feed.views.iter().any(|shown| shown.report.as_ref().unwrap().layers.iter().any(|layer|
        layer.provider == "navigation.graph" && matches!(&layer.answer, ObserverAnswer::Observed { geometry } if !geometry.is_empty()))),
        "the room view shows the graphs that navigating bodies use");

    let bob_id = subjects.iter().find(|(_, body)| *body == bob).unwrap().0.clone();
    world.resource_mut::<DeveloperTools>().observed_geometry_subject = ambition_platformer2d::dev_tools::dev_tools::GeometrySubjectSelection::Body(bob_id);
    collect_observed_geometry(world);
    let reports = &world.resource::<ObservedGeometryFrame>().views;
    assert_eq!(reports.iter().filter(|shown| shown.report.is_err()).count(), 1, "a foreign selection is explicit: {reports:?}");
}

#[test]
fn the_geometry_lab_observes_a_missed_swing_and_a_hit_on_the_practice_bag() {
    use ambition_app::AgentAction;
    use ambition_platformer2d::{characters::actor::{BodyCombat, CharacterForm}};
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_flag("intro_wake_seen", true);
    let mut sim = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options("intro_wake_room").with_save(save)).unwrap();
    sim.step_n(crate::common::base(), 30);
    sim.teleport_player((540.0, 328.0));
    sim.step(AgentAction { move_x: 1.0, ..crate::common::base() });
    let player = {
        let world = sim.world_mut();
        world.query_filtered::<Entity, With<PrimaryPlayer>>().single(world).unwrap()
    };
    let mut observed_miss = false;
    for tick in 0..60 {
        sim.step(AgentAction { attack: tick == 0, ..crate::common::base() });
        let world = sim.world_mut();
        observed_miss |= world.resource::<ambition_platformer2d::sim_view::CombatStrikesView>().strikes.iter()
            .any(|strike| strike.owner == player);
        assert!(!world.query::<(&CharacterForm, &BodyCombat)>().iter(world)
            .any(|(form, combat)| form.id() == "practice_sandbag" && combat.recently_struck()));
    }
    assert!(observed_miss, "the missed swing had a live hit volume");
    let mut saw_hit = false;
    let mut saw_shape = false;
    for tick in 0..150 {
        sim.step(AgentAction { move_x: if tick < 12 { -1.0 } else { 0.0 }, attack: tick >= 12 && tick % 20 == 0, ..crate::common::base() });
        let world = sim.world_mut();
        let room = world.get::<InRoomInstance>(player).unwrap().0;
        let root = world.query_filtered::<(Entity, &LiveRoomInstance), With<RoomInstanceRoot>>().iter(world)
            .find(|(_, live)| **live == room).unwrap().0;
        let report = ambition_platformer2d::sim_view::geometry_inspection::inspect_geometry(
            world, ObservationQuery { room: Some(root), subject: Some(player) }, |name| name == "combat.volumes").unwrap();
        let ObserverAnswer::Observed { geometry } = &report.layers[0].answer else { panic!("{report:?}") };
        saw_shape |= geometry.iter().any(|piece| piece.roles.iter().any(|role| role == "hit") && piece.state == ObservedState::Active);
        saw_hit |= world.query::<(&CharacterForm, &BodyCombat)>().iter(world)
            .any(|(form, combat)| form.id() == "practice_sandbag" && combat.recently_struck());
        if saw_hit { break; }
    }
    assert!(saw_shape && saw_hit, "the shared attack path has a reported shape ({saw_shape}) and lands a real hit ({saw_hit})");
}

/// F1 must reach the installed collector through the production host schedule.
#[test]
fn f1_collects_geometry_through_the_visible_apps_schedule() {
    use ambition_app::app::{build_visible_app_with, StartRoomMustResolve, StartRoomOverride, VisibleRenderMode};
    use bevy::prelude::*;
    use leafwing_input_manager::prelude::Buttonlike;
    let mut app = build_visible_app_with(VisibleRenderMode::NoWindow, false, |app| {
        app.insert_resource(StartRoomOverride(ROOM.into()));
        app.insert_resource(StartRoomMustResolve);
    });
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f64(1.0 / 60.0)));
    for _ in 0..120 { app.update(); }
    assert!(!app.world().resource::<DeveloperRuntimeState>().debug_enabled());
    {
        let mut tools = app.world_mut().resource_mut::<DeveloperTools>();
        tools.gizmos_enabled = true;
        tools.show_observed_geometry = true;
        tools.observed_geometry_only = Some("collision.composed".into());
    }
    Buttonlike::press(&KeyCode::F1, app.world_mut());
    app.update();
    Buttonlike::release(&KeyCode::F1, app.world_mut());
    app.update();
    let frame = app.world().resource::<ObservedGeometryFrame>();
    assert!(!frame.views.is_empty(), "F1 runs the registered collector");
    for view in &frame.views {
        let report = view.report.as_ref().unwrap();
        assert_eq!(report.context.room, ROOM);
        assert!(matches!(&report.layers[..], [layer] if layer.provider == "collision.composed"
            && matches!(&layer.answer, ObserverAnswer::Observed { geometry } if !geometry.is_empty())));
    }
    Buttonlike::press(&KeyCode::F1, app.world_mut());
    app.update();
    Buttonlike::release(&KeyCode::F1, app.world_mut());
    app.update();
    assert!(app.world().resource::<ObservedGeometryFrame>().views.is_empty());
}

#[test]
fn f1_shows_the_guitars_actual_carried_surface_with_default_settings() {
    use ambition_app::app::{build_visible_app_with, StartRoomMustResolve, StartRoomOverride, VisibleRenderMode};
    use ambition_platformer2d::{characters::actor::CharacterForm, engine_core as ae, mount::BackLedge};
    use bevy::prelude::*;
    use leafwing_input_manager::prelude::Buttonlike;
    let mut app = build_visible_app_with(VisibleRenderMode::NoWindow, false, |app| {
        app.insert_resource(StartRoomOverride("boss_main_stage".into()));
        app.insert_resource(StartRoomMustResolve);
    });
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f64(1.0 / 60.0)));
    for _ in 0..120 { app.update(); }
    Buttonlike::press(&KeyCode::F1, app.world_mut());
    app.update();
    Buttonlike::release(&KeyCode::F1, app.world_mut());
    app.update();
    let world = app.world_mut();
    let guitar = world.query::<(&CharacterForm, &ae::BodyKinematics, &BackLedge, Has<ae::Unmirrored>)>()
        .iter(world).find(|(form, ..)| form.id() == "npc_full_stack")
        .and_then(|(_, kin, back, unmirrored)| back.slope_line(kin, unmirrored))
        .expect("the guitar has a walkable surface");
    let feed = world.resource::<ObservedGeometryFrame>();
    assert!(!feed.views.is_empty(), "F1 collects with the shipped settings");
    let pieces: Vec<_> = feed.views.iter().flat_map(|view| &view.report.as_ref().unwrap().layers)
        .filter(|layer| layer.provider == "collision.for_subject")
        .flat_map(|layer| match &layer.answer { ObserverAnswer::Observed { geometry } => geometry.as_slice(), _ => &[] })
        .filter(|piece| piece.key == "mount_slope").collect();
    assert!(!pieces.is_empty(), "F1 reports the guitar: {feed:?}");
    assert!(pieces.iter().any(|piece| matches!(&piece.shape, ObservedShape::Polyline { points }
        if points.len() == guitar.len() && points.iter().zip(&guitar).all(|(a, b)|
            (ae::Vec2::from_array(*a) - *b).length() < 2.0))), "the outline follows the real guitar: {pieces:?}");
    assert!(pieces.iter().all(|piece| piece.roles.iter().any(|role| role == "drawn_by_art")));
    world.resource_mut::<DeveloperTools>().apply_debug_view_mode(
        ambition_platformer2d::dev_tools::dev_tools::DebugViewMode::Combat, false);
    app.update();
    let feed = app.world().resource::<ObservedGeometryFrame>();
    assert!(feed.views.iter().flat_map(|view| &view.report.as_ref().unwrap().layers)
        .any(|layer| matches!(&layer.answer, ObserverAnswer::Observed { geometry }
            if layer.provider == "collision.for_subject" && geometry.iter().any(|piece| piece.key == "mount_slope"))),
        "combat keeps the real climbing surfaces: {feed:?}");
}
