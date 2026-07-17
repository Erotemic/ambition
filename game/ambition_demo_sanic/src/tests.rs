//! Unit tests for the standalone Sanic content and rules plugin.

mod speedway_oracles;

use super::*;

#[test]
fn sanic_demo_content_plugin_installs() {
    let mut app = App::new();
    add_demo_content(&mut app);

    let audio = app
        .world()
        .resource::<ambition::audio::catalog::AudioCatalogRegistry>();
    let music = audio
        .music_for(provider::SANIC_EXPERIENCE)
        .expect("Sanic music fragment");
    assert_eq!(music.default_track, "you_are_too_slow");
    assert_eq!(music.tracks.len(), 1);
    assert_eq!(
        audio
            .sfx_for(provider::SANIC_EXPERIENCE)
            .expect("Sanic SFX fragment")
            .sample_rate,
        44_100
    );
    let catalog = app
        .world()
        .resource::<ambition::characters::actor::character_catalog::CharacterCatalog>();
    assert!(catalog.get(SANIC_CHARACTER_ID).is_some());
    assert!(catalog.get(SUPER_SANIC_CHARACTER_ID).is_some());
}

/// The super-form transform-cue latch does not leak across a session turnover:
/// a session that ended super must not make the next session emit a phantom
/// detransform, and two consecutive super sessions each emit their own transform.
#[test]
fn super_form_edge_does_not_leak_across_sessions() {
    // Within a session: a rising edge transforms, holding is silent, a falling
    // edge detransforms.
    assert_eq!(super_form_edge(Some(true), false), (Some(true), true));
    assert_eq!(super_form_edge(Some(true), true), (None, true));
    assert_eq!(super_form_edge(Some(false), true), (Some(false), false));

    // Session A ends WHILE super (latch true): no controlled player resets the
    // latch and fires NO cue.
    assert_eq!(super_form_edge(None, true), (None, false));
    // Session B starts normal with the reset latch: no phantom detransform.
    assert_eq!(super_form_edge(Some(false), false), (None, false));

    // Two consecutive super sessions each emit their own transform, because the
    // latch resets to false between them.
    assert_eq!(super_form_edge(Some(true), false), (Some(true), true)); // A transforms
    assert_eq!(super_form_edge(None, true), (None, false)); // A retires, latch reset
    assert_eq!(super_form_edge(Some(true), false), (Some(true), true)); // B transforms
}

