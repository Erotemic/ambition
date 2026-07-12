//! Unit tests for the standalone Sanic content and rules plugin.

use super::*;

#[test]
fn sanic_demo_content_plugin_installs() {
    let mut app = App::new();
    add_demo_content(&mut app);

    let music = ambition::actors::session::data::authored_music_registry();
    assert_eq!(music.default_track, "you_are_too_slow");
    assert_eq!(music.tracks.len(), 1);
    assert_eq!(
        ambition::actors::session::data::authored_sfx_registry().sample_rate,
        44_100
    );
    let catalog = ambition::actors::character_roster::catalog();
    assert!(catalog.characters.contains_key(SANIC_CHARACTER_ID));
    assert!(catalog.characters.contains_key(SUPER_SANIC_CHARACTER_ID));
}

/// The oracle: the momentum showcase room composes through the umbrella
/// surface alone — floor geometry present, the Sonic loop validates, and the
/// spawn sits inside the room bounds.
#[test]
fn sanic_speedway_composes_through_the_umbrella() {
    let room = sanic_speedway();
    assert_eq!(room.id, SPEEDWAY_ROOM_ID);

    // Solid floor + visible landmarks made it into the world.
    assert!(
        room.world.blocks.iter().any(|b| b.name == "speedway_floor"),
        "the speedway floor block is present"
    );
    let floor = room
        .world
        .blocks
        .iter()
        .find(|block| block.name == "speedway_floor")
        .unwrap();
    assert!(
        matches!(&floor.id.source, ae::GeoSource::TileLayer { .. }),
        "the procedural ground opts into the canonical tiled terrain path"
    );
    for expected in [
        "start_gantry",
        "distance_marker_1",
        "speed_booster",
        "finish_warning_spikes",
        "finish_tower",
    ] {
        assert!(
            room.world.blocks.iter().any(|b| b.name == expected),
            "landmark '{expected}' is present"
        );
    }
    assert_eq!(
        room.metadata.visual_profile.parallax_theme.as_deref(),
        Some("skybridge"),
        "the speedway borrows Ambition's skybridge parallax stack"
    );
    assert!(
        room.debug_labels
            .iter()
            .any(|label| label.payload.text == "LOOP"),
        "the speedway labels its major features in world space"
    );
    assert!(
        room.debug_labels
            .iter()
            .any(|label| label.payload.text == "1200"),
        "distance labels make displacement measurable"
    );

    // The raised entry ramp and loop are ONE valid rideable route. The ramp's
    // final tangent agrees with the loop's first tangent, so the join is not a
    // launch/reattach edge. The open release points down and right, lands before
    // the raised ramp, and leaves enough clearance to run underneath it.
    let loop_chain = room
        .world
        .chains
        .iter()
        .find(|c| c.name == "sanic_loop")
        .expect("the sanic ramp+loop chain is present");
    assert_eq!(
        loop_chain.points.len(),
        1 + LOOP_RAMP_SEGMENTS + LOOP_SEGMENTS
    );
    assert!(
        !loop_chain.closed,
        "the showcase loop must release rather than wrap forever"
    );
    assert!(
        loop_chain.validate().is_empty(),
        "the generated ramp+loop route is valid: {:?}",
        loop_chain.validate()
    );

    let ramp_start = loop_chain.points[0];
    let entry = loop_chain.points[LOOP_ENTRY_POINT_INDEX];
    let exit = loop_chain.points[LOOP_EXIT_POINT_INDEX];
    let ramp_tangent = (entry - loop_chain.points[LOOP_ENTRY_POINT_INDEX - 1]).normalize_or_zero();
    let loop_tangent = (loop_chain.points[LOOP_ENTRY_POINT_INDEX + 1] - entry).normalize_or_zero();
    assert!(
        ramp_tangent.dot(loop_tangent) > 0.995,
        "the ramp must meet the loop without a tangent edge: ramp={ramp_tangent:?}, loop={loop_tangent:?}"
    );

    let exit_tangent = (exit - loop_chain.points[LOOP_EXIT_POINT_INDEX - 1]).normalize_or_zero();
    assert!(
        exit_tangent.x > 0.35 && exit_tangent.y > 0.7,
        "the open endpoint must send a right-moving rider steeply down toward the floor: {exit_tangent:?}"
    );
    let floor_top = floor.aabb.min.y;
    let straight_line_landing_x = exit.x + (floor_top - exit.y) * exit_tangent.x / exit_tangent.y;
    assert!(
        straight_line_landing_x < ramp_start.x,
        "even the gravity-free exit ray must reach the floor before the raised ramp: landing_x={straight_line_landing_x}, ramp_start={ramp_start:?}"
    );
    let standing_radius = 16.0;
    assert!(
        floor_top - ramp_start.y > standing_radius * 2.0,
        "a floor rider must pass underneath the raised ramp with full-body clearance: floor_top={floor_top}, ramp_start={ramp_start:?}"
    );

    // A local smoothness oracle catches the exact class of coarse edge that
    // originally stranded the rider. Every adjacent pair around the ramp/loop
    // join must turn gently rather than presenting a polygonal launch lip.
    for joint in (LOOP_ENTRY_POINT_INDEX - 2)..=(LOOP_ENTRY_POINT_INDEX + 2) {
        let before = (loop_chain.points[joint] - loop_chain.points[joint - 1]).normalize_or_zero();
        let after = (loop_chain.points[joint + 1] - loop_chain.points[joint]).normalize_or_zero();
        assert!(
            before.dot(after) > 0.99,
            "ramp/loop joint {joint} is too sharp: before={before:?}, after={after:?}"
        );
    }

    // Spawn is inside the room bounds (not floating/falling on load).
    let s = room.world.spawn;
    assert!(
        s.x >= 0.0 && s.x <= room.world.size.x && s.y >= 0.0 && s.y <= room.world.size.y,
        "spawn {s:?} is inside room bounds {:?}",
        room.world.size
    );
}

