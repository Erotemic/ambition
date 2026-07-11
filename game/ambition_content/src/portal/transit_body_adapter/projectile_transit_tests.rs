//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod projectile_transit_tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module (a direct
//! sibling, so `super` path depth is unchanged) with `use super::*;`.

//! Headless projectile-transit tests for the generic portal core plus the real
//! projectile adapter. A projectile near a pair should emerge with rotated
//! velocity; one nowhere near a portal should keep its straight-line path.

use bevy::prelude::*;

use ambition_portal::{
    portal_half_extent, portal_transit, PlacedPortal, PortalBody, PortalChannel, PortalGunColor,
};
use ambition_projectiles::ProjectileGameplay;

use super::ensure_projectile_portal_bodies;

const BLUE: PortalChannel = PortalChannel::Gun(PortalGunColor::BLUE);
const ORANGE: PortalChannel = PortalChannel::Gun(PortalGunColor::ORANGE);

use ambition_actors::platformer_runtime::body::BodyKinematics;

/// A straight-flying, gravity-free projectile gameplay half (Hadouken: no
/// bounce, no arc) so the test isolates the portal velocity rotation.
fn straight_projectile() -> ProjectileGameplay {
    ProjectileGameplay {
        age: 0.0,
        max_lifetime: 100.0,
        gravity: 0.0,
        damage: 1,
        bounces_remaining: 0,
        world_hit: ambition_projectiles::WorldHitPolicy::ExpireOnContact,
    }
}

/// Minimal app: projectile tagging adapter + generic transit core, wired
/// `ensure → transit` as in the real plugin.
fn app_with_transit() -> App {
    let mut app = App::new();
    app.add_message::<ambition_portal::PortalBodyEntered>();
    app.add_message::<ambition_portal::PortalBodyTransited>();
    app.init_resource::<ambition_portal::PortalTuning>();
    app.add_systems(
        Update,
        (ensure_projectile_portal_bodies, portal_transit).chain(),
    );
    app
}

/// Place a left-wall portal (normal +x) and a right-wall portal (normal -x),
/// the same pair the actor transit unit test uses.
fn place_wall_pair(app: &mut App) {
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
}

#[test]
fn projectile_fired_into_portal_a_emerges_from_portal_b_with_rotated_velocity() {
    let mut app = app_with_transit();
    place_wall_pair(&mut app);

    // A small projectile at the blue (left-wall) portal, flying INTO it
    // (-x, toward the +x-normal face). Speed 400 px/s — well above the
    // MIN_EXIT_SPEED floor (220) so the assertion measures the pure rotation.
    let proj = app
        .world_mut()
        .spawn((
            BodyKinematics {
                pos: Vec2::new(20.0, 200.0),
                vel: Vec2::new(-400.0, 0.0),
                size: Vec2::new(8.0, 8.0),
                facing: -1.0,
            },
            straight_projectile(),
        ))
        .id();

    // Frame 1 tags + begins (leading edge in the opening); frame 2 transfers
    // (the centroid is already on the plane) — same two-frame aperture cadence
    // the actor transit test relies on.
    app.update();
    // The adapter must have opted the projectile in.
    assert!(
        app.world().get::<PortalBody>(proj).is_some(),
        "ensure_projectile_portal_bodies must tag the projectile PortalBody",
    );
    app.update();

    let kin = app.world().get::<BodyKinematics>(proj).unwrap();
    // Emerged from the orange portal (x=380, normal -x): a transited body pops
    // out clear of the exit, so it sits just inside the room on the far side.
    assert!(
        kin.pos.x > 300.0,
        "projectile should emerge from the orange portal on the far side, pos={:?}",
        kin.pos,
    );
    // Velocity rotated by the pair transform: the body emerges travelling ALONG
    // the EXIT normal (orange faces -x, into the room), so it flies out of B and
    // keeps going — exactly the demo claim. (Entry normal +x → exit normal -x is
    // the wall↔wall 180° map, which reverses the horizontal component.)
    assert!(
        kin.vel.x < 0.0,
        "exit velocity must be rotated to travel along the orange normal (-x), vel={:?}",
        kin.vel,
    );
    // Speed preserved by the rotation (400 px/s is above the MIN_EXIT_SPEED
    // floor, so no flooring masks it).
    assert!(
        (kin.vel.length() - 400.0).abs() < 1.0,
        "the rotation preserves speed (~400 px/s), got {:?}",
        kin.vel,
    );
    // It KEEPS flying past B (not stalled in the aperture): the exit speed is
    // well above zero along the emergence direction.
    assert!(
        kin.vel.length() > 100.0,
        "the projectile keeps flying out of B, vel={:?}",
        kin.vel,
    );
    // No re-orientation: facing is unchanged (reorient:false for projectiles).
    assert_eq!(
        kin.facing, -1.0,
        "a projectile is not re-oriented by transit (reorient:false), facing={}",
        kin.facing,
    );
}

#[test]
fn projectile_nowhere_near_a_portal_flies_straight_through() {
    // No-regression guard: with a portal pair placed but the projectile far
    // from both, transit is a pure no-op and the body keeps its velocity.
    let mut app = app_with_transit();
    place_wall_pair(&mut app);

    let proj = app
        .world_mut()
        .spawn((
            BodyKinematics {
                pos: Vec2::new(200.0, 50.0), // far from both wall portals
                vel: Vec2::new(150.0, 0.0),
                size: Vec2::new(8.0, 8.0),
                facing: 1.0,
            },
            straight_projectile(),
        ))
        .id();

    app.update();
    app.update();

    let kin = app.world().get::<BodyKinematics>(proj).unwrap();
    // transit_step → Idle: velocity untouched (portal_transit does NOT
    // integrate motion — that is the Combat-set step system's job — so the
    // body stays exactly where it was spawned with its velocity intact).
    assert_eq!(
        kin.vel,
        Vec2::new(150.0, 0.0),
        "a projectile away from any portal must not be touched by transit, vel={:?}",
        kin.vel,
    );
    assert!(
        app.world()
            .get::<ambition_portal::PortalTransit>(proj)
            .is_none(),
        "no PortalTransit latch should be set for a projectile away from portals",
    );
}