/// The oracle: the momentum showcase room composes through the umbrella
/// surface alone — floor geometry present, the Sonic loop validates, and the
/// spawn sits inside the room bounds.
#[test]
fn sanic_speedway_composes_through_the_umbrella() {
    let room = sanic_speedway();
    assert_eq!(room.id, SPEEDWAY_ROOM_ID);

    // The LDtk-authored course made it into the world: solid ground (on the
    // tiled terrain path), the pit gap, the pad trio, one-way platforms, the
    // hazards, the named monitors, and the badnik spawns.
    let ground: Vec<_> = room
        .world
        .blocks
        .iter()
        .filter(|b| {
            matches!(b.kind, ae::BlockKind::Solid)
                && (b.aabb.min.y - FLOOR_TOP).abs() < 0.5
                && matches!(&b.id.source, ae::GeoSource::TileLayer { .. })
        })
        .collect();
    assert_eq!(
        ground.len(),
        2,
        "the ground is two tiled solids split by the pit"
    );
    assert!(
        ground
            .iter()
            .any(|b| (b.aabb.max.x - PIT_LEFT_X).abs() < 0.5)
            && ground
                .iter()
                .any(|b| (b.aabb.min.x - PIT_RIGHT_X).abs() < 0.5),
        "the pit gap sits exactly between the two ground slabs"
    );
    let pads: Vec<ae::Vec2> = room
        .world
        .blocks
        .iter()
        .filter_map(|b| match b.kind {
            ae::BlockKind::Rebound { impulse } => Some(impulse),
            _ => None,
        })
        .collect();
    assert!(
        pads.contains(&ae::Vec2::new(1120.0, -260.0))
            && pads.contains(&ae::Vec2::new(0.0, -1000.0))
            && pads.contains(&ae::Vec2::new(700.0, -700.0)),
        "the booster, the vertical spring, and the diagonal spring are authored: {pads:?}"
    );
    let one_ways = room
        .world
        .blocks
        .iter()
        .filter(|b| matches!(b.kind, ae::BlockKind::OneWay))
        .count();
    assert!(
        one_ways >= 8,
        "the gantry, marker platforms, and the two spring perches are one-ways: {one_ways}"
    );
    let hazards = room
        .world
        .blocks
        .iter()
        .filter(|b| matches!(b.kind, ae::BlockKind::Hazard))
        .count();
    assert!(
        hazards >= 3,
        "the pit floor and both spike strips are hazards: {hazards}"
    );
    for monitor in [monitors::SUPER_MONITOR, monitors::SPEED_MONITOR] {
        assert!(
            room.world.blocks.iter().any(|b| b.name == monitor),
            "monitor '{monitor}' is authored as a named block"
        );
    }
    assert_eq!(room.enemy_spawns.len(), 4, "four badniks pace the flats");
    assert!(
        room.enemy_spawns
            .iter()
            .all(|spawn| spawn.name == badnik::BADNIK_DISPLAY_NAME),
        "every enemy spawn resolves the badnik identity row"
    );
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
            .any(|label| label.payload.text == "1608"),
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
    // The LDtk-authored floor route carries the two rolling hills as real
    // polyline geometry: many samples, all rising FROM the flat floor (the
    // solid ground beneath never pokes through).
    assert!(
        floor_route.points.len() > 40,
        "the hills are sampled into the floor route: {} points",
        floor_route.points.len()
    );
    assert!(
        floor_route.points.iter().all(|p| p.y <= FLOOR_TOP + 1.0e-3),
        "hills only rise from the floor; the route never dips below the ground"
    );
    assert!(
        floor_route.points.iter().any(|p| p.y < FLOOR_TOP - 80.0),
        "the tall hill genuinely rises"
    );
    assert!(
        room.world
            .chains
            .iter()
            .any(|chain| chain.name == "sanic_floor_runout"),
        "the pit splits the ground into two authored route chains"
    );
    assert!(
        room.world.validate_surface_junctions().is_empty(),
        "every local and cross-chain route port resolves to the same projected point: {:?}",
        room.world.validate_surface_junctions()
    );
    let ramp_fork_vertex = floor_route
        .points
        .iter()
        .position(|p| (p.x - 1740.0).abs() < 0.5)
        .expect("the floor route keeps its ramp-fork anchor vertex");
    assert!(
        loop_chain.junctions.iter().any(|junction| {
            junction.ports
                == vec![
                    ae::SurfacePort::local(0),
                    ae::SurfacePort::chain(1, ramp_fork_vertex),
                ]
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

    let floor_top = FLOOR_TOP;
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

/// A surface-momentum test rig: the body-state scratch plus the motion model,
/// stepped one tick at a time through the ONE public movement gateway
/// (`ae::step_motion`), exactly as production does. The kernel derives the
/// ride circle radius as `size.min_element() * 0.5`, so a `splat(32.0)` body
/// box rides as the old radius-16 circle proxy.
struct MomentumRig {
    scratch: ae::BodyClusterScratch,
    model: ae::MotionModel,
}

impl MomentumRig {
    /// A radius-16 rider attached to `world.chains[chain_index]` at arc
    /// length `s`, moving at signed tangential speed `v_t`.
    fn riding(
        chain: &ae::SurfaceChain,
        chain_index: usize,
        s: f32,
        v_t: f32,
        params: ae::MomentumParams,
    ) -> Self {
        let frame = chain.frame_at(s);
        let mut scratch = ae::BodyClusterScratch::new_with_abilities(
            frame.point + frame.normal * 16.0,
            ae::AbilitySet::default(),
        );
        scratch.kinematics.size = ae::Vec2::splat(32.0);
        scratch.kinematics.vel = frame.tangent * v_t;
        let mut model = ae::MotionModel::surface_momentum(params);
        let ae::MotionModel::SurfaceMomentum(m) = &mut model else {
            unreachable!()
        };
        m.state = ae::SurfaceMotion::Riding {
            on: ae::SurfaceRef::Chain(chain_index),
            s,
            v_t,
        };
        m.depth_lane = chain.segment_depth(frame.segment);
        Self { scratch, model }
    }

    /// One 60 Hz kernel tick under the standard downward gravity frame.
    fn step(&mut self, world: &ae::World, steer: ae::Vec2) {
        let mut clusters = self.scratch.as_mut();
        ae::step_motion(
            &mut self.model,
            &mut clusters,
            ae::MotionStepContext {
                world,
                input: ae::InputState {
                    movement: ae::ActionEdges::EMPTY.with(
                        ae::MovementAction::Jump,
                        ae::Edge {
                            pressed: false,
                            held: false,
                            released: false,
                        },
                    ),
                    axes: ae::LocalAxes::new(steer.x, steer.y),
                    ..ae::InputState::default()
                },
                frame: ae::MotionFrame::from_acceleration(ae::Vec2::new(0.0, 1450.0))
                    .expect("non-zero acceleration"),
                facing_intent: 0.0,
                dt: 1.0 / 60.0,
            },
        );
    }

    /// The ride state, read back from the model (the kernel's authority).
    fn motion(&self) -> ae::SurfaceMotion {
        let ae::MotionModel::SurfaceMomentum(m) = &self.model else {
            unreachable!()
        };
        m.state
    }
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
    let speed = 1000.0;
    let params = ae::MomentumParams {
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
    let mut rig = MomentumRig::riding(chain, 0, start_s, speed, params);

    let mut reached_runout = false;
    for _ in 0..180 {
        rig.step(&room.world, ae::Vec2::ZERO);
        let ae::SurfaceMotion::Riding { s, .. } = rig.motion() else {
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
        rig.motion()
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
    let speed = 1120.0;
    let params = ae::MomentumParams {
        ground_accel: 900.0,
        top_speed: 1200.0,
        jump_speed: 700.0,
        ..Default::default()
    };
    let mut rig = MomentumRig::riding(chain, 0, entry_s, speed, params);

    let clear_s = closure_s + 160.0;
    for _ in 0..180 {
        rig.step(&room.world, ae::Vec2::X);
        let ae::SurfaceMotion::Riding { s, .. } = rig.motion() else {
            panic!(
                "authored Sanic speed must stay attached through the loop mouth; model={:?}, kinematics={:?}",
                rig.model, rig.scratch.kinematics
            );
        };
        if s > clear_s {
            return;
        }
    }
    panic!(
        "authored Sanic speed never cleared the foreground overpass; motion={:?}",
        rig.motion()
    );
}

#[test]
fn crossing_a_visible_distance_marker_emits_the_standard_sfx_message() {
    let mut app = App::new();
    app.add_message::<ambition::sfx::OwnedSfxMessage>();
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
        .resource::<bevy::prelude::Messages<ambition::sfx::OwnedSfxMessage>>();
    assert!(
        messages
            .iter_current_update_messages()
            .any(|message| matches!(message.request, ambition::sfx::SfxMessage::Dash { .. })),
        "the first visual marker emits the first standard diagnostic cue"
    );
    let mut q = app.world_mut().query::<&SanicActState>();
    assert_eq!(q.single(app.world()).unwrap().next_milestone, 1);
}

#[test]
fn semantic_utility_toggles_both_sanic_forms_and_is_consumed() {
    let mut app = App::new();
    app.add_message::<ambition::sfx::OwnedSfxMessage>();
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
        app.add_message::<ambition::sfx::OwnedSfxMessage>();
        ambition::platformer::lifecycle::insert_session_world_component(
            app.world_mut(),
            ActiveRoomMetadata(RoomMetadata {
                mode: mode.map(str::to_string),
                ..Default::default()
            }),
        );
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
    ambition::platformer::lifecycle::insert_session_world_component(
        app.world_mut(),
        ActiveRoomMetadata::default(),
    ); // left the Sanic rooms
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
    ambition::platformer::lifecycle::insert_session_world_component(
        app.world_mut(),
        ActiveRoomMetadata(room.metadata.clone()),
    );
    let awake = app
        .world_mut()
        .run_system_once(in_mode(SANIC_MODE))
        .expect("the mode condition runs");
    assert!(awake, "a hosted Sanic ruleset wakes inside the speedway");

    // Ambition's own rooms carry no mode, so the demo's rules sleep there.
    ambition::platformer::lifecycle::insert_session_world_component(
        app.world_mut(),
        ActiveRoomMetadata::default(),
    );
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
    let params = ae::MomentumParams {
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
        let mut rig = MomentumRig::riding(chain, 0, s, v_t, params);
        rig.step(&room.world, steer);
        rig.motion()
    };

    let up_into_loop = step_from(entry_s - 3.0, 600.0, ae::Vec2::new(1.0, -1.0));
    let ae::SurfaceMotion::Riding { s, .. } = up_into_loop else {
        panic!("the authored route switch guides the rider instead of launching");
    };
    assert!(s > entry_s && s < closure_s, "up-right enters the loop");

    let down_to_runout = step_from(entry_s - 3.0, 600.0, ae::Vec2::new(1.0, 1.0));
    let ae::SurfaceMotion::Riding { s, .. } = down_to_runout else {
        panic!("the authored route switch guides the rider instead of launching");
    };
    assert!(s > closure_s, "down-right selects the lower/outbound route");

    let up_into_reverse_loop = step_from(closure_s + 3.0, -600.0, ae::Vec2::new(-1.0, -1.0));
    let ae::SurfaceMotion::Riding { s, .. } = up_into_reverse_loop else {
        panic!("the authored route switch guides the rider instead of launching");
    };
    assert!(
        s > entry_s && s < closure_s,
        "up-left enters the loop in reverse"
    );

    let down_to_ramp = step_from(closure_s + 3.0, -600.0, ae::Vec2::new(-1.0, 1.0));
    let ae::SurfaceMotion::Riding { s, .. } = down_to_ramp else {
        panic!("the authored route switch guides the rider instead of launching");
    };
    assert!(s < entry_s, "down-left selects the descending ramp");

    let forward_default = step_from(closure_s - 3.0, 600.0, ae::Vec2::X);
    let ae::SurfaceMotion::Riding { s, .. } = forward_default else {
        panic!("horizontal input preserves the authored forward exit");
    };
    assert!(s > closure_s, "holding Right exits after one forward lap");

    let reverse_default = step_from(entry_s + 3.0, -600.0, -ae::Vec2::X);
    let ae::SurfaceMotion::Riding { s, .. } = reverse_default else {
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
    // The ramp-fork junction vertex is located by POSITION: the hills give the
    // floor route many vertices before it, so a fixed index would drift.
    let branch_vertex = floor
        .points
        .iter()
        .position(|p| (p.x - 1740.0).abs() < 0.5)
        .expect("the floor route has its ramp-fork anchor vertex");
    let branch_s = floor.arc_at_vertex(branch_vertex);
    let params = ae::MomentumParams {
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
        let mut rig = MomentumRig::riding(floor, floor_index, branch_s - 3.0, 600.0, params);
        rig.step(&room.world, steer);
        rig.motion()
    };

    let raised = step(ae::Vec2::new(1.0, -1.0));
    assert!(
        matches!(
            raised,
            ae::SurfaceMotion::Riding {
                on: ae::SurfaceRef::Chain(0),
                ..
            }
        ),
        "up-right transfers directly from the floor guide onto the ramp: {raised:?}"
    );

    let flat = step(ae::Vec2::X);
    assert!(
        matches!(
            flat,
            ae::SurfaceMotion::Riding {
                on: ae::SurfaceRef::Chain(index),
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
    // Isolate route topology from feel tuning: this oracle asks whether the
    // authored reverse continuation exits after one lap, not whether a
    // particular speed/stick-factor combination sheds from a convex ramp.
    let params = ae::MomentumParams {
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
    let mut rig = MomentumRig::riding(chain, 0, start_s, -900.0, params);

    let mut entered_loop = false;
    for _ in 0..420 {
        rig.step(&room.world, ae::Vec2::NEG_X);
        match rig.motion() {
            ae::SurfaceMotion::Riding { s, .. } => {
                entered_loop |= s > entry_s + 100.0 && s < closure_s - 100.0;
                if entered_loop && s < entry_s - 0.5 {
                    return;
                }
            }
            ae::SurfaceMotion::Airborne => {
                panic!(
                    "the topology oracle uses sticky, slope-free tuning and must remain attached; model={:?}, kinematics={:?}",
                    rig.model, rig.scratch.kinematics
                );
            }
        }
    }
    panic!(
        "reverse traversal must leave after one revolution instead of re-entering; model={:?}, kinematics={:?}",
        rig.model, rig.scratch.kinematics
    );
}

#[test]
fn super_form_traits_track_the_worn_identity_both_ways() {
    use ambition::characters::actor::{BodyHealth, Health, WornCharacter};

    let mut app = App::new();
    app.insert_resource(ambition::time::WorldTime {
        raw_dt: 1.0 / 60.0,
        scaled_dt: 1.0 / 60.0,
    });
    app.add_message::<ambition::vfx::VfxMessage>();
    // `sync_super_form_traits` now emits the transform cue on the worn-identity
    // edge, so the SFX channel must exist for the SfxWriter system param.
    app.add_message::<ambition::sfx::OwnedSfxMessage>();
    app.add_systems(bevy::prelude::Update, sync_super_form_traits);
    let player = app
        .world_mut()
        .spawn((
            ambition::actors::actor::PrimaryPlayer,
            WornCharacter::new(SUPER_SANIC_CHARACTER_ID),
            BodyHealth::new(Health::new(3)),
            ae::BodyKinematics::default(),
        ))
        .id();
    app.update();
    assert!(
        app.world()
            .get::<BodyHealth>(player)
            .unwrap()
            .health
            .invulnerable,
        "wearing the super form derives invincibility"
    );

    // Toggle the identity off — the derived trait reverts the same frame,
    // because it is derived, never stored.
    *app.world_mut().get_mut::<WornCharacter>(player).unwrap() =
        WornCharacter::new(SANIC_CHARACTER_ID);
    app.update();
    assert!(
        !app.world()
            .get::<BodyHealth>(player)
            .unwrap()
            .health
            .invulnerable,
        "wearing the form off revokes invincibility"
    );
}

#[test]
fn the_super_row_authors_a_real_movement_boost() {
    // The transformation must be more than a sprite swap: the super row's
    // authored momentum strictly dominates the base row's. Read through the
    // same catalog hydration the runtime wear uses, so a RON edit that
    // flattens the form (or a hydration regression) trips this.
    let fragment =
        ambition::characters::actor::character_catalog::CharacterCatalogFragment::from_ron(
            provider::SANIC_EXPERIENCE,
            Some(SANIC_CHARACTER_ID),
            SANIC_CATALOG_RON,
        )
        .expect("demo catalog parses");
    let catalog = ambition::characters::actor::character_catalog::CharacterCatalog::from_data(
        fragment.catalog().clone(),
    );
    let base = catalog
        .momentum_params(SANIC_CHARACTER_ID)
        .expect("base row authors momentum");
    let super_form = catalog
        .momentum_params(SUPER_SANIC_CHARACTER_ID)
        .expect("super row authors momentum");
    assert!(
        super_form.top_speed > base.top_speed
            && super_form.ground_accel > base.ground_accel
            && super_form.jump_speed > base.jump_speed,
        "super movement strictly dominates base: {super_form:?} vs {base:?}"
    );
}

#[test]
fn rules_plugin_registers_its_mandatory_sfx_message_channel() {
    let mut app = App::new();
    assert!(
        !app.world()
            .contains_resource::<bevy::prelude::Messages<ambition::sfx::OwnedSfxMessage>>(),
        "the test must begin without the engine group's SFX registrar"
    );

    app.add_plugins(SanicRulesPlugin::global());

    assert!(
        app.world().contains_resource::<
            bevy::prelude::Messages<ambition::sfx::OwnedSfxMessage>,
        >(),
        "SanicRulesPlugin owns a mandatory SfxWriter dependency and must register it when a thin host has not"
    );
}

#[test]
fn rev_tier_climbs_with_charge() {
    // The three buckets land on the three natural tap counts (rev_per_tap 0.4).
    assert_eq!(rev_tier_id(0.0), SFX_REV_TIERS[0]);
    assert_eq!(rev_tier_id(0.4), SFX_REV_TIERS[0]);
    assert_eq!(rev_tier_id(0.6), SFX_REV_TIERS[1]);
    assert_eq!(rev_tier_id(0.8), SFX_REV_TIERS[1]);
    assert_eq!(rev_tier_id(1.0), SFX_REV_TIERS[2]);
    // Monotonic: never steps down as charge rises.
    let mut prev = 0usize;
    let mut c = 0.0;
    while c <= 1.0 {
        let tier = SFX_REV_TIERS
            .iter()
            .position(|id| *id == rev_tier_id(c))
            .unwrap();
        assert!(tier >= prev, "rev tier must not decrease with charge");
        prev = tier;
        c += 0.05;
    }
}

#[test]
fn the_sanic_sfx_registry_validates_with_every_new_cue() {
    let registry = ambition::audio::spec::SfxRegistry {
        sample_rate: 44_100,
        sfx: sanic_sfx_specs(),
    };
    // No duplicate ids across the expanded table (rev tiers, launch, transform,
    // monitor, badnik, skid, rings, Pogo/Land/Reset, menu).
    registry
        .validate()
        .expect("the Sanic SFX table must have unique, well-formed ids");
    // The mode-local techniques and the newly-voiced engine cues are all present.
    let ids = registry.authorized_cue_ids();
    for open in [
        SFX_REV_TIERS[0],
        SFX_REV_TIERS[1],
        SFX_REV_TIERS[2],
        SFX_LAUNCH,
        SFX_TRANSFORM,
        SFX_DETRANSFORM,
        SFX_MONITOR,
        SFX_BADNIK,
        SFX_SKID,
    ] {
        assert!(
            ids.contains(&ambition::sfx::SfxId::from_static(open)),
            "registry must authorize {open}"
        );
    }
    for cue in [
        ambition::audio::spec::SoundCueKey::Pogo,
        ambition::audio::spec::SoundCueKey::Land,
        ambition::audio::spec::SoundCueKey::Reset,
    ] {
        assert!(
            ids.contains(&cue.sfx_id()),
            "registry must authorize the {cue:?} engine cue it now voices"
        );
    }
}

#[test]
fn the_speedway_authors_a_field_of_collectible_rings() {
    use ambition::entity_catalog::placements::{PickupKindSpec, PlacementSchema};
    let room = sanic_speedway();
    let rings = room
        .placements
        .iter()
        .filter(|record| {
            record.name == "ring"
                && matches!(
                    &record.schema,
                    PlacementSchema::Pickup(pickup)
                        if matches!(pickup.kind, PickupKindSpec::Currency { amount } if amount >= 1)
                )
        })
        .count();
    // Rings are lowered as `currency:1` pickups, so the shared collection loop
    // (magnetize + collect_ecs_pickups) credits the player's wallet — the ring
    // counter — with no demo-side collection code.
    assert!(
        rings >= 30,
        "the speedway must author a field of collectible rings; got {rings}"
    );
}

#[test]
fn the_ring_collect_cue_is_the_shared_currency_pickup_id() {
    // Rings ride the shared Currency pickup path, so `collect_ecs_pickups` emits
    // `WORLD_COIN_PICKUP` on collect. The demo authorises + voices exactly that
    // id (a private `sanic.ring` would be silently dropped by the authority gate).
    assert_eq!(
        ambition::sfx::SfxId::from_static(SFX_RING),
        ambition::sfx::ids::WORLD_COIN_PICKUP,
        "the ring ding must voice the id the shared currency-pickup loop emits"
    );
    // And the demo's registry authorises it.
    let registry = ambition::audio::spec::SfxRegistry {
        sample_rate: 44_100,
        sfx: sanic_sfx_specs(),
    };
    assert!(
        registry
            .authorized_cue_ids()
            .contains(&ambition::sfx::ids::WORLD_COIN_PICKUP),
        "the Sanic registry must authorise the ring/coin pickup cue"
    );
}

#[test]
fn the_speedway_tags_every_ring_with_the_animated_sprite() {
    use ambition::entity_catalog::placements::PlacementSchema;
    let room = sanic_speedway();
    let rings: Vec<_> = room
        .placements
        .iter()
        .filter(|record| is_ring_placement(record))
        .collect();
    assert!(
        rings.len() >= 30,
        "expected a field of rings; got {}",
        rings.len()
    );
    // The demo assigns the render identity in code (like the badnik name), so the
    // pickup renderer binds the spinning `sanic_ring_prop` sheet instead of the
    // static coin.
    for record in rings {
        let PlacementSchema::Pickup(pickup) = &record.schema else {
            unreachable!("is_ring_placement guarantees a pickup");
        };
        assert_eq!(
            pickup.sprite.as_deref(),
            Some(RING_SPRITE_KIND),
            "every ring must name the animated sprite sheet"
        );
    }
}
