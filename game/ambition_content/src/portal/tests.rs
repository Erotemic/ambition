use bevy::prelude::*;

use ambition_characters::brain::ActionSet;
use ambition_platformer2d_core::BodyBaseSize;
use ambition_platformer2d_core::ControlFrame;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_core::RoomGeometry;
use ambition_platformer2d_core::{self as ae};
use ambition_platformer2d_shared_tangle::gravity::gravity_upright_angle;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
use ambition_platformer2d_shared_tangle::transit::rotate_velocity_between_normals as portal_transform_velocity;

#[allow(unused_imports)]
use super::*;
use ambition_platformer2d_core::cast::raycast_solids;
use ambition_platformer2d_core::frame::MapConvention;
use ambition_portal2d::*;

// Channel shorthands for the tests: the gun's pair (Blue/Orange) and two authored
// pairs (Purple/Yellow). These map the old `PortalColor::X` literals onto the new
// `PortalChannel` so the tests still pin byte-identical pairing/transit behavior.
const BLUE: PortalChannel = PortalChannel::Gun(PortalGunColor::BLUE);
const ORANGE: PortalChannel = PortalChannel::Gun(PortalGunColor::ORANGE);
const PURPLE: PortalChannel = PortalChannel::Authored(PortalChannelColor::Purple);
const YELLOW: PortalChannel = PortalChannel::Authored(PortalChannelColor::Yellow);

/// Step a scratch body one tick through the REAL unified kernel with an
/// axis-swept policy refreshed from `tuning` (the scratch's own model is
/// threaded, so maneuver state persists across ticks like a live entity).
fn step_axis_scratch(
    world: &ae::World,
    scratch: &mut ae::BodyClusterScratch,
    input: ae::InputState,
    dt: f32,
    tuning: ae::movement::MovementTuning,
) {
    let frame = ae::MotionFrame::from_direction(ae::DEFAULT_GRAVITY_DIR, tuning.gravity);
    let (model, mut clusters) = scratch.parts();
    ae::switch_motion_model(
        model,
        ae::MotionModelSpec::AxisSwept(tuning.axis_swept_params()),
    );
    ae::step_motion(
        model,
        &mut clusters,
        ae::MotionStepContext {
            admission: None,
            world,
            input,
            frame,
            facing_intent: input.axes.x,
            dt,
            contact: ambition_platformer2d_core::BodyContactField::NONE,
            pose_owned_externally: false,
            recovery_commitment_outstanding: false,
        },
    );
}

fn world_with_two_walls() -> RoomGeometry {
    // Left wall x[0,20], right wall x[380,400], both y[0,400].
    let blocks = vec![
        ae::Block::solid("left", Vec2::new(0.0, 0.0), Vec2::new(20.0, 400.0)),
        ae::Block::solid("right", Vec2::new(380.0, 0.0), Vec2::new(20.0, 400.0)),
    ];
    RoomGeometry(ae::World::new(
        "portal_test",
        Vec2::new(400.0, 400.0),
        Vec2::new(200.0, 360.0),
        blocks,
    ))
}

fn spawn_player(app: &mut App, pos: Vec2, facing: f32) -> Entity {
    app.world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            // The seat: the portal input adapter shapes the frame of whichever
            // seat drives the transiting body.
            ambition_characters::control::DrivingParticipant(
                ambition_characters::control::PlayerSlot::PRIMARY,
            ),
            BodyKinematics { material: Default::default(),
                pos,
                vel: Vec2::ZERO,
                canonical_size: Vec2::new(24.0, 40.0),
                facing,
            },
            BodyBaseSize {
                base_size: Vec2::new(24.0, 40.0),
            },
            PortalGenerator::default(),
            ActionSet::default(),
            // Every production body carries an intent frame, and
            // `resolve_portal_fire_intent` spends the Attack press on it when
            // the gun accepts a fire.
            ambition_characters::control::ActorControl::default(),
            // ⛔ AND AN IDENTITY AND ITS MINT STREAM. A portal shot mints under
            // the FIRER, and `resolve_portal_fire_intent` refuses a firer it
            // cannot name (ADR 0030). A production body has both from
            // `ensure_sim_id`; a fixture without them tested the road that no
            // longer exists.
            ambition_platformer2d_shared_tangle::sim_id::SimId::placement("test_portal_player"),
            ambition_platformer2d_shared_tangle::sim_id::SimIdCounter::default(),
        ))
        .id()
}

fn find_portal(app: &mut App, channel: PortalChannel) -> Option<PlacedPortal> {
    let mut q = app.world_mut().query::<&PlacedPortal>();
    let world = app.world();
    q.iter(world).find(|p| p.channel == channel).cloned()
}

fn set_control(app: &mut App, attack: bool, interact: bool) {
    let mut cf = app.world_mut().resource_mut::<ControlFrame>();
    cf.attack_pressed = attack;
    cf.interact_pressed = interact;
}

/// Emit a `FirePortalGenerator` intent for the primary player, resolving aim the same
/// way the input adapter does (facing-ahead, since these tests set no stick).
fn fire_portal(app: &mut App) {
    // The gesture names its body now, so the fixture has to name one too — the
    // primary player, which is who these tests are about.
    let (body, facing) = {
        let mut q = app
            .world_mut()
            .query_filtered::<(Entity, &BodyKinematics), (With<PlayerEntity>, With<PrimaryPlayer>)>(
            );
        q.iter(app.world())
            .next()
            .map_or((Entity::PLACEHOLDER, 1.0), |(entity, k)| (entity, k.facing))
    };
    app.world_mut().write_message(FirePortalGenerator {
        aim: Vec2::new(if facing >= 0.0 { 1.0 } else { -1.0 }, 0.0),
        body,
    });
}

#[test]
fn raycast_hits_nearest_solid_face_with_outward_normal() {
    let world = world_with_two_walls().0;
    // Fire left from mid-room: hit the left wall's right face at x=20,
    // normal pointing back toward the shooter (+x).
    let (hit, normal) = raycast_solids(
        &world,
        Vec2::new(200.0, 200.0),
        Vec2::new(-1.0, 0.0),
        6000.0,
        false,
    )
    .expect("ray should hit the left wall");
    assert!((hit.x - 20.0).abs() < 0.001, "hit x={}", hit.x);
    assert!(
        normal.x > 0.5 && normal.y.abs() < 0.001,
        "normal={normal:?}"
    );
}

#[test]
fn portals_adhere_to_one_way_platforms_but_blink_passes_through() {
    use ambition_platformer2d_core::world::{Block, World};
    let world = World {
        name: "one-way".to_string(),
        size: Vec2::new(400.0, 400.0),
        spawn: Vec2::new(200.0, 200.0),
        blocks: (vec![Block::one_way(
            "ledge",
            Vec2::new(100.0, 300.0),
            Vec2::new(200.0, 12.0),
        )]).into(),
        climbable_regions: Vec::new(),
        chains: (Vec::new()).into(),
        edges: Default::default(),
        water_regions: Vec::new(),
    };
    let from = Vec2::new(200.0, 100.0);
    let dir = Vec2::new(0.0, 1.0); // down toward the one-way's top (y=300)
                                   // A portal shot adheres to the one-way (#39).
    let portal_hit = raycast_solids(&world, from, dir, 6000.0, true);
    assert!(
        portal_hit.is_some_and(|(hit, n)| (hit.y - 300.0).abs() < 1.0 && n.y < -0.5),
        "a portal shot should adhere to a one-way's face (#39), got {portal_hit:?}"
    );
    // ...but blink / dive pass straight through one-ways.
    assert!(
        raycast_solids(&world, from, dir, 6000.0, false).is_none(),
        "blink/dive should pass through one-way platforms"
    );
}

#[test]
fn raycast_sees_through_a_portal_pair_and_recurses() {
    // Only block: a left wall at x[0,20]. A ray cast straight DOWN hits no
    // solid — unless it transits the floor portal and emerges from the wall
    // portal heading left into that wall.
    let world = ae::World::new(
        "portal-los",
        Vec2::new(400.0, 400.0),
        Vec2::new(200.0, 200.0),
        vec![ae::Block::solid(
            "left",
            Vec2::new(0.0, 0.0),
            Vec2::new(20.0, 400.0),
        )],
    );
    let portals = vec![
        PlacedPortal::fixed(
            BLUE,
            Vec2::new(200.0, 380.0),
            Vec2::new(0.0, -1.0),
            ambition_portal2d::PORTAL_OPENING_HALF,
        ).unwrap(),
        PlacedPortal::fixed(
            ORANGE,
            Vec2::new(380.0, 200.0),
            Vec2::new(-1.0, 0.0),
            ambition_portal2d::PORTAL_OPENING_HALF,
        ).unwrap(),
    ];
    // Without portals, casting down hits nothing.
    assert!(raycast_solids(
        &world,
        Vec2::new(200.0, 300.0),
        Vec2::new(0.0, 1.0),
        6000.0,
        false
    )
    .is_none());
    // Through the portal pair, the ray emerges from the wall portal heading
    // left and lands on the left wall's right face (x≈20, normal +x).
    let hit = raycast_through_portals(
        &world,
        &portals,
        Vec2::new(200.0, 300.0),
        Vec2::new(0.0, 1.0),
        6000.0,
        false,
        ae::cast::RayBudget { max_hops: 2, max_scale_exponent: 12, max_pieces: 32 },
        MapConvention::Reflection,
    ).unwrap();
    assert!(
        hit.is_some_and(|(p, n)| (p.x - 20.0).abs() < 1.0 && n.x > 0.5),
        "ray should recurse through the pair and hit the left wall, got {hit:?}"
    );
}

#[test]
fn ground_ground_keeps_horizontal_direction() {
    // Two floor portals (both normals up). Falling in moving RIGHT + down must
    // come out moving RIGHT (and up out of the exit floor) — the tangent
    // (horizontal) component is preserved, NOT mirrored to the left.
    let out = portal_transform_velocity(
        Vec2::new(120.0, 200.0),
        Vec2::new(0.0, -1.0),
        Vec2::new(0.0, -1.0),
        MapConvention::Reflection,
    );
    assert!(
        out.x > 0.0,
        "horizontal direction kept (right stays right), got {out:?}"
    );
    assert!(
        out.y < 0.0,
        "into-floor reverses to out-of-floor (down → up), got {out:?}"
    );
}

