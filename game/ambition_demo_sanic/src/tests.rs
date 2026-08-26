//! Unit tests for the standalone Sanic content and rules plugin.

mod speedway_oracles;

use super::*;

#[test]
fn sanic_demo_content_plugin_installs() {
    // The direct-entry content plugin publishes an exact PreparedContent root at
    // plugin-build time. That contract deliberately depends on the engine having
    // installed its construction registries first, matching the standalone Sanic
    // shell's real composition order. A bare App only tests catalog registration
    // and cannot validate or fingerprint the speedway's authored ring placements.
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
    let reset_blocks: Vec<&str> = room
        .world
        .blocks
        .iter()
        .filter(|b| matches!(b.kind, ae::BlockKind::Hazard))
        .map(|b| b.name.as_str())
        .collect();
    assert_eq!(
        reset_blocks.len(),
        1,
        "the PIT is the speedway's only reset-to-spawn hazard: {reset_blocks:?}"
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
        ["mid_spikes"],
        "and the mid-course strip is a DAMAGE volume, so a hit costs rings \
         rather than the whole run"
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
                contact: ae::BodyContactField::NONE,
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
    app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
    app.world_mut().spawn((
        ambition_platformer2d::actors::actor::PrimaryPlayer,
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

/// The transformation fires from the DECLARED Utility technique, and the
/// declaration is what consumes the raw verb.
///
/// Both halves are the engine's now: because the body declares `transform` on
/// `ControlSlot::Utility`, `resolve_control_slots` routes the press to the sanctioned edge AND
/// clears the verb, so generic flight can never see it.
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

    // The body has WINGS, so the engine's own `fly_toggle` action would otherwise
    // claim Utility. That the technique wins is the override this relies on.
    let mut abilities = ae::AbilitySet::basic();
    abilities.fly = true;
    abilities.fly_toggle = true;
    let scheme = derive_action_scheme(&abilities, None, None, &[super::transform_technique()]);

    // Stand in for the persona gate: press the device verb, run THE resolver.
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
/// The engine's attribution sweep converted every ability, damage path and projectile impact, which
/// made the infrastructure look finished while the flagship character content was still writing
/// through the session context . In a Sanic-only game that is invisible, because the session's
/// provider and the character's provider are the same string.
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
            ambition_platformer2d::actors::actor::PrimaryPlayer,
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
/// H2 classified every call site as body-owned or world-owned, and the
/// world-owned half was still wrong: `write_global`
/// reaches for the session context, so under a shell host a distance marker was
/// credited to the launcher. A distance marker is not a body's sound — no body
/// caused it, the ROOM did — but it is emphatically Sanic's, and the third
/// operation is what lets a call site say so.
///
/// Same fixture shape as the transformation test above and for the same reason:
/// the session belongs to `some_host`, so the two answers differ. In a
/// Sanic-only game they are the same string and nothing is observable.
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
        ambition_platformer2d::actors::actor::PrimaryPlayer,
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
    use ambition_platformer2d::world::rooms::{ActiveRoomMetadata, RoomMetadata};

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
            ActiveRoomMetadata(RoomMetadata {
                mode: mode.map(str::to_string),
                ..Default::default()
            }),
        );
        app.insert_resource(ambition_platformer2d::time::WorldTime {
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
    ambition_platformer2d::platformer::lifecycle::insert_session_world_component(
        app.world_mut(),
        ActiveRoomMetadata::default(),
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
/// condition that wakes a hosted ruleset inside it reaches this crate
/// through the `ambition_platformer2d` umbrella alone. If gating a hosted demo ever
/// needs a lower `ambition_*` crate, it fails to compile HERE.
///
/// The condition is evaluated directly rather than through `.run_if` on a
/// bespoke marker resource: a crate whose manifest names only `ambition_platformer2d`
/// cannot `#[derive(Resource)]`, because bevy's derive macros resolve
/// `bevy_ecs` through the CONSUMER's manifest and a re-export does not
/// satisfy them. The `.run_if` wiring itself is pinned in
/// `ambition_platformer2d_runtime/tests/mode_scope.rs`.
#[test]
fn the_speedway_claims_the_sanic_mode_and_wakes_a_hosted_ruleset() {
    use ambition_platformer2d::bevy::ecs::system::RunSystemOnce as _;
    use ambition_platformer2d::runtime::in_mode;
    use ambition_platformer2d::world::rooms::ActiveRoomMetadata;

    let room = sanic_speedway();
    assert_eq!(room.metadata.mode.as_deref(), Some(SANIC_MODE));

    let mut app = App::new();
    ambition_platformer2d::platformer::lifecycle::insert_session_world_component(
        app.world_mut(),
        ActiveRoomMetadata(room.metadata.clone()),
    );
    let awake = app
        .world_mut()
        .run_system_once(in_mode(SANIC_MODE))
        .expect("the mode condition runs");
    assert!(awake, "a hosted Sanic ruleset wakes inside the speedway");

    // Ambition's own rooms carry no mode, so the demo's rules sleep there.
    ambition_platformer2d::platformer::lifecycle::insert_session_world_component(
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
    // BOTH halves, ordered as the app orders them. `sync_super_form_traits`
    // states the super form's TRAITS and the engine's empowerment runs them —
    // running only the first would assert that a grant was made, which is
    // exactly the half-wiring that makes an opt-in component do nothing.
    //
    // the second half is no longer this test's to install.
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
            ambition_platformer2d::actors::actor::PrimaryPlayer,
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
    // The transformation must be more than a sprite swap: the super row's authored momentum
    // strictly dominates the base row's.
    let fragment =
        ambition_platformer2d::characters::actor::character_catalog::CharacterCatalogFragment::from_ron(
            provider::SANIC_EXPERIENCE,
            Some(SANIC_CHARACTER_ID),
            SANIC_CATALOG_RON,
        )
        .expect("demo catalog parses");
    let catalog =
        ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog::from_data(
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
    use ambition_platformer2d::entity_catalog::PickupKind;
    use ambition_platformer2d::entity_catalog::placements::PlacementSchema;
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
    // Rings ride the shared Currency pickup path, so `collect_ecs_pickups` emits
    // `WORLD_COIN_PICKUP` on collect. The demo authorises + voices exactly that
    // id (a private `sanic.ring` would be silently dropped by the authority gate).
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
    app.world_mut().write_message(
        ambition_platformer2d::actors::features::ecs::damage_apply::WalletShieldSpent {
            victim,
            amount,
            pos,
        },
    );
}

#[test]
fn a_hit_spends_rings_instead_of_health_and_drops_them_back_as_real_pickups() {
    use ambition_platformer2d::characters::actor::{BodyHealth, BodyWallet, Health};
    use ambition_platformer2d::platformer::lifecycle::ActiveSessionScope;

    fn app_with_session() -> App {
        let mut app = App::new();
        app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
        app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
        app.add_message::<ambition_platformer2d::actors::features::ecs::damage_apply::WalletShieldSpent>();
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
            .query::<&ambition_platformer2d::actors::features::PickupFeature>();
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

/// that PLACES rings in a static fan looks nothing like the classic burst. So
/// each dropped ring must launch with a real outward velocity, ARC away from the
/// body, and only THEN hand back to the ordinary pickup economy (so the coin
/// magnet can't refund them the same instant they drop).
#[test]
fn scattered_rings_burst_outward_and_then_become_collectible() {
    use ambition_platformer2d::characters::actor::{BodyHealth, BodyWallet, Health};
    use ambition_platformer2d::platformer::lifecycle::ActiveSessionScope;

    let mut app = App::new();
    app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
    app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
    app.add_message::<ambition_platformer2d::actors::features::ecs::damage_apply::WalletShieldSpent>();
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

    // Every lost ring launches with a REAL outward speed (not a static placement)
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

    // RADIAL, not a fan. Every quadrant gets a ring, and the velocities sum to
    // (nearly) nothing — which is what "even spray in all directions" means and
    // what an upward fan can never satisfy, however wide you make it.
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

    // Arc them: they move AWAY from the body (the whole point of "explode
    // outward"), then after the lock they hand off to the ordinary economy.
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

    // …and then it expires. A scatter you can come back to forever is not a cost.
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
            .query_filtered::<(), bevy::prelude::With<ambition_platformer2d::actors::features::PickupFeature>>();
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

/// classic scatter. This pins the BEHAVIOUR half — a ring bounces off the floor
/// instead of falling through the level — against real room geometry.
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

/// The isolation test above ran the arc alone and so never proved the rings aren't reclaimed the
/// instant they spawn on top of the player. Here the whole chain runs in production order.
#[test]
fn the_ring_burst_is_not_reclaimed_on_spawn_under_the_real_chain() {
    use ambition_platformer2d::characters::actor::{BodyHealth, BodyWallet, Health};
    use ambition_platformer2d::platformer::lifecycle::ActiveSessionScope;
    use bevy::prelude::{IntoScheduleConfigs, With};

    let mut app = App::new();
    app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
    app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
    app.add_message::<ambition_platformer2d::actors::features::ecs::damage_apply::WalletShieldSpent>();
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
    // The REAL production order: magnet, then the burst arc, then collect.
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
            .query_filtered::<&ae::CenteredAabb, With<ambition_platformer2d::actors::features::PickupFeature>>();
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

/// be DETERMINISTIC and unique — never `entity.index()`, which collides when a
/// second burst by the SAME player lands while the first burst's rings still
/// exist. Minting each ring from the SPAWNER's own `SimIdCounter` (one
/// monotonic stream per body, ADR 0030) makes two overlapping bursts mint
/// disjoint ids — and every ring carries a real `SimId::spawned` +
/// `SpawnOrigin::Dynamic` parented to the player, not just a bare label.
#[test]
fn overlapping_ring_bursts_never_reuse_a_dropped_ring_id() {
    use ambition_platformer2d::characters::actor::{BodyHealth, BodyWallet, Health};
    use ambition_platformer2d::platformer::construction::SpawnOrigin;
    use ambition_platformer2d::platformer::lifecycle::ActiveSessionScope;
    use ambition_platformer2d::platformer::sim_id::{SimId, SimIdCounter};

    let mut app = App::new();
    app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
    app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
    app.add_message::<ambition_platformer2d::actors::features::ecs::damage_apply::WalletShieldSpent>();
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
            .query::<&ambition_platformer2d::actors::features::FeatureId>();
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

    // Every dropped ring is a first-class dynamic entity: a `SimId::spawned`
    // parented to THIS player plus the matching `SpawnOrigin::Dynamic`, so a
    // rollback rebase can reconstruct it — the identity the old global counter
    // never gave it. The eight sequences are the player's own stream 0..8.
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

/// Going fast has to PAY, and rings have to cost something to keep.
///
/// The act score is the only place the demo's premise is expressed as a number,
/// and it is pure arithmetic that reads correctly while being backwards: swap
/// the time term's sign and a slow run wins; drop the ring term and the scatter
/// mechanic stops mattering. Neither shows on screen — you would just have a
/// game where the safe line is always right, which is the exact thing this demo
/// exists to disprove.
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

    // Rings kept are worth something, all else equal. Together with the scatter
    // rule this is the tension: the fast line is usually the one that costs you
    // rings, so the two terms have to pull against each other.
    assert!(
        act_score(20.0, 30) > act_score(20.0, 0),
        "rings you finish holding must be worth keeping"
    );

    // Past par the time bonus floors instead of going negative — a slow run
    // scores poorly, it does not owe the player's ring bonus back.
    let past_par = act_score(ACT_PAR_SECONDS + 30.0, 10);
    assert_eq!(
        past_par,
        act_score(ACT_PAR_SECONDS, 10),
        "the time bonus clamps at par rather than turning negative"
    );
    assert!(past_par > 0, "and a slow run still keeps its ring bonus");

    assert_eq!(act_time_text(83.0), "1:23");
}

/// The splash is wide enough to be a scramble.
///
/// opportunity to recollect some of them after his hitstun wears off and before
/// they disappear."*
///
/// What it defends is the property: a lost purse throws rings far enough that getting them back is
/// a RUN.
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

    // Run until they have fallen a tile below the launch height — with no room
    // geometry there is nothing to land on, so "it would have hit the ground" is
    // the honest stopping point for measuring the SPRAY rather than the drift.
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

/// The sign at the start line names the keys the player actually has.
///
/// it did not. The generated text is the honest default; the presentation pass replaces it once a
/// seat exists.
#[test]
fn the_start_line_legend_follows_the_seats_real_bindings() {
    use ambition_platformer2d::bevy::ecs::system::RunSystemOnce as _;
    use ambition_platformer2d::input::{
        ActionBindings, InputParticipant, KeyboardPreset, SeatBindings,
    };
    use ambition_platformer2d::render::rendering::{WorldLabel, WorldLabelFamily};

    // The text the room really ships with — read from the built room rather
    // than restated, so this cannot pass against a legend that moved.
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

/// Losing your rings buys you a few seconds, the way it always has.
///
/// iframes. He should also have some hitstun and be knocked back a bit, and then
/// have a few second of recovery iframes."*
///
/// the i-frames were never missing — they were 0.75s, the engine's
/// `knockback_invulnerability_time`, whose own comment calls it "the longest window in the game".
/// It is, for Ambition.
///
/// `WalletShieldSpent`'s own contract says where this belongs: *"The generic resolver owns
/// survival; game content owns how it is expressed."* Losing your rings IS the classic trigger for
/// the flashing window, so Sanic extends it in the same handler that already decides what losing
/// them means.
#[test]
fn losing_the_purse_buys_a_classic_length_recovery() {
    use ambition_platformer2d::characters::actor::{BodyCombat, BodyHealth, BodyWallet, Health};
    use ambition_platformer2d::platformer::lifecycle::ActiveSessionScope;

    let mut app = App::new();
    app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
    app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
    app.add_message::<ambition_platformer2d::actors::features::ecs::damage_apply::WalletShieldSpent>();
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

    // and it must RAISE rather than replace: a longer window already running
    // (a hazard respawn, say) is not shortened by dropping rings inside it.
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

/// but nothing pinned the LABEL, which is the half he could actually see. The two are
/// independent: the technique could keep routing correctly while an authored `display_name`, or
/// the engine's `fly_toggle` reclaiming the slot, put "Fly" back on the button — and every
/// existing test would stay green.
#[test]
fn the_utility_button_reads_transform_and_never_fly() {
    use ambition_platformer2d::characters::action_scheme::derive_action_scheme;
    use ambition_platformer2d::entity_catalog::action_scheme::ControlSlot;

    // The body has WINGS, so the engine's own `fly_toggle` would claim Utility if the declared
    // technique did not outrank it.
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
