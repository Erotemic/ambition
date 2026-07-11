//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod simple_geometry_tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module (a direct
//! sibling, so `super` path depth is unchanged) with `use super::*;`.

use super::*;

fn geom(frame_down: ae::Vec2) -> SimpleActorGeometry {
    SimpleActorGeometry {
        pos: ae::Vec2::new(10.0, 20.0),
        size: ae::Vec2::new(30.0, 48.0),
        facing: 1.0,
        frame_down,
    }
}

#[test]
fn upright_gravity_is_the_plain_centered_box() {
    // Under screen-down gravity the oriented body box is identity — upright
    // play stays byte-for-byte the old `kin.aabb()` / CenteredAabb.
    let aabb = collision_aabb(&geom(ae::Vec2::new(0.0, 1.0)));
    assert_eq!(aabb.center(), ae::Vec2::new(10.0, 20.0));
    assert_eq!(aabb.half_size(), ae::Vec2::new(15.0, 24.0));
    // The single damageable volume agrees with the collision box.
    let vols = damageable_volumes(&geom(ae::Vec2::new(0.0, 1.0)));
    assert_eq!(vols.len(), 1);
    assert_eq!(vols[0].half_size(), ae::Vec2::new(15.0, 24.0));
}

#[test]
fn sideways_gravity_lays_the_body_along_the_wall() {
    // Under sideways gravity the footprint rotates: width<->height swap so
    // the body lies along the wall (the relativity principle). Same box the
    // gizmo's `aabb_oriented` draws.
    let aabb = collision_aabb(&geom(ae::Vec2::new(1.0, 0.0)));
    assert_eq!(aabb.center(), ae::Vec2::new(10.0, 20.0));
    assert_eq!(aabb.half_size(), ae::Vec2::new(24.0, 15.0));
}
