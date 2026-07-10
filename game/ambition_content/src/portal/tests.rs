//! Portal mechanic tests. These were previously inline in the monolithic portal
//! module; they exercise the public portal surface through the facade.

use bevy::prelude::*;

use ambition_actors::actor::BodyBaseSize;
use ambition_actors::actor::{BodyKinematics, PlayerEntity, PrimaryPlayer};
use ambition_actors::platformer_runtime::gravity::{gravity_upright_angle, GravityField};
use ambition_actors::platformer_runtime::orientation::{update_actor_roll, ActorRoll};
use ambition_actors::platformer_runtime::transit::rotate_velocity_between_normals as portal_transform_velocity;
use ambition_characters::brain::ActionSet;
use ambition_engine_core::RoomGeometry;
use ambition_engine_core::{self as ae};
use ambition_input::ControlFrame;

#[allow(unused_imports)]
use super::*;
use ambition_actors::platformer_runtime::collision::raycast_solids;
use ambition_portal::*;

// Channel shorthands for the tests: the gun's pair (Blue/Orange) and two authored
// pairs (Purple/Yellow). These map the old `PortalColor::X` literals onto the new
// `PortalChannel` so the tests still pin byte-identical pairing/transit behavior.
const BLUE: PortalChannel = PortalChannel::Gun(PortalGunColor::BLUE);
const ORANGE: PortalChannel = PortalChannel::Gun(PortalGunColor::ORANGE);
const PURPLE: PortalChannel = PortalChannel::Authored(PortalChannelColor::Purple);
const YELLOW: PortalChannel = PortalChannel::Authored(PortalChannelColor::Yellow);

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
            BodyKinematics {
                pos,
                vel: Vec2::ZERO,
                size: Vec2::new(24.0, 40.0),
                facing,
            },
            BodyBaseSize {
                base_size: Vec2::new(24.0, 40.0),
            },
            PortalGun::default(),
            ActionSet::default(),
            // Opt the player into the generic transit core with the player
            // policy (re-orient + carry velocity), as the Ambition tagging
            // adapter does in the real app.
            ambition_portal::PortalBody,
            ambition_portal::PortalPolicy {
                reorient: true,
                carry_velocity: true,
            },
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

/// Emit a `FirePortalGun` intent for the primary player, resolving aim the same
/// way the input adapter does (facing-ahead, since these tests set no stick).
fn fire_portal(app: &mut App) {
    let facing = {
        let mut q = app
            .world_mut()
            .query_filtered::<&BodyKinematics, (With<PlayerEntity>, With<PrimaryPlayer>)>();
        q.iter(app.world()).next().map_or(1.0, |k| k.facing)
    };
    app.world_mut().write_message(FirePortalGun {
        aim: Vec2::new(if facing >= 0.0 { 1.0 } else { -1.0 }, 0.0),
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
    use ambition_engine_core::world::{Block, World};
    let world = World {
        name: "one-way".to_string(),
        size: Vec2::new(400.0, 400.0),
        spawn: Vec2::new(200.0, 200.0),
        blocks: vec![Block::one_way(
            "ledge",
            Vec2::new(100.0, 300.0),
            Vec2::new(200.0, 12.0),
        )],
        climbable_regions: Vec::new(),
        chains: Vec::new(),
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
            portal_half_extent(Vec2::new(0.0, -1.0)),
        ),
        PlacedPortal::fixed(
            ORANGE,
            Vec2::new(380.0, 200.0),
            Vec2::new(-1.0, 0.0),
            portal_half_extent(Vec2::new(-1.0, 0.0)),
        ),
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
        2,
    );
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
    );
    assert!(
        (out.x + 100.0).abs() < 0.01 && out.y.abs() < 0.01,
        "fall-in should exit left at the same speed, got {out:?}"
    );
}