#[test]
fn velocity_transform_rotates_through_perpendicular_portals() {
    // Entry: a floor portal whose normal points up (in y-down world, up = -y).
    // Exit: a right-wall portal whose normal points left (-x).
    // The player falls in moving down (+y) and should emerge moving left
    // (out of the exit portal) at the same speed — a 90° turn.
    let out = portal_transform_velocity(
        Vec2::new(0.0, 100.0),
        Vec2::new(0.0, -1.0),
        Vec2::new(-1.0, 0.0),
        MapConvention::Reflection,
    );
    assert!(
        (out.x + 100.0).abs() < 0.01 && out.y.abs() < 0.01,
        "fall-in should exit left at the same speed, got {out:?}"
    );
}

fn ground_transit_app() -> App {
    let mut app = App::new();
    app.init_resource::<ambition_portal2d::PortalTuning>();
    app.insert_resource(ambition_time::WorldTime::new(1.0 / 60.0, 1.0 / 60.0));
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(),
        RoomGeometry(ae::World::new("item transit", Vec2::splat(800.0), Vec2::ZERO, vec![])));
    app.add_message::<ambition_portal2d::PortalBodyEntered>();
    app.add_message::<ambition_portal2d::PortalBodyTransited>();
    app.add_systems(Update, (ambition_held_items::ground_item_physics,
        ambition_held_items::project_item_body_presence,
        ambition_portal2d::portal_transit).chain());
    app
}

#[test]
fn in_flight_ground_item_travels_through_the_portal_pair() {
    use ambition_held_items::GroundItem;
    let mut app = ground_transit_app();
    // Blue portal facing right at x=20, orange facing left at x=380.
    app.world_mut().spawn(PlacedPortal::fixed(
        BLUE,
        Vec2::new(20.0, 200.0),
        Vec2::new(1.0, 0.0),
        ambition_portal2d::PORTAL_OPENING_HALF,
    ).unwrap());
    app.world_mut().spawn(PlacedPortal::fixed(
        ORANGE,
        Vec2::new(380.0, 200.0),
        Vec2::new(-1.0, 0.0),
        ambition_portal2d::PORTAL_OPENING_HALF,
    ).unwrap());
    // A thrown item flying into the blue portal.
    let item = app
        .world_mut()
        .spawn(GroundItem::released(
            ambition_held_items::axe_spec(),
            Vec2::new(20.0, 200.0),
            Vec2::new(-300.0, 0.0),
            Vec2::splat(12.0),
        ))
        .id();
    app.update();
    let g = app.world().get::<ae::BodyKinematics>(item).unwrap();
    assert!(
        g.pos.x > 250.0,
        "item should have come out of the orange (right) portal, pos={:?}",
        g.pos
    );
    assert!(
        (g.vel.length() - 300.0).abs() < 1.0,
        "momentum carries through the portal, vel={:?}",
        g.vel
    );
}

/// A pair that turns what it carries turns a thrown item too, as it turns a
/// body: its far-side image is drawn turned while it straddles, and the item
/// must come out as that image, not snap upright (measured in the seam lab:
/// a 45-degree snap on the spinning plank, G).
#[test]
fn a_thrown_item_comes_out_turned_as_the_pair_turns_it() {
    use ambition_held_items::GroundItem;
    let mut app = ground_transit_app();
    app.insert_resource(ambition_portal2d::PortalTuning {
        convention: ambition_portal2d::PortalConvention::Reflection, ..Default::default()
    });
    // A floor portal (facing up) and a wall portal (facing right): the pair
    // turns what falls in by a quarter.
    app.world_mut().spawn(PlacedPortal::fixed(
        BLUE,
        Vec2::new(100.0, 300.0),
        Vec2::new(0.0, -1.0),
        ambition_portal2d::PORTAL_OPENING_HALF,
    ).unwrap());
    app.world_mut().spawn(PlacedPortal::fixed(
        ORANGE,
        Vec2::new(400.0, 200.0),
        Vec2::new(1.0, 0.0),
        ambition_portal2d::PORTAL_OPENING_HALF,
    ).unwrap());
    let item = app
        .world_mut()
        .spawn(GroundItem::released(
            ambition_held_items::axe_spec(),
            Vec2::new(100.0, 300.0),
            Vec2::new(0.0, 300.0),
            Vec2::splat(12.0),
        ))
        .id();
    app.update();
    let g = app.world().get::<ae::BodyKinematics>(item).unwrap();
    assert!(g.pos.x > 390.0, "the item came out of the wall portal, pos={:?}", g.pos);
    let roll = g.material.render_angle();
    assert!(g.material.reflected());
    assert!(roll.cos().abs() < 1e-3, "turned a quarter, as the pair turns it: roll={roll}");
}

#[test]
fn portal_fit_gate_keys_on_the_opening_perpendicular_to_the_normal() {
    let wall = PlacedPortal::fixed(
        BLUE,
        Vec2::ZERO,
        Vec2::new(1.0, 0.0),
        ambition_portal2d::PORTAL_OPENING_HALF,
    ).unwrap();
    // The opening is the SAME size in every orientation (2*46=92). A wall
    // portal gates on HEIGHT: a short actor fits, a 200-tall boss does not.
    assert!(portal_fits(Vec2::new(24.0, 40.0), &wall));
    assert!(!portal_fits(Vec2::new(80.0, 200.0), &wall));
    // A floor portal gates on WIDTH — same 92 opening, so the threshold
    // matches the wall's.
    let floor = PlacedPortal::fixed(
        ORANGE,
        Vec2::ZERO,
        Vec2::new(0.0, -1.0),
        ambition_portal2d::PORTAL_OPENING_HALF,
    ).unwrap();
    assert!(portal_fits(Vec2::new(40.0, 200.0), &floor));
    assert!(!portal_fits(Vec2::new(100.0, 20.0), &floor));
}

#[test]
fn portals_teleport_a_fitting_actor_and_skip_an_oversized_one() {
    use ambition_platformer2d_core::body_clusters::BodyKinematics;
    let mut app = App::new();
    app.add_message::<ambition_portal2d::PortalBodyEntered>();
    app.add_message::<ambition_portal2d::PortalBodyTransited>();
    app.init_resource::<ambition_portal2d::PortalTuning>();
    app.add_systems(Update, portal_transit);
    app.world_mut().spawn(PlacedPortal::fixed(
        BLUE,
        Vec2::new(20.0, 200.0),
        Vec2::new(1.0, 0.0),
        ambition_portal2d::PORTAL_OPENING_HALF,
    ).unwrap());
    app.world_mut().spawn(PlacedPortal::fixed(
        ORANGE,
        Vec2::new(380.0, 200.0),
        Vec2::new(-1.0, 0.0),
        ambition_portal2d::PORTAL_OPENING_HALF,
    ).unwrap());
    let small = app
        .world_mut()
        .spawn((
            BodyKinematics { material: Default::default(),
                pos: Vec2::new(20.0, 200.0),
                vel: Vec2::new(-100.0, 0.0),
                canonical_size: Vec2::new(24.0, 40.0),
                facing: -1.0,
            },
        ))
        .id();
    let big = app
        .world_mut()
        .spawn((
            BodyKinematics { material: Default::default(),
                pos: Vec2::new(20.0, 200.0),
                vel: Vec2::new(-100.0, 0.0),
                canonical_size: Vec2::new(80.0, 200.0),
                facing: -1.0,
            },
        ))
        .id();
    for entity in [small, big] {
        let kin = *app.world().get::<BodyKinematics>(entity).unwrap();
        app.world_mut().entity_mut(entity).insert(ae::SweepSample {
            prev: kin.pos + Vec2::X * 4.0, vel: kin.vel, ..ae::SweepSample::at_rest(kin, Vec2::Y)
        });
    }
    // Aperture transit: frame 1 begins (leading edge in the opening), frame 2
    // transfers (centroid already on the plane).
    app.update();
    app.update();
    let s = app.world().get::<BodyKinematics>(small).unwrap();
    assert!(
        s.pos.x > 250.0,
        "a fitting actor transits out the orange portal, pos={:?}",
        s.pos
    );
    let b = app.world().get::<BodyKinematics>(big).unwrap();
    assert!(
        b.pos.x < 100.0,
        "an oversized actor does not fit and stays put, pos={:?}",
        b.pos
    );
}

#[test]
fn n_pairs_transit_routes_to_the_matching_partner() {
    let he = ambition_portal2d::PORTAL_OPENING_HALF;
    let floor = |channel, x: f32| {
        PlacedPortal::fixed(channel, Vec2::new(x, 300.0), Vec2::new(0.0, -1.0), he).unwrap()
    };
    // Two INDEPENDENT floor pairs placed at once.
    let portals = vec![
        floor(BLUE, 100.0),
        floor(ORANGE, 200.0),
        floor(PURPLE, 400.0),
        floor(YELLOW, 700.0),
    ];
    // A body whose centroid has crossed the PURPLE plane transfers to YELLOW
    // (its partner) — never to the unrelated orange portal.
    let step = ambition_portal2d::transit_step_with_tuning(ambition_platformer2d_core::BodyKinematics { pos: Vec2::new(400.0, 305.0), canonical_size: Vec2::new(24.0, 40.0), vel: Vec2::new(0.0, 50.0), ..Default::default() },
        Some(ae::SweepSample { turn: None, down: Vec2::ZERO, travel: Default::default(), prev: Vec2::new(400.0, 295.0), vel: Vec2::Y * 50.0, time: [0.0, 1.0], curr: Vec2::new(400.0, 305.0), material: Default::default(), half: Vec2::ZERO }),
        Some(PortalTransit {
            straddling: PURPLE,
            crossed: false,
        }),
        None,
        &portals,
        &ambition_portal2d::PortalTuning::default(),);
    match step {
        TransitStep::Transfer { transfer, .. } => {
                let exit_channel = transfer.exit_channel;
                let pos = transfer.exit_pos;
            assert_eq!(exit_channel, YELLOW, "purple links to yellow");
            assert!(pos.x > 600.0, "emerges at the yellow portal, got {pos:?}");
        }
        other => panic!("expected a transfer to yellow, got {other:?}"),
    }
}





