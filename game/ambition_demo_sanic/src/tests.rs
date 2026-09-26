//! Unit tests for the standalone Sanic content and rules plugin.

mod speedway_oracles;

/// The painted floor under `x`: the top-most `terrain:` chain with a floor
/// segment (authored left to right) spanning it. Act 1's ground is painted, so
/// its chains are named by the tracer; a test finds one by where it is.
fn floor_chain_at(world: &ae::World, x: f32) -> usize {
    world
        .chains
        .iter()
        .enumerate()
        .filter(|(_, chain)| chain.name.starts_with("terrain:"))
        .flat_map(|(index, chain)| {
            chain.points.windows(2).filter_map(move |pair| {
                let (a, b) = (pair[0], pair[1]);
                (a.x < x && x < b.x).then(|| (index, a.y + (b.y - a.y) * (x - a.x) / (b.x - a.x)))
            })
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(index, _)| index)
        .unwrap_or_else(|| panic!("no painted floor under x={x}"))
}

/// West of the pit: the floor that carries the hills and the loop.
const WEST_FLOOR_X: f32 = 1600.0;
/// East of the pit: the runout to the finish.
const EAST_FLOOR_X: f32 = 5000.0;

use super::*;

/// A session whose one room claims `mode`: the room set is the only place a
/// session's mode lives.
fn rooms_in_mode(mode: Option<&str>) -> ambition_platformer2d::world::rooms::RoomSet {
    let mut room = ambition_platformer2d::world::rooms::RoomSpec::new(
        "mode_fixture",
        ae::World::new(
            "mode_fixture",
            ae::Vec2::splat(64.0),
            ae::Vec2::ZERO,
            Vec::new(),
        ),
    );
    room.metadata.mode = mode.map(str::to_string);
    ambition_platformer2d::world::rooms::RoomSet::from_parts_or_panic(
        "mode_fixture",
        vec![room],
        Vec::new(),
    )
}

#[test]
fn sanic_demo_content_plugin_installs() {
    // The direct-entry content plugin publishes an exact PreparedContent root
    // at plugin-build time, which needs the engine's construction registries
    // first (the standalone shell's real order). A bare App tests only catalog
    // registration and cannot validate the speedway's ring placements.
    let mut app = App::new();
    ambition_platformer2d::engine::add_headless_foundation(&mut app);
    app.add_plugins(ambition_platformer2d::engine::PlatformerEnginePlugins::fixed_tick());
    add_demo_content(&mut app);

    let placement_lowering =
        app.world()
            .resource::<ambition_platformer2d::runtime::demo_fixture::PlacementLoweringRegistry>();
    assert!(
        placement_lowering
            .schema_descriptors()
            .iter()
            .any(|(kind, _, _, schema)| kind == "pickup" && schema == "placement.pickup.v1"),
        "the engine must install the pickup lowering before Sanic content is prepared"
    );

    let mut prepared_query = app
        .world_mut()
        .query::<&ambition_platformer2d::runtime::PreparedContent>();
    let prepared = prepared_query
        .single(app.world())
        .expect("Sanic direct entry publishes one prepared-content root");
    assert!(
        prepared
            .sections()
            .iter()
            .any(|section| section.name == "construction.placement-lowering"),
        "Sanic's exact content identity includes the installed lowering schema"
    );

    let audio = app
        .world()
        .resource::<ambition_platformer2d::audio::catalog::AudioCatalogRegistry>();
    let music = audio
        .music_for(provider::SANIC_EXPERIENCE)
        .expect("Sanic music fragment");
    assert_eq!(music.default_track, "you_are_too_slow");
    // Each act has a score.
    assert_eq!(music.tracks.len(), 3);
    assert_eq!(
        audio
            .sfx_for(provider::SANIC_EXPERIENCE)
            .expect("Sanic SFX fragment")
            .sample_rate,
        44_100
    );
    let catalog = app
        .world()
        .resource::<ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog>(
    );
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

    // Session A ends while super (latch true): no controlled player resets the
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

    // The LDtk-authored course made it into the world: the painted ground and
    // its pit, the pad trio, one-way platforms, the hazards, the named
    // monitors, and the badnik spawns.
    assert_ne!(
        floor_chain_at(&room.world, WEST_FLOOR_X),
        floor_chain_at(&room.world, EAST_FLOOR_X),
        "the pit splits the painted ground into a west floor and an east one"
    );
    let pit = floor_chain_at(&room.world, (PIT_LEFT_X + PIT_RIGHT_X) * 0.5);
    assert!(
        room.world.chains[pit].points.iter().any(|p| (p.y - PIT_FLOOR_Y).abs() < 1.0),
        "the pit's floor is painted at {PIT_FLOOR_Y}"
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
    let reset_blocks: Vec<&str> = room
        .world
        .blocks
        .iter()
        .filter(|b| matches!(b.kind, ae::BlockKind::Hazard))
        .map(|b| b.name.as_str())
        .collect();
    assert!(
        reset_blocks.is_empty(),
        "the speedway has no reset-to-spawn hazard; its pit is a spike bed: {reset_blocks:?}"
    );
    let spikes = room
        .placements
        .iter()
        // `stable_id()`, not the schema variant: the enum is not re-exported to
        // games, and the stable id is the compatibility contract anyway.
        .filter(|record| record.kind().stable_id() == "hazard")
        .map(|record| record.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        spikes,
        ["pit_spikes", "mid_spikes"],
        "the pit's bed and the mid-course strip are DAMAGE volumes, so a hit \
         costs rings rather than the whole run"
    );
    let authored = room
        .world
        .blocks
        .iter()
        .filter(|b| b.name.starts_with(monitors::MONITOR_PREFIX))
        .map(|b| b.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        authored,
        [monitors::SPEED_MONITOR],
        "the speedway authors the speed shoes and no other monitor — the super \
         form is reachable only from the Utility action"
    );
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

    // The raised ramp, complete loop, and runout are one valid rideable
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
    let floor_index = floor_chain_at(&room.world, WEST_FLOOR_X);
    let floor_route = &room.world.chains[floor_index];
    // The painted west floor carries the two rolling hills: they rise from the
    // flat floor and never dip below it.
    let hills: Vec<_> = floor_route
        .points
        .iter()
        .filter(|p| p.x > 300.0 && p.x < 1500.0)
        .collect();
    assert!(
        hills.iter().all(|p| p.y <= FLOOR_TOP + 1.0),
        "hills only rise from the floor; the route never dips below the ground"
    );
    assert!(
        hills.iter().any(|p| p.y < FLOOR_TOP - 70.0),
        "the tall hill genuinely rises"
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
                    ae::SurfacePort::chain(floor_index, ramp_fork_vertex),
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

    // The loop as authored: a `SurfaceLoop` box in the LDtk, measured off the
    // revolution it built rather than restated here.
    let revolution = &loop_chain.points[LOOP_ENTRY_POINT_INDEX..=LOOP_CLOSURE_POINT_INDEX];
    let (min_x, max_x) = revolution
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), p| (lo.min(p.x), hi.max(p.x)));
    let (min_y, max_y) = revolution
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), p| (lo.min(p.y), hi.max(p.y)));
    let radius = (max_x - min_x) * 0.5;
    let center_x = (max_x + min_x) * 0.5;

    let floor_top = FLOOR_TOP;
    // The painted floor is smoothed: its flats hold to float noise, not bits.
    assert!((ramp_start.y - floor_top).abs() < 1.0e-2, "ramp foot {ramp_start:?}");
    assert!(entry.y < floor_top - 60.0, "the loop is visibly raised");
    assert!((exit.y - floor_top).abs() < 1.0e-2, "runout end {exit:?}");
    assert!(
        overpass_end.x > center_x + radius + 80.0,
        "the flat foreground deck must clear the loop before descending"
    );
    assert!(
        (overpass_end.y - closure.y).abs() < 1.0e-3,
        "the crossover deck must stay flat while it clears the back rail"
    );
    assert!(
        exit.x > closure.x + radius * 3.0,
        "the runout must carry the rider clear of the completed loop"
    );

    // THE LOOP IS BUILT WHERE ITS BOX IS: the `SurfaceLoop` entity's box is its
    // circle, so an author sizes a loop by drawing it.
    let project = ambition_platformer2d::ldtk_map::LdtkProject::from_json_str(SPEEDWAY_WORLD_JSON)
        .expect("the speedway world parses");
    let boxed = project
        .levels
        .iter()
        .flat_map(|level| level.all_entity_instances())
        .find(|entity| entity.identifier == "SurfaceLoop")
        .expect("the speedway authors its loop as a SurfaceLoop");
    assert_eq!(boxed.width, boxed.height, "a loop's box is square");
    let box_radius = boxed.width as f32 * 0.5;
    let box_center_x = boxed.px[0] as f32 + box_radius;
    assert!(
        (radius - box_radius).abs() < 1.0 && (center_x - box_center_x).abs() < 1.0,
        "the loop is its box: built radius {radius} about x={center_x}, box radius \
         {box_radius} about x={box_center_x}"
    );
    let box_bottom = boxed.px[1] as f32 + boxed.height as f32;
    assert!((max_y - box_bottom).abs() < 1.0, "the loop's bottom is its box's: {max_y} vs {box_bottom}");

    // The loop samples all four quadrants around its centre (a full loop, not
    // three quarters): as tall as it is wide, and a real loop's size.
    assert!(radius > 150.0, "the speedway's loop is a big one: radius {radius}");
    assert!(
        ((max_y - min_y) - (max_x - min_x)).abs() < radius * 0.02,
        "a full revolution is as tall as it is wide: {}x{}",
        max_x - min_x,
        max_y - min_y
    );

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
/// stepped one tick at a time through the public movement gateway
/// (`ae::step_motion`), as production does. The kernel derives the ride
/// circle radius as `size.min_element() * 0.5`, so a `splat(32.0)` box rides
/// as a radius-16 circle.
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
                contact: ae::BodyContactField::NONE,
                pose_owned_externally: false,
                recovery_commitment_outstanding: false,
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
    let mut rig = MomentumRig::riding(
        chain,
        room.world.chain_named("sanic_loop").expect("the loop"),
        start_s,
        speed,
        params,
    );

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
    let mut rig = MomentumRig::riding(
        chain,
        room.world.chain_named("sanic_loop").expect("the loop"),
        entry_s,
        speed,
        params,
    );

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
    app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
    app.world_mut().spawn((
        ambition_platformer2d::platformer::markers::PrimaryPlayer,
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
        .resource::<bevy::prelude::Messages<ambition_platformer2d::sfx::OwnedSfxMessage>>();
    assert!(
        messages
            .iter_current_update_messages()
            .any(|message| matches!(
                message.request,
                ambition_platformer2d::sfx::SfxMessage::Dash { .. }
            )),
        "the first visual marker emits the first standard diagnostic cue"
    );
    let mut q = app.world_mut().query::<&SanicActState>();
    assert_eq!(q.single(app.world()).unwrap().next_milestone, 1);
}

/// The transformation fires from the declared Utility technique, and the
/// declaration consumes the raw verb.
///
/// The body declares `transform` on `ControlSlot::Utility`, so
/// `resolve_control_slots` routes the press to the technique edge and clears
/// the verb; generic flight never sees it.
#[test]
fn the_declared_utility_technique_toggles_both_forms_and_eats_the_fly_verb() {
    use ambition_platformer2d::characters::action_scheme::{
        derive_action_scheme, resolve_control_slots, ActorTechniques, ResolvedTechniqueEdges,
    };

    let mut app = App::new();
    app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
    let entity = app
        .world_mut()
        .spawn((
            ambition_platformer2d::characters::control::ActorControl::default(),
            ambition_platformer2d::characters::actor::WornCharacter::new(SANIC_CHARACTER_ID),
            ae::BodyKinematics::default(),
            // `#[require]` pulls in `ResolvedTechniqueEdges` — the seam the gate
            // writes and the toggle reads.
            ActorTechniques(vec![super::transform_technique()]),
        ))
        .id();
    app.insert_resource(
        ambition_platformer2d::platformer::markers::ControlledSubject(Some(entity)),
    );
    app.add_systems(bevy::app::Update, toggle_sanic_form);

    // The body has wings, so the engine's `fly_toggle` would otherwise claim
    // Utility. The technique must win.
    let mut abilities = ae::AbilitySet::basic();
    abilities.fly = true;
    abilities.fly_toggle = true;
    let scheme = derive_action_scheme(&abilities, None, None, &[super::transform_technique()]);

    // Stand in for the persona gate: press the device verb, run the resolver.
    let press_utility = |app: &mut App| {
        let mut control = app
            .world_mut()
            .get_mut::<ambition_platformer2d::characters::control::ActorControl>(entity)
            .unwrap();
        control.0.fly_toggle_pressed = true;
        let mut frame = control.0.clone();
        let mut edges = ResolvedTechniqueEdges::default();
        let unroutable = resolve_control_slots(&scheme, &mut frame, &mut edges, false);
        assert!(
            unroutable.is_empty(),
            "the declared Utility technique must have a wired path, got {unroutable:?}"
        );
        assert!(
            !frame.fly_toggle_pressed,
            "the resolver consumes Utility, so generic flight never sees the press"
        );
        app.world_mut()
            .get_mut::<ambition_platformer2d::characters::control::ActorControl>(entity)
            .unwrap()
            .0 = frame;
        *app.world_mut()
            .get_mut::<ResolvedTechniqueEdges>(entity)
            .unwrap() = edges;
        app.update();
    };

    let worn = |app: &App| {
        app.world()
            .get::<ambition_platformer2d::characters::actor::WornCharacter>(entity)
            .unwrap()
            .id()
            .to_string()
    };

    press_utility(&mut app);
    assert_eq!(worn(&app), SUPER_SANIC_CHARACTER_ID);
    press_utility(&mut app);
    assert_eq!(worn(&app), SANIC_CHARACTER_ID);
}

/// H2: Sanic's transformation sounds like Sanic, not like the host.
///
/// In a Sanic-only game the session's provider and the character's provider
/// are the same string, so this uses a session owned by another provider.
#[test]
fn the_super_transformation_sounds_like_sanic_and_not_like_the_session_owner() {
    let mut app = App::new();
    app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
    app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
    app.init_resource::<ambition_platformer2d::time::WorldTime>();
    // A session whose speakers belong to somebody else.
    let mut context = ambition_platformer2d::sfx::SfxEmissionContext::default();
    context.set(
        ambition_platformer2d::sfx::AudioContextOwner::Gameplay(1),
        "some_host",
    );
    app.insert_resource(context);

    let entity = app
        .world_mut()
        .spawn((
            ambition_platformer2d::platformer::markers::PrimaryPlayer,
            ambition_platformer2d::characters::actor::BodyHealth::new(
                ambition_platformer2d::characters::actor::Health::new(3),
            ),
            ambition_platformer2d::characters::control::ActorControl::default(),
            ambition_platformer2d::characters::actor::WornCharacter::new(SUPER_SANIC_CHARACTER_ID),
            ae::BodyKinematics::default(),
            // What `publish_body_presentation_sources` derives in production; the
            // derivation itself is tested in `character_runtime::presentation`.
            ambition_platformer2d::sfx::BodyPresentationSource(
                ambition_platformer2d::sfx::PresentationSourceId::new("sanic_demo"),
            ),
        ))
        .id();
    app.insert_resource(
        ambition_platformer2d::platformer::markers::ControlledSubject(Some(entity)),
    );
    app.add_systems(bevy::app::Update, sync_super_form_traits);
    app.update();

    let sources: Vec<String> = app
        .world()
        .resource::<bevy::prelude::Messages<ambition_platformer2d::sfx::OwnedSfxMessage>>()
        .iter_current_update_messages()
        .map(|message| message.source.as_str().to_string())
        .collect();
    assert_eq!(
        sources,
        vec!["sanic_demo".to_string()],
        "the transformation is the most character-defining sound a body makes, and \
         it was credited to whoever owned the session — so in a crossover it played \
         out of the host's bank, or was denied outright because `sanic_demo` was \
         not the authorized source for that cue"
    );
}

/// I3: the course's own sound belongs to the course, not to the host.
///
/// `write_global` uses the session context, so under a shell host a distance
/// marker would be credited to the launcher. No body caused the sound (the
/// room did), but it is Sanic's; `write_from` says so. Same fixture shape as
/// the transformation test: the session belongs to `some_host`, so the two
/// answers differ.
#[test]
fn a_distance_marker_sounds_like_the_course_and_not_like_the_host() {
    let mut app = App::new();
    app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
    let mut context = ambition_platformer2d::sfx::SfxEmissionContext::default();
    context.set(
        ambition_platformer2d::sfx::AudioContextOwner::Gameplay(1),
        "some_host",
    );
    app.insert_resource(context);

    // Parked just past the first marker, so one milestone fires this update.
    app.world_mut().spawn((
        ambition_platformer2d::platformer::markers::PrimaryPlayer,
        ae::BodyKinematics {
            pos: ae::Vec2::new(SPEED_MARKER_XS[0] + 1.0, 0.0),
            ..Default::default()
        },
    ));
    app.world_mut().spawn(SanicActState::default());
    app.add_systems(bevy::app::Update, emit_sanic_milestone_sfx);
    app.update();

    let sources: Vec<String> = app
        .world()
        .resource::<bevy::prelude::Messages<ambition_platformer2d::sfx::OwnedSfxMessage>>()
        .iter_current_update_messages()
        .map(|message| message.source.as_str().to_string())
        .collect();
    assert_eq!(
        sources,
        vec![provider::SANIC_EXPERIENCE.to_string()],
        "the course announcing its own marker was attributed to whoever was \
         hosting the session, so in a crossover it resolved against the host's \
         bank — which has never heard of this cue"
    );
}

/// The D-C pattern, end to end. `SanicRulesPlugin::hosted()` ticks the act
/// timer only inside the Sanic rooms; `::global()` ticks it everywhere. The
/// mode-owner entity is `spawn_mode_scoped`, so the engine tears it down when
/// the active room leaves the mode — this demo writes no teardown code.
#[test]
fn hosted_rules_run_only_in_sanic_rooms_and_global_rules_run_everywhere() {
    use ambition_platformer2d::bevy::ecs::system::RunSystemOnce as _;
    fn elapsed(app: &mut App) -> Option<f32> {
        let mut q = app.world_mut().query::<&SanicActState>();
        q.iter(app.world()).next().map(|s| s.elapsed)
    }
    fn shell(rules: SanicRulesPlugin, mode: Option<&str>) -> App {
        let mut app = App::new();
        ambition_platformer2d::engine::add_headless_foundation(&mut app);
        // The focused rules-only shell omits PlatformerEnginePlugins, whose
        // SimCoreResourcesPlugin normally registers the shared SFX message.
        app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
        ambition_platformer2d::platformer::lifecycle::insert_session_world_component(
            app.world_mut(),
            rooms_in_mode(mode),
        );
        app.insert_resource(ambition_platformer2d::time::WorldTime {
            scaled_dt: 0.5,
            ..Default::default()
        });
        app.add_plugins(rules);
        app
    }

    // hosted, inside a `sanic` room: the mode owner spawns and the act ticks.
    // `.chain()` puts a sync point between spawn and tick, so the owner exists
    // in time to tick on its own first frame: two frames = two ticks.
    let mut app = shell(SanicRulesPlugin::hosted(), Some(SANIC_MODE));
    app.update();
    app.update();
    assert_eq!(elapsed(&mut app), Some(1.0), "hosted rules tick in-mode");

    // hosted, in one of Ambition's own rooms: nothing spawns, nothing ticks.
    let mut app = shell(SanicRulesPlugin::hosted(), None);
    app.update();
    app.update();
    assert_eq!(elapsed(&mut app), None, "hosted rules sleep out of mode");

    // global (the demo IS the game): the rules run with no mode at all.
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
    ambition_platformer2d::platformer::lifecycle::insert_session_world_component(
        app.world_mut(),
        rooms_in_mode(None),
    ); // left the Sanic rooms
    app.world_mut()
        .run_system_once(ambition_platformer2d::runtime::despawn_departed_mode_entities)
        .expect("the engine's mode sweep runs");
    assert_eq!(
        elapsed(&mut app),
        None,
        "leaving the mode tears the act state down — no demo teardown code"
    );
}

/// The D-C hosting oracle: a demo's room claims its mode, and the run
/// condition that wakes a hosted ruleset reaches this crate through the
/// `ambition_platformer2d` umbrella alone. If gating a hosted demo needs a
/// lower `ambition_*` crate, this fails to compile.
///
/// The condition is evaluated directly, not through `.run_if` on a marker
/// resource: a crate whose manifest names only `ambition_platformer2d` cannot
/// `#[derive(Resource)]`, because bevy's derive macros resolve `bevy_ecs`
/// through the consumer's manifest. The `.run_if` wiring is covered in
/// `ambition_platformer2d_runtime/tests/mode_scope.rs`.
#[test]
fn the_speedway_claims_the_sanic_mode_and_wakes_a_hosted_ruleset() {
    use ambition_platformer2d::bevy::ecs::system::RunSystemOnce as _;
    use ambition_platformer2d::runtime::in_mode;
    let room = sanic_speedway();
    assert_eq!(room.metadata.mode.as_deref(), Some(SANIC_MODE));

    let mut app = App::new();
    ambition_platformer2d::platformer::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d::world::rooms::RoomSet::from_parts_or_panic(
            SPEEDWAY_ROOM_ID,
            vec![room.clone()],
            Vec::new(),
        ),
    );
    let awake = app
        .world_mut()
        .run_system_once(in_mode(SANIC_MODE))
        .expect("the mode condition runs");
    assert!(awake, "a hosted Sanic ruleset wakes inside the speedway");

    // Ambition's own rooms carry no mode, so the demo's rules sleep there.
    ambition_platformer2d::platformer::lifecycle::insert_session_world_component(
        app.world_mut(),
        rooms_in_mode(None),
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
        let mut rig = MomentumRig::riding(
            chain,
            room.world.chain_named("sanic_loop").expect("the loop"),
            s,
            v_t,
            params,
        );
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
    let floor_index = floor_chain_at(&room.world, WEST_FLOOR_X);
    let floor = &room.world.chains[floor_index];
    // The ramp-fork junction vertex is located by position: the hills give the
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

    let loop_index = room
        .world
        .chain_named("sanic_loop")
        .expect("the speedway attaches its loop");
    let raised = step(ae::Vec2::new(1.0, -1.0));
    assert!(
        matches!(
            raised,
            ae::SurfaceMotion::Riding {
                on: ae::SurfaceRef::Chain(index),
                ..
            } if index == loop_index
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
    let mut rig = MomentumRig::riding(
        chain,
        room.world.chain_named("sanic_loop").expect("the loop"),
        start_s,
        -900.0,
        params,
    );

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
    use ambition_platformer2d::characters::actor::{BodyHealth, Health, WornCharacter};

    let mut app = App::new();
    app.insert_resource(ambition_platformer2d::time::WorldTime {
        raw_dt: 1.0 / 60.0,
        scaled_dt: 1.0 / 60.0,
    });
    app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
    // `sync_super_form_traits` now emits the transform cue on the worn-identity
    // edge, so the SFX channel must exist for the SfxWriter system param.
    app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
    // Both halves, in app order. `sync_super_form_traits` states the traits
    // and the engine's empowerment applies them; running only the first would
    // test a grant that does nothing.
    {
        use bevy::ecs::schedule::IntoScheduleConfigs as _;
        app.add_systems(bevy::prelude::Update, sync_super_form_traits);
        app.add_plugins(
            ambition_platformer2d::actors::features::empowerment::EmpowermentLifecyclePlugin,
        );
        app.configure_sets(
            bevy::prelude::Update,
            ambition_platformer2d::actors::features::empowerment::EmpowermentExpiry
                .after(sync_super_form_traits),
        );
    }
    let player = app
        .world_mut()
        .spawn((
            ambition_platformer2d::platformer::markers::PrimaryPlayer,
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
            .invulnerable
            .any(),
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
            .invulnerable
            .any(),
        "wearing the form off revokes invincibility"
    );
}

#[test]
fn the_super_row_authors_a_real_movement_boost() {
    // The transformation is more than a sprite swap: the super row's authored
    // momentum strictly dominates the base row's.
    use ambition_platformer2d::characters::actor::character_catalog::{
        lowered_catalog, CharacterCatalog,
    };
    let catalog = CharacterCatalog::from_data(
        lowered_catalog(crate::pack::PACK.prepared())
            .expect("the Sanic pack states its cast")
            .clone(),
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
            .contains_resource::<bevy::prelude::Messages<ambition_platformer2d::sfx::OwnedSfxMessage>>(),
        "the test must begin without the engine group's SFX registrar"
    );

    app.add_plugins(SanicRulesPlugin::global());

    assert!(
        app.world()
            .contains_resource::<bevy::prelude::Messages<ambition_platformer2d::sfx::OwnedSfxMessage>>(),
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
    let registry = ambition_platformer2d::audio::spec::SfxRegistry {
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
            ids.contains(&ambition_platformer2d::sfx::SfxId::from_static(open)),
            "registry must authorize {open}"
        );
    }
    for cue in [
        ambition_platformer2d::audio::spec::SoundCueKey::Pogo,
        ambition_platformer2d::audio::spec::SoundCueKey::Land,
        ambition_platformer2d::audio::spec::SoundCueKey::Reset,
    ] {
        assert!(
            ids.contains(&cue.sfx_id()),
            "registry must authorize the {cue:?} engine cue it now voices"
        );
    }
}

#[test]
fn the_speedway_authors_a_field_of_collectible_rings() {
    use ambition_platformer2d::entity_catalog::placements::PlacementSchema;
    use ambition_platformer2d::entity_catalog::PickupKind;
    let room = sanic_speedway();
    let rings = room
        .placements
        .iter()
        .filter(|record| {
            record.name == "ring"
                && matches!(
                    &record.schema,
                    PlacementSchema::Pickup(pickup)
                        if matches!(pickup.kind, PickupKind::Currency { amount } if amount >= 1)
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
    // Rings use the shared Currency pickup path, so `collect_ecs_pickups` emits
    // `WORLD_COIN_PICKUP`. The demo authorizes and voices that id; a private
    // `sanic.ring` would be dropped by the authority gate.
    assert_eq!(
        ambition_platformer2d::sfx::SfxId::from_static(SFX_RING),
        ambition_platformer2d::sfx::ids::WORLD_COIN_PICKUP,
        "the ring ding must voice the id the shared currency-pickup loop emits"
    );
    // And the demo's registry authorises it.
    let registry = ambition_platformer2d::audio::spec::SfxRegistry {
        sample_rate: 44_100,
        sfx: sanic_sfx_specs(),
    };
    assert!(
        registry
            .authorized_cue_ids()
            .contains(&ambition_platformer2d::sfx::ids::WORLD_COIN_PICKUP),
        "the Sanic registry must authorise the ring/coin pickup cue"
    );
}

#[test]
fn the_speedway_tags_every_ring_with_the_animated_sprite() {
    use ambition_platformer2d::entity_catalog::placements::PlacementSchema;
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

#[test]
fn the_highway_is_act_two_and_every_loop_on_it_is_attached_data() {
    let room = sanic_highway();
    assert_eq!(room.metadata.mode.as_deref(), Some(SANIC_MODE));
    assert_eq!(
        room.metadata.music_track.as_deref(),
        Some(HIGHWAY_MUSIC_TRACK)
    );
    assert!(
        room.world.size.x > 2.0 * LEVEL_WIDTH,
        "Act 2 is the bigger course: {} wide against the speedway's {LEVEL_WIDTH}",
        room.world.size.x
    );
    // Each loop attaches to the PAINTED floor under it: loops A and D to the
    // ground (`terrain:`), loop B to the sky bridge (`track:`).
    for (loop_name, floor_prefix) in [
        ("highway_loop_a", "terrain:"),
        ("highway_loop_b", "track:"),
        ("highway_loop_d", "terrain:"),
    ] {
        let loop_chain = &room.world.chains[room
            .world
            .chain_named(loop_name)
            .unwrap_or_else(|| panic!("the highway authors {loop_name}"))];
        let floors: Vec<usize> = loop_chain
            .junctions
            .iter()
            .flat_map(|junction| junction.ports.iter())
            .filter_map(|port| match port {
                ae::SurfacePort::Chain { chain, .. } => Some(*chain),
                _ => None,
            })
            .collect();
        assert_eq!(
            floors.len(),
            2,
            "{loop_name} joins its floor at its ramp foot and its runout end"
        );
        assert!(
            floors.iter().all(|&f| f == floors[0]
                && room.world.chains[f].name.starts_with(floor_prefix)),
            "{loop_name} joins one painted {floor_prefix} floor: {:?}",
            floors.iter().map(|&f| &room.world.chains[f].name).collect::<Vec<_>>()
        );
    }
    assert!(
        room.world.validate_surface_junctions().is_empty(),
        "{:?}",
        room.world.validate_surface_junctions()
    );
}

/// Act 2 has height: its ground climbs and drops by more than half a screen,
/// and it is drawn as ground (painted earth), not as a line over the sky.
#[test]
fn the_highway_rolls_climbs_and_drops() {
    let room = sanic_highway();
    let ground: Vec<&ae::SurfaceChain> = room
        .world
        .chains
        .iter()
        .filter(|c| c.name.starts_with("terrain:"))
        .collect();
    assert!(
        ground.iter().any(|c| !c.earth.is_empty()),
        "the highway's ground is painted earth: {:?}",
        room.world.chains.iter().map(|c| &c.name).collect::<Vec<_>>()
    );
    // Its floors: the left → right runs of the painted outline.
    let heights = ground.iter().flat_map(|c| {
        c.points
            .windows(2)
            .filter(|p| p[1].x > p[0].x)
            .flat_map(|p| [p[0].y, p[1].y])
    });
    let (top, bottom) = heights.fold((f32::MAX, f32::MIN), |(lo, hi), y| (lo.min(y), hi.max(y)));
    assert!(
        bottom - top > 500.0,
        "its ground spans {top:.0}..{bottom:.0}: a flat course by another name"
    );
}

#[test]
fn the_three_acts_form_one_course() {
    assert_eq!(
        sanic_speedway().metadata.next_room.as_deref(),
        Some(HIGHWAY_ROOM_ID),
        "the speedway's goal leads to Act 2"
    );
    assert_eq!(
        sanic_highway().metadata.next_room.as_deref(),
        Some(DARKNESS_ROOM_ID),
        "the highway's goal leads to Act 3"
    );
    assert_eq!(
        sanic_darkness().metadata.next_room.as_deref(),
        Some(SPEEDWAY_ROOM_ID),
        "Act 3's goal leads back to Act 1"
    );
    let rooms = provider::sanic_session_world().room_set;
    for id in [SPEEDWAY_ROOM_ID, HIGHWAY_ROOM_ID, DARKNESS_ROOM_ID] {
        assert!(
            rooms.rooms.iter().any(|room| room.id == id),
            "the session's room set holds {id}"
        );
    }
}

#[test]
fn the_dark_act_has_three_portal_pairs_and_room_to_run() {
    use ambition_platformer2d::entity_catalog::placements::PlacementSchema;

    let room = sanic_darkness();
    assert_eq!(
        room.metadata.music_track.as_deref(),
        Some(DARKNESS_MUSIC_TRACK)
    );
    assert!(room.world.size.x >= 2.0 * sanic_highway().world.size.x);
    assert!(room.world.size.y > sanic_highway().world.size.y);
    let portals: Vec<_> = room
        .placements
        .iter()
        .filter_map(|record| match &record.schema {
            PlacementSchema::Portal(portal) => Some(portal.color),
            _ => None,
        })
        .collect();
    assert_eq!(portals.len(), 6);
    for color in &portals {
        assert!(
            portals.contains(&color.partner()),
            "{color:?} needs an exit"
        );
    }
    assert!(room.world.validate_surface_junctions().is_empty());
    assert!(
        room.placements
            .iter()
            .filter(|record| is_ring_placement(record))
            .count()
            >= 200
    );
    assert!(sanic_music_registry().tracks.iter().any(|track| {
        track.id == DARKNESS_MUSIC_TRACK
            && track.asset_path.as_deref() == Some(DARKNESS_MUSIC_ASSET_PATH)
    }));
}

#[test]
fn the_highway_score_is_a_track_the_sanic_catalog_carries() {
    let catalogs = sanic_music_registry();
    assert!(
        catalogs
            .tracks
            .iter()
            .any(|track| track.id == HIGHWAY_MUSIC_TRACK),
        "the highway names `{HIGHWAY_MUSIC_TRACK}` and the Sanic catalog must carry it, or \
         the director falls back to the default track"
    );
}

#[test]
fn every_ring_the_highway_places_is_the_size_a_hit_scatters() {
    let room = sanic_highway();
    let sizes: Vec<_> = room
        .placements
        .iter()
        .filter(|record| is_ring_placement(record))
        .map(|record| record.aabb.max - record.aabb.min)
        .collect();
    assert!(
        sizes.len() >= 40,
        "the premise: a field of rings; got {}",
        sizes.len()
    );
    assert!(sizes
        .iter()
        .all(|size| (*size - ae::Vec2::splat(RING_SIZE)).abs().max_element() < 0.01));
}

#[test]
fn every_ring_the_speedway_places_is_the_size_a_hit_scatters() {
    let room = sanic_speedway();
    let sizes: Vec<_> = room
        .placements
        .iter()
        .filter(|record| is_ring_placement(record))
        .map(|record| record.aabb.max - record.aabb.min)
        .collect();
    assert!(
        sizes.len() >= 30,
        "the premise: a field of rings; got {}",
        sizes.len()
    );
    for size in sizes {
        assert!(
            (size - ae::Vec2::splat(RING_SIZE)).abs().max_element() < 0.01,
            "a placed ring is {size:?} and a scattered one is {RING_SIZE}x{RING_SIZE}, \
             so a ring changes size when it is dropped"
        );
    }
}

/// Rings are a life, not a score. A hit taken holding rings is survived and
/// costs the rings; a hit taken holding none lands normally.
///
/// Feed the Sanic presentation boundary directly. Shared-resolver tests pin
/// survival and wallet spending; these tests pin only the deterministic burst
/// produced from that settled victim-side fact.
fn emit_ring_shield_spend(app: &mut App, victim: bevy::prelude::Entity, amount: i32) {
    let pos = app
        .world()
        .get::<ae::BodyKinematics>(victim)
        .expect("ring-shield victim has kinematics")
        .pos;
    if let Some(mut wallet) = app
        .world_mut()
        .get_mut::<ambition_platformer2d::characters::actor::BodyWallet>(victim)
    {
        wallet.balance = 0;
    }
    app.world_mut()
        .write_message(ambition_platformer2d::damage::WalletShieldSpent {
            victim,
            amount,
            pos,
        });
}

#[test]
fn a_hit_spends_rings_instead_of_health_and_drops_them_back_as_real_pickups() {
    use ambition_platformer2d::characters::actor::{BodyHealth, BodyWallet, Health};
    use ambition_platformer2d::platformer::lifecycle::ActiveSessionScope;

    fn app_with_session() -> App {
        let mut app = App::new();
        app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
        app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
        app.add_message::<ambition_platformer2d::damage::WalletShieldSpent>();
        let mut scope = ActiveSessionScope::default();
        scope.begin();
        app.insert_resource(scope);
        app.add_systems(bevy::prelude::Update, crate::scatter_rings_on_hit);
        app
    }
    fn spawn_sanic(app: &mut App, rings: i32) -> bevy::prelude::Entity {
        let mut kin = ae::BodyKinematics::default();
        kin.size = ae::Vec2::new(28.0, 32.0);
        app.world_mut()
            .spawn((
                ambition_platformer2d::platformer::markers::PlayerEntity,
                ambition_platformer2d::platformer::markers::PrimaryPlayer,
                kin,
                BodyHealth::new(Health::new(3)),
                BodyWallet { balance: rings },
                // Identity the scatter path mints ring ids from — `ensure_sim_id`
                // supplies these at runtime; the harness stamps them directly.
                ambition_platformer2d::platformer::sim_id::SimId::player_slot(0),
                ambition_platformer2d::platformer::sim_id::SimIdCounter::default(),
            ))
            .id()
    }
    fn health(app: &mut App, e: bevy::prelude::Entity) -> i32 {
        app.world().get::<BodyHealth>(e).unwrap().health.current
    }
    fn rings(app: &mut App, e: bevy::prelude::Entity) -> i32 {
        app.world().get::<BodyWallet>(e).unwrap().balance
    }
    fn dropped(app: &mut App) -> usize {
        let mut q = app
            .world_mut()
            .query::<&ambition_platformer2d::combat::components::PickupFeature>();
        q.iter(app.world()).count()
    }

    // ── Holding rings: the hit is spent on them ─────────────────────────────
    let mut app = app_with_session();
    let sanic = spawn_sanic(&mut app, 7);
    emit_ring_shield_spend(&mut app, sanic, 7);
    app.update();

    assert_eq!(
        health(&mut app, sanic),
        3,
        "a hit taken with rings never reaches HP — that is what carrying rings buys"
    );
    assert_eq!(rings(&mut app, sanic), 0, "and it costs every ring");
    assert_eq!(
        dropped(&mut app),
        7,
        "which scatter as real pickups, so they can be run back down"
    );

    // The no-currency lethal path is pinned in the shared resolver tests; this
    // content test owns only the presentation of a successful spend.
}

/// A static fan of rings does not look like the classic burst. Each dropped
/// ring must launch with a real outward velocity, arc away from the body, and
/// only then return to the ordinary pickup economy (so the coin magnet cannot
/// refund it at once).
#[test]
fn scattered_rings_burst_outward_and_then_become_collectible() {
    use ambition_platformer2d::characters::actor::{BodyHealth, BodyWallet, Health};
    use ambition_platformer2d::platformer::lifecycle::ActiveSessionScope;

    let mut app = App::new();
    app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
    app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
    app.add_message::<ambition_platformer2d::damage::WalletShieldSpent>();
    let mut scope = ActiveSessionScope::default();
    scope.begin();
    app.insert_resource(scope);
    app.insert_resource(ambition_platformer2d::time::WorldTime {
        scaled_dt: 0.1,
        ..Default::default()
    });
    app.add_systems(bevy::prelude::Update, crate::scatter_rings_on_hit);

    let body = ae::Vec2::new(100.0, 100.0);
    let mut kin = ae::BodyKinematics::default();
    kin.pos = body;
    kin.size = ae::Vec2::new(28.0, 32.0);
    let sanic = app
        .world_mut()
        .spawn((
            ambition_platformer2d::platformer::markers::PlayerEntity,
            ambition_platformer2d::platformer::markers::PrimaryPlayer,
            kin,
            BodyHealth::new(Health::new(3)),
            BodyWallet { balance: 6 },
            ambition_platformer2d::platformer::sim_id::SimId::player_slot(0),
            ambition_platformer2d::platformer::sim_id::SimIdCounter::default(),
        ))
        .id();

    emit_ring_shield_spend(&mut app, sanic, 6);
    app.update(); // the hit spends the rings → they burst

    // Every lost ring launches with a real outward speed (not a static placement)
    // and is born AT the body.
    let bursts: Vec<crate::ScatteredRing> = {
        let mut q = app.world_mut().query::<&crate::ScatteredRing>();
        q.iter(app.world()).copied().collect()
    };
    assert_eq!(bursts.len(), 6, "all six lost rings burst outward");
    for r in &bursts {
        assert!(
            r.vel.length() >= crate::SCATTER_BURST_SPEED - 0.01,
            "the outer shell launches at the full burst speed, got {}",
            r.vel.length()
        );
        assert_eq!(r.life, crate::SCATTER_LIFE_S, "each ring starts its clock");
    }

    // Radial, not a fan: every quadrant gets a ring, and the velocities sum to
    // nearly zero.
    for (name, right, down) in [
        ("up-right", true, false),
        ("down-right", true, true),
        ("up-left", false, false),
        ("down-left", false, true),
    ] {
        assert!(
            bursts
                .iter()
                .any(|r| (r.vel.x > 0.0) == right && (r.vel.y > 0.0) == down),
            "the burst must throw a ring {name}; got {:?}",
            bursts.iter().map(|r| r.vel).collect::<Vec<_>>()
        );
    }
    let net: ae::Vec2 = bursts.iter().fold(ae::Vec2::ZERO, |acc, r| acc + r.vel);
    assert!(
        net.length() < crate::SCATTER_BURST_SPEED * 0.1,
        "an even radial spray has (almost) no net direction; got {net:?}"
    );

    // Arc them: they move away from the body, then after the lock they return
    // to the ordinary economy.
    app.add_systems(bevy::prelude::Update, crate::arc_scattered_rings);
    app.update();
    let max_dist = ring_spread(&mut app, body);
    assert!(
        max_dist > 0.0,
        "the rings travel outward from the body under the arc"
    );

    // The lock ends but the ring does NOT: it keeps arcing, now collectible.
    for _ in 0..8 {
        app.update();
    }
    let mid_life = {
        let mut q = app.world_mut().query::<&crate::ScatteredRing>();
        q.iter(app.world()).count()
    };
    assert_eq!(
        mid_life, 6,
        "past the untouchable window a ring is still a ring — collectible, not gone"
    );

    // …and then it expires. A scatter that lasts forever is not a cost.
    for _ in 0..40 {
        app.update();
    }
    let remaining = {
        let mut q = app.world_mut().query::<&crate::ScatteredRing>();
        q.iter(app.world()).count()
    };
    assert_eq!(remaining, 0, "every uncollected ring eventually disappears");
    let pickups = {
        let mut q = app
            .world_mut()
            .query_filtered::<(), bevy::prelude::With<ambition_platformer2d::combat::components::PickupFeature>>();
        q.iter(app.world()).count()
    };
    assert_eq!(
        pickups, 0,
        "an expired ring leaves no orphan pickup behind to be collected later"
    );
}

/// Max distance any live scattered ring has travelled from `origin`.
fn ring_spread(app: &mut App, origin: ae::Vec2) -> f32 {
    let mut q = app
        .world_mut()
        .query_filtered::<&ae::CenteredAabb, bevy::prelude::With<crate::ScatteredRing>>();
    q.iter(app.world())
        .map(|a| (a.center - origin).length())
        .fold(0.0_f32, f32::max)
}

/// A ring bounces off the floor instead of falling through the level, against
/// real room geometry.
#[test]
fn a_scattered_ring_bounces_off_the_floor_it_lands_on() {
    let floor_y = 260.0;
    let world = ae::World::new(
        "ring-bounce",
        ae::Vec2::new(800.0, 600.0),
        ae::Vec2::new(64.0, 64.0),
        vec![ae::Block::solid(
            "floor",
            ae::Vec2::new(0.0, floor_y),
            ae::Vec2::new(800.0, 40.0),
        )],
    );

    let mut app = App::new();
    app.insert_resource(ambition_platformer2d::time::WorldTime {
        scaled_dt: 1.0 / 60.0,
        ..Default::default()
    });
    let mut scope = ambition_platformer2d::platformer::lifecycle::ActiveSessionScope::default();
    let session = scope.begin();
    app.insert_resource(scope);
    app.world_mut().spawn((
        ambition_platformer2d::platformer::lifecycle::SessionRoot(session),
        ae::RoomGeometry(world),
    ));
    let ring = app
        .world_mut()
        .spawn((
            crate::ScatteredRing {
                // Straight down, fast, from just above the floor.
                vel: ae::Vec2::new(0.0, 400.0),
                lock: crate::SCATTER_LOCK_S,
                life: crate::SCATTER_LIFE_S,
            },
            ae::CenteredAabb::from_center_size(
                ae::Vec2::new(400.0, floor_y - 40.0),
                ae::Vec2::splat(18.0),
            ),
        ))
        .id();
    app.add_systems(bevy::prelude::Update, crate::arc_scattered_rings);

    let mut rebounded = false;
    let mut deepest = f32::MIN;
    for _ in 0..30 {
        app.update();
        let Some(aabb) = app.world().get::<ae::CenteredAabb>(ring).copied() else {
            break;
        };
        deepest = deepest.max(aabb.center.y + aabb.half_size.y);
        if app.world().get::<crate::ScatteredRing>(ring).unwrap().vel.y < 0.0 {
            rebounded = true;
        }
    }
    assert!(
        rebounded,
        "the ring must come back UP off the floor — a ring that only falls is          the bug (it sinks through the level and is never recoverable)"
    );
    assert!(
        deepest <= floor_y + 1.0,
        "the ring never penetrates the floor; deepest edge {deepest} vs floor {floor_y}"
    );
}

/// The whole chain in production order: the rings are not reclaimed the
/// instant they spawn on top of the player.
#[test]
fn the_ring_burst_is_not_reclaimed_on_spawn_under_the_real_chain() {
    use ambition_platformer2d::characters::actor::{BodyHealth, BodyWallet, Health};
    use ambition_platformer2d::platformer::lifecycle::ActiveSessionScope;
    use bevy::prelude::{IntoScheduleConfigs, With};

    let mut app = App::new();
    app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
    app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
    app.add_message::<ambition_platformer2d::damage::WalletShieldSpent>();
    app.add_message::<ambition_platformer2d::actors::avatar::PlayerHealRequested>();
    app.add_message::<ambition_platformer2d::combat::events::SetFlagRequested>();
    app.insert_resource(ambition_platformer2d::combat::events::GameplayBanner::default());
    let mut scope = ActiveSessionScope::default();
    let session = scope.begin();
    app.insert_resource(scope);
    app.insert_resource(ambition_platformer2d::time::WorldTime {
        scaled_dt: 0.1,
        ..Default::default()
    });
    app.world_mut().spawn((
        ambition_platformer2d::platformer::lifecycle::SessionRoot(session),
        ae::RoomGeometry(ae::World::new(
            "ring-chain",
            ae::Vec2::new(800.0, 600.0),
            ae::Vec2::new(200.0, 200.0),
            vec![ae::Block::solid(
                "floor",
                ae::Vec2::new(0.0, 260.0),
                ae::Vec2::new(800.0, 40.0),
            )],
        )),
    ));
    // The real production order: magnet, then the burst arc, then collect.
    app.add_systems(
        bevy::prelude::Update,
        (
            crate::scatter_rings_on_hit,
            ambition_platformer2d::actors::features::magnetize_pickups,
            crate::arc_scattered_rings,
            ambition_platformer2d::actors::features::collect_ecs_pickups,
        )
            .chain(),
    );

    let body = ae::Vec2::new(200.0, 200.0);
    let mut kin = ae::BodyKinematics::default();
    kin.pos = body;
    kin.size = ae::Vec2::new(28.0, 32.0);
    let sanic = app
        .world_mut()
        .spawn((
            ambition_platformer2d::platformer::markers::PlayerEntity,
            ambition_platformer2d::platformer::markers::PrimaryPlayer,
            kin,
            BodyHealth::new(Health::new(3)),
            BodyWallet { balance: 6 },
            ambition_platformer2d::platformer::sim_id::SimId::player_slot(0),
            ambition_platformer2d::platformer::sim_id::SimIdCounter::default(),
        ))
        .id();
    let wallet = |app: &App| app.world().get::<BodyWallet>(sanic).unwrap().balance;
    let locked = |app: &mut App| {
        let mut q = app
            .world_mut()
            .query_filtered::<(), With<ambition_platformer2d::actors::features::PickupCollectLock>>(
            );
        q.iter(app.world()).count()
    };

    emit_ring_shield_spend(&mut app, sanic, 6);
    app.update(); // the hit spends the rings → they burst (spawned via commands)
    app.update(); // the burst entities now exist; the full chain processes them

    assert_eq!(wallet(&app), 0, "the burst must NOT be refunded on spawn");
    assert!(
        locked(&mut app) > 0,
        "the burst rings carry the collection lock"
    );

    // Still inside the lock window: no refund, and the rings travel away.
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(wallet(&app), 0, "still uncollected while locked");
    let max_dist = ring_spread(&mut app, body);
    assert!(
        max_dist > 20.0,
        "the rings separated from the body, got {max_dist}"
    );

    // Past the lock, the rings unlock and become ordinary collectibles. Park the
    // player on any survivor and prove it now credits through the shared path.
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(
        locked(&mut app),
        0,
        "the lock is gone once the burst settles"
    );
    let ring_pos = {
        let mut q = app
            .world_mut()
            .query_filtered::<&ae::CenteredAabb, With<ambition_platformer2d::combat::components::PickupFeature>>();
        q.iter(app.world()).next().map(|a| a.center)
    };
    if let Some(pos) = ring_pos {
        app.world_mut()
            .get_mut::<ae::BodyKinematics>(sanic)
            .unwrap()
            .pos = pos;
        app.update();
    }
    assert!(
        wallet(&app) > 0,
        "an unlocked ring the player reaches is collected (run back and grab them)"
    );
}

/// Dropped ring ids must be deterministic and unique. `entity.index()` would
/// collide when a second burst by the same player lands while the first
/// burst's rings exist. Each ring mints from the spawner's own `SimIdCounter`
/// (one monotonic stream per body, ADR 0030), so overlapping bursts get
/// disjoint ids. Each ring has a real `SimId::spawned` and
/// `SpawnOrigin::Dynamic` parented to the player.
#[test]
fn overlapping_ring_bursts_never_reuse_a_dropped_ring_id() {
    use ambition_platformer2d::characters::actor::{BodyHealth, BodyWallet, Health};
    use ambition_platformer2d::platformer::construction::SpawnOrigin;
    use ambition_platformer2d::platformer::lifecycle::ActiveSessionScope;
    use ambition_platformer2d::platformer::sim_id::{SimId, SimIdCounter};

    let mut app = App::new();
    app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
    app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
    app.add_message::<ambition_platformer2d::damage::WalletShieldSpent>();
    let mut scope = ActiveSessionScope::default();
    scope.begin();
    app.insert_resource(scope);
    app.add_systems(bevy::prelude::Update, crate::scatter_rings_on_hit);

    let mut kin = ae::BodyKinematics::default();
    kin.size = ae::Vec2::new(28.0, 32.0);
    let player_id = SimId::player_slot(0);
    let sanic = app
        .world_mut()
        .spawn((
            ambition_platformer2d::platformer::markers::PlayerEntity,
            ambition_platformer2d::platformer::markers::PrimaryPlayer,
            kin,
            BodyHealth::new(Health::new(9)),
            BodyWallet { balance: 4 },
            player_id.clone(),
            SimIdCounter::default(),
        ))
        .id();
    emit_ring_shield_spend(&mut app, sanic, 4);
    app.update(); // burst 1 (four rings); health was never touched
    app.world_mut()
        .get_mut::<BodyWallet>(sanic)
        .unwrap()
        .balance = 4;
    emit_ring_shield_spend(&mut app, sanic, 4);
    app.update(); // burst 2 (four more) while burst-1 rings still exist

    // The FeatureId string is derived from the ring's SimId, so uniqueness there
    // is uniqueness of identity.
    let ids: Vec<String> = {
        let mut q = app
            .world_mut()
            .query::<&ambition_platformer2d::combat::components::FeatureId>();
        q.iter(app.world()).map(|f| f.0.clone()).collect()
    };
    let unique: std::collections::HashSet<_> = ids.iter().cloned().collect();
    assert_eq!(
        ids.len(),
        unique.len(),
        "two overlapping bursts must never reuse a dropped-ring id: {ids:?}"
    );
    assert_eq!(
        ids.len(),
        8,
        "four rings per burst, two bursts, every id distinct"
    );

    // Every dropped ring is a dynamic entity: a `SimId::spawned` parented to
    // this player plus the matching `SpawnOrigin::Dynamic`, so a rollback
    // rebase can rebuild it. The eight sequences are the player's stream 0..8.
    let mut rings: Vec<(SimId, SpawnOrigin)> = {
        let mut q = app
            .world_mut()
            .query_filtered::<(&SimId, &SpawnOrigin), bevy::prelude::With<crate::ScatteredRing>>();
        q.iter(app.world())
            .map(|(id, origin)| (id.clone(), origin.clone()))
            .collect()
    };
    assert_eq!(rings.len(), 8, "each burst ring carries a dynamic SimId");
    let mut sequences: Vec<u64> = rings
        .iter()
        .map(|(_, origin)| match origin {
            SpawnOrigin::Dynamic { parent, sequence } => {
                assert_eq!(
                    parent, &player_id,
                    "the ring's spawn parent is the player that dropped it"
                );
                *sequence
            }
            other => panic!("a scattered ring must be SpawnOrigin::Dynamic, got {other:?}"),
        })
        .collect();
    sequences.sort_unstable();
    assert_eq!(
        sequences,
        (0..8).collect::<Vec<_>>(),
        "the two bursts draw one contiguous per-spawner stream, no gaps or reuse"
    );
    // The SimId string is the spawner's id with the sequence appended.
    rings.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
    for (id, _) in &rings {
        assert!(
            id.as_str().starts_with(player_id.as_str()),
            "the ring SimId descends from the player's id: {}",
            id.as_str()
        );
    }
}

/// Going fast has to pay, and rings have to cost something to keep.
///
/// The act score is pure arithmetic that can be backwards and still look
/// right: swap the time term's sign and a slow run wins; drop the ring term
/// and the scatter stops mattering.
#[test]
fn the_act_score_pays_for_speed_and_for_rings_kept() {
    use crate::{act_score, act_time_text, ACT_PAR_SECONDS};

    // Faster is worth more, all else equal.
    let quick = act_score(20.0, 0);
    let slow = act_score(50.0, 0);
    assert!(
        quick > slow,
        "a faster run must score higher ({quick} vs {slow}) — this is the whole \
         premise of a momentum demo"
    );

    // Rings kept are worth something, all else equal. The fast line usually
    // costs rings, so the two terms must pull against each other.
    assert!(
        act_score(20.0, 30) > act_score(20.0, 0),
        "rings you finish holding must be worth keeping"
    );

    // Past par the time bonus floors at zero; a slow run does not owe back
    // its ring bonus.
    let past_par = act_score(ACT_PAR_SECONDS + 30.0, 10);
    assert_eq!(
        past_par,
        act_score(ACT_PAR_SECONDS, 10),
        "the time bonus clamps at par rather than turning negative"
    );
    assert!(past_par > 0, "and a slow run still keeps its ring bonus");

    assert_eq!(act_time_text(83.0), "1:23");
}

/// The splash is wide enough to be a scramble: a lost purse throws rings far
/// enough that getting them back is a run.
#[test]
fn the_ring_splash_is_wide_enough_to_be_a_scramble() {
    let mut app = App::new();
    app.insert_resource(ambition_platformer2d::time::WorldTime {
        scaled_dt: 1.0 / 60.0,
        ..Default::default()
    });
    app.add_systems(bevy::prelude::Update, crate::arc_scattered_rings);

    // The six directions one shell launches in, at the shell's own speed.
    let launched: Vec<bevy::prelude::Entity> = (0..6)
        .map(|i| {
            let t = (i as f32 + 0.5) / 6.0;
            let angle = std::f32::consts::TAU * t;
            let vel = ae::Vec2::new(angle.cos(), angle.sin()) * crate::SCATTER_BURST_SPEED;
            app.world_mut()
                .spawn((
                    crate::ScatteredRing {
                        vel,
                        lock: crate::SCATTER_LOCK_S,
                        life: crate::SCATTER_LIFE_S,
                    },
                    ae::CenteredAabb::from_center_size(ae::Vec2::ZERO, ae::Vec2::splat(18.0)),
                ))
                .id()
        })
        .collect();

    // Run until they fall a tile below the launch height. With no room
    // geometry there is nothing to land on, so this measures the spray, not
    // the drift.
    let mut widest = 0.0f32;
    for _ in 0..(60 * 2) {
        app.update();
        for entity in &launched {
            if let Some(aabb) = app.world().get::<ae::CenteredAabb>(*entity) {
                if aabb.center.y <= 32.0 {
                    widest = widest.max(aabb.center.x.abs());
                }
            }
        }
    }

    let tiles = widest / 32.0;
    println!(
        "[ring splash] half-width {widest:.1}px = {tiles:.1} tiles; full {:.1} tiles",
        tiles * 2.0
    );
    assert!(
        tiles >= 8.0,
        "the ring spray reaches only {tiles:.1} tiles from the body, so a lost \
         purse lands at your feet. Jon asked for a splash you have to chase."
    );
}

/// The sign at the start line names the keys the player has.
///
/// The generated text is the default; the presentation pass replaces it once
/// a seat exists.
#[test]
fn the_start_line_legend_follows_the_seats_real_bindings() {
    use ambition_platformer2d::bevy::ecs::system::RunSystemOnce as _;
    use ambition_platformer2d::input::{
        ActionBindings, InputParticipant, KeyboardPreset, SeatBindings,
    };
    use ambition_platformer2d::render::rendering::{WorldLabel, WorldLabelFamily};

    // The text the built room ships with, read from the room, so this cannot
    // pass against a legend that moved.
    let room = crate::sanic_speedway();
    let baked = room
        .debug_labels
        .iter()
        .find(|label| label.id.ends_with(crate::LEGEND_LABEL_ID))
        .map(|label| label.payload.text.clone())
        .expect("the speedway signs its start line");

    let mut app = App::new();
    app.init_resource::<SeatBindings>();
    let sign = app
        .world_mut()
        .spawn((
            bevy::prelude::Text2d::new(baked.clone()),
            // The owner id the renderer really builds: `signage:{index}:{id}`
            // over the room's already-prefixed authored id.
            WorldLabel::new(
                format!(
                    "signage:0:{}",
                    room.debug_labels
                        .iter()
                        .find(|label| label.id.ends_with(crate::LEGEND_LABEL_ID))
                        .map(|label| label.id.clone())
                        .expect("the speedway signs its start line")
                ),
                WorldLabelFamily::Signage,
                bevy::prelude::Vec3::ZERO,
            ),
        ))
        .id();
    // A seat on a preset that is NOT the one generation could see.
    let wasd = KeyboardPreset::wasd_jkl();
    app.world_mut()
        .spawn((InputParticipant::primary(), wasd.input_map()));
    app.world_mut()
        .run_system_once(ambition_platformer2d::input::publish_seat_bindings)
        .expect("the projection runs");
    app.world_mut()
        .run_system_once(crate::refresh_sanic_control_legend)
        .expect("the legend refresh runs");

    let shown = app
        .world()
        .get::<bevy::prelude::Text2d>(sign)
        .unwrap()
        .0
        .clone();
    assert_ne!(
        shown, baked,
        "the sign still shows the preset room generation guessed at"
    );
    let jump = ActionBindings::from_map(&wasd.input_map())
        .label(&ambition_platformer2d::input::Platformer2dInputActionMonolith::Jump)
        .expect("wasd binds Jump");
    assert!(
        shown.contains(&format!("{jump}: JUMP")),
        "the sign names the key this seat jumps with ({jump}), got: {shown}"
    );
}

/// Losing your rings buys a few seconds of recovery, as in the classic games.
///
/// The engine's `knockback_invulnerability_time` (0.75s) is too short here.
/// `WalletShieldSpent` leaves the expression to content, so Sanic extends the
/// window in the handler that decides what losing rings means.
#[test]
fn losing_the_purse_buys_a_classic_length_recovery() {
    use ambition_platformer2d::characters::actor::{BodyCombat, BodyHealth, BodyWallet, Health};
    use ambition_platformer2d::platformer::lifecycle::ActiveSessionScope;

    let mut app = App::new();
    app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
    app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
    app.add_message::<ambition_platformer2d::damage::WalletShieldSpent>();
    let mut scope = ActiveSessionScope::default();
    scope.begin();
    app.insert_resource(scope);
    app.insert_resource(ambition_platformer2d::time::WorldTime {
        scaled_dt: 0.1,
        ..Default::default()
    });
    app.add_systems(bevy::prelude::Update, crate::scatter_rings_on_hit);

    let mut kin = ae::BodyKinematics::default();
    kin.pos = ae::Vec2::new(100.0, 100.0);
    kin.size = ae::Vec2::new(28.0, 32.0);
    let sanic = app
        .world_mut()
        .spawn((
            ambition_platformer2d::platformer::markers::PlayerEntity,
            ambition_platformer2d::platformer::markers::PrimaryPlayer,
            kin,
            BodyHealth::new(Health::new(3)),
            BodyWallet { balance: 6 },
            // What the resolver armed on the way in — the window this is about.
            BodyCombat {
                damage_invuln_timer: 0.75,
                ..Default::default()
            },
            ambition_platformer2d::platformer::sim_id::SimId::player_slot(0),
            ambition_platformer2d::platformer::sim_id::SimIdCounter::default(),
        ))
        .id();

    emit_ring_shield_spend(&mut app, sanic, 6);
    app.update();

    let armed = app
        .world()
        .get::<BodyCombat>(sanic)
        .expect("Sanic keeps his combat state")
        .damage_invuln_timer;
    assert!(
        armed >= crate::RING_LOSS_INVULN_S,
        "losing the purse left only {armed}s of recovery — the rings have not \
         even landed yet, and the badnik that hit him is still touching him"
    );

    // Raise, do not replace: a longer running window (a hazard respawn) is
    // not shortened by dropping rings inside it.
    app.world_mut()
        .get_mut::<BodyCombat>(sanic)
        .expect("still there")
        .damage_invuln_timer = crate::RING_LOSS_INVULN_S + 5.0;
    emit_ring_shield_spend(&mut app, sanic, 1);
    app.update();
    let after = app
        .world()
        .get::<BodyCombat>(sanic)
        .expect("still there")
        .damage_invuln_timer;
    assert!(
        after >= crate::RING_LOSS_INVULN_S + 5.0,
        "a longer window already running was CUT SHORT to {after}s by a later \
         ring loss"
    );
}

/// The Utility button's label reads "Transform", never "Fly". Routing and
/// label are independent: an authored `display_name` or the engine's
/// `fly_toggle` reclaiming the slot could put "Fly" back while routing still
/// works.
#[test]
fn the_utility_button_reads_transform_and_never_fly() {
    use ambition_platformer2d::characters::action_scheme::derive_action_scheme;
    use ambition_platformer2d::entity_catalog::action_scheme::ControlSlot;

    // The body has wings, so the engine's `fly_toggle` would claim Utility if
    // the declared technique did not outrank it.
    let mut abilities = ae::AbilitySet::basic();
    abilities.fly = true;
    abilities.fly_toggle = true;
    let scheme = derive_action_scheme(&abilities, None, None, &[super::transform_technique()]);

    let utility = scheme
        .action_for_slot(ControlSlot::Utility)
        .expect("Sanic claims the Utility slot");
    assert_eq!(
        utility.display(),
        "Transform",
        "⛔ the transform button reads {:?}",
        utility.display()
    );
    // The utility label must not expose the generic flight verb.
    assert!(
        !utility.display().to_lowercase().contains("fly"),
        "the button is wearing the generic flight verb again"
    );
}

/// Past any vertex by the smallest step a rider takes, `frame_at` names the
/// NEXT segment. A painted outline is one chain around the whole level
/// (~66k px), and while `frame_at` walked `s -= len` and `arc_at_vertex`
/// summed, the two disagreed by more than that step at s ≈ 60k: the rider
/// crossed the same 14° joint forever, frozen at full speed at x = 26374.
#[test]
fn every_painted_joint_can_be_ridden_past() {
    for room in [crate::sanic_darkness(), crate::sanic_highway()] {
        for chain in room.world.chains.iter().filter(|c| c.name.starts_with("terrain:")) {
            for v in 1..chain.segment_count() {
                let arc = chain.arc_at_vertex(v);
                let step = (arc.abs() * f32::EPSILON * 8.0).max(1.0e-4);
                assert_eq!(
                    chain.frame_at(arc + step).segment,
                    v,
                    "{} vertex {v} at arc {arc}",
                    chain.name
                );
            }
        }
    }
}
