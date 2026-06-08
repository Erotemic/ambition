//! Portal mechanic tests. These were previously inline in the monolithic portal
//! module; they exercise the public portal surface through the facade.

use bevy::prelude::*;

use crate::brain::ActionSet;
use crate::engine_core::{self as ae};
use crate::input::ControlFrame;
use crate::platformer_runtime::gravity::{gravity_upright_angle, GravityField};
use crate::platformer_runtime::orientation::{update_actor_roll, ActorRoll};
use crate::platformer_runtime::transit::rotate_velocity_between_normals as portal_transform_velocity;
use crate::player::{BodyKinematics, PlayerBaseSize, PlayerEntity, PrimaryPlayer};
use crate::GameWorld;

use super::types::MIN_EXIT_SPEED;
use super::*;
use crate::platformer_runtime::collision::raycast_solids;

// Channel shorthands for the tests: the gun's pair (Blue/Orange) and two authored
// pairs (Purple/Yellow). These map the old `PortalColor::X` literals onto the new
// `PortalChannel` so the tests still pin byte-identical pairing/transit behavior.
const BLUE: PortalChannel = PortalChannel::Gun(PortalGunColor::Blue);
const ORANGE: PortalChannel = PortalChannel::Gun(PortalGunColor::Orange);
const PURPLE: PortalChannel = PortalChannel::Authored(PortalChannelColor::Purple);
const YELLOW: PortalChannel = PortalChannel::Authored(PortalChannelColor::Yellow);

fn world_with_two_walls() -> GameWorld {
    // Left wall x[0,20], right wall x[380,400], both y[0,400].
    let blocks = vec![
        ae::Block::solid("left", Vec2::new(0.0, 0.0), Vec2::new(20.0, 400.0)),
        ae::Block::solid("right", Vec2::new(380.0, 0.0), Vec2::new(20.0, 400.0)),
    ];
    GameWorld(ae::World::new(
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
            PlayerBaseSize {
                base_size: Vec2::new(24.0, 40.0),
            },
            PortalGun::default(),
            ActionSet::default(),
            // Opt the player into the generic transit core with the player
            // policy (re-orient + carry velocity), as the Ambition tagging
            // adapter does in the real app.
            crate::portal::PortalBody,
            crate::portal::PortalPolicy {
                reorient: true,
                carry_velocity: true,
            },
        ))
        .id()
}

