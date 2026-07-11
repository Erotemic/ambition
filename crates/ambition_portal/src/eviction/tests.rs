//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::color::{PortalChannelColor, PortalGunColor};
use crate::transit::PortalPolicy;
use crate::types::portal_half_extent;

fn floor_portal(channel: PortalChannel, pos: Vec2) -> PlacedPortal {
    PlacedPortal::fixed(
        channel,
        pos,
        Vec2::new(0.0, -1.0),
        portal_half_extent(Vec2::new(0.0, -1.0)),
    )
}

fn app() -> App {
    let mut app = App::new();
    app.init_resource::<PortalFrameHistory>();
    app.add_systems(Update, evict_straddlers_on_portal_change);
    app
}

fn straddling_body(app: &mut App, pos: Vec2) -> Entity {
    app.world_mut()
        .spawn((
            BodyKinematics {
                pos,
                vel: Vec2::ZERO,
                size: Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            PortalBody,
            PortalPolicy {
                reorient: true,
                carry_velocity: true,
            },
        ))
        .id()
}

const BLUE: PortalChannel = PortalChannel::Gun(PortalGunColor::BLUE);
const PURPLE: PortalChannel = PortalChannel::Authored(PortalChannelColor::Purple);

#[test]
fn vanished_portal_evicts_a_straddler_to_its_centroids_side() {
    let mut app = app();
    // Floor portal at y=300; body centered just ABOVE the plane (centroid
    // front, feet dipped through).
    let portal = app
        .world_mut()
        .spawn(floor_portal(BLUE, Vec2::new(100.0, 300.0)))
        .id();
    let body = straddling_body(&mut app, Vec2::new(100.0, 290.0));
    // Frame 1: history records the portal; body still straddles.
    app.update();
    assert!(pp::straddles(
        ae::Aabb::new(Vec2::new(100.0, 290.0), Vec2::new(12.0, 20.0)),
        &floor_portal(BLUE, Vec2::new(100.0, 300.0)).aperture()
    ));
    // Portal vanishes; frame 2 evicts the straddler upward (centroid front).
    app.world_mut().entity_mut(portal).despawn();
    app.update();
    let kin = app.world().get::<BodyKinematics>(body).unwrap();
    // Pushed UP (—y) fully clear of the old plane: bottom edge above y=300.
    assert!(
        kin.pos.y + kin.size.y * 0.5 <= 300.0 + 1e-3,
        "evicted clear: {:?}",
        kin.pos
    );
}

#[test]
fn teleported_portal_evicts_straddler_but_stable_portal_does_not() {
    let mut app = app();
    let portal = app
        .world_mut()
        .spawn(floor_portal(BLUE, Vec2::new(100.0, 300.0)))
        .id();
    let body = straddling_body(&mut app, Vec2::new(100.0, 290.0));
    // A different, stable channel a SECOND body straddles — must be left alone.
    app.world_mut()
        .spawn(floor_portal(PURPLE, Vec2::new(500.0, 300.0)));
    let stable_body = straddling_body(&mut app, Vec2::new(500.0, 290.0));
    app.update();
    // Teleport BLUE far away; PURPLE unchanged.
    app.world_mut()
        .entity_mut(portal)
        .insert(floor_portal(BLUE, Vec2::new(900.0, 300.0)));
    app.update();
    let moved = app.world().get::<BodyKinematics>(body).unwrap().pos;
    let stable = app.world().get::<BodyKinematics>(stable_body).unwrap().pos;
    assert!(
        moved.y < 290.0 - 1.0,
        "BLUE straddler evicted up: {moved:?}"
    );
    assert!(
        (stable.y - 290.0).abs() < 1e-3,
        "PURPLE straddler untouched: {stable:?}"
    );
}

/// CC6: a HOSTED aperture riding its face is the same portal in motion,
/// not a close — a straddling body must NOT be evicted (the dynamic
/// straddle re-evaluates; eviction stays a CLOSE-only pushout). A
/// teleport of the same channel still evicts (covered above).
#[test]
fn host_carried_motion_does_not_evict_a_straddler() {
    let mut app = app();
    let portal = app
        .world_mut()
        .spawn(floor_portal(BLUE, Vec2::new(100.0, 300.0)))
        .id();
    let body = straddling_body(&mut app, Vec2::new(100.0, 290.0));
    app.update(); // history primes

    // The host refresh carried the aperture up 8px this frame.
    {
        let mut p = app.world_mut().get_mut::<PlacedPortal>(portal).unwrap();
        p.host = Some(ambition_engine_core::GeoFaceRef::new(
            ambition_engine_core::GeoId::anon(),
            ambition_engine_core::Face::Top,
            0.0,
        ));
        p.prev_pos = p.pos;
        p.pos += Vec2::new(0.0, -8.0);
    }
    let before = app.world().get::<BodyKinematics>(body).unwrap().pos;
    app.update();
    let after = app.world().get::<BodyKinematics>(body).unwrap().pos;
    assert_eq!(before, after, "host-carried motion is not a close");
}