#[test]
fn held_input_warp_gate_is_convention_aware() {
    let up = Vec2::new(0.0, -1.0); // floor
    let left = Vec2::new(-1.0, 0.0); // right-wall normal

    // Reflection: same-wall flips horizontal movement; floor↔floor preserves it.
    assert!(portal_input_warp_flips_horizontal_for_convention(
        MapConvention::Reflection,
        left,
        left
    ));
    assert!(!portal_input_warp_flips_horizontal_for_convention(
        MapConvention::Reflection,
        up,
        up
    ));

    // Rotation: both same-wall and floor↔floor are proper 180-degree rotations,
    // so held horizontal movement must flip to keep helping the transformed
    // velocity instead of fighting it.
    assert!(portal_input_warp_flips_horizontal_for_convention(
        MapConvention::Rotation,
        left,
        left
    ));
    assert!(portal_input_warp_flips_horizontal_for_convention(
        MapConvention::Rotation,
        up,
        up
    ));

    // A 90-degree floor→wall turn maps horizontal input to vertical; the
    // platformer movement controller cannot express that as ordinary movement.
    assert!(!portal_input_warp_flips_horizontal_for_convention(
        MapConvention::Rotation,
        up,
        left
    ));
    assert!(!portal_input_warp_flips_horizontal_for_convention(
        MapConvention::Reflection,
        up,
        left
    ));
}





#[test]
fn gravity_upright_angle_tracks_the_gravity_direction() {
    use std::f32::consts::FRAC_PI_2;
    // Default gravity (down, +Y world) → upright is 0.
    assert!(gravity_upright_angle(Vec2::new(0.0, 1.0)).abs() < 1e-5);
    // Gravity to the right (+X) → the body stands rotated +90° (render).
    assert!((gravity_upright_angle(Vec2::new(1.0, 0.0)) - FRAC_PI_2).abs() < 1e-5);
}

#[test]
fn actors_carry_their_physical_material_through_portals() {
    use ambition_platformer2d_core::body_clusters::BodyKinematics;
    let mut app = App::new();
    app.add_message::<ambition_portal2d::PortalBodyEntered>();
    app.add_message::<ambition_portal2d::PortalBodyTransited>();
    app.init_resource::<ambition_portal2d::PortalTuning>();
    app.insert_resource(ambition_portal2d::PortalTuning {
        convention: ambition_portal2d::PortalConvention::Reflection, ..Default::default()
    });
    app.add_systems(Update, portal_transit);
    // The pair reflects the physical material across a floor-to-wall crossing.
    app.world_mut().spawn(PlacedPortal::fixed(
        BLUE,
        Vec2::new(200.0, 380.0),
        Vec2::new(0.0, -1.0),
        ambition_portal2d::PORTAL_OPENING_HALF,
    ).unwrap());
    app.world_mut().spawn(PlacedPortal::fixed(
        ORANGE,
        Vec2::new(380.0, 200.0),
        Vec2::new(-1.0, 0.0),
        ambition_portal2d::PORTAL_OPENING_HALF,
    ).unwrap());
    let actor = app
        .world_mut()
        .spawn((
            BodyKinematics { material: Default::default(),
                pos: Vec2::new(200.0, 380.0),
                vel: Vec2::new(0.0, 100.0),
                canonical_size: Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
        ))
        .id();
    let kin = *app.world().get::<BodyKinematics>(actor).unwrap();
    app.world_mut().entity_mut(actor).insert(ae::SweepSample {
        prev: kin.pos - Vec2::Y * 4.0, vel: kin.vel, ..ae::SweepSample::at_rest(kin, Vec2::Y)
    });
    // The second update transfers the root and its physical material.
    app.update();
    app.update();
    let material = app.world().get::<BodyKinematics>(actor).unwrap().material;
    assert!(material.reflected());
    let map = material.at(Vec2::ZERO).unwrap();
    assert_eq!(map.vector(Vec2::X).unwrap(), -Vec2::Y);
    assert_eq!(map.vector(Vec2::Y).unwrap(), -Vec2::X);

}

#[test]
fn portal_pair_teleports_player_carrying_momentum() {
    let mut app = App::new();
    app.add_message::<ambition_portal2d::PortalBodyEntered>();
    app.add_message::<BodyTeleported>();
    app.add_message::<ambition_portal2d::PortalBodyTransited>();
    app.insert_resource(ambition_time::WorldTime::default());
    app.init_resource::<ambition_portal2d::PortalTuning>();
    app.insert_resource(ambition_portal2d::PortalTuning {
        convention: ambition_portal2d::PortalConvention::Reflection, ..Default::default()
    });
    app.add_systems(Update, portal_transit);
    // Blue on the left (facing right), orange on the right (facing left).
    app.world_mut().spawn(PlacedPortal::fixed(
        BLUE,
        Vec2::new(22.0, 200.0),
        Vec2::new(1.0, 0.0),
        ambition_portal2d::PORTAL_OPENING_HALF,
    ).unwrap());
    app.world_mut().spawn(PlacedPortal::fixed(
        ORANGE,
        Vec2::new(378.0, 200.0),
        Vec2::new(-1.0, 0.0),
        ambition_portal2d::PORTAL_OPENING_HALF,
    ).unwrap());
    let player = spawn_player(&mut app, Vec2::new(22.0, 200.0), 1.0);
    app.world_mut()
        .get_mut::<BodyKinematics>(player)
        .unwrap()
        .vel = Vec2::new(-100.0, 0.0);
    // An existing tilt must compose with the complete portal map.
    app.world_mut().get_mut::<BodyKinematics>(player).unwrap().material =
        ae::frame::BodyMaterial::IDENTITY.turned(-0.5).unwrap();
    let kin = *app.world().get::<BodyKinematics>(player).unwrap();
    app.world_mut().entity_mut(player).insert(ae::SweepSample {
        prev: kin.pos + Vec2::X * 4.0, vel: kin.vel, ..ae::SweepSample::at_rest(kin, Vec2::Y)
    });
    let expected_material = kin.material.carried(ae::frame::Sim2::through(
        &ae::frame::PortalAperture { frame: ae::frame::PortalFrame::fixed(Vec2::new(22.0, 200.0), Vec2::X), half_length: ambition_portal2d::PORTAL_OPENING_HALF },
        &ae::frame::PortalAperture { frame: ae::frame::PortalFrame::fixed(Vec2::new(378.0, 200.0), -Vec2::X), half_length: ambition_portal2d::PORTAL_OPENING_HALF },
        MapConvention::Reflection).unwrap()).unwrap();
    // Frame 1 begins transit (leading edge in the aperture); frame 2 sees the
    // centroid already across the plane and transfers the authoritative body.
    app.update();
    app.update();
    let kin = *app.world().get::<BodyKinematics>(player).unwrap();
    assert!(
        kin.pos.x > 250.0,
        "player should have teleported to the orange (right) side, got {:?}",
        kin.pos
    );
    assert!(
        kin.vel.length() >= MIN_EXIT_SPEED - 1.0,
        "exit should carry momentum (>= min exit speed), got {:?}",
        kin.vel
    );
    assert_eq!(kin.material, expected_material);
    assert_eq!(kin.facing, 1.0);

}

#[test]
fn a_gunless_player_transits_an_authored_pair() {
    // The portal_lab scenario: pre-placed portals, player has NOT picked up
    // the gun. Transit must still work — crossing a placed pair is independent
    // of holding the gun, and the cooldown lives on the body.
    let mut app = App::new();
    app.add_message::<ambition_portal2d::PortalBodyEntered>();
    app.add_message::<BodyTeleported>();
    app.add_message::<ambition_portal2d::PortalBodyTransited>();
    app.insert_resource(ambition_time::WorldTime::default());
    app.init_resource::<ambition_portal2d::PortalTuning>();
    app.add_systems(Update, portal_transit);
    let he = ambition_portal2d::PORTAL_OPENING_HALF;
    app.world_mut().spawn(PlacedPortal::fixed(
        PURPLE,
        Vec2::new(200.0, 300.0),
        Vec2::new(0.0, -1.0),
        he,
    ).unwrap());
    app.world_mut().spawn(PlacedPortal::fixed(
        YELLOW,
        Vec2::new(600.0, 300.0),
        Vec2::new(0.0, -1.0),
        he,
    ).unwrap());
    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyKinematics { material: Default::default(),
                pos: Vec2::new(200.0, 285.0),
                vel: Vec2::ZERO,
                canonical_size: Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            BodyBaseSize {
                base_size: Vec2::new(24.0, 40.0),
            },
            // No PortalGenerator on purpose.
        ))
        .id();
    // Frame 1 begins transit (standing on the purple floor portal).
    app.update();
    assert!(
        app.world().get::<PortalTransit>(player).is_some(),
        "a gun-less player standing on an authored portal begins transit"
    );
    // Sink the centroid past the plane → transfer to the yellow partner.
    app.world_mut()
        .get_mut::<BodyKinematics>(player)
        .unwrap()
        .pos
        .y = 305.0;
    let kin = *app.world().get::<BodyKinematics>(player).unwrap();
    app.world_mut().entity_mut(player).insert(ae::SweepSample {
        prev: Vec2::new(200.0, 285.0), vel: kin.vel, ..ae::SweepSample::at_rest(kin, Vec2::Y)
    });
    app.update();
    let pos = app.world().get::<BodyKinematics>(player).unwrap().pos;
    assert!(
        pos.x > 550.0,
        "transfers to the yellow portal without a gun, got {pos:?}"
    );
}