#[test]
fn momentum_body_crosses_the_smoothed_ramp_loop_join_without_stalling() {
    let room = sanic_speedway();
    let chain = room
        .world
        .chains
        .iter()
        .find(|chain| chain.name == "sanic_loop")
        .expect("the speedway owns its ramp+loop route");
    let entry_s: f32 = (0..LOOP_ENTRY_POINT_INDEX)
        .map(|segment| chain.segment_length(segment))
        .sum();
    let start_s = entry_s - 30.0;
    let frame = chain.frame_at(start_s);
    let mut body = ae::surface::SurfaceBody {
        pos: frame.point + frame.normal * 16.0,
        vel: frame.tangent * 600.0,
        radius: 16.0,
        motion: ae::surface::SurfaceMotion::Riding {
            on: ae::surface::SurfaceRef::Chain(0),
            s: start_s,
            v_t: 600.0,
        },
    };
    let params = ae::surface::MomentumParams {
        ground_accel: 0.0,
        brake: 0.0,
        friction: 0.0,
        slope_factor: 0.0,
        top_speed: 2000.0,
        air_accel: 0.0,
        stick_factor: 1000.0,
        min_stick_speed: 0.0,
        ..Default::default()
    };

    for _ in 0..30 {
        ae::surface::step_surface_body(
            &mut body,
            &room.world,
            &params,
            ae::Vec2::new(0.0, 1450.0),
            ae::surface::SurfaceInputs::default(),
            1.0 / 60.0,
            None,
        );
    }

    let ae::surface::SurfaceMotion::Riding { s, .. } = body.motion else {
        panic!("the smoothed ramp/loop join must not shed the rider");
    };
    assert!(
        s > entry_s + 200.0,
        "the rider must advance well into the loop instead of freezing at the ramp edge: entry_s={entry_s}, s={s}"
    );
}

#[test]
fn crossing_a_visible_distance_marker_emits_the_standard_sfx_message() {
    let mut app = App::new();
    app.add_message::<ambition::sfx::SfxMessage>();
    app.world_mut().spawn((
        ambition::actors::actor::PrimaryPlayer,
        ae::BodyKinematics {
            pos: ae::Vec2::new(SPEED_MARKER_XS[0] + 1.0, 0.0),
            ..Default::default()
        },
    ));
    app.world_mut().spawn(SanicActState::default());
    app.add_systems(bevy::app::Update, emit_sanic_milestone_sfx);

    app.update();

    let messages = app
        .world()
        .resource::<bevy::prelude::Messages<ambition::sfx::SfxMessage>>();
    assert!(
        messages
            .iter_current_update_messages()
            .any(|message| matches!(message, ambition::sfx::SfxMessage::Dash { .. })),
        "the first visual marker emits the first standard diagnostic cue"
    );
    let mut q = app.world_mut().query::<&SanicActState>();
    assert_eq!(q.single(app.world()).unwrap().next_milestone, 1);
}

#[test]
fn semantic_utility_toggles_both_sanic_forms_and_is_consumed() {
    let mut app = App::new();
    app.add_message::<ambition::sfx::SfxMessage>();
    let entity = app
        .world_mut()
        .spawn((
            ambition::characters::brain::ActorControl::default(),
            ambition::characters::actor::WornCharacter::new(SANIC_CHARACTER_ID),
            ae::BodyKinematics::default(),
        ))
        .id();
    app.insert_resource(ambition::platformer::markers::ControlledSubject(Some(
        entity,
    )));
    app.add_systems(bevy::app::Update, toggle_sanic_form);

    app.world_mut()
        .get_mut::<ambition::characters::brain::ActorControl>(entity)
        .unwrap()
        .0
        .fly_toggle_pressed = true;
    app.update();
    assert_eq!(
        app.world()
            .get::<ambition::characters::actor::WornCharacter>(entity)
            .unwrap()
            .id(),
        SUPER_SANIC_CHARACTER_ID
    );
    assert!(
        !app.world()
            .get::<ambition::characters::brain::ActorControl>(entity)
            .unwrap()
            .0
            .fly_toggle_pressed,
        "the transformation consumes Utility before generic flight can see it"
    );

    app.world_mut()
        .get_mut::<ambition::characters::brain::ActorControl>(entity)
        .unwrap()
        .0
        .fly_toggle_pressed = true;
    app.update();
    assert_eq!(
        app.world()
            .get::<ambition::characters::actor::WornCharacter>(entity)
            .unwrap()
            .id(),
        SANIC_CHARACTER_ID
    );
}