#[test]
fn in_flight_ground_item_travels_through_the_portal_pair() {
    use crate::portal::{sync_ground_items_to_transitable, sync_transitable_to_ground_items};
    use ambition_actors::items::pickup::GroundItem;
    let mut app = App::new();
    // The content adapter brackets the core teleport: attach + sync the
    // PortalTransitable body before, mirror it back to GroundItem after.
    app.add_systems(
        Update,
        (
            sync_ground_items_to_transitable,
            portal_teleport_ground_items,
            sync_transitable_to_ground_items,
        )
            .chain(),
    );
    // Blue portal facing right at x=20, orange facing left at x=380.
    app.world_mut().spawn(PlacedPortal::fixed(
        BLUE,
        Vec2::new(20.0, 200.0),
        Vec2::new(1.0, 0.0),
        portal_half_extent(Vec2::new(1.0, 0.0)),
    ));
    app.world_mut().spawn(PlacedPortal::fixed(
        ORANGE,
        Vec2::new(380.0, 200.0),
        Vec2::new(-1.0, 0.0),
        portal_half_extent(Vec2::new(1.0, 0.0)),
    ));
    // A thrown item flying into the blue portal.
    let item = app
        .world_mut()
        .spawn(GroundItem {
            spec: ambition_actors::items::pickup::axe_spec(),
            pos: Vec2::new(20.0, 200.0),
            vel: Vec2::new(-300.0, 0.0),
            half_extent: Vec2::splat(12.0),
        })
        .id();
    app.update();
    let g = app.world().get::<GroundItem>(item).unwrap();
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

#[test]
fn portal_fit_gate_keys_on_the_opening_perpendicular_to_the_normal() {
    let wall = PlacedPortal::fixed(
        BLUE,
        Vec2::ZERO,
        Vec2::new(1.0, 0.0),
        portal_half_extent(Vec2::new(1.0, 0.0)),
    );
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
        portal_half_extent(Vec2::new(0.0, -1.0)),
    );
    assert!(portal_fits(Vec2::new(40.0, 200.0), &floor));
    assert!(!portal_fits(Vec2::new(100.0, 20.0), &floor));
}

