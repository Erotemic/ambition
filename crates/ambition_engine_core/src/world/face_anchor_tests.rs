//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod face_anchor_tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module with
//! private access via `use super::*;` (a direct sibling, so `super` depth is
//! unchanged).

use super::*;
use crate::{Face, GeoFaceRef, GeoId, PlacementId};

fn hosted_block() -> Block {
    let mut b = Block::solid("host", Vec2::new(100.0, 200.0), Vec2::new(80.0, 40.0));
    b.id = GeoId::placement(PlacementId::new("host-iid"), 0);
    b.velocity = Vec2::new(3.0, 0.0);
    b
}

fn world_with(block: Block) -> World {
    let mut w = World::new("t", Vec2::new(640.0, 480.0), Vec2::ZERO, Vec::new());
    w.blocks.push(block);
    w
}

/// attribute ∘ resolve round-trips on every AABB face, carries the host
/// velocity, and clamps `along` to the face extent.
#[test]
fn attribute_then_resolve_round_trips_each_face() {
    let w = world_with(hosted_block());
    // (probe point, outward normal, expected along)
    let cases = [
        (Vec2::new(150.0, 199.0), Vec2::new(0.0, -1.0), 10.0), // Top
        (Vec2::new(150.0, 241.0), Vec2::new(0.0, 1.0), -10.0), // Bottom (tangent flips)
        (Vec2::new(99.0, 215.0), Vec2::new(-1.0, 0.0), 5.0),   // Left
        (Vec2::new(181.0, 215.0), Vec2::new(1.0, 0.0), -5.0),  // Right
    ];
    for (probe, normal, along) in cases {
        let r = w
            .attribute_face(probe, normal, 4.0)
            .unwrap_or_else(|| panic!("face under {normal:?} attributes"));
        assert_eq!(r.geo, GeoId::placement(PlacementId::new("host-iid"), 0));
        assert!(
            (r.along - along).abs() < 1e-4,
            "{normal:?}: along {} != {along}",
            r.along
        );
        let a = w.resolve_face(&r).expect("resolves");
        assert_eq!(a.normal, normal);
        assert_eq!(a.velocity, Vec2::new(3.0, 0.0));
        // The anchor sits ON the face plane at the probe's tangent offset.
        assert!(
            (a.origin - probe)
                .dot(crate::frame::tangent_of(normal))
                .abs()
                < 1e-4
        );
        assert!((a.origin - probe).dot(normal).abs() <= 4.0);
    }
}

#[test]
fn anon_blocks_never_host_and_missing_ids_do_not_resolve() {
    let mut anon = hosted_block();
    anon.id = GeoId::anon();
    let w = world_with(anon);
    assert!(w
        .attribute_face(Vec2::new(150.0, 199.0), Vec2::new(0.0, -1.0), 4.0)
        .is_none());
    let r = GeoFaceRef::new(
        GeoId::placement(PlacementId::new("gone"), 0),
        Face::Top,
        0.0,
    );
    assert!(w.resolve_face(&r).is_none());
}

#[test]
fn resolve_clamps_along_to_the_face_extent() {
    let w = world_with(hosted_block());
    let r = GeoFaceRef::new(
        GeoId::placement(PlacementId::new("host-iid"), 0),
        Face::Top,
        999.0,
    );
    let a = w.resolve_face(&r).expect("resolves");
    // Face runs x ∈ [100, 180]; clamped along = +40 → x = 180.
    assert_eq!(a.origin, Vec2::new(180.0, 200.0));
}
