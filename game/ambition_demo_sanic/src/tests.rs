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

    // The raised ramp, complete loop, and runout are ONE valid rideable
    // route. The loop returns to its entry point after a full revolution, but
    // at a later arc length; the continuation then descends to the floor.
    let loop_chain = room
        .world
        .chains
        .iter()
        .find(|c| c.name == "sanic_loop")
        .expect("the sanic ramp+loop+runout chain is present");
    assert_eq!(
        loop_chain.points.len(),
        1 + LOOP_RAMP_SEGMENTS + LOOP_SEGMENTS + LOOP_RUNOUT_SEGMENTS
    );
    assert!(
        !loop_chain.closed,
        "the route is open even though the loop body makes a full revolution"
    );
    assert!(
        loop_chain.validate().is_empty(),
        "the generated full-loop route is valid: {:?}",
        loop_chain.validate()
    );

    assert_eq!(
        loop_chain.depth_lanes.len(),
        loop_chain.segment_count(),
        "the 2.5D loop authors one depth lane per segment"
    );
    assert_eq!(
        loop_chain.segment_depth(LOOP_ENTRY_POINT_INDEX - 1),
        -1,
        "the inbound ramp passes behind the player at the crossover"
    );
    assert_eq!(
        loop_chain.segment_depth(LOOP_ENTRY_POINT_INDEX),
        1,
        "the lower front shoulder occludes the player entering the loop"
    );
    assert_eq!(
        loop_chain.segment_depth(LOOP_ENTRY_POINT_INDEX + LOOP_SEGMENTS / 2),
        0,
        "the upper loop body remains on the ordinary track plane"
    );
    assert_eq!(
        loop_chain.segment_depth(LOOP_CLOSURE_POINT_INDEX),
        1,
        "the outbound runout occupies the foreground lane"
    );
    assert_eq!(loop_chain.junctions.len(), 3);
    let loop_mouth = loop_chain
        .junctions
        .iter()
        .find(|junction| {
            junction.ports
                == vec![
                    ae::SurfacePort::local(LOOP_ENTRY_POINT_INDEX),
                    ae::SurfacePort::local(LOOP_CLOSURE_POINT_INDEX),
                ]
        })
        .expect("the two loop-mouth occurrences form an explicit switch");
    assert_eq!(
        loop_mouth.ports.len(),
        2,
        "the loop mouth has exactly its inbound and outbound route occurrences"
    );
    let floor_route = room
        .world
        .chains
        .iter()
        .find(|chain| chain.name == "sanic_floor_route")
        .expect("momentum bodies have a floor guide that can branch into the ramp");
    assert_eq!(floor_route.points.len(), 4);
    assert!(
        room.world.validate_surface_junctions().is_empty(),
        "every local and cross-chain route port resolves to the same projected point: {:?}",
        room.world.validate_surface_junctions()
    );
    assert!(
        loop_chain.junctions.iter().any(|junction| {
            junction.ports == vec![ae::SurfacePort::local(0), ae::SurfacePort::chain(1, 1)]
        }),
        "the tiled floor and the ramp are one steerable route junction"
    );

    let ramp_start = loop_chain.points[0];
    let entry = loop_chain.points[LOOP_ENTRY_POINT_INDEX];
    let closure = loop_chain.points[LOOP_CLOSURE_POINT_INDEX];
    let exit = loop_chain.points[LOOP_EXIT_POINT_INDEX];
    let overpass_end = loop_chain.points[LOOP_CLOSURE_POINT_INDEX + LOOP_OVERPASS_SEGMENTS];
    assert!(
        entry.distance(closure) < 1.0e-2,
        "a complete loop returns to its entry world point: entry={entry:?}, closure={closure:?}"
    );

    let ramp_tangent = (entry - loop_chain.points[LOOP_ENTRY_POINT_INDEX - 1]).normalize_or_zero();
    let loop_entry_tangent =
        (loop_chain.points[LOOP_ENTRY_POINT_INDEX + 1] - entry).normalize_or_zero();
    assert!(
        ramp_tangent.dot(loop_entry_tangent) > 0.995,
        "the ramp must meet the loop without a tangent edge: ramp={ramp_tangent:?}, loop={loop_entry_tangent:?}"
    );

    let loop_closure_tangent =
        (closure - loop_chain.points[LOOP_CLOSURE_POINT_INDEX - 1]).normalize_or_zero();
    let runout_tangent =
        (loop_chain.points[LOOP_CLOSURE_POINT_INDEX + 1] - closure).normalize_or_zero();
    assert!(
        loop_closure_tangent.dot(runout_tangent) > 0.995,
        "the completed loop must flow into its runout without a tangent edge: loop={loop_closure_tangent:?}, runout={runout_tangent:?}"
    );

    let floor_top = floor.aabb.min.y;
    assert!((ramp_start.y - floor_top).abs() < 1.0e-3);
    assert!(entry.y < floor_top - 60.0, "the loop is visibly raised");
    assert!((exit.y - floor_top).abs() < 1.0e-3);
    assert!(
        overpass_end.x > LOOP_CENTER_X + LOOP_RADIUS + 80.0,
        "the flat foreground deck must clear the loop before descending"
    );
    assert!(
        (overpass_end.y - closure.y).abs() < 1.0e-3,
        "the crossover deck must stay flat while it clears the back rail"
    );
    assert!(
        exit.x > closure.x + LOOP_RADIUS * 3.0,
        "the runout must carry the rider clear of the completed loop"
    );

    // The loop samples all four quadrants around the label/visual center. This
    // rejects the earlier three-quarter-loop compromise.
    let loop_points = &loop_chain.points[LOOP_ENTRY_POINT_INDEX..=LOOP_CLOSURE_POINT_INDEX];
    let min_x = loop_points
        .iter()
        .map(|p| p.x)
        .fold(f32::INFINITY, f32::min);
    let max_x = loop_points
        .iter()
        .map(|p| p.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = loop_points
        .iter()
        .map(|p| p.y)
        .fold(f32::INFINITY, f32::min);
    let max_y = loop_points
        .iter()
        .map(|p| p.y)
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(max_x - min_x > LOOP_RADIUS * 1.99);
    assert!(max_y - min_y > LOOP_RADIUS * 1.99);

    // Local smoothness oracles cover both repeated-world-point visits. The
    // route may touch itself at the bottom, but neither arc-length join may be
    // a polygonal collision lip.
    for joint in [LOOP_ENTRY_POINT_INDEX, LOOP_CLOSURE_POINT_INDEX] {
        for i in (joint - 2)..=(joint + 2) {
            let before = (loop_chain.points[i] - loop_chain.points[i - 1]).normalize_or_zero();
            let after = (loop_chain.points[i + 1] - loop_chain.points[i]).normalize_or_zero();
            assert!(
                before.dot(after) > 0.99,
                "full-loop joint {i} is too sharp: before={before:?}, after={after:?}"
            );
        }
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
fn momentum_body_crosses_the_ramp_full_loop_and_runout_without_stalling() {
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
    let closure_s: f32 = (0..LOOP_CLOSURE_POINT_INDEX)
        .map(|segment| chain.segment_length(segment))
        .sum();
    let start_s = entry_s - 30.0;
    let frame = chain.frame_at(start_s);
    let speed = 1000.0;
    let mut body = ae::surface::SurfaceBody {
        pos: frame.point + frame.normal * 16.0,
        vel: frame.tangent * speed,
        radius: 16.0,
        depth_lane: chain.segment_depth(frame.segment),
        motion: ae::surface::SurfaceMotion::Riding {
            on: ae::surface::SurfaceRef::Chain(0),
            s: start_s,
            v_t: speed,
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

    let mut reached_runout = false;
    for _ in 0..180 {
        ae::surface::step_surface_body(
            &mut body,
            &room.world,
            &params,
            ae::Vec2::new(0.0, 1450.0),
            ae::surface::SurfaceInputs::default(),
            1.0 / 60.0,
            None,
        );
        let ae::surface::SurfaceMotion::Riding { s, .. } = body.motion else {
            panic!("the continuous ramp/full-loop route must not shed the rider");
        };
        if s > closure_s + 120.0 {
            reached_runout = true;
            break;
        }
    }
    assert!(
        reached_runout,
        "the rider must complete the full loop and enter the runout: entry_s={entry_s}, closure_s={closure_s}, motion={:?}",
        body.motion
    );
}

#[test]
fn authored_sanic_speed_clears_the_depth_crossover_before_any_launch() {
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
    let closure_s: f32 = (0..LOOP_CLOSURE_POINT_INDEX)
        .map(|segment| chain.segment_length(segment))
        .sum();
    let frame = chain.frame_at(entry_s);
    let speed = 1120.0;
    let mut body = ae::surface::SurfaceBody {
        pos: frame.point + frame.normal * 16.0,
        vel: frame.tangent * speed,
        radius: 16.0,
        depth_lane: chain.segment_depth(frame.segment),
        motion: ae::surface::SurfaceMotion::Riding {
            on: ae::surface::SurfaceRef::Chain(0),
            s: entry_s,
            v_t: speed,
        },
    };
    let params = ae::surface::MomentumParams {
        ground_accel: 900.0,
        top_speed: 1200.0,
        jump_speed: 700.0,
        ..Default::default()
    };

    let clear_s = closure_s + 160.0;
    for _ in 0..180 {
        ae::surface::step_surface_body(
            &mut body,
            &room.world,
            &params,
            ae::Vec2::new(0.0, 1450.0),
            ae::surface::SurfaceInputs {
                run: 1.0,
                steer: ae::Vec2::ZERO,
                jump_pressed: false,
            },
            1.0 / 60.0,
            None,
        );
        let ae::surface::SurfaceMotion::Riding { s, .. } = body.motion else {
            panic!("authored Sanic speed must stay attached through the loop mouth; body={body:?}");
        };
        if s > clear_s {
            return;
        }
    }
    panic!(
        "authored Sanic speed never cleared the foreground overpass; motion={:?}",
        body.motion
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

#[test]
fn loop_mouth_steering_selects_the_up_or_down_route_in_both_directions() {
    let room = sanic_speedway();
    let chain = room
        .world
        .chains
        .iter()
        .find(|chain| chain.name == "sanic_loop")
        .expect("the speedway owns its ramp+loop route");
    let entry_s = chain.arc_at_vertex(LOOP_ENTRY_POINT_INDEX);
    let closure_s = chain.arc_at_vertex(LOOP_CLOSURE_POINT_INDEX);
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

    let step_from = |s: f32, v_t: f32, steer: ae::Vec2| {
        let frame = chain.frame_at(s);
        let mut body = ae::surface::SurfaceBody {
            pos: frame.point + frame.normal * 16.0,
            vel: frame.tangent * v_t,
            radius: 16.0,
            depth_lane: chain.segment_depth(frame.segment),
            motion: ae::surface::SurfaceMotion::Riding {
                on: ae::surface::SurfaceRef::Chain(0),
                s,
                v_t,
            },
        };
        ae::surface::step_surface_body(
            &mut body,
            &room.world,
            &params,
            ae::Vec2::new(0.0, 1450.0),
            ae::surface::SurfaceInputs {
                run: v_t.signum(),
                steer,
                jump_pressed: false,
            },
            1.0 / 60.0,
            None,
        );
        body
    };

    let up_into_loop = step_from(entry_s - 3.0, 600.0, ae::Vec2::new(1.0, -1.0));
    let ae::surface::SurfaceMotion::Riding { s, .. } = up_into_loop.motion else {
        panic!("the authored route switch guides the rider instead of launching");
    };
    assert!(s > entry_s && s < closure_s, "up-right enters the loop");

    let down_to_runout = step_from(entry_s - 3.0, 600.0, ae::Vec2::new(1.0, 1.0));
    let ae::surface::SurfaceMotion::Riding { s, .. } = down_to_runout.motion else {
        panic!("the authored route switch guides the rider instead of launching");
    };
    assert!(s > closure_s, "down-right selects the lower/outbound route");

    let up_into_reverse_loop = step_from(closure_s + 3.0, -600.0, ae::Vec2::new(-1.0, -1.0));
    let ae::surface::SurfaceMotion::Riding { s, .. } = up_into_reverse_loop.motion else {
        panic!("the authored route switch guides the rider instead of launching");
    };
    assert!(
        s > entry_s && s < closure_s,
        "up-left enters the loop in reverse"
    );

    let down_to_ramp = step_from(closure_s + 3.0, -600.0, ae::Vec2::new(-1.0, 1.0));
    let ae::surface::SurfaceMotion::Riding { s, .. } = down_to_ramp.motion else {
        panic!("the authored route switch guides the rider instead of launching");
    };
    assert!(s < entry_s, "down-left selects the descending ramp");

    let forward_default = step_from(closure_s - 3.0, 600.0, ae::Vec2::X);
    let ae::surface::SurfaceMotion::Riding { s, .. } = forward_default.motion else {
        panic!("horizontal input preserves the authored forward exit");
    };
    assert!(s > closure_s, "holding Right exits after one forward lap");

    let reverse_default = step_from(entry_s + 3.0, -600.0, -ae::Vec2::X);
    let ae::surface::SurfaceMotion::Riding { s, .. } = reverse_default.motion else {
        panic!("horizontal input preserves the authored reverse exit");
    };
    assert!(s < entry_s, "holding Left exits after one reverse lap");
}

#[test]
fn floor_route_steering_enters_the_ramp_without_jumping() {
    let room = sanic_speedway();
    let floor_index = room
        .world
        .chains
        .iter()
        .position(|chain| chain.name == "sanic_floor_route")
        .expect("the speedway owns a momentum floor route");
    let floor = &room.world.chains[floor_index];
    let branch_s = floor.arc_at_vertex(1);
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

    let step = |steer: ae::Vec2| {
        let frame = floor.frame_at(branch_s - 3.0);
        let mut body = ae::surface::SurfaceBody {
            pos: frame.point + frame.normal * 16.0,
            vel: frame.tangent * 600.0,
            radius: 16.0,
            depth_lane: floor.segment_depth(frame.segment),
            motion: ae::surface::SurfaceMotion::Riding {
                on: ae::surface::SurfaceRef::Chain(floor_index),
                s: branch_s - 3.0,
                v_t: 600.0,
            },
        };
        ae::surface::step_surface_body(
            &mut body,
            &room.world,
            &params,
            ae::Vec2::new(0.0, 1450.0),
            ae::surface::SurfaceInputs {
                run: 1.0,
                steer,
                jump_pressed: false,
            },
            1.0 / 60.0,
            None,
        );
        body
    };

    let raised = step(ae::Vec2::new(1.0, -1.0));
    assert!(
        matches!(
            raised.motion,
            ae::surface::SurfaceMotion::Riding {
                on: ae::surface::SurfaceRef::Chain(0),
                ..
            }
        ),
        "up-right transfers directly from the floor guide onto the ramp: {raised:?}"
    );

    let flat = step(ae::Vec2::X);
    assert!(
        matches!(
            flat.motion,
            ae::surface::SurfaceMotion::Riding {
                on: ae::surface::SurfaceRef::Chain(index),
                ..
            } if index == floor_index
        ),
        "plain Right preserves the flat route: {flat:?}"
    );
}

#[test]
fn reverse_loop_exits_after_one_revolution_instead_of_reentering_forever() {
    let room = sanic_speedway();
    let chain = room
        .world
        .chains
        .iter()
        .find(|chain| chain.name == "sanic_loop")
        .expect("the speedway owns its ramp+loop route");
    let entry_s = chain.arc_at_vertex(LOOP_ENTRY_POINT_INDEX);
    let closure_s = chain.arc_at_vertex(LOOP_CLOSURE_POINT_INDEX);
    let start_s = closure_s + 180.0;
    let frame = chain.frame_at(start_s);
    let mut body = ae::surface::SurfaceBody {
        pos: frame.point + frame.normal * 16.0,
        vel: frame.tangent * -900.0,
        radius: 16.0,
        depth_lane: chain.segment_depth(frame.segment),
        motion: ae::surface::SurfaceMotion::Riding {
            on: ae::surface::SurfaceRef::Chain(0),
            s: start_s,
            v_t: -900.0,
        },
    };
    // Isolate route topology from feel tuning: this oracle asks whether the
    // authored reverse continuation exits after one lap, not whether a
    // particular speed/stick-factor combination sheds from a convex ramp.
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

    let mut entered_loop = false;
    for _ in 0..420 {
        ae::surface::step_surface_body(
            &mut body,
            &room.world,
            &params,
            ae::Vec2::new(0.0, 1450.0),
            ae::surface::SurfaceInputs {
                run: -1.0,
                steer: ae::Vec2::new(-1.0, 0.0),
                jump_pressed: false,
            },
            1.0 / 60.0,
            None,
        );
        match body.motion {
            ae::surface::SurfaceMotion::Riding { s, .. } => {
                entered_loop |= s > entry_s + 100.0 && s < closure_s - 100.0;
                if entered_loop && s < entry_s - 0.5 {
                    return;
                }
            }
            ae::surface::SurfaceMotion::Airborne => {
                panic!(
                    "the topology oracle uses sticky, slope-free tuning and must remain attached; body={body:?}"
                );
            }
        }
    }
    panic!(
        "reverse traversal must leave after one revolution instead of re-entering; body={body:?}"
    );
}
