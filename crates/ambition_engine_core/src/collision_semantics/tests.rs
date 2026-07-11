//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::geometry::aabb_from_min_size;
use crate::world::BlinkWallTier;

const CARDINALS: [Vec2; 4] = [
    Vec2::new(0.0, 1.0),  // down
    Vec2::new(0.0, -1.0), // up
    Vec2::new(1.0, 0.0),  // right
    Vec2::new(-1.0, 0.0), // left
];

#[test]
fn gravity_axis_and_role_are_cardinal_consistent() {
    for dir in CARDINALS {
        let g = gravity_axis(dir);
        assert_eq!(axis_role(g, dir), AxisRole::Gravity);
        assert_eq!(axis_role(g.perpendicular(), dir), AxisRole::Side);
    }
}

#[test]
fn one_way_blocks_only_on_the_gravity_axis() {
    for dir in CARDINALS {
        let g = gravity_axis(dir);
        assert!(is_solid_for_axis(BlockKind::OneWay, g, dir));
        assert!(!is_solid_for_axis(
            BlockKind::OneWay,
            g.perpendicular(),
            dir
        ));
        // Full solids block both axes in every frame.
        assert!(is_solid_for_axis(BlockKind::Solid, g, dir));
        assert!(is_solid_for_axis(BlockKind::Solid, g.perpendicular(), dir));
    }
}

#[test]
fn non_collision_kinds_never_block() {
    for dir in CARDINALS {
        let g = gravity_axis(dir);
        for kind in [BlockKind::Hazard, BlockKind::PogoOrb] {
            assert!(!is_solid_for_axis(kind, g, dir));
            assert!(!is_solid_for_axis(kind, g.perpendicular(), dir));
            assert!(!is_support_surface(kind));
        }
    }
}

#[test]
fn support_classification_matches_intent() {
    assert!(is_support_surface(BlockKind::Solid));
    assert!(is_support_surface(BlockKind::OneWay));
    assert!(is_support_surface(BlockKind::BlinkWall {
        tier: BlinkWallTier::Soft
    }));
    assert!(is_full_collision_surface(BlockKind::Solid));
    assert!(!is_full_collision_surface(BlockKind::OneWay));
}

#[test]
fn moving_toward_feet_is_gravity_relative() {
    // Toward feet means along +gravity_dir in every frame.
    assert!(moving_toward_feet(Vec2::new(0.0, 5.0), Vec2::new(0.0, 1.0)));
    assert!(!moving_toward_feet(
        Vec2::new(0.0, -5.0),
        Vec2::new(0.0, 1.0)
    ));
    assert!(moving_toward_feet(
        Vec2::new(-5.0, 0.0),
        Vec2::new(-1.0, 0.0)
    ));
    assert!(!moving_toward_feet(
        Vec2::new(5.0, 0.0),
        Vec2::new(-1.0, 0.0)
    ));
}

// --- Canonical resolutions of the three former player/enemy drifts ---

#[test]
fn perpendicular_overlap_requires_real_overlap_not_a_sliver() {
    // Drift #1: the slack now applies to every actor. A body overlapping a
    // surface by less than EDGE_OVERLAP_SLOP on a side is NOT resting.
    let surface = aabb_from_min_size(Vec2::new(0.0, 100.0), Vec2::new(100.0, 20.0));
    let dir = Vec2::new(0.0, 1.0);
    // Body whose right edge clears the surface left by only 0.5px -> sliver.
    let sliver = Aabb::new(Vec2::new(-9.5, 80.0), Vec2::new(10.0, 10.0)); // right = 0.5
    assert!(!perpendicular_overlap(sliver, surface, dir));
    // Two px of real overlap -> rests.
    let resting = Aabb::new(Vec2::new(-8.0, 80.0), Vec2::new(10.0, 10.0)); // right = 2.0
    assert!(perpendicular_overlap(resting, surface, dir));
}

#[test]
fn at_rest_uses_the_body_on_support_side_guard() {
    // Drift #3: surface_supports_body_at_rest now also requires the body's
    // center to be on the support side. `body_on_support_side` compares
    // CENTERS, so for a normally-resting body it is always true (feet near
    // the head => center above the surface center) — the guard is inert for
    // normal actors and only excludes a huge/embedded body whose center has
    // passed the surface center (the mockingbird OOB class). This documents
    // that semantics rather than claiming the guard flips a resting contact.
    let surface = aabb_from_min_size(Vec2::new(0.0, 100.0), Vec2::new(100.0, 20.0));
    let dir = Vec2::new(0.0, 1.0); // head(top)=100, center=110
    let resting = Aabb::new(Vec2::new(40.0, 89.0), Vec2::new(10.0, 10.0)); // feet=99
    assert!(body_on_support_side(resting, surface, dir));
    assert!(surface_supports_body_at_rest(
        BlockKind::Solid,
        resting,
        surface,
        dir,
        false
    ));
    // Center past the surface center: not on the support side, not resting.
    let embedded = Aabb::new(Vec2::new(40.0, 130.0), Vec2::new(10.0, 10.0));
    assert!(!body_on_support_side(embedded, surface, dir));
    assert!(!surface_supports_body_at_rest(
        BlockKind::Solid,
        embedded,
        surface,
        dir,
        false
    ));
    // A one-way dropping through is never a resting support.
    assert!(!surface_supports_body_at_rest(
        BlockKind::OneWay,
        resting,
        surface,
        dir,
        true
    ));
}