#[test]
fn transit_is_gradual_centroid_crossing_flags_the_teleport_then_clears() {
    let mut app = App::new();
    app.add_message::<ambition_portal2d::PortalBodyEntered>();
    app.add_message::<ambition_portal2d::PortalBodyTransited>();
    app.insert_resource(ambition_time::WorldTime::default());
    app.init_resource::<ambition_portal2d::PortalTuning>();
    // The trace and trail notices of the transfer are the runtime's
    // (`portal_seat::tests`); this measures the core's gradual crossing.
    app.add_systems(Update, portal_transit);
    // Two FLOOR portals (normal up): blue at x=200, orange at x=600.
    app.world_mut().spawn(PlacedPortal::fixed(
        BLUE,
        Vec2::new(200.0, 300.0),
        Vec2::new(0.0, -1.0),
        ambition_portal2d::PORTAL_OPENING_HALF,
    ).unwrap());
    app.world_mut().spawn(PlacedPortal::fixed(
        ORANGE,
        Vec2::new(600.0, 300.0),
        Vec2::new(0.0, -1.0),
        ambition_portal2d::PORTAL_OPENING_HALF,
    ).unwrap());
    // Player straddling the blue floor: feet (max.y=305) below the plane,
    // centroid (285) still above it.
    let player = spawn_player(&mut app, Vec2::new(200.0, 285.0), 1.0);

    // Frame 1: leading edge in the aperture → transit BEGINS, no transfer.
    app.update();
    assert!(
        app.world()
            .get::<PortalTransit>(player)
            .is_some_and(|t| !t.crossed),
        "transit begins without an instant teleport"
    );
    assert!(
        app.world().get::<BodyKinematics>(player).unwrap().pos.x < 250.0,
        "still entry-side"
    );

    // Push the centroid across the plane (as the integrator would as the body
    // sinks into the carved opening).
    app.world_mut()
        .get_mut::<BodyKinematics>(player)
        .unwrap()
        .pos
        .y = 305.0;
    let kin = *app.world().get::<BodyKinematics>(player).unwrap();
    app.world_mut().entity_mut(player).insert(ae::SweepSample {
        prev: Vec2::new(200.0, 285.0), vel: kin.vel, ..ae::SweepSample::at_rest(kin, Vec2::Y)
    });
    app.update();
    assert!(
        app.world()
            .get::<PortalTransit>(player)
            .is_some_and(|t| t.crossed),
        "centroid crossing transfers the authoritative body"
    );
    let pos = app.world().get::<BodyKinematics>(player).unwrap().pos;
    assert!(
        pos.x > 550.0,
        "authoritative body is now exit-side, got {pos:?}"
    );

    // Move clear of the exit plane → transit ends (re-armed via cooldown).
    app.world_mut()
        .get_mut::<BodyKinematics>(player)
        .unwrap()
        .pos
        .y = 270.0;
    app.update();
    assert!(
        app.world().get::<PortalTransit>(player).is_none(),
        "transit clears once the body fully clears the plane"
    );
}

#[test]
fn portal_carve_is_transient_and_pair_gated() {
    let mut app = App::new();
    // Carve output is now the portal-owned `PortalCarves` resource (Phase 2
    // Seam 1); the Ambition bridge copies it into the host overlay. Portal core
    // (and this core test) reads the portal-owned resource directly.
    app.init_resource::<ambition_portal2d::PortalCarves>();
    app.add_systems(Update, publish_portal_carves);
    // A lone portal must NOT carve (no exit → no bottomless hole).
    let blue = app
        .world_mut()
        .spawn(PlacedPortal::fixed(
            BLUE,
            Vec2::new(200.0, 300.0),
            Vec2::new(0.0, -1.0),
            ambition_portal2d::PORTAL_OPENING_HALF,
        ).unwrap())
        .id();
    app.update();
    assert!(
        app.world()
            .resource::<ambition_portal2d::PortalCarves>()
            .holes
            .is_empty(),
        "a lone portal does not carve"
    );
    // Complete the pair — but with NO body transiting, still nothing carves
    // (so you can't wiggle into a wall pocket between crossings).
    app.world_mut().spawn(PlacedPortal::fixed(
        ORANGE,
        Vec2::new(600.0, 300.0),
        Vec2::new(0.0, -1.0),
        ambition_portal2d::PORTAL_OPENING_HALF,
    ).unwrap());
    app.update();
    assert!(
        app.world()
            .resource::<ambition_portal2d::PortalCarves>()
            .holes
            .is_empty(),
        "a placed pair with no body transiting stays solid (no walk-in pocket)"
    );
    // A body transiting the blue portal carves that pair: it straddles the
    // pair, so it is at both of its ends (a shot the pair takes this tick
    // and that moves later in the tick must not meet the exit's wall).
    let _ = blue;
    app.world_mut().spawn(PortalTransit {
        straddling: BLUE,
        crossed: false,
    });
    app.update();
    assert_eq!(
        app.world()
            .resource::<ambition_portal2d::PortalCarves>()
            .holes
            .len(),
        2,
        "the pair a body is passing through is carved, both its ends"
    );
}

/// A booted app with just the shot adapter over two walls, plus the messages it
/// and the fire system write.
fn app_with_the_shot_adapter() -> App {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<ambition_portal2d::PortalShotFired>();
    app.add_message::<ambition_portal2d::PortalFireIntent>();
    ambition_platformer2d::platformer::lifecycle::insert_live_room_component(
        app.world_mut(),
        world_with_two_walls(),
    );
    app.insert_resource(ambition_time::WorldTime::new(1.0 / 60.0, 1.0 / 60.0));
    app.add_systems(
        Update,
        (portal_fire_system, crate::portal::portal_projectile_step).chain(),
    );
    app
}

fn blue_portals(app: &mut App) -> Vec<PlacedPortal> {
    let mut q = app.world_mut().query::<&PlacedPortal>();
    let world = app.world();
    q.iter(world)
        .filter(|p| p.channel == BLUE)
        .cloned()
        .collect()
}

/// OW1: a placement replaces the portal of its channel in its OWN live room
/// only. #0 holds a blue; a blue shot in #1 lands on #1's wall. Both blues
/// stand after the tick: #0's is the same entity, and the new one is in #1.
/// Before, the shot closed the blue of every room, so a player in one room
/// shut the door of a player in the other.
#[test]
fn a_shot_replaces_the_portal_of_its_channel_in_its_own_room_only() {
    use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<ambition_portal2d::PortalShotFired>();
    app.add_message::<ambition_portal2d::PortalFireIntent>();
    app.insert_resource(ambition_time::WorldTime::new(1.0 / 60.0, 1.0 / 60.0));
    app.add_systems(
        Update,
        (portal_fire_system, crate::portal::portal_projectile_step).chain(),
    );
    let live = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next()];
    for room in live {
        app.world_mut().spawn((RoomInstanceRoot, room, world_with_two_walls()));
    }
    let kept = app
        .world_mut()
        .spawn((
            PlacedPortal::fixed(BLUE, Vec2::new(382.0, 200.0), Vec2::new(-1.0, 0.0), ambition_portal2d::PORTAL_OPENING_HALF).unwrap(),
            InRoomInstance(live[0]),
        ))
        .id();
    // 25px from the left wall: the shot lands on this tick.
    app.world_mut().write_message(ambition_portal2d::PortalFireIntent {
        origin: Vec2::new(45.0, 200.0),
        dir: Vec2::new(-1.0, 0.0),
        channel: BLUE,
        id: None,
        room: Some(live[1]),
    });
    app.update();
    let mut blues: Vec<(Entity, Option<LiveRoomInstance>)> = app
        .world_mut()
        .query::<(Entity, &PlacedPortal, Option<&InRoomInstance>)>()
        .iter(app.world())
        .filter(|(_, portal, _)| portal.channel == BLUE)
        .map(|(entity, _, room)| (entity, room.map(|room| room.0)))
        .collect();
    blues.sort_by_key(|(_, room)| *room);
    assert_eq!(blues.len(), 2, "(entity, room) of each blue: the shot in #1 did not leave #0's blue: {blues:?}");
    assert_eq!(blues[0], (kept, Some(live[0])), "#0's blue is not the same portal");
    assert_eq!(blues[1].1, Some(live[1]), "the new blue is not in #1");
}

/// ⛔⛔ TWO BLUE SHOTS LANDING ON ONE TICK LEFT TWO BLUE PORTALS. The adapter
/// applied each shot against the same PRE-SYSTEM `portals` query while its
/// despawn/spawn sat in a deferred command buffer, so neither could see what the
/// other had queued: both despawned the old blue and both spawned a new one.
///
/// ⚠ AND IT WAS UNREACHABLE UNTIL THE UPSTREAM FIX. While `portal_fire_system`
/// kept only `read().last()`, one tick could never produce two shots — so
/// widening the producer is what made this consumer's assumption live. Both
/// origins are 25px from the left wall, inside one 31.7px step, so both resolve
/// on the same tick.
#[test]
fn two_same_channel_shots_landing_on_one_tick_leave_exactly_one_portal() {
    let mut app = app_with_the_shot_adapter();
    for y in [100.0, 200.0] {
        app.world_mut()
            .write_message(ambition_portal2d::PortalFireIntent {
                origin: Vec2::new(45.0, y),
                dir: Vec2::new(-1.0, 0.0),
                channel: BLUE,
                // A direct intent with no firing body has no identity to derive
                // from; this test is about where the portal lands.
                id: None,
                room: None,
            });
    }

    app.update();

    let blue = blue_portals(&mut app);
    assert_eq!(
        blue.len(),
        1,
        "two blue shots landed on one tick and left {} blue portals — \
         `find_portal` is a `.find()`, so the extra one is unreachable geometry \
         that still carves the world: {blue:?}",
        blue.len()
    );
    assert_eq!(
        {
            let mut q = app.world_mut().query::<&PortalShot>();
            q.iter(app.world()).count()
        },
        0,
        "both shots are spent, whether or not their placement won the channel"
    );
}