fn find_portal(app: &mut App, channel: PortalChannel) -> Option<PlacedPortal> {
    let mut q = app.world_mut().query::<&PlacedPortal>();
    let world = app.world();
    q.iter(world).find(|p| p.channel == channel).copied()
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
    use crate::engine_core::world::{Block, World};
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
        PlacedPortal {
            channel: BLUE,
            pos: Vec2::new(200.0, 380.0),
            normal: Vec2::new(0.0, -1.0),
            half_extent: portal_half_extent(Vec2::new(0.0, -1.0)),
        },
        PlacedPortal {
            channel: ORANGE,
            pos: Vec2::new(380.0, 200.0),
            normal: Vec2::new(-1.0, 0.0),
            half_extent: portal_half_extent(Vec2::new(-1.0, 0.0)),
        },
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
    use crate::ambition_content::portal::{
        sync_ground_items_to_transitable, sync_transitable_to_ground_items,
    };
    use crate::items::pickup::GroundItem;
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
    app.world_mut().spawn(PlacedPortal {
        channel: BLUE,
        pos: Vec2::new(20.0, 200.0),
        normal: Vec2::new(1.0, 0.0),
        half_extent: portal_half_extent(Vec2::new(1.0, 0.0)),
    });
    app.world_mut().spawn(PlacedPortal {
        channel: ORANGE,
        pos: Vec2::new(380.0, 200.0),
        normal: Vec2::new(-1.0, 0.0),
        half_extent: portal_half_extent(Vec2::new(1.0, 0.0)),
    });
    // A thrown item flying into the blue portal.
    let item = app
        .world_mut()
        .spawn(GroundItem {
            spec: crate::items::pickup::axe_spec(),
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
    let wall = PlacedPortal {
        channel: BLUE,
        pos: Vec2::ZERO,
        normal: Vec2::new(1.0, 0.0),
        half_extent: portal_half_extent(Vec2::new(1.0, 0.0)),
    };
    // The opening is the SAME size in every orientation (2*46=92). A wall
    // portal gates on HEIGHT: a short actor fits, a 200-tall boss does not.
    assert!(portal_fits(Vec2::new(24.0, 40.0), &wall));
    assert!(!portal_fits(Vec2::new(80.0, 200.0), &wall));
    // A floor portal gates on WIDTH — same 92 opening, so the threshold
    // matches the wall's.
    let floor = PlacedPortal {
        channel: ORANGE,
        pos: Vec2::ZERO,
        normal: Vec2::new(0.0, -1.0),
        half_extent: portal_half_extent(Vec2::new(0.0, -1.0)),
    };
    assert!(portal_fits(Vec2::new(40.0, 200.0), &floor));
    assert!(!portal_fits(Vec2::new(100.0, 20.0), &floor));
}

#[test]
fn portals_teleport_a_fitting_actor_and_skip_an_oversized_one() {
    use crate::features::BodyKinematics;
    let mut app = App::new();
    app.add_message::<ambition_sfx::SfxMessage>();
    app.add_message::<crate::portal::PortalBodyTransited>();
    app.add_systems(Update, portal_transit);
    // Actor policy: carry velocity, no re-orient (facing follows AI).
    let actor_policy = crate::portal::PortalPolicy {
        reorient: false,
        carry_velocity: true,
    };
    app.world_mut().spawn(PlacedPortal {
        channel: BLUE,
        pos: Vec2::new(20.0, 200.0),
        normal: Vec2::new(1.0, 0.0),
        half_extent: portal_half_extent(Vec2::new(1.0, 0.0)),
    });
    app.world_mut().spawn(PlacedPortal {
        channel: ORANGE,
        pos: Vec2::new(380.0, 200.0),
        normal: Vec2::new(-1.0, 0.0),
        half_extent: portal_half_extent(Vec2::new(1.0, 0.0)),
    });
    let small = app
        .world_mut()
        .spawn((
            BodyKinematics {
                pos: Vec2::new(20.0, 200.0),
                vel: Vec2::new(-100.0, 0.0),
                size: Vec2::new(24.0, 40.0),
                facing: -1.0,
            },
            crate::portal::PortalBody,
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
            crate::portal::PortalBody,
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
    let floor = |channel, x: f32| PlacedPortal {
        channel,
        pos: Vec2::new(x, 300.0),
        normal: Vec2::new(0.0, -1.0),
        half_extent: he,
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
        0.0,
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
fn facing_flips_only_for_a_same_wall_turn_around() {
    let g = Vec2::new(0.0, 1.0); // gravity down
    let up = Vec2::new(0.0, -1.0); // floor
    let down = Vec2::new(0.0, 1.0); // ceiling
    let left = Vec2::new(-1.0, 0.0); // right-wall normal
    let right = Vec2::new(1.0, 0.0); // left-wall normal
                                     // Same wall (both normals left) is the only "face in, back out" case →
                                     // facing mirrors so it comes out face-first.
    assert!(portal_facing_flips(left, left, g));
    // Walls facing EACH OTHER (portal_bridge) go straight through → no flip.
    assert!(!portal_facing_flips(right, left, g));
    // Floor/ceiling pairs carry orientation in the somersault rotation → no
    // separate facing flip (the 180° rotation already mirrors).
    assert!(!portal_facing_flips(up, up, g));
    assert!(!portal_facing_flips(down, down, g));
    assert!(!portal_facing_flips(up, left, g));
}

#[test]
fn somersault_is_suppressed_for_a_wall_to_wall_turn_around() {
    use std::f32::consts::PI;
    let g = Vec2::new(0.0, 1.0); // gravity down
    let up = Vec2::new(0.0, -1.0); // floor normal
    let down = Vec2::new(0.0, 1.0); // ceiling normal
    let left = Vec2::new(-1.0, 0.0); // right-wall normal
                                     // Floor↔floor and ceiling↔ceiling KEEP the 180° tumble (feet-in → reorient).
    assert!((somersault_roll(up, up, g).abs() - PI).abs() < 1e-5);
    assert!((somersault_roll(down, down, g).abs() - PI).abs() < 1e-5);
    // Two portals on the SAME wall (both normals horizontal) impart NO tumble —
    // the body just turns around and comes out upright.
    assert!(somersault_roll(left, left, g).abs() < 1e-5);
    // A floor→wall pair still tumbles 90° (it genuinely reorients).
    assert!(somersault_roll(up, left, g).abs() > 1.0);
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
    app.insert_resource(ambition_platformer_runtime::time::SimDt { dt: 1.0 / 60.0 });
    app.init_resource::<GravityField>();
    app.add_systems(Update, update_actor_roll);
    // Start rolled 180° (just exited a floor↔floor portal), airborne. The
    // righting system reads each body's own position via BodyKinematics, so the
    // test body carries one (the dual-arm query collapsed to a single
    // With<BodyKinematics> in Stage 16 / S5).
    let player = app
        .world_mut()
        .spawn((
            ambition_platformer_runtime::body::BodyKinematics {
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
    use crate::features::BodyKinematics;
    let mut app = App::new();
    app.add_message::<ambition_sfx::SfxMessage>();
    app.add_message::<crate::portal::PortalBodyTransited>();
    app.add_systems(Update, portal_transit);
    // Floor portal (normal up) + right-wall portal (normal left): a
    // floor→wall pair, so transit imparts a -90° roll. Player and non-player
    // actors alike now tumble + reorient (the somersault is ported to the
    // aperture model and applied on the centroid transfer).
    app.world_mut().spawn(PlacedPortal {
        channel: BLUE,
        pos: Vec2::new(200.0, 380.0),
        normal: Vec2::new(0.0, -1.0),
        half_extent: portal_half_extent(Vec2::new(0.0, -1.0)),
    });
    app.world_mut().spawn(PlacedPortal {
        channel: ORANGE,
        pos: Vec2::new(380.0, 200.0),
        normal: Vec2::new(-1.0, 0.0),
        half_extent: portal_half_extent(Vec2::new(-1.0, 0.0)),
    });
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
            crate::portal::PortalBody,
            crate::portal::PortalPolicy {
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
    app.add_message::<ambition_sfx::SfxMessage>();
    app.add_message::<BodyTeleported>();
    app.add_message::<crate::portal::PortalBodyTransited>();
    app.insert_resource(crate::WorldTime::default());
    app.add_systems(Update, portal_transit);
    // Blue on the left (facing right), orange on the right (facing left).
    app.world_mut().spawn(PlacedPortal {
        channel: BLUE,
        pos: Vec2::new(22.0, 200.0),
        normal: Vec2::new(1.0, 0.0),
        half_extent: portal_half_extent(Vec2::new(1.0, 0.0)),
    });
    app.world_mut().spawn(PlacedPortal {
        channel: ORANGE,
        pos: Vec2::new(378.0, 200.0),
        normal: Vec2::new(-1.0, 0.0),
        half_extent: portal_half_extent(Vec2::new(1.0, 0.0)),
    });
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
fn portal_input_warp_transforms_held_input_then_clears() {
    use crate::ambition_content::portal::{
        apply_movement_intent_to_control, sync_movement_intent_from_control,
    };
    let mut app = App::new();
    app.insert_resource(ControlFrame::default());
    app.init_resource::<PlayerMovementIntent>();
    // The content adapter brackets the core warp: mirror ControlFrame -> intent
    // before the warp, and the warped intent -> ControlFrame after, so this
    // exercises the full content+core chain on the ControlFrame surface exactly
    // as the game does.
    app.add_systems(
        Update,
        (
            sync_movement_intent_from_control,
            warp_portal_input,
            apply_movement_intent_to_control,
        )
            .chain(),
    );
    // A 180° warp (a same-wall pair). Player holds RIGHT (anchor right).
    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            PortalInputWarp {
                n_in: Vec2::new(-1.0, 0.0),
                n_out: Vec2::new(-1.0, 0.0),
                anchor: Vec2::new(1.0, 0.0),
            },
        ))
        .id();

    // Still holding right → input is warped to LEFT (keeps you moving out).
    app.world_mut().resource_mut::<ControlFrame>().axis_x = 1.0;
    app.update();
    assert!(
        app.world().resource::<ControlFrame>().axis_x < -0.5,
        "held right is warped to left while the warp is active"
    );
    assert!(
        app.world().get::<PortalInputWarp>(player).is_some(),
        "warp persists while held"
    );

    // Release movement → warp drops, input passes through untouched next frame.
    app.world_mut().resource_mut::<ControlFrame>().axis_x = 0.0;
    app.update();
    assert!(
        app.world().get::<PortalInputWarp>(player).is_none(),
        "release drops the warp"
    );

    // Re-arm, then press a clearly different direction (left) → warp drops.
    app.world_mut().entity_mut(player).insert(PortalInputWarp {
        n_in: Vec2::new(-1.0, 0.0),
        n_out: Vec2::new(-1.0, 0.0),
        anchor: Vec2::new(1.0, 0.0),
    });
    app.world_mut().resource_mut::<ControlFrame>().axis_x = -1.0;
    app.update();
    assert!(
        app.world().get::<PortalInputWarp>(player).is_none(),
        "a clearly different direction drops the warp"
    );
}

#[test]
fn wall_ability_suppression_reapplies_every_frame_against_the_loadout_reset() {
    use crate::player::PlayerAbilities;
    let mut app = App::new();
    app.init_resource::<SuppressWallAbilitiesInPortal>();
    // Stand in for the per-frame loadout reset that clobbered the old
    // save-once suppression: re-enable ledge_grab BEFORE the suppressor runs.
    fn reenable_ledge_grab(mut q: Query<&mut PlayerAbilities>) {
        for mut a in &mut q {
            a.abilities.ledge_grab = true;
        }
    }
    app.add_systems(
        Update,
        (reenable_ledge_grab, suppress_ledge_grab_during_transit).chain(),
    );
    let player = app
        .world_mut()
        .spawn((PlayerEntity, PrimaryPlayer, PlayerAbilities::default()))
        .id();
    app.world_mut()
        .get_mut::<PlayerAbilities>(player)
        .unwrap()
        .abilities
        .ledge_grab = true;

    // Not transiting: the reset wins, ledge_grab stays enabled.
    app.update();
    assert!(
        app.world()
            .get::<PlayerAbilities>(player)
            .unwrap()
            .abilities
            .ledge_grab
    );

    // Transiting: even though the reset re-enables it first, the suppressor
    // re-applies every frame, so it stays disabled across MANY frames.
    app.world_mut().entity_mut(player).insert(PortalTransit {
        straddling: BLUE,
        crossed: false,
    });
    for _ in 0..5 {
        app.update();
        assert!(
            !app.world()
                .get::<PlayerAbilities>(player)
                .unwrap()
                .abilities
                .ledge_grab,
            "ledge_grab must stay suppressed every frame while transiting"
        );
    }

    // Transit ends: the per-frame reset restores it (no save/restore needed).
    app.world_mut().entity_mut(player).remove::<PortalTransit>();
    app.update();
    assert!(
        app.world()
            .get::<PlayerAbilities>(player)
            .unwrap()
            .abilities
            .ledge_grab
    );
}

#[test]
fn emission_guard_strips_input_pushing_back_into_the_exit_wall() {
    use crate::ambition_content::portal::{
        apply_movement_intent_to_control, sync_movement_intent_from_control,
    };
    let mut app = App::new();
    app.insert_resource(ControlFrame::default());
    app.init_resource::<PlayerMovementIntent>();
    app.add_systems(
        Update,
        (
            sync_movement_intent_from_control,
            warp_portal_input,
            apply_movement_intent_to_control,
        )
            .chain(),
    );
    // Emerging from a right-wall portal — exit_normal points LEFT (into room).
    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            PortalEmission {
                exit_normal: Vec2::new(-1.0, 0.0),
                timer: 1.0,
            },
        ))
        .id();
    // Holding RIGHT (back into the wall) is stripped so physics carries you out.
    app.world_mut().resource_mut::<ControlFrame>().axis_x = 1.0;
    app.update();
    assert!(
        app.world().resource::<ControlFrame>().axis_x.abs() < 0.01,
        "input pushing back into the exit wall is stripped during emergence"
    );
    // Holding LEFT (the emergence direction) passes through untouched.
    app.world_mut().resource_mut::<ControlFrame>().axis_x = -1.0;
    app.update();
    assert!(
        app.world().resource::<ControlFrame>().axis_x < -0.5,
        "input in the emergence direction is preserved"
    );
    let _ = player;
}

#[test]
fn a_gunless_player_transits_an_authored_pair() {
    // The portal_lab scenario: pre-placed portals, player has NOT picked up
    // the gun. Transit must still work — crossing a placed pair is independent
    // of holding the gun, and the cooldown lives on the body.
    let mut app = App::new();
    app.add_message::<ambition_sfx::SfxMessage>();
    app.add_message::<BodyTeleported>();
    app.add_message::<crate::portal::PortalBodyTransited>();
    app.insert_resource(crate::WorldTime::default());
    app.add_systems(Update, portal_transit);
    let he = portal_half_extent(Vec2::new(0.0, -1.0));
    app.world_mut().spawn(PlacedPortal {
        channel: PURPLE,
        pos: Vec2::new(200.0, 300.0),
        normal: Vec2::new(0.0, -1.0),
        half_extent: he,
    });
    app.world_mut().spawn(PlacedPortal {
        channel: YELLOW,
        pos: Vec2::new(600.0, 300.0),
        normal: Vec2::new(0.0, -1.0),
        half_extent: he,
    });
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
            PlayerBaseSize {
                base_size: Vec2::new(24.0, 40.0),
            },
            // No PortalGun on purpose.
            crate::portal::PortalBody,
            crate::portal::PortalPolicy {
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
    fn record_teleport(
        mut flag: ResMut<TeleportedThisFrame>,
        mut reader: MessageReader<BodyTeleported>,
    ) {
        flag.0 = reader.read().next().is_some();
    }

    let mut app = App::new();
    app.add_message::<ambition_sfx::SfxMessage>();
    app.add_message::<BodyTeleported>();
    app.add_message::<crate::portal::PortalBodyTransited>();
    app.init_resource::<TeleportedThisFrame>();
    app.insert_resource(crate::WorldTime::default());
    // The player-input adapter now emits `BodyTeleported` from the core's
    // `PortalBodyTransited` event (the trace bit moved out of core), so include
    // it in the chain ahead of the recorder.
    app.add_systems(
        Update,
        (
            portal_transit,
            crate::ambition_content::portal::portal_player_input_adapter,
            record_teleport,
        )
            .chain(),
    );
    // Two FLOOR portals (normal up): blue at x=200, orange at x=600.
    app.world_mut().spawn(PlacedPortal {
        channel: BLUE,
        pos: Vec2::new(200.0, 300.0),
        normal: Vec2::new(0.0, -1.0),
        half_extent: portal_half_extent(Vec2::new(0.0, -1.0)),
    });
    app.world_mut().spawn(PlacedPortal {
        channel: ORANGE,
        pos: Vec2::new(600.0, 300.0),
        normal: Vec2::new(0.0, -1.0),
        half_extent: portal_half_extent(Vec2::new(0.0, -1.0)),
    });
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
}

#[test]
fn partial_render_keeps_the_sprite_and_masks_the_through_slice() {
    use crate::presentation::rendering::PlayerVisual;
    let mut app = App::new();
    app.insert_resource(world_with_two_walls());
    app.add_systems(Update, sync_portal_body_pieces);
    // Floor pair so a body standing on the blue portal straddles its plane.
    app.world_mut().spawn(PlacedPortal {
        channel: BLUE,
        pos: Vec2::new(200.0, 300.0),
        normal: Vec2::new(0.0, -1.0),
        half_extent: portal_half_extent(Vec2::new(0.0, -1.0)),
    });
    app.world_mut().spawn(PlacedPortal {
        channel: ORANGE,
        pos: Vec2::new(300.0, 300.0),
        normal: Vec2::new(0.0, -1.0),
        half_extent: portal_half_extent(Vec2::new(0.0, -1.0)),
    });
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
            PlayerBaseSize {
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
    // The real sprite stays visible — only the through-wall slice is masked.
    assert_eq!(
        *app.world().get::<Visibility>(player).unwrap(),
        Visibility::Inherited,
        "the real character sprite is NOT hidden; the box masks the invisible part"
    );
    // An exit copy of the sprite + a mask over each invisible slice (entry
    // through-wall + exit not-yet-emerged) = three transient pieces.
    let pieces = {
        let mut q = app.world_mut().query::<&PortalBodyPiece>();
        q.iter(app.world()).count()
    };
    assert_eq!(pieces, 3, "exit sprite copy + entry mask + exit mask");
}

#[test]
fn portal_carve_is_transient_and_pair_gated() {
    let mut app = App::new();
    // Carve output is now the portal-owned `PortalCarves` resource (Phase 2
    // Seam 1); the Ambition bridge copies it into the host overlay. Portal core
    // (and this core test) reads the portal-owned resource directly.
    app.init_resource::<crate::portal::PortalCarves>();
    app.add_systems(Update, publish_portal_carves);
    // A lone portal must NOT carve (no exit → no bottomless hole).
    let blue = app
        .world_mut()
        .spawn(PlacedPortal {
            channel: BLUE,
            pos: Vec2::new(200.0, 300.0),
            normal: Vec2::new(0.0, -1.0),
            half_extent: portal_half_extent(Vec2::new(0.0, -1.0)),
        })
        .id();
    app.update();
    assert!(
        app.world()
            .resource::<crate::portal::PortalCarves>()
            .holes
            .is_empty(),
        "a lone portal does not carve"
    );
    // Complete the pair — but with NO body transiting, still nothing carves
    // (so you can't wiggle into a wall pocket between crossings).
    app.world_mut().spawn(PlacedPortal {
        channel: ORANGE,
        pos: Vec2::new(600.0, 300.0),
        normal: Vec2::new(0.0, -1.0),
        half_extent: portal_half_extent(Vec2::new(0.0, -1.0)),
    });
    app.update();
    assert!(
        app.world()
            .resource::<crate::portal::PortalCarves>()
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
            .resource::<crate::portal::PortalCarves>()
            .holes
            .len(),
        1,
        "only the portal a body is passing through is carved"
    );
}

#[test]
fn portal_shot_travels_and_opens_a_portal_on_a_wall() {
    let mut app = App::new();
    app.add_message::<ambition_sfx::SfxMessage>();
    app.insert_resource(world_with_two_walls());
    app.insert_resource(crate::WorldTime {
        raw_dt: 1.0 / 60.0,
        scaled_dt: 1.0 / 60.0,
    });
    app.insert_resource(ControlFrame::default());
    app.add_message::<FirePortalGun>();
    // The GameWorld-reading shot stepper is now the Ambition world-seam adapter
    // (Phase 2 Seam 2); portal core keeps only the pure `step_portal_shot` helper.
    app.add_systems(
        Update,
        (
            portal_fire_system,
            crate::ambition_content::portal::portal_projectile_step,
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
        blue.is_some_and(|p| p.pos.x < 60.0 && p.normal.x > 0.5),
        "the shot should open a blue portal on the left wall, got {blue:?}"
    );
    // The opened portal is room-scoped, so a room transition despawns it —
    // no lingering portals that reappear when you leave and come back (#41).
    let scoped = {
        let mut q = app.world_mut().query_filtered::<(), (
            With<PlacedPortal>,
            With<crate::platformer_runtime::lifecycle::RoomScopedEntity>,
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