#[test]
fn portals_teleport_a_fitting_actor_and_skip_an_oversized_one() {
    use ambition_actors::features::BodyKinematics;
    let mut app = App::new();
    app.add_message::<ambition_portal::PortalBodyEntered>();
    app.add_message::<ambition_portal::PortalBodyTransited>();
    app.init_resource::<ambition_portal::PortalTuning>();
    app.add_systems(Update, portal_transit);
    // Actor policy: carry velocity, no re-orient (facing follows AI).
    let actor_policy = ambition_portal::PortalPolicy {
        reorient: false,
        carry_velocity: true,
    };
    app.world_mut().spawn(PlacedPortal::fixed(
        BLUE,
        Vec2::new(20.0, 200.0),
        Vec2::new(1.0, 0.0),
        portal_half_extent(Vec2::new(1.0, 0.0)),
    ));
    app.world_mut().spawn(PlacedPortal::fixed(
        ORANGE,
        Vec2::new(380.0, 200.0),
        Vec2::new(-1.0, 0.0),
        portal_half_extent(Vec2::new(1.0, 0.0)),
    ));
    let small = app
        .world_mut()
        .spawn((
            BodyKinematics {
                pos: Vec2::new(20.0, 200.0),
                vel: Vec2::new(-100.0, 0.0),
                size: Vec2::new(24.0, 40.0),
                facing: -1.0,
            },
            ambition_portal::PortalBody,
            actor_policy,
        ))
        .id();
    let big = app
        .world_mut()
        .spawn((
            BodyKinematics {
                pos: Vec2::new(20.0, 200.0),
                vel: Vec2::new(-100.0, 0.0),
                size: Vec2::new(80.0, 200.0),
                facing: -1.0,
            },
            ambition_portal::PortalBody,
            actor_policy,
        ))
        .id();
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
    let he = portal_half_extent(Vec2::new(0.0, -1.0));
    let floor = |channel, x: f32| {
        PlacedPortal::fixed(channel, Vec2::new(x, 300.0), Vec2::new(0.0, -1.0), he)
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
    let step = transit_step(
        Vec2::new(400.0, 305.0),
        Vec2::new(24.0, 40.0),
        Vec2::new(0.0, 50.0),
        Some(PortalTransit {
            straddling: PURPLE,
            crossed: false,
        }),
        None,
        &portals,
        Vec2::new(0.0, 1.0),
    );
    match step {
        TransitStep::Transfer {
            exit_channel, pos, ..
        } => {
            assert_eq!(exit_channel, YELLOW, "purple links to yellow");
            assert!(pos.x > 600.0, "emerges at the yellow portal, got {pos:?}");
        }
        other => panic!("expected a transfer to yellow, got {other:?}"),
    }
}

#[test]
fn facing_flip_policy_is_convention_aware() {
    let g = Vec2::new(0.0, 1.0); // gravity down
    let up = Vec2::new(0.0, -1.0); // floor
    let down = Vec2::new(0.0, 1.0); // ceiling
    let left = Vec2::new(-1.0, 0.0); // right-wall normal
    let right = Vec2::new(1.0, 0.0); // left-wall normal

    // Reflection convention: same-wall is the only suppressed-roll mirror, so
    // facing flips to keep the leading side leading out.
    assert!(portal_facing_flips_for_convention(false, left, left, g));
    // Walls facing EACH OTHER (portal_bridge) go straight through.
    assert!(!portal_facing_flips_for_convention(false, right, left, g));
    // Floor/ceiling pairs carry their visible turn in roll, not facing.
    assert!(!portal_facing_flips_for_convention(false, up, up, g));
    assert!(!portal_facing_flips_for_convention(false, down, down, g));
    assert!(!portal_facing_flips_for_convention(false, up, left, g));

    // Rotation convention is a proper orientation map: no extra mirror is ever
    // needed, including the same-wall 180-degree case.
    assert!(!portal_facing_flips_for_convention(true, left, left, g));
}

#[test]
fn somersault_policy_is_convention_aware() {
    use std::f32::consts::PI;
    let g = Vec2::new(0.0, 1.0); // gravity down
    let up = Vec2::new(0.0, -1.0); // floor normal
    let down = Vec2::new(0.0, 1.0); // ceiling normal
    let left = Vec2::new(-1.0, 0.0); // right-wall normal

    // Reflection convention keeps the historical gravity-platformer
    // accommodation: floor/ceiling tumble, wall↔wall stays upright.
    assert!((somersault_roll_for_convention(false, up, up, g).abs() - PI).abs() < 1e-5);
    assert!((somersault_roll_for_convention(false, down, down, g).abs() - PI).abs() < 1e-5);
    assert!(somersault_roll_for_convention(false, left, left, g).abs() < 1e-5);
    // A floor→wall pair still tumbles 90° (it genuinely reorients).
    assert!(somersault_roll_for_convention(false, up, left, g).abs() > 1.0);

    // Rotation convention is a proper map, so same-wall is now a true 180° roll.
    assert!((somersault_roll_for_convention(true, left, left, g).abs() - PI).abs() < 1e-5);
}

#[test]
fn held_input_warp_gate_is_convention_aware() {
    let up = Vec2::new(0.0, -1.0); // floor
    let left = Vec2::new(-1.0, 0.0); // right-wall normal

    // Reflection: same-wall flips horizontal movement; floor↔floor preserves it.
    assert!(portal_input_warp_flips_horizontal_for_convention(
        false, left, left
    ));
    assert!(!portal_input_warp_flips_horizontal_for_convention(
        false, up, up
    ));

    // Rotation: both same-wall and floor↔floor are proper 180-degree rotations,
    // so held horizontal movement must flip to keep helping the transformed
    // velocity instead of fighting it.
    assert!(portal_input_warp_flips_horizontal_for_convention(
        true, left, left
    ));
    assert!(portal_input_warp_flips_horizontal_for_convention(
        true, up, up
    ));

    // A 90-degree floor→wall turn maps horizontal input to vertical; the
    // platformer movement controller cannot express that as ordinary movement.
    assert!(!portal_input_warp_flips_horizontal_for_convention(
        true, up, left
    ));
    assert!(!portal_input_warp_flips_horizontal_for_convention(
        false, up, left
    ));
}

#[test]
fn portal_transit_roll_is_general_and_matches_on_screen_turn() {
    use std::f32::consts::{FRAC_PI_2, PI};
    let up = Vec2::new(0.0, -1.0); // floor portal faces up (y-down world)
    let down = Vec2::new(0.0, 1.0); // ceiling portal faces down
    let left = Vec2::new(-1.0, 0.0); // right wall faces left
    let right = Vec2::new(1.0, 0.0); // left wall faces right

    // Floor↔floor flips 180° (you somersault).
    assert!((portal_transit_roll(up, up).abs() - PI).abs() < 1e-5);
    // A straight-through wall pair (enter into +x wall, exit -x wall) keeps
    // orientation — no turn.
    assert!(portal_transit_roll(right, left).abs() < 1e-5);
    // Floor→right-wall: falling in, you exit moving LEFT, so the body turns
    // -90° (render) — feet swing from down to left, leaving feet-first.
    assert!((portal_transit_roll(up, left) - (-FRAC_PI_2)).abs() < 1e-5);
    // The reverse pair turns the opposite way.
    assert!((portal_transit_roll(left, up) - FRAC_PI_2).abs() < 1e-5);
    // Ceiling↔ceiling also flips 180°.
    assert!((portal_transit_roll(down, down).abs() - PI).abs() < 1e-5);
}

#[test]
fn roll_eases_back_to_gravity_upright_in_air() {
    let mut app = App::new();
    app.insert_resource(ambition_platformer_primitives::time::SimDt { dt: 1.0 / 60.0 });
    app.init_resource::<GravityField>();
    app.add_systems(Update, update_actor_roll);
    // Start rolled 180° (just exited a floor↔floor portal), airborne. The
    // righting system reads each body's own position via BodyKinematics, so the
    // test body carries one (the dual-arm query collapsed to a single
    // With<BodyKinematics> in Stage 16 / S5).
    let player = app
        .world_mut()
        .spawn((
            ambition_platformer_primitives::body::BodyKinematics {
                pos: Vec2::ZERO,
                vel: Vec2::ZERO,
                size: Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            ActorRoll {
                angle: std::f32::consts::PI,
            },
        ))
        .id();
    // It rights itself toward gravity-upright (0) over time WITHOUT needing
    // to be grounded (the orient-to-gravity reflex).
    for _ in 0..120 {
        app.update();
    }
    let angle = app.world().get::<ActorRoll>(player).unwrap().angle;
    let from_upright = angle.min(std::f32::consts::TAU - angle); // distance to 0 mod 2π
    assert!(
        from_upright < 1e-2,
        "should right itself to gravity-up, got {angle}"
    );
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
fn actors_get_an_aerial_roll_through_portals() {
    use ambition_actors::features::BodyKinematics;
    let mut app = App::new();
    app.add_message::<ambition_portal::PortalBodyEntered>();
    app.add_message::<ambition_portal::PortalBodyTransited>();
    app.init_resource::<ambition_portal::PortalTuning>();
    app.add_systems(Update, portal_transit);
    // Floor portal (normal up) + right-wall portal (normal left): a
    // floor→wall pair, so transit imparts a -90° roll. Player and non-player
    // actors alike now tumble + reorient (the somersault is ported to the
    // aperture model and applied on the centroid transfer).
    app.world_mut().spawn(PlacedPortal::fixed(
        BLUE,
        Vec2::new(200.0, 380.0),
        Vec2::new(0.0, -1.0),
        portal_half_extent(Vec2::new(0.0, -1.0)),
    ));
    app.world_mut().spawn(PlacedPortal::fixed(
        ORANGE,
        Vec2::new(380.0, 200.0),
        Vec2::new(-1.0, 0.0),
        portal_half_extent(Vec2::new(-1.0, 0.0)),
    ));
    let actor = app
        .world_mut()
        .spawn((
            BodyKinematics {
                pos: Vec2::new(200.0, 380.0),
                vel: Vec2::new(0.0, 100.0),
                size: Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            ActorRoll::default(),
            ambition_portal::PortalBody,
            ambition_portal::PortalPolicy {
                reorient: false,
                carry_velocity: true,
            },
        ))
        .id();
    // Frame 1 begins transit; frame 2 transfers (centroid on the plane) and
    // imparts the somersault roll.
    app.update();
    app.update();
    let roll = app.world().get::<ActorRoll>(actor).unwrap().angle;
    let expected = portal_transit_roll(Vec2::new(0.0, -1.0), Vec2::new(-1.0, 0.0))
        .rem_euclid(std::f32::consts::TAU);
    assert!(
        (roll.rem_euclid(std::f32::consts::TAU) - expected).abs() < 1e-4,
        "a teleported actor should pick up the same aerial roll as the player; got {roll}, expected {expected}"
    );
}

#[test]
fn portal_pair_teleports_player_carrying_momentum() {
    let mut app = App::new();
    app.add_message::<ambition_portal::PortalBodyEntered>();
    app.add_message::<BodyTeleported>();
    app.add_message::<ambition_portal::PortalBodyTransited>();
    app.insert_resource(ambition_time::WorldTime::default());
    app.init_resource::<ambition_portal::PortalTuning>();
    app.add_systems(Update, portal_transit);
    // Blue on the left (facing right), orange on the right (facing left).
    app.world_mut().spawn(PlacedPortal::fixed(
        BLUE,
        Vec2::new(22.0, 200.0),
        Vec2::new(1.0, 0.0),
        portal_half_extent(Vec2::new(1.0, 0.0)),
    ));
    app.world_mut().spawn(PlacedPortal::fixed(
        ORANGE,
        Vec2::new(378.0, 200.0),
        Vec2::new(-1.0, 0.0),
        portal_half_extent(Vec2::new(1.0, 0.0)),
    ));
    let player = spawn_player(&mut app, Vec2::new(22.0, 200.0), 1.0);
    app.world_mut()
        .get_mut::<BodyKinematics>(player)
        .unwrap()
        .vel = Vec2::new(-100.0, 0.0);
    // Give the player a pre-set roll so we can prove the portal leaves the
    // player's orientation alone (#47 — no upside-down flip).
    app.world_mut()
        .entity_mut(player)
        .insert(ActorRoll { angle: 0.5 });
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
    let roll = app.world().get::<ActorRoll>(player).unwrap().angle;
    assert!(
        (roll - 0.5).abs() < 1e-5,
        "player keeps its orientation through the portal (#47 — no flip), got {roll}"
    );
}

#[test]
fn a_gunless_player_transits_an_authored_pair() {
    // The portal_lab scenario: pre-placed portals, player has NOT picked up
    // the gun. Transit must still work — crossing a placed pair is independent
    // of holding the gun, and the cooldown lives on the body.
    let mut app = App::new();
    app.add_message::<ambition_portal::PortalBodyEntered>();
    app.add_message::<BodyTeleported>();
    app.add_message::<ambition_portal::PortalBodyTransited>();
    app.insert_resource(ambition_time::WorldTime::default());
    app.init_resource::<ambition_portal::PortalTuning>();
    app.add_systems(Update, portal_transit);
    let he = portal_half_extent(Vec2::new(0.0, -1.0));
    app.world_mut().spawn(PlacedPortal::fixed(
        PURPLE,
        Vec2::new(200.0, 300.0),
        Vec2::new(0.0, -1.0),
        he,
    ));
    app.world_mut().spawn(PlacedPortal::fixed(
        YELLOW,
        Vec2::new(600.0, 300.0),
        Vec2::new(0.0, -1.0),
        he,
    ));
    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyKinematics {
                pos: Vec2::new(200.0, 285.0),
                vel: Vec2::ZERO,
                size: Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            BodyBaseSize {
                base_size: Vec2::new(24.0, 40.0),
            },
            // No PortalGun on purpose.
            ambition_portal::PortalBody,
            ambition_portal::PortalPolicy {
                reorient: true,
                carry_velocity: true,
            },
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
    app.update();
    let pos = app.world().get::<BodyKinematics>(player).unwrap().pos;
    assert!(
        pos.x > 550.0,
        "transfers to the yellow portal without a gun, got {pos:?}"
    );
}

#[test]
fn transit_is_gradual_centroid_crossing_flags_the_teleport_then_clears() {
    // Drain the BodyTeleported messages each frame into a flag so the test can
    // assert "did the player teleport THIS frame" without juggling the
    // double-buffered message store.
    #[derive(Resource, Default)]
    struct TeleportedThisFrame(bool);
    #[derive(Resource, Default)]
    struct TrailBreakThisFrame(bool);
    fn record_teleport(
        mut flag: ResMut<TeleportedThisFrame>,
        mut trail_flag: ResMut<TrailBreakThisFrame>,
        mut reader: MessageReader<BodyTeleported>,
        mut trail_reader: MessageReader<ambition_actors::player::trail::TrailContinuityBreak>,
    ) {
        flag.0 = reader.read().next().is_some();
        trail_flag.0 = trail_reader.read().next().is_some();
    }

    let mut app = App::new();
    app.add_message::<ambition_portal::PortalBodyEntered>();
    app.add_message::<BodyTeleported>();
    app.add_message::<ambition_actors::player::trail::TrailContinuityBreak>();
    app.add_message::<ambition_portal::PortalBodyTransited>();
    app.init_resource::<TeleportedThisFrame>();
    app.init_resource::<TrailBreakThisFrame>();
    app.insert_resource(ambition_time::WorldTime::default());
    app.init_resource::<ambition_portal::PortalTuning>();
    // The player-input adapter now emits `BodyTeleported` from the core's
    // `PortalBodyTransited` event (the trace bit moved out of core), so include
    // it in the chain ahead of the recorder.
    app.add_systems(
        Update,
        (
            portal_transit,
            crate::portal::portal_player_input_adapter,
            record_teleport,
        )
            .chain(),
    );
    // Two FLOOR portals (normal up): blue at x=200, orange at x=600.
    app.world_mut().spawn(PlacedPortal::fixed(
        BLUE,
        Vec2::new(200.0, 300.0),
        Vec2::new(0.0, -1.0),
        portal_half_extent(Vec2::new(0.0, -1.0)),
    ));
    app.world_mut().spawn(PlacedPortal::fixed(
        ORANGE,
        Vec2::new(600.0, 300.0),
        Vec2::new(0.0, -1.0),
        portal_half_extent(Vec2::new(0.0, -1.0)),
    ));
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
    assert!(
        !app.world().resource::<TeleportedThisFrame>().0,
        "no teleport message yet"
    );

    // Push the centroid across the plane (as the integrator would as the body
    // sinks into the carved opening).
    app.world_mut()
        .get_mut::<BodyKinematics>(player)
        .unwrap()
        .pos
        .y = 305.0;
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
    assert!(
        app.world().resource::<TeleportedThisFrame>().0,
        "the centroid transfer emits BodyTeleported (suppresses the trace auto-dump)"
    );
    assert!(
        app.world().resource::<TrailBreakThisFrame>().0,
        "the centroid transfer emits a neutral trail continuity break"
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
    assert!(
        !app.world().resource::<TeleportedThisFrame>().0,
        "the teleport message is a single frame"
    );
    assert!(
        !app.world().resource::<TrailBreakThisFrame>().0,
        "the trail continuity break is a single frame"
    );
}

/// The two host-side bridges the presentation chain needs, restated locally.
///
/// They live in `ambition_host::portal` — a crate ABOVE this one, so a content
/// test cannot call them. (They were once reachable as `ambition_portal::*`,
/// which is where this test imported them from; the E-track carve moved them and
/// never updated this `portal_render`-gated test, so it had not compiled since.
/// Fixed while landing R3.) Each is a five-line field copy / marker insert, and
/// the host owns testing them; what THIS test exercises is
/// `sync_portal_body_pieces`, which presentation owns.
#[cfg(feature = "portal_render")]
mod host_bridges {
    use super::*;
    use ambition_portal_presentation::{PortalSceneBody, PortalWorldFrame};
    use ambition_render::rendering::PlayerVisual;

    pub fn sync_portal_world_frame(world: Res<RoomGeometry>, mut frame: ResMut<PortalWorldFrame>) {
        if frame.size != world.0.size {
            frame.size = world.0.size;
        }
    }

    pub fn tag_portal_scene_bodies(
        mut commands: Commands,
        untagged: Query<Entity, (With<PlayerVisual>, Without<PortalSceneBody>)>,
    ) {
        for entity in &untagged {
            commands.entity(entity).insert(PortalSceneBody);
        }
    }
}

#[cfg(feature = "portal_render")]
#[test]
fn partial_render_keeps_the_sprite_and_adds_the_exit_copy() {
    use ambition_portal_presentation::{
        sync_portal_body_pieces, PortalBodyPiece, PortalWorldFrame,
    };
    use ambition_render::rendering::PlayerVisual;
    use host_bridges::{sync_portal_world_frame, tag_portal_scene_bodies};
    let mut app = App::new();
    app.insert_resource(world_with_two_walls());
    // Drive the visual through the same adapter chain the host runs: world-frame
    // sync + scene-body tagging bridge the sandbox types (RoomGeometry /
    // PlayerVisual) to the crate-owned seams the presentation system reads (auto
    // sync points flush the tag's commands between the chained systems).
    app.init_resource::<PortalWorldFrame>();
    // The body-pieces system reads the live effect selection (for the legacy
    // mask mode); default = first compiled effect.
    app.init_resource::<ambition_portal_presentation::PortalEffectSelection>();
    app.add_systems(
        Update,
        (
            sync_portal_world_frame,
            tag_portal_scene_bodies,
            sync_portal_body_pieces,
        )
            .chain(),
    );
    // Floor pair so a body standing on the blue portal straddles its plane.
    app.world_mut().spawn(PlacedPortal::fixed(
        BLUE,
        Vec2::new(200.0, 300.0),
        Vec2::new(0.0, -1.0),
        portal_half_extent(Vec2::new(0.0, -1.0)),
    ));
    app.world_mut().spawn(PlacedPortal::fixed(
        ORANGE,
        Vec2::new(300.0, 300.0),
        Vec2::new(0.0, -1.0),
        portal_half_extent(Vec2::new(0.0, -1.0)),
    ));
    // Body whose feet have dipped below the floor plane (y 275..315, plane 300).
    let player = app
        .world_mut()
        .spawn((
            PlayerVisual,
            BodyKinematics {
                pos: Vec2::new(200.0, 295.0),
                vel: Vec2::ZERO,
                size: Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            BodyBaseSize {
                base_size: Vec2::new(24.0, 40.0),
            },
            Sprite::from_color(Color::WHITE, Vec2::new(24.0, 40.0)),
            Visibility::Inherited,
            PortalTransit {
                straddling: BLUE,
                crossed: false,
            },
        ))
        .id();
    app.update();
    // The real sprite stays visible; the exit copy is additive (no masking).
    assert_eq!(
        *app.world().get::<Visibility>(player).unwrap(),
        Visibility::Inherited,
        "the real character sprite is NOT hidden"
    );
    // Exactly one transient piece now: the exit copy of the sprite. The opaque
    // "feet in, feet out" mask boxes were removed — the view windows show the
    // emerging slice and the copy overlays it.
    let pieces = {
        let mut q = app.world_mut().query::<&PortalBodyPiece>();
        q.iter(app.world()).count()
    };
    assert_eq!(pieces, 1, "exit sprite copy only, no masks");
}

#[test]
fn portal_carve_is_transient_and_pair_gated() {
    let mut app = App::new();
    // Carve output is now the portal-owned `PortalCarves` resource (Phase 2
    // Seam 1); the Ambition bridge copies it into the host overlay. Portal core
    // (and this core test) reads the portal-owned resource directly.
    app.init_resource::<ambition_portal::PortalCarves>();
    app.add_systems(Update, publish_portal_carves);
    // A lone portal must NOT carve (no exit → no bottomless hole).
    let blue = app
        .world_mut()
        .spawn(PlacedPortal::fixed(
            BLUE,
            Vec2::new(200.0, 300.0),
            Vec2::new(0.0, -1.0),
            portal_half_extent(Vec2::new(0.0, -1.0)),
        ))
        .id();
    app.update();
    assert!(
        app.world()
            .resource::<ambition_portal::PortalCarves>()
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
        portal_half_extent(Vec2::new(0.0, -1.0)),
    ));
    app.update();
    assert!(
        app.world()
            .resource::<ambition_portal::PortalCarves>()
            .holes
            .is_empty(),
        "a placed pair with no body transiting stays solid (no walk-in pocket)"
    );
    // A body transiting the blue portal carves EXACTLY that portal.
    let _ = blue;
    app.world_mut().spawn(PortalTransit {
        straddling: BLUE,
        crossed: false,
    });
    app.update();
    assert_eq!(
        app.world()
            .resource::<ambition_portal::PortalCarves>()
            .holes
            .len(),
        1,
        "only the portal a body is passing through is carved"
    );
}

#[test]
fn portal_shot_travels_and_opens_a_portal_on_a_wall() {
    let mut app = App::new();
    // `portal_projectile_step` (the Ambition shot adapter) still writes sfx;
    // `portal_fire_system` now emits the portal-owned `PortalShotFired` signal
    // (the FIRE/TRAVEL sfx moved to the `play_portal_sfx` adapter, Phase 5a).
    app.add_message::<ambition_sfx::SfxMessage>();
    app.add_message::<ambition_portal::PortalShotFired>();
    app.insert_resource(world_with_two_walls());
    app.insert_resource(ambition_time::WorldTime {
        raw_dt: 1.0 / 60.0,
        scaled_dt: 1.0 / 60.0,
    });
    app.insert_resource(ControlFrame::default());
    app.add_message::<FirePortalGun>();
    // The `FirePortalGun` gesture is resolved into the generic `PortalFireIntent`
    // by the Ambition resolver (Phase 2 Seam 3) before the core fire system reads
    // it; the RoomGeometry-reading shot stepper is the Ambition world-seam adapter
    // (Phase 2 Seam 2). Portal core keeps the pure `step_portal_shot` helper.
    app.add_message::<ambition_portal::PortalFireIntent>();
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
            With<ambition_actors::platformer_runtime::lifecycle::RoomScopedEntity>,
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
    use ambition_engine_core::body_clusters::BodyClusterScratch;
    use ambition_engine_core::movement::{
        update_player_with_tuning_scratch, InputState, DEFAULT_TUNING,
    };
    use ambition_portal::{transit_step, TransitStep};

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
            portal_half_extent(up),
        ),
        PlacedPortal::fixed(
            YELLOW,
            Vec2::new(554.0, floor_y),
            up,
            portal_half_extent(up),
        ),
    ];

    let mut scratch = BodyClusterScratch::new_with_abilities(
        Vec2::new(254.0, 700.0),
        ambition_engine_core::AbilitySet::default(),
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
        update_player_with_tuning_scratch(
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
        let step = transit_step(
            scratch.kinematics.pos,
            size,
            scratch.kinematics.vel,
            transit,
            cooldown.map(|(c, _)| c),
            &portals,
            Vec2::new(0.0, 1.0),
        );
        match step {
            TransitStep::Begin { channel, .. } => {
                transit = Some(PortalTransit {
                    straddling: channel,
                    crossed: false,
                });
            }
            TransitStep::Transfer {
                pos,
                vel,
                exit_channel,
                ..
            } => {
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
    use ambition_engine_core::body_clusters::BodyClusterScratch;
    use ambition_engine_core::movement::{update_player_with_tuning_scratch, InputState};
    use ambition_portal::{transit_step, TransitStep};

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
            portal_half_extent(up),
        ),
        PlacedPortal::fixed(
            YELLOW,
            Vec2::new(554.0, floor_y),
            up,
            portal_half_extent(up),
        ),
    ];

    let mut scratch = BodyClusterScratch::new_with_abilities(
        Vec2::new(254.0, floor_y - drop),
        ambition_engine_core::AbilitySet::default(),
    );
    scratch.ground.on_ground = false;
    let size = Vec2::new(24.0, 40.0);
    let dt = 1.0 / 60.0;

    let mut transit: Option<PortalTransit> = None;
    let mut transferred = false;
    let mut apex_y = f32::INFINITY;
    for _ in 0..4000 {
        update_player_with_tuning_scratch(&world, &mut scratch, InputState::default(), dt, tuning);
        let step = transit_step(
            scratch.kinematics.pos,
            size,
            scratch.kinematics.vel,
            transit,
            None,
            &portals,
            Vec2::new(0.0, 1.0),
        );
        match step {
            TransitStep::Begin { channel, .. } => {
                transit = Some(PortalTransit {
                    straddling: channel,
                    crossed: false,
                });
            }
            TransitStep::Transfer {
                pos,
                vel,
                exit_channel,
                ..
            } => {
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
    use ambition_engine_core::movement::DEFAULT_TUNING;
    let drop = 400.0; // entry ≈ 1342 px/s < DEFAULT max_fall_speed 1900
    let apex = floor_floor_round_trip_apex(drop, DEFAULT_TUNING);
    assert!(
        (apex - drop).abs() <= drop * 0.05,
        "sub-terminal round trip must conserve height: dropped {drop}, rebounded {apex}"
    );
}

/// Energy round trip WITHOUT a terminal velocity: with `max_fall_speed`
/// effectively unbounded, a fall from ANY height returns to that height.
/// This pins the fix for Jon's "fall from a tall distance, don't come all the
/// way back up" — the loss was never the portal (the transfer is an isometry);
/// it was the fall cap turning the down-leg into a drag zone. Per the Round 5
/// carried-momentum principle (the WORLD has no air drag; assists belong to
/// the controller), world-imparted fall speed is conserved when the cap is
/// out of the way — the player's shipped tuning keeps it out of the way.
#[test]
fn floor_floor_round_trip_conserves_any_height_without_terminal_velocity() {
    use ambition_engine_core::movement::DEFAULT_TUNING;
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
    use ambition_engine_core::movement::DEFAULT_TUNING;
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
    use ambition_actors::world::platforms::MovingPlatformState;
    use ambition_world::collision::MovingPlatformSet;

    let mut app = App::new();
    // Authored base: one anon fixture wall (unattributable on purpose).
    app.insert_resource(RoomGeometry(ae::World::new(
        "cc6",
        Vec2::new(2000.0, 1000.0),
        Vec2::ZERO,
        vec![ae::Block::solid(
            "anon-wall",
            Vec2::new(0.0, 0.0),
            Vec2::new(20.0, 400.0),
        )],
    )));
    // One moving platform sweeping +x at 120 px/s — its collision block
    // carries the placement GeoId + per-tick displacement.
    let mut platform = MovingPlatformState::from_authored(
        Vec2::new(400.0, 500.0),
        Vec2::new(120.0, 20.0),
        400.0,
        120.0,
    );
    platform.update(1.0 / 60.0); // publish last_delta (2px)
    app.insert_resource(MovingPlatformSet(vec![platform]));
    app.insert_resource(ambition_time::WorldTime {
        scaled_dt: 1.0 / 60.0,
        ..Default::default()
    });
    app.add_systems(
        Update,
        (attach_portal_hosts, refresh_hosted_portal_frames).chain(),
    );

    // A floor portal ON the platform's top face (placed 2px proud, like the
    // gun does), and a second portal on the anon wall.
    let plat_top = app.world().resource::<MovingPlatformSet>().0[0].pos.y
        - app.world().resource::<MovingPlatformSet>().0[0].size.y * 0.5;
    let plat_x = app.world().resource::<MovingPlatformSet>().0[0].pos.x;
    let hosted = app
        .world_mut()
        .spawn(PlacedPortal::fixed(
            PURPLE,
            Vec2::new(plat_x, plat_top - 2.0),
            Vec2::new(0.0, -1.0),
            portal_half_extent(Vec2::new(0.0, -1.0)),
        ))
        .id();
    let unhosted = app
        .world_mut()
        .spawn(PlacedPortal::fixed(
            YELLOW,
            Vec2::new(22.0, 200.0),
            Vec2::new(1.0, 0.0),
            portal_half_extent(Vec2::new(1.0, 0.0)),
        ))
        .id();

    app.update(); // attach + first refresh

    let p = app.world().get::<PlacedPortal>(hosted).unwrap().clone();
    assert!(p.host.is_some(), "portal on identified platform attaches");
    assert!(
        (p.host_lift - 2.0).abs() < 1e-3,
        "authored lift preserved, got {}",
        p.host_lift
    );
    let u = app.world().get::<PlacedPortal>(unhosted).unwrap().clone();
    assert!(u.host.is_none(), "anon fixture wall cannot host");
    assert_eq!(u.pos, Vec2::new(22.0, 200.0));
    assert_eq!(u.frame_delta(), Vec2::ZERO);

    // Advance the platform one frame (+2px at 120 px/s / 60fps) and refresh:
    // the aperture rides, its frame delta and px/s velocity match the host.
    let before = p.pos;
    {
        let mut set = app.world_mut().resource_mut::<MovingPlatformSet>();
        set.0[0].update(1.0 / 60.0);
    }
    app.update();
    let p = app.world().get::<PlacedPortal>(hosted).unwrap().clone();
    let ridden = p.pos - before;
    assert!(
        (ridden.x - 2.0).abs() < 1e-3 && ridden.y.abs() < 1e-3,
        "aperture rides the host displacement, moved {ridden:?}"
    );
    assert_eq!(p.frame_delta(), ridden, "the aperture's own sweep sample");
    assert!(
        (p.vel.x - 120.0).abs() < 1e-3,
        "px/s velocity derived from the host block, got {:?}",
        p.vel
    );

    // The host face vanishing closes the portal.
    app.world_mut()
        .resource_mut::<MovingPlatformSet>()
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