/// ⭐ AND THE SURVIVOR IS THE NEWEST SHOT, which is what "opens **or replaces**"
/// already meant. Among shots resolving on one tick there is no later, so the
/// rule reads the shot's own age: `traveled` is distance at a fixed speed, so the
/// SMALLEST traveled was fired most recently.
///
/// ⛔ REVERSING THE SPAWN ORDER MUST NOT MOVE THE PORTAL. Two shots in one
/// archetype are visited in spawn order, so a rule that read "the last one the
/// query yielded" would flip here and be green in only one of the two arms.
#[test]
fn the_newest_of_two_same_tick_shots_is_the_one_that_places() {
    fn surviving_portal_y(order: [f32; 2]) -> f32 {
        let mut app = app_with_the_shot_adapter();
        // Same wall, different heights, so the two placements are distinguishable.
        // The OLDER shot (traveled 900) is deliberately spawned first in one arm
        // and second in the other.
        for traveled in order {
            let y = if traveled > 0.0 { 300.0 } else { 100.0 };
            app.world_mut().spawn(PortalShot {
                channel: BLUE,
                pos: Vec2::new(45.0, y),
                vel: Vec2::new(-1900.0, 0.0),
                traveled,
            });
        }
        app.update();
        let blue = blue_portals(&mut app);
        assert_eq!(blue.len(), 1, "exactly one blue portal, got {blue:?}");
        blue[0].pos.y
    }

    let older_first = surviving_portal_y([900.0, 0.0]);
    let newer_first = surviving_portal_y([0.0, 900.0]);
    assert_eq!(
        older_first, newer_first,
        "which of two same-tick placements survived changed with the order the \
         shots were spawned in ({older_first} vs {newer_first}), so a resimulated \
         tick opens the portal somewhere else"
    );
    assert_eq!(
        older_first, 100.0,
        "the NEWEST shot places: the one that has traveled least is the one fired \
         most recently, and a later placement replaces an earlier one"
    );
}

#[test]
fn portal_shot_travels_and_opens_a_portal_on_a_wall() {
    let mut app = App::new();
    // `portal_projectile_step` (the Ambition shot adapter) still writes sfx;
    // `portal_fire_system` now emits the portal-owned `PortalShotFired` signal
    // (the FIRE/TRAVEL sfx moved to the `play_portal_sfx` adapter, Phase 5a).
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<ambition_portal2d::PortalShotFired>();
    ambition_platformer2d::platformer::lifecycle::insert_live_room_component(
        app.world_mut(),
        world_with_two_walls(),
    );
    app.insert_resource(ambition_time::WorldTime::new(1.0 / 60.0, 1.0 / 60.0));
    app.insert_resource(ControlFrame::default());
    app.add_message::<FirePortalGenerator>();
    // The `FirePortalGenerator` gesture is resolved into the generic `PortalFireIntent`
    // by the Ambition resolver (Phase 2 Seam 3) before the core fire system reads
    // it; the RoomGeometry-reading shot stepper is the Ambition world-seam adapter
    // (Phase 2 Seam 2). Portal core keeps the pure `step_portal_shot` helper.
    app.add_message::<ambition_portal2d::PortalFireIntent>();
    app.add_systems(
        Update,
        (
            crate::portal::resolve_portal_fire_intent,
            portal_fire_system,
            crate::portal::portal_projectile_step,
        )
            .chain(),
    );
    // Player mid-room facing left.
    spawn_player(&mut app, Vec2::new(200.0, 200.0), -1.0);

    // One fire intent fires a single shot.
    fire_portal(&mut app);
    app.update();
    assert_eq!(
        {
            let mut q = app.world_mut().query::<&PortalShot>();
            q.iter(app.world()).count()
        },
        1,
        "firing spawns a traveling portal shot"
    );
    // No portal yet — it has to travel there.
    assert!(find_portal(&mut app, BLUE).is_none());

    // Let the shot fly into the left wall.
    set_control(&mut app, false, false);
    for _ in 0..40 {
        app.update();
    }
    let blue = find_portal(&mut app, BLUE);
    assert!(
        blue.as_ref()
            .is_some_and(|p| p.pos.x < 60.0 && p.normal.x > 0.5),
        "the shot should open a blue portal on the left wall, got {blue:?}"
    );
    // The opened portal is room-scoped, so a room transition despawns it —
    // no lingering portals that reappear when you leave and come back (#41).
    let scoped = {
        let mut q = app.world_mut().query_filtered::<(), (
            With<PlacedPortal>,
            With<ambition_platformer2d_shared_tangle::lifecycle::RoomScopedEntity>,
        )>();
        q.iter(app.world()).count()
    };
    assert!(
        scoped >= 1,
        "an opened portal must be RoomScopedEntity (#41)"
    );
    assert_eq!(
        {
            let mut q = app.world_mut().query::<&PortalShot>();
            q.iter(app.world()).count()
        },
        0,
        "the shot is consumed when it lands"
    );
}

/// Energy audit for the free-fall bounce between two same-plane floor portals
/// (the c138/c139 "pop back and forth forever" loop): the REAL player
/// integrator + the REAL `transit_step` machine, no input. Gravity is
/// conservative and the portal map is an isometry, so the crossing speed must
/// NOT decay across many transfers — any drift here is integrator/transfer
/// instability, not physics.
#[test]
fn floor_floor_bounce_conserves_crossing_speed_over_many_transfers() {
    use ambition_platformer2d_core::body_clusters::BodyClusterScratch;
    use ambition_platformer2d_core::movement::{InputState, DEFAULT_TUNING};
    use ambition_portal2d::{transit_step_with_tuning, TransitStep};

    // A floor at y ∈ [880, 920] with the two apertures ALREADY carved (three
    // segments): this isolates the integrator + transfer math from carve
    // timing. Portals A (254) and B (554) on the floor top, both facing up.
    let floor_y = 880.0;
    let world = ae::World::new(
        "bounce audit",
        Vec2::new(1600.0, 1200.0),
        Vec2::new(254.0, 700.0),
        vec![
            ae::Block::solid("left", Vec2::new(0.0, floor_y), Vec2::new(208.0, 40.0)),
            ae::Block::solid("mid", Vec2::new(300.0, floor_y), Vec2::new(208.0, 40.0)),
            ae::Block::solid("right", Vec2::new(600.0, floor_y), Vec2::new(1000.0, 40.0)),
        ],
    );
    let up = Vec2::new(0.0, -1.0);
    let portals = [
        PlacedPortal::fixed(
            PURPLE,
            Vec2::new(254.0, floor_y),
            up,
            ambition_portal2d::PORTAL_OPENING_HALF,
        ).unwrap(),
        PlacedPortal::fixed(
            YELLOW,
            Vec2::new(554.0, floor_y),
            up,
            ambition_portal2d::PORTAL_OPENING_HALF,
        ).unwrap(),
    ];

    let mut scratch = BodyClusterScratch::new_with_abilities(
        Vec2::new(254.0, 700.0),
        ambition_platformer2d_core::AbilitySet::default(),
    );
    scratch.ground.on_ground = false;
    scratch.kinematics.vel = Vec2::new(0.0, 300.0); // falling straight down
    let size = Vec2::new(24.0, 40.0);
    let dt = 1.0 / 60.0;

    let mut transit: Option<PortalTransit> = None;
    let mut cooldown: Option<(PortalChannel, f32)> = None;
    let mut crossing_speeds: Vec<f32> = Vec::new();

    for _ in 0..4000 {
        if crossing_speeds.len() >= 40 {
            break;
        }
        let previous = scratch.kinematics;
        step_axis_scratch(
            &world,
            &mut scratch,
            InputState::default(),
            dt,
            DEFAULT_TUNING,
        );
        if let Some((_, t)) = cooldown.as_mut() {
            *t -= dt;
        }
        cooldown = cooldown.filter(|(_, t)| *t > 0.0);
        let step = transit_step_with_tuning(ambition_platformer2d_core::BodyKinematics { pos: scratch.kinematics.pos, canonical_size: size, vel: scratch.kinematics.vel, ..Default::default() },
            Some(ae::SweepSample { turn: None, down: Vec2::ZERO, travel: Default::default(), prev: previous.pos, vel: previous.vel, time: [0.0, 1.0], curr: scratch.kinematics.pos, material: Default::default(), half: Vec2::ZERO }),
            transit,
            cooldown.map(|(c, _)| c),
            &portals,
            &ambition_portal2d::PortalTuning::default(),);
        match step {
            TransitStep::Begin { channel, .. } => {
                transit = Some(PortalTransit {
                    straddling: channel,
                    crossed: false,
                });
            }
            TransitStep::Transfer { transfer, .. } => {
                let pos = transfer.exit_pos;
                let vel = transfer.exit_vel;
                let exit_channel = transfer.exit_channel;
                crossing_speeds.push(scratch.kinematics.vel.length());
                scratch.kinematics.pos = pos;
                scratch.kinematics.vel = vel;
                transit = Some(PortalTransit {
                    straddling: exit_channel,
                    crossed: true,
                });
                cooldown = Some((exit_channel, 0.25));
            }
            TransitStep::Clear => transit = None,
            TransitStep::Idle | TransitStep::Continue => {}
            TransitStep::Refused { reason, .. } => panic!("unexpected transit refusal: {reason:?}"),
        }
        assert!(
            scratch.kinematics.vel.x.abs() < 1e-3,
            "a vertical bounce stays vertical (no lateral leak), vx={}",
            scratch.kinematics.vel.x
        );
        assert!(
            !scratch.ground.on_ground,
            "the body must never ground mid-bounce (carve open), pos={:?}",
            scratch.kinematics.pos
        );
    }

    assert!(
        crossing_speeds.len() >= 40,
        "the bounce should keep transferring, got {} crossings",
        crossing_speeds.len()
    );
    let early: f32 = crossing_speeds[..5].iter().sum::<f32>() / 5.0;
    let late: f32 = crossing_speeds[35..40].iter().sum::<f32>() / 5.0;
    assert!(
        (late - early).abs() <= early * 0.02,
        "crossing speed must not drift over 40 transfers: early={early:.2}, late={late:.2}, all={crossing_speeds:?}"
    );
}