#[test]
fn one_way_landing_is_false_without_gravity() {
    // Drift #2: no gravity direction -> no one-way "landing".
    let block = aabb_from_min_size(Vec2::new(0.0, 100.0), Vec2::new(100.0, 14.0));
    let body = Aabb::new(Vec2::new(40.0, 88.0), Vec2::new(10.0, 10.0));
    assert!(!one_way_landing_from_previous_feet(
        body,
        block,
        Vec2::new(0.0, 5.0),
        Vec2::ZERO,
        false,
        88.0,
    ));
    // With down gravity and a feet-side crossing, it lands.
    assert!(one_way_landing_from_previous_feet(
        body,
        block,
        Vec2::new(0.0, 5.0),
        Vec2::new(0.0, 1.0),
        false,
        96.0,
    ));
}

#[test]
fn contact_tangent_winding_is_consistent() {
    // Floor under down-gravity: normal up (0,-1) -> tangent rightward (1,0).
    let c = Contact {
        point: Vec2::ZERO,
        normal: Vec2::new(0.0, -1.0),
        toi: 0.0,
        surface_velocity: Vec2::ZERO,
        source: ContactSource::Block {
            kind: BlockKind::Solid,
        },
    };
    assert_eq!(c.tangent(), Vec2::new(1.0, 0.0));
    // Round trip: n = (t.y, -t.x).
    let t = c.tangent();
    assert_eq!(Vec2::new(t.y, -t.x), c.normal);
}

#[test]
fn block_face_contact_point_lies_on_the_face_for_all_cardinals() {
    let block = Block::solid("floor", Vec2::new(0.0, 100.0), Vec2::new(100.0, 20.0));
    // Body resting on top of the block (normal up).
    let body = aabb_from_min_size(Vec2::new(30.0, 80.0), Vec2::new(20.0, 20.0));
    let c = block_face_contact(body, &block, Vec2::new(0.0, -1.0), 0.25);
    assert!((c.point.y - 100.0).abs() < 1e-4, "on the top face");
    assert!((c.point.x - 40.0).abs() < 1e-4, "midpoint of x overlap");
    assert_eq!(c.toi, 0.25);
    assert_eq!(c.surface_velocity, Vec2::ZERO);
    assert_eq!(
        c.source,
        ContactSource::Block {
            kind: BlockKind::Solid
        }
    );
    // Body pressed against the block's left face (normal pointing -x).
    let side_body = aabb_from_min_size(Vec2::new(-20.0, 105.0), Vec2::new(20.0, 10.0));
    let side = block_face_contact(side_body, &block, Vec2::new(-1.0, 0.0), 0.0);
    assert!((side.point.x - 0.0).abs() < 1e-4, "on the left face");
    assert!((side.point.y - 110.0).abs() < 1e-4, "midpoint of y overlap");
    // A moving block stamps its velocity onto the contact.
    let mut mover = block.clone();
    mover.velocity = Vec2::new(3.0, 0.0);
    let carried = block_face_contact(body, &mover, Vec2::new(0.0, -1.0), 0.0);
    assert_eq!(carried.surface_velocity, Vec2::new(3.0, 0.0));
}

#[test]
fn feet_snap_and_separation_are_gravity_relative() {
    // Body resting just above a floor (down gravity): feet face is the
    // bottom; separation small-negative; snap pushes down onto the head.
    let floor = aabb_from_min_size(Vec2::new(0.0, 100.0), Vec2::new(100.0, 20.0));
    let body = Aabb::new(Vec2::new(40.0, 88.0), Vec2::new(10.0, 10.0));
    let dir = Vec2::new(0.0, 1.0);
    // feet at y=98, floor head at y=100 -> separation -2.
    assert!((support_face_separation(body, floor, dir) - (-2.0)).abs() < 1e-3);
    assert!(body_on_support_side(body, floor, dir));
    let snap = snap_feet_to_surface(body, floor, dir);
    assert!((snap.y - 2.0).abs() < 1e-3 && snap.x.abs() < 1e-6);
}
