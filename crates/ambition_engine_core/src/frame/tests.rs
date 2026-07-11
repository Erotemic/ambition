//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;

const UP: Vec2 = Vec2::new(0.0, -1.0); // y-down world: a floor's outward normal
const RIGHT: Vec2 = Vec2::new(1.0, 0.0);

fn floor_at(x: f32, y: f32) -> PortalFrame {
    PortalFrame::fixed(Vec2::new(x, y), UP)
}

#[test]
fn tangent_handedness_is_rot90_ccw() {
    // Pinned: floor (up-normal in y-down) → tangent +x is rot90 of (0,-1)
    // = (1, 0); right-wall normal (1,0) → (0,1) etc.
    assert_eq!(tangent_of(Vec2::new(0.0, -1.0)), Vec2::new(1.0, 0.0));
    assert_eq!(tangent_of(Vec2::new(1.0, 0.0)), Vec2::new(0.0, 1.0));
    assert_eq!(tangent_of(Vec2::new(0.0, 1.0)), Vec2::new(-1.0, 0.0));
    assert_eq!(tangent_of(Vec2::new(-1.0, 0.0)), Vec2::new(0.0, -1.0));
}

#[test]
fn local_roundtrip_is_exact_for_cardinals_and_tight_for_angles() {
    let f = floor_at(100.0, 300.0);
    let p = Vec2::new(112.5, 297.25);
    assert_eq!(f.from_local(f.to_local(p)), p);

    let angled = PortalFrame::fixed(
        Vec2::new(40.0, 8.0),
        Vec2::new(
            std::f32::consts::FRAC_1_SQRT_2,
            -std::f32::consts::FRAC_1_SQRT_2,
        ),
    );
    let q = Vec2::new(37.0, 15.0);
    let rt = angled.from_local(angled.to_local(q));
    assert!(
        (rt - q).length() < 1e-4,
        "roundtrip drifted: {rt:?} vs {q:?}"
    );
}

#[test]
fn front_is_positive_on_the_room_side() {
    let f = floor_at(100.0, 300.0);
    // Above the floor (smaller y in y-down) is the room side.
    assert!(f.to_local(Vec2::new(100.0, 290.0)).y > 0.0);
    assert!(f.to_local(Vec2::new(100.0, 310.0)).y < 0.0);
}

#[test]
fn reflection_preserves_along_and_flips_depth() {
    // Two floor portals; a point sunk 5 into the entry, 3 along.
    let a = floor_at(0.0, 0.0);
    let b = floor_at(400.0, 0.0);
    let p = a.from_local(Vec2::new(3.0, -5.0));
    let mapped = map_point(&a, &b, MapConvention::Reflection, p);
    assert_eq!(b.to_local(mapped), Vec2::new(3.0, 5.0));
}

#[test]
fn rotation_flips_along_and_depth() {
    let a = floor_at(0.0, 0.0);
    let b = floor_at(400.0, 0.0);
    let p = a.from_local(Vec2::new(3.0, -5.0));
    let mapped = map_point(&a, &b, MapConvention::Rotation, p);
    assert_eq!(b.to_local(mapped), Vec2::new(-3.0, 5.0));
}

#[test]
fn map_point_inverts_with_endpoints_swapped() {
    for conv in [MapConvention::Reflection, MapConvention::Rotation] {
        let a = PortalFrame::fixed(Vec2::new(10.0, 20.0), RIGHT);
        let b = floor_at(300.0, 50.0);
        let p = Vec2::new(6.0, 22.0);
        let there = map_point(&a, &b, conv, p);
        let back = map_point(&b, &a, conv, there);
        assert!(
            (back - p).length() < 1e-4,
            "{conv:?}: map ∘ swapped-map must be identity, got {back:?} vs {p:?}"
        );
    }
}

#[test]
fn map_vec_between_matches_the_historical_formulas() {
    // The exact op-shape the platformer math used; cardinal cases are
    // bit-identical by construction (0/±1 products).
    let v = Vec2::new(120.0, -340.0);
    let n_in = Vec2::new(0.0, -1.0);
    let n_out = Vec2::new(1.0, 0.0);
    let refl = {
        let into = -v.dot(n_in);
        let along = v.dot(tangent_of(n_in));
        into * n_out + along * tangent_of(n_out)
    };
    assert_eq!(
        map_vec_between(v, n_in, n_out, MapConvention::Reflection),
        refl
    );
    let rot = {
        let into = -v.dot(n_in);
        let along = v.dot(tangent_of(n_in));
        into * n_out - along * tangent_of(n_out)
    };
    assert_eq!(
        map_vec_between(v, n_in, n_out, MapConvention::Rotation),
        rot
    );
}

#[test]
fn galilean_velocity_composition() {
    // Entry aperture moving down (+y), exit moving right: a body falling
    // WITH the entry (zero relative velocity) exits carried by the exit.
    let a = PortalFrame {
        origin: Vec2::ZERO,
        normal: UP,
        velocity: Vec2::new(0.0, 50.0),
    };
    let b = PortalFrame {
        origin: Vec2::new(500.0, 0.0),
        normal: UP,
        velocity: Vec2::new(30.0, 0.0),
    };
    let v_out = map_velocity(&a, &b, MapConvention::Reflection, Vec2::new(0.0, 50.0));
    assert_eq!(v_out, Vec2::new(30.0, 0.0));
    // Static frames: composition degenerates to the plain map.
    let sa = floor_at(0.0, 0.0);
    let sb = floor_at(500.0, 0.0);
    let v = Vec2::new(80.0, 200.0);
    assert_eq!(
        map_velocity(&sa, &sb, MapConvention::Reflection, v),
        map_vec(&sa, &sb, MapConvention::Reflection, v)
    );
}