/// Drive the REAL integrator + transit machine through ONE floor→floor
/// round trip from `drop` px above portal A and return the rebound apex
/// height above the exit portal. Shared harness for the energy round-trip
/// pins below.
fn floor_floor_round_trip_apex(drop: f32, tuning: ae::movement::MovementTuning) -> f32 {
    use ambition_platformer2d_core::body_clusters::BodyClusterScratch;
    use ambition_platformer2d_core::movement::InputState;
    use ambition_portal2d::{transit_step_with_tuning, TransitStep};

    let floor_y = 3000.0;
    let world = ae::World::new(
        "round trip audit",
        Vec2::new(1600.0, 3400.0),
        Vec2::new(254.0, floor_y - drop),
        vec![
            ae::Block::solid("left", Vec2::new(0.0, floor_y), Vec2::new(208.0, 40.0)),
            ae::Block::solid("mid", Vec2::new(300.0, floor_y), Vec2::new(208.0, 40.0)),
            ae::Block::solid("right", Vec2::new(600.0, floor_y), Vec2::new(1000.0, 40.0)),
        ],
    );
    let up = Vec2::new(0.0, -1.0);
    let portals = [
        PlacedPortal::fixed(
            PURPLE,
            Vec2::new(254.0, floor_y),
            up,
            ambition_portal2d::PORTAL_OPENING_HALF,
        ).unwrap(),
        PlacedPortal::fixed(
            YELLOW,
            Vec2::new(554.0, floor_y),
            up,
            ambition_portal2d::PORTAL_OPENING_HALF,
        ).unwrap(),
    ];

    let mut scratch = BodyClusterScratch::new_with_abilities(
        Vec2::new(254.0, floor_y - drop),
        ambition_platformer2d_core::AbilitySet::default(),
    );
    scratch.ground.on_ground = false;
    let size = Vec2::new(24.0, 40.0);
    let dt = 1.0 / 60.0;

    let mut transit: Option<PortalTransit> = None;
    let mut transferred = false;
    let mut apex_y = f32::INFINITY;
    for _ in 0..4000 {
        let previous = scratch.kinematics;
        step_axis_scratch(&world, &mut scratch, InputState::default(), dt, tuning);
        let step = transit_step_with_tuning(ambition_platformer2d_core::BodyKinematics { pos: scratch.kinematics.pos, canonical_size: size, vel: scratch.kinematics.vel, ..Default::default() },
            Some(ae::SweepSample { turn: None, down: Vec2::ZERO, travel: Default::default(), prev: previous.pos, vel: previous.vel, time: [0.0, 1.0], curr: scratch.kinematics.pos, material: Default::default(), half: Vec2::ZERO }),
            transit,
            None,
            &portals,
            &ambition_portal2d::PortalTuning::default(),);
        match step {
            TransitStep::Begin { channel, .. } => {
                transit = Some(PortalTransit {
                    straddling: channel,
                    crossed: false,
                });
            }
            TransitStep::Transfer { transfer, .. } => {
                let pos = transfer.exit_pos;
                let vel = transfer.exit_vel;
                let exit_channel = transfer.exit_channel;
                if transferred {
                    break; // falling back in — the round trip is complete
                }
                transferred = true;
                scratch.kinematics.pos = pos;
                scratch.kinematics.vel = vel;
                transit = Some(PortalTransit {
                    straddling: exit_channel,
                    crossed: true,
                });
            }
            TransitStep::Clear => transit = None,
            TransitStep::Idle | TransitStep::Continue => {}
            TransitStep::Refused { reason, .. } => panic!("unexpected transit refusal: {reason:?}"),
        }
        if transferred {
            apex_y = apex_y.min(scratch.kinematics.pos.y);
            // Apex passed once the body is falling again.
            if scratch.kinematics.vel.y > 0.0 && scratch.kinematics.pos.y > apex_y + 4.0 {
                break;
            }
        }
    }
    assert!(transferred, "the drop must transfer through the pair");
    floor_y - apex_y
}

/// Energy round trip BELOW terminal velocity: falling into a ground pair from
/// a height the fall cap never touches must pop back up to the SAME height
/// (gravity is conservative, the portal map is an isometry — the transfer
/// itself is lossless). This is the "fall in from a tower, come back up to
/// the tower" promise at sub-terminal scale.
#[test]
fn floor_floor_round_trip_returns_to_drop_height_below_terminal() {
    use ambition_platformer2d_core::movement::DEFAULT_TUNING;
    let drop = 400.0; // entry ≈ 1342 px/s < DEFAULT max_fall_speed 1900
    let apex = floor_floor_round_trip_apex(drop, DEFAULT_TUNING);
    assert!(
        (apex - drop).abs() <= drop * 0.05,
        "sub-terminal round trip must conserve height: dropped {drop}, rebounded {apex}"
    );
}

/// Energy round trip WITHOUT a terminal velocity: with `max_fall_speed` effectively unbounded, a
/// fall from ANY height returns to that height. Per the Round 5 carried-momentum principle (the
/// WORLD has no air drag; assists belong to the controller), world-imparted fall speed is conserved
/// when the cap is out of the way — the player's shipped tuning keeps it out of the way.
#[test]
fn floor_floor_round_trip_conserves_any_height_without_terminal_velocity() {
    use ambition_platformer2d_core::movement::DEFAULT_TUNING;
    let tuning = ae::movement::MovementTuning {
        max_fall_speed: f32::INFINITY,
        ..DEFAULT_TUNING
    };
    let drop = 2400.0; // far beyond DEFAULT 1900's ~802px capped apex
    let apex = floor_floor_round_trip_apex(drop, tuning);
    assert!(
        (apex - drop).abs() <= drop * 0.05,
        "uncapped round trip must conserve height: dropped {drop}, rebounded {apex}"
    );
}

/// Documentation pin for the capped behavior: WITH a terminal velocity, a
/// tall drop's rebound saturates at `max_fall_speed² / (2·gravity)` no matter
/// the height — the number to check when a "portal damped my fall" report
/// comes in (it isn't the portal). With the old shipped player tuning
/// (cap 950, gravity 2250) this apex was only ≈ 200px.
#[test]
fn floor_floor_round_trip_saturates_at_the_terminal_apex_when_capped() {
    use ambition_platformer2d_core::movement::DEFAULT_TUNING;
    let tuning = ae::movement::MovementTuning {
        max_fall_speed: 950.0,
        ..DEFAULT_TUNING
    };
    let drop = 1600.0;
    let apex = floor_floor_round_trip_apex(drop, tuning);
    let terminal_apex = 950.0 * 950.0 / (2.0 * tuning.gravity);
    assert!(
        (apex - terminal_apex).abs() <= terminal_apex * 0.06,
        "capped rebound saturates at cap²/2g ≈ {terminal_apex:.0}: dropped {drop}, rebounded {apex}"
    );
}

/// CC6 end-to-end: a portal placed on an identified moving platform attaches
/// to its host face and RIDES it — attach attributes the `GeoFaceRef`, refresh
/// re-derives the aperture pose each frame from the platform's live block,
/// records the aperture's own sweep sample, and derives px/s velocity. An
/// unattributable portal (anon fixture wall) stays byte-static.
#[test]
fn a_portal_on_a_moving_platform_rides_its_host_face() {
    use crate::portal::host_adapter::{attach_portal_hosts, refresh_hosted_portal_frames};
    use ambition_platformer2d_world::collision::MovingPlatformSet;
    use ambition_platformer2d_world::platforms::MovingPlatformState;

    let mut app = App::new();
    // Authored base: one anon fixture wall (unattributable on purpose).
    ambition_platformer2d::platformer::lifecycle::insert_live_room_component(
        app.world_mut(),
        RoomGeometry(ae::World::new(
            "cc6",
            Vec2::new(2000.0, 1000.0),
            Vec2::ZERO,
            vec![ae::Block::solid(
                "anon-wall",
                Vec2::new(0.0, 0.0),
                Vec2::new(20.0, 400.0),
            )],
        )),
    );
    // One moving platform sweeping +x at 120 px/s — its collision block
    // carries the placement GeoId + per-tick displacement.
    let mut platform = MovingPlatformState::from_authored(
        Vec2::new(400.0, 500.0),
        Vec2::new(120.0, 20.0),
        400.0,
        120.0,
    );
    platform.update(1.0 / 60.0); // publish last_delta (2px)
    ambition_platformer2d::session::insert_live_room_component(app.world_mut(), MovingPlatformSet(vec![platform]));
    app.insert_resource(ambition_time::WorldTime::new(0.0, 1.0 / 60.0));
    app.add_systems(
        Update,
        (attach_portal_hosts, refresh_hosted_portal_frames).chain(),
    );

    // A floor portal ON the platform's top face (placed 2px proud, like the
    // gun does), and a second portal on the anon wall.
    let plat_top = ambition_platformer2d::session::sole_live_room_component::<MovingPlatformSet>(app.world()).expect("the live room has moving platforms").0[0].pos.y
        - ambition_platformer2d::session::sole_live_room_component::<MovingPlatformSet>(app.world()).expect("the live room has moving platforms").0[0].size.y * 0.5;
    let plat_x = ambition_platformer2d::session::sole_live_room_component::<MovingPlatformSet>(app.world()).expect("the live room has moving platforms").0[0].pos.x;
    let hosted = app
        .world_mut()
        .spawn(PlacedPortal::fixed(
            PURPLE,
            Vec2::new(plat_x, plat_top - 2.0),
            Vec2::new(0.0, -1.0),
            ambition_portal2d::PORTAL_OPENING_HALF,
        ).unwrap())
        .id();
    let unhosted = app
        .world_mut()
        .spawn(PlacedPortal::fixed(
            YELLOW,
            Vec2::new(22.0, 200.0),
            Vec2::new(1.0, 0.0),
            ambition_portal2d::PORTAL_OPENING_HALF,
        ).unwrap())
        .id();

    app.update(); // attach + first refresh

    let p = app.world().get::<PlacedPortal>(hosted).unwrap().clone();
    let lift = p.host.face().map(|(_, lift)| lift);
    assert!(lift.is_some(), "portal on identified platform attaches");
    assert!(
        (lift.unwrap() - 2.0).abs() < 1e-3,
        "authored lift preserved, got {lift:?}"
    );
    let u = app.world().get::<PlacedPortal>(unhosted).unwrap().clone();
    assert_eq!(u.host, PortalHost::Static, "anon fixture wall cannot host");
    assert_eq!(u.pos, Vec2::new(22.0, 200.0));
    assert!(!u.moved_this_tick());

    // Advance the platform one frame (+2px at 120 px/s / 60fps) and refresh:
    // the aperture rides, its frame delta and px/s velocity match the host.
    let before = p.pos;
    {
        let mut set = ambition_platformer2d::session::sole_live_room_component_mut::<MovingPlatformSet>(app.world_mut()).expect("the live room has moving platforms");
        set.0[0].update(1.0 / 60.0);
    }
    app.update();
    let p = app.world().get::<PlacedPortal>(hosted).unwrap().clone();
    let ridden = p.pos - before;
    assert!(
        (ridden.x - 2.0).abs() < 1e-3 && ridden.y.abs() < 1e-3,
        "aperture rides the host displacement, moved {ridden:?}"
    );
    assert_eq!(p.pos - p.step.origin, ridden, "the aperture's own sweep sample");
    assert_eq!(p.step.displacement,Vec2::new(2.0,0.0));
    assert_eq!(p.step.turn,0.0);
    assert!(
        (p.vel.x - 120.0).abs() < 1e-3,
        "px/s velocity derived from the host block, got {:?}",
        p.vel
    );

    // The host face vanishing closes the portal.
    ambition_platformer2d::session::sole_live_room_component_mut::<MovingPlatformSet>(app.world_mut()).expect("the live room has moving platforms")
        .0
        .clear();
    app.update();
    assert!(
        app.world().get::<PlacedPortal>(hosted).is_none(),
        "a portal cannot outlive its host face"
    );
    assert!(
        app.world().get::<PlacedPortal>(unhosted).is_some(),
        "unhosted portals are unaffected"
    );
}