/// **The D-C pattern, end to end.** `SanicRulesPlugin::hosted()` ticks the act
/// timer only inside the Sanic rooms; `::global()` ticks it everywhere. The
/// mode-owner entity is `spawn_mode_scoped`, so the engine tears it down when
/// the active room leaves the mode — this demo writes no teardown code.
#[test]
fn hosted_rules_run_only_in_sanic_rooms_and_global_rules_run_everywhere() {
    use ambition::bevy::ecs::system::RunSystemOnce as _;
    use ambition::world::rooms::{ActiveRoomMetadata, RoomMetadata};

    fn elapsed(app: &mut App) -> Option<f32> {
        let mut q = app.world_mut().query::<&SanicActState>();
        q.iter(app.world()).next().map(|s| s.elapsed)
    }
    fn shell(rules: SanicRulesPlugin, mode: Option<&str>) -> App {
        let mut app = App::new();
        ambition::engine::add_headless_foundation(&mut app);
        // The focused rules-only shell omits PlatformerEnginePlugins, whose
        // SimCoreResourcesPlugin normally registers the shared SFX message.
        app.add_message::<ambition::sfx::SfxMessage>();
        app.insert_resource(ActiveRoomMetadata(RoomMetadata {
            mode: mode.map(str::to_string),
            ..Default::default()
        }));
        app.insert_resource(ambition::time::WorldTime {
            scaled_dt: 0.5,
            ..Default::default()
        });
        app.add_plugins(rules);
        app
    }

    // HOSTED, inside a `sanic` room: the mode owner spawns and the act ticks.
    // `.chain()` puts a sync point between spawn and tick, so the owner exists
    // in time to tick on its own first frame: two frames = two ticks.
    let mut app = shell(SanicRulesPlugin::hosted(), Some(SANIC_MODE));
    app.update();
    app.update();
    assert_eq!(elapsed(&mut app), Some(1.0), "hosted rules tick in-mode");

    // HOSTED, in one of Ambition's own rooms: nothing spawns, nothing ticks.
    let mut app = shell(SanicRulesPlugin::hosted(), None);
    app.update();
    app.update();
    assert_eq!(elapsed(&mut app), None, "hosted rules sleep out of mode");

    // GLOBAL (the demo IS the game): the rules run with no mode at all.
    let mut app = shell(SanicRulesPlugin::global(), None);
    app.update();
    app.update();
    assert_eq!(
        elapsed(&mut app),
        Some(1.0),
        "standalone rules need no mode"
    );

    // The mode owner really is mode-scoped: the engine's own sweep retires it.
    let mut app = shell(SanicRulesPlugin::hosted(), Some(SANIC_MODE));
    app.update();
    app.update();
    assert!(elapsed(&mut app).is_some());
    app.insert_resource(ActiveRoomMetadata::default()); // left the Sanic rooms
    app.world_mut()
        .run_system_once(ambition::runtime::despawn_departed_mode_entities)
        .expect("the engine's mode sweep runs");
    assert_eq!(
        elapsed(&mut app),
        None,
        "leaving the mode tears the act state down — no demo teardown code"
    );
}

/// The D-C hosting oracle: a demo's room claims its mode, and the run
/// condition that wakes a hosted ruleset inside it reaches this crate
/// through the `ambition` umbrella alone. If gating a hosted demo ever
/// needs a lower `ambition_*` crate, it fails to compile HERE.
///
/// The condition is evaluated directly rather than through `.run_if` on a
/// bespoke marker resource: a crate whose manifest names only `ambition`
/// cannot `#[derive(Resource)]`, because bevy's derive macros resolve
/// `bevy_ecs` through the CONSUMER's manifest and a re-export does not
/// satisfy them. The `.run_if` wiring itself is pinned in
/// `ambition_runtime/tests/mode_scope.rs`.
#[test]
fn the_speedway_claims_the_sanic_mode_and_wakes_a_hosted_ruleset() {
    use ambition::bevy::ecs::system::RunSystemOnce as _;
    use ambition::runtime::in_mode;
    use ambition::world::rooms::ActiveRoomMetadata;

    let room = sanic_speedway();
    assert_eq!(room.metadata.mode.as_deref(), Some(SANIC_MODE));

    let mut app = App::new();
    app.insert_resource(ActiveRoomMetadata(room.metadata.clone()));
    let awake = app
        .world_mut()
        .run_system_once(in_mode(SANIC_MODE))
        .expect("the mode condition runs");
    assert!(awake, "a hosted Sanic ruleset wakes inside the speedway");

    // Ambition's own rooms carry no mode, so the demo's rules sleep there.
    app.insert_resource(ActiveRoomMetadata::default());
    let awake = app
        .world_mut()
        .run_system_once(in_mode(SANIC_MODE))
        .expect("the mode condition runs");
    assert!(!awake, "and it sleeps in a room that claims no mode");
}