/// A portal on a plank that spins rides the plank's line: each tick it takes
/// the line's point and normal, and the motion the plank published, its
/// turn per second about its center. That motion is what a body crossing it
/// is carried by.
#[test]
fn a_portal_on_a_spinning_plank_turns_with_it_and_carries_its_spin() {
    use crate::portal::host_adapter::{attach_portal_hosts, refresh_hosted_portal_frames};
    use ambition_platformer2d_world::collision::MovingPlatformSet;
    use ambition_platformer2d_world::platforms::{MovingPlatformState, Plank, PlatformTurn};

    let mut app = App::new();
    ambition_platformer2d::platformer::lifecycle::insert_live_room_component(
        app.world_mut(),
        RoomGeometry(ae::World::new("plank", Vec2::new(2000.0, 1000.0), Vec2::ZERO, vec![])),
    );
    let mut plank = MovingPlatformState::from_authored(Vec2::new(400.0, 500.0), Vec2::new(200.0, 8.0), 0.0, 0.0);
    plank.plank = Some(Plank { turn: PlatformTurn::Spin { deg_per_s: 30.0 }, ..Plank::default() });
    ambition_platformer2d::session::insert_live_room_component(app.world_mut(), MovingPlatformSet(vec![plank]));
    app.insert_resource(ambition_time::WorldTime::new(0.0, 1.0 / 60.0));
    app.add_systems(Update, (attach_portal_hosts, refresh_hosted_portal_frames).chain());
    let portal = app
        .world_mut()
        .spawn(PlacedPortal::fixed(
            PURPLE,
            Vec2::new(440.0, 500.0),
            Vec2::new(0.0, -1.0),
            ambition_portal2d::PORTAL_OPENING_HALF,
        ).unwrap())
        .id();
    app.update();
    let p = app.world().get::<PlacedPortal>(portal).unwrap().clone();
    assert!(matches!(p.host, PortalHost::Line { .. }), "rides the plank's line: {:?}", p.host);

    {
        let mut set = ambition_platformer2d::session::sole_live_room_component_mut::<MovingPlatformSet>(app.world_mut())
            .expect("the live room has moving platforms");
        set.0[0].update(1.0 / 60.0);
    }
    app.update();
    let p = app.world().get::<PlacedPortal>(portal).unwrap().clone();
    let spin = 30f32.to_radians();
    assert!((p.spin - spin).abs() < 1e-3, "the plank's turn per second: {}", p.spin);
    assert!(p.pivot.distance(Vec2::new(400.0, 500.0)) < 1e-3, "about its center: {:?}", p.pivot);
    let turned = p.step.normal.angle_to(p.normal);
    assert!((turned - spin / 60.0).abs() < 1e-4, "turned with it this tick: {turned}");
    let line=ambition_platformer2d::session::sole_live_room_component::<MovingPlatformSet>(app.world())
        .unwrap().0[0].as_collision_line().unwrap();
    assert_eq!(p.step.turn,line.turn);
    assert_eq!(p.step.displacement,line.velocity);
    let half_step=ae::SurfaceChain {velocity:line.velocity*0.5,turn:line.turn*0.5,
        pivot:line.pivot-line.velocity*0.5,..line};
    let middle=p.step.sample(p.aperture(),0.5).unwrap();
    assert!(middle.frame.origin.distance(half_step.carried(p.step.origin))<0.001);
    // Still 40 px along the line from its center.
    assert!((p.pos.distance(p.pivot) - 40.0).abs() < 0.5, "{:?}", p.pos);
}

#[test]
fn a_wrapping_host_closes_its_old_plane_without_sweeping_the_wrap() {
    use crate::portal::host_adapter::{attach_portal_hosts, refresh_hosted_portal_frames};
    use ambition_platformer2d_world::collision::MovingPlatformSet;
    use ambition_platformer2d_world::platforms::{MovingPlatformState, Plank, PlatformTurn};

    for spinning in [false, true] {
        let mut app = App::new();
        ambition_platformer2d::session::insert_live_room_component(
            app.world_mut(),
            RoomGeometry(ae::World::new("wrap", Vec2::splat(1000.0), Vec2::ZERO, vec![])),
        );
        let mut host = MovingPlatformState::from_vertical_loop(
            "wrap", "wrap", Vec2::new(400.0, 499.0), Vec2::new(200.0, 20.0),
            100.0, 500.0, 120.0, true,
        );
        if spinning {
            host.plank = Some(Plank {
                turn: PlatformTurn::Spin { deg_per_s: 30.0 },
                ..Plank::default()
            });
        }
        let start = host.pos + Vec2::new(40.0, if spinning { 0.0 } else { -10.0 });
        ambition_platformer2d::session::insert_live_room_component(
            app.world_mut(), MovingPlatformSet(vec![host]),
        );
        app.insert_resource(ambition_time::WorldTime::new(0.0, 1.0 / 60.0));
        app.init_resource::<ambition_portal2d::PortalFrameHistory>();
        app.add_systems(Update, (
            attach_portal_hosts,
            refresh_hosted_portal_frames,
            ambition_portal2d::evict_straddlers_on_portal_change,
        ).chain());
        let portal = app.world_mut().spawn(PlacedPortal::fixed(
            PURPLE, start, Vec2::NEG_Y, ambition_portal2d::PORTAL_OPENING_HALF,
        ).unwrap()).id();
        let body_start = start - Vec2::Y * 2.0;
        let body = app.world_mut().spawn(BodyKinematics {
            pos: body_start, canonical_size: Vec2::new(24.0, 40.0),
            vel: Vec2::ZERO, facing: 1.0, material: Default::default(),
        }).id();
        app.update();
        assert_eq!(app.world().get::<BodyKinematics>(body).unwrap().pos, body_start);

        ambition_platformer2d::session::sole_live_room_component_mut::<MovingPlatformSet>(
            app.world_mut(),
        ).unwrap().0[0].update(1.0 / 60.0);
        app.update();

        let p = app.world().get::<PlacedPortal>(portal).unwrap();
        assert!(p.host.is_hosted());
        assert_eq!(p.step.displacement, Vec2::new(0.0, 2.0));
        let continuous_start = p.step.sample(p.aperture(), 0.0).unwrap();
        assert!(continuous_start.frame.origin.distance(start - Vec2::Y * 400.0) < 0.001);
        assert!(continuous_start.frame.normal.distance(Vec2::NEG_Y) < 0.00001);
        assert!(p.step.sample(p.aperture(), 0.5).unwrap().frame.origin.y < 110.0);
        let evicted = app.world().get::<BodyKinematics>(body).unwrap();
        assert!((evicted.pos.y - (start.y - 21.0)).abs() < 0.001);
    }
}

/// OW1: a portal finds its host face in its OWN live room. A moving platform
/// is in #1 only; a portal stamped #1 on its top attaches to it, and a portal
/// stamped #0 at the same place finds no face and is `Static`. Before, the
/// adapter read the sole live room, so while two rooms were live it did not
/// look at all, and both portals stayed unattributed.
#[test]
fn a_portal_finds_its_host_face_in_its_own_live_room() {
    use crate::portal::host_adapter::attach_portal_hosts;
    use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
    use ambition_platformer2d_world::collision::MovingPlatformSet;
    use ambition_platformer2d_world::platforms::MovingPlatformState;

    let mut app = App::new();
    let platform = MovingPlatformState::from_authored(
        Vec2::new(400.0, 500.0),
        Vec2::new(120.0, 20.0),
        400.0,
        120.0,
    );
    let top = platform.pos.y - platform.size.y * 0.5;
    let x = platform.pos.x;
    let live = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next()];
    for room in live {
        let geometry = RoomGeometry(ae::World::new("ow1", Vec2::new(2000.0, 1000.0), Vec2::ZERO, vec![]));
        let mut root = app.world_mut().spawn((RoomInstanceRoot, room, geometry));
        if room == live[1] {
            root.insert(MovingPlatformSet(vec![platform.clone()]));
        }
    }
    app.add_systems(Update, attach_portal_hosts);
    let portal = |room| {
        (
            PlacedPortal::fixed(
                YELLOW,
                Vec2::new(x, top - 2.0),
                Vec2::new(0.0, -1.0),
                ambition_portal2d::PORTAL_OPENING_HALF,
            ).unwrap(),
            InRoomInstance(room),
        )
    };
    let on_platform = app.world_mut().spawn(portal(live[1])).id();
    let elsewhere = app.world_mut().spawn(portal(live[0])).id();
    app.update();
    let host = |portal| app.world().get::<PlacedPortal>(portal).unwrap().host.clone();
    assert!(host(on_platform).face().is_some(), "the portal in #1 did not attach to #1's platform: {:?}", host(on_platform));
    assert_eq!(host(elsewhere), PortalHost::Static, "the portal in #0 is not static");
}

/// A portal is looked at once. The answer is part of the portal, so a
/// `Static` portal is not attached later, even on a face that could host it.
/// A later scan would see moving platforms in other places than the confirmed
/// timeline did. The unattributed twin at the same place is the control: the
/// face under both of them can host.
#[test]
fn a_portal_the_adapter_has_decided_is_not_scanned_again() {
    use crate::portal::host_adapter::attach_portal_hosts;
    use ambition_platformer2d_world::collision::MovingPlatformSet;
    use ambition_platformer2d_world::platforms::MovingPlatformState;

    let mut app = App::new();
    ambition_platformer2d::platformer::lifecycle::insert_live_room_component(
        app.world_mut(),
        RoomGeometry(ae::World::new("cc6", Vec2::new(2000.0, 1000.0), Vec2::ZERO, vec![])),
    );
    let platform = MovingPlatformState::from_authored(
        Vec2::new(400.0, 500.0),
        Vec2::new(120.0, 20.0),
        400.0,
        120.0,
    );
    let top = platform.pos.y - platform.size.y * 0.5;
    let x = platform.pos.x;
    ambition_platformer2d::session::insert_live_room_component(app.world_mut(), MovingPlatformSet(vec![platform]));
    app.add_systems(Update, attach_portal_hosts);

    let place = |channel| {
        PlacedPortal::fixed(
            channel,
            Vec2::new(x, top - 2.0),
            Vec2::new(0.0, -1.0),
            ambition_portal2d::PORTAL_OPENING_HALF,
        ).unwrap()
    };
    let decided = app
        .world_mut()
        .spawn(PlacedPortal { host: PortalHost::Static, ..place(PURPLE) })
        .id();
    let fresh = app.world_mut().spawn(place(YELLOW)).id();
    app.update();

    assert!(
        app.world().get::<PlacedPortal>(fresh).unwrap().host.face().is_some(),
        "control: the face under both portals can host"
    );
    assert_eq!(
        app.world().get::<PlacedPortal>(decided).unwrap().host,
        PortalHost::Static,
        "a decided portal was scanned again"
    );
}

/// Two live room roots, each with its own overlay, and the carves published
/// this frame, one for each entry of `holes`: the index of the live room it
/// was cut in, or `None` for no live room. The carves each overlay holds
/// after [`crate::portal::bridge_portal_carves`] runs, by room.
fn carves_by_room(holes: &[Option<usize>]) -> [usize; 2] {
    use ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay;
    use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, RoomInstanceRoot};
    let mut app = App::new();
    let live = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next()];
    let roots = live.map(|room| {
        app.world_mut()
            .spawn((RoomInstanceRoot, room, FeatureEcsWorldOverlay::default()))
            .id()
    });
    let body = app.world_mut().spawn_empty().id();
    app.insert_resource(PortalCarves {
        holes: holes
            .iter()
            .map(|room| ambition_portal2d::PortalHole {
                room: room.map(|room| live[room]),
                face: ae::frame::FaceRect {
                    center: Vec2::new(200.0, 390.0),
                    normal: Vec2::new(0.0, -1.0),
                    half_along: 24.0,
                    half_out: 10.0,
                },
                open_for: vec![body],
            })
            .collect(),
        glues: Vec::new(),
    });
    app.add_systems(Update, crate::portal::bridge_portal_carves);
    app.update();
    roots.map(|root| {
        app.world()
            .get::<FeatureEcsWorldOverlay>(root)
            .expect("the root keeps its overlay")
            .portal_holes
            .len()
    })
}

/// OW1 cut 7l: with two live rooms, each carve goes to the overlay of the
/// room it was cut in, and a room with no carve stays sealed. When the bridge
/// wrote the sole live room's overlay, no room was carved while two were
/// live; when it wrote the one room all portals were in, a pair in each room
/// carved neither.
#[test]
fn each_carve_goes_to_the_live_room_it_was_cut_in() {
    assert_eq!(carves_by_room(&[Some(1)]), [0, 1], "a carve in #1 goes to #1 only");
    assert_eq!(carves_by_room(&[Some(0)]), [1, 0], "a carve in #0 goes to #0 only");
    assert_eq!(carves_by_room(&[Some(0), Some(1), Some(1)]), [1, 2], "each room keeps its own carves");
    assert_eq!(carves_by_room(&[None]), [0, 0], "a carve in no live room is not written");
}

/// The bridge runs while a carve stands or a room still holds a hole, so the
/// hole of a portal that went is cleared and the wall seals again.
#[test]
fn a_hole_left_after_the_last_carve_is_sealed() {
    use ambition_platformer2d_shared_tangle::feature_overlay::{FeatureEcsWorldOverlay, PortalHolePass};
    use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, RoomInstanceRoot};
    let mut app = App::new();
    let mut overlay = FeatureEcsWorldOverlay::default();
    overlay.portal_holes.push(PortalHolePass {
        face: ae::frame::FaceRect {
            center: Vec2::new(200.0, 390.0),
            normal: Vec2::new(0.0, -1.0),
            half_along: 24.0,
            half_out: 10.0,
        },
        bodies: Vec::new(),
    });
    let root = app.world_mut().spawn((RoomInstanceRoot, LiveRoomInstance::ACTIVATION, overlay)).id();
    app.insert_resource(PortalCarves::default());
    app.add_systems(
        Update,
        crate::portal::bridge_portal_carves.run_if(crate::portal::carve_adapter::carves_or_holes_stand),
    );
    app.update();
    assert!(
        app.world().get::<FeatureEcsWorldOverlay>(root).unwrap().portal_holes.is_empty(),
        "the hole of a portal that went stayed open"
    );
}

/// The host a portal at `at` (facing up) attaches to on `platform`, and the
/// depth the adapter measures behind it.
fn host_and_depth(
    platform: ambition_platformer2d_world::platforms::MovingPlatformState,
    at: Vec2,
) -> (PortalHost, f32) {
    use crate::portal::carve_adapter::sync_portal_host_depths;
    use crate::portal::host_adapter::{attach_portal_hosts, refresh_hosted_portal_frames};
    use ambition_platformer2d_world::collision::MovingPlatformSet;

    let mut app = App::new();
    ambition_platformer2d::platformer::lifecycle::insert_live_room_component(
        app.world_mut(),
        RoomGeometry(ae::World::new("hosts", Vec2::new(2000.0, 1000.0), Vec2::ZERO, vec![])),
    );
    ambition_platformer2d::session::insert_live_room_component(app.world_mut(), MovingPlatformSet(vec![platform]));
    app.insert_resource(ambition_time::WorldTime::new(0.0, 1.0 / 60.0));
    app.init_resource::<PortalHostDepthsByRoom>();
    app.add_systems(
        Update,
        (attach_portal_hosts, refresh_hosted_portal_frames, sync_portal_host_depths).chain(),
    );
    let portal = app.world_mut().spawn(PlacedPortal::fixed(PURPLE, at, Vec2::new(0.0, -1.0), 30.0).unwrap()).id();
    app.update();
    let host = app.world().get::<PlacedPortal>(portal).expect("the portal stays on its host").host.clone();
    let depth = app.world().resource::<PortalHostDepthsByRoom>().0.iter().map(|(_, d)| d.depth(PURPLE)).find(|d| d.is_finite());
    (host, depth.expect("the portal is measured"))
}

/// The rescue reaches as deep as the host is: a portal on a moving block is
/// measured through that block where it is, and a portal on a plank (a line
/// with open air under it) has nothing behind it. Before, both read the
/// authored room only, found nothing, and took the 60 px probe, so a body
/// under the plank moving away from its portal was taken through it.
#[test]
fn a_portal_is_as_deep_as_the_host_it_rides() {
    use ambition_platformer2d_world::platforms::{MovingPlatformState, Plank};
    // A block 20 tall with its top at y = 490; the portal 2 px proud of it.
    let block = MovingPlatformState::from_authored(Vec2::new(400.0, 500.0), Vec2::new(120.0, 20.0), 0.0, 0.0);
    let (host, depth) = host_and_depth(block, Vec2::new(400.0, 488.0));
    assert!(matches!(host, PortalHost::Face { .. }), "{host:?}");
    assert!((depth - 22.0).abs() < 1.0, "through the block from 2 px proud: {depth}");

    let mut plank = MovingPlatformState::from_authored(Vec2::new(400.0, 500.0), Vec2::new(200.0, 8.0), 0.0, 0.0);
    plank.plank = Some(Plank::default());
    let (host, depth) = host_and_depth(plank, Vec2::new(440.0, 500.0));
    assert!(matches!(host, PortalHost::Line { .. }), "{host:?}");
    assert_eq!(depth, 0.0, "nothing is behind a plank");
}

/// ⛔ Two portals never share a place (Jon, 2026-10-30: "if a portal detects
/// an overlap they just fizzle"). An orange shot that lands on the blue's
/// place on the left wall fizzles and the blue stays; the control lands on
/// the same wall clear of it and opens.
#[test]
fn a_shot_that_would_overlap_another_portal_fizzles() {
    for (y, overlaps) in [(220.0, true), (330.0, false)] {
        let mut app = app_with_the_shot_adapter();
        let blue = app
            .world_mut()
            .spawn(PlacedPortal::fixed(BLUE, Vec2::new(20.0, 200.0), Vec2::new(1.0, 0.0), ambition_portal2d::PORTAL_OPENING_HALF).unwrap())
            .id();
        app.world_mut().spawn(PortalShot { channel: ORANGE, pos: Vec2::new(45.0, y), vel: Vec2::new(-1900.0, 0.0), traveled: 0.0 });
        app.update();
        let portals: Vec<(PortalChannel, Vec2)> =
            app.world_mut().query::<&PlacedPortal>().iter(app.world()).map(|p| (p.channel, p.pos)).collect();
        assert!(app.world().get_entity(blue).is_ok(), "the blue that was there first stays");
        let oranges = portals.iter().filter(|(channel, _)| *channel == ORANGE).count();
        assert_eq!(oranges, usize::from(!overlaps), "an orange at y {y} opens only clear of the blue: {portals:?}");
    }
}
