//! Property tests for `BodyShape::fits_at`.
//!
//! `fits_at` is the contract every body-mode resize (Crouch / Crawl /
//! Slide / MorphBall) calls before committing to a shape change. The
//! pre-existing focused tests pin canonical "low-ceiling crouch" cases;
//! this proptest fuzzes random body shapes against random
//! single-block worlds and asserts the geometric invariants:
//!
//! 1. In an empty world (no solids), any finite shape at any finite
//!    center fits.
//! 2. With a solid block, the shape "fits" iff the shape AABB and the
//!    block AABB do not strictly intersect — `fits_at` is the
//!    boolean inverse of `world.body_overlaps_any` for the same
//!    predicate.
//! 3. Standing → MorphBall always shrinks the AABB; if Standing fits,
//!    MorphBall fits at the same center.

use ambition_engine::world::{Block, World};
use ambition_engine::{geometry::Aabb, AabbExt, BlockKind, BodyMode, BodyShape, Vec2};
use proptest::prelude::*;

fn solid_predicate() -> impl FnMut(&Block) -> bool {
    |b: &Block| matches!(b.kind, BlockKind::Solid)
}

proptest! {
    /// Empty world: a finite body shape at any finite center always
    /// fits because there's nothing to overlap.
    #[test]
    fn empty_world_any_shape_at_any_center_fits(
        center_x in -2_000.0f32..2_000.0,
        center_y in -2_000.0f32..2_000.0,
        size_x in 1.0f32..400.0,
        size_y in 1.0f32..400.0,
    ) {
        let world = World::new(
            "empty",
            Vec2::new(4_000.0, 4_000.0),
            Vec2::ZERO,
            Vec::new(),
        );
        let shape = BodyShape {
            mode: BodyMode::Standing,
            size: Vec2::new(size_x, size_y),
        };
        prop_assert!(shape.fits_at(Vec2::new(center_x, center_y), &world, solid_predicate()));
    }

    /// One-block world: the shape fits iff the shape's AABB does NOT
    /// strictly intersect the block AABB. Computed independently from
    /// `fits_at` and asserted equal.
    #[test]
    fn one_block_fits_iff_no_strict_intersection(
        center_x in -1_000.0f32..1_000.0,
        center_y in -1_000.0f32..1_000.0,
        size_x in 1.0f32..200.0,
        size_y in 1.0f32..200.0,
        block_min_x in -1_000.0f32..1_000.0,
        block_min_y in -1_000.0f32..1_000.0,
        block_size_x in 1.0f32..200.0,
        block_size_y in 1.0f32..200.0,
    ) {
        let world = World::new(
            "one_block",
            Vec2::new(4_000.0, 4_000.0),
            Vec2::ZERO,
            vec![Block::solid(
                "block",
                Vec2::new(block_min_x, block_min_y),
                Vec2::new(block_size_x, block_size_y),
            )],
        );
        let shape = BodyShape {
            mode: BodyMode::Standing,
            size: Vec2::new(size_x, size_y),
        };
        let center = Vec2::new(center_x, center_y);

        // Independent computation: do the AABBs strictly intersect?
        let shape_aabb = Aabb::new(center, shape.size * 0.5);
        let block_aabb = Aabb::new(
            Vec2::new(block_min_x + block_size_x * 0.5, block_min_y + block_size_y * 0.5),
            Vec2::new(block_size_x * 0.5, block_size_y * 0.5),
        );
        let strict_intersect = shape_aabb.right() > block_aabb.left()
            && shape_aabb.left() < block_aabb.right()
            && shape_aabb.bottom() > block_aabb.top()
            && shape_aabb.top() < block_aabb.bottom();

        let fits = shape.fits_at(center, &world, solid_predicate());
        prop_assert_eq!(fits, !strict_intersect);
    }

    /// MorphBall is a strict shrink of every dimension relative to
    /// Standing for a typical base_size — so wherever Standing fits,
    /// MorphBall fits. The size factors are 0.55x in both axes (with
    /// width = base_size.x * 0.55 in MorphBall, vs base_size.x in
    /// Standing; height = base_size.x * 0.55 in MorphBall, vs
    /// base_size.y in Standing). Constrain `base_size.y >=
    /// base_size.x * 0.55` so MorphBall is also shorter than Standing.
    #[test]
    fn morphball_fits_wherever_standing_fits(
        base_x in 8.0f32..40.0,
        base_y_factor in 1.0f32..3.0, // base_size.y = base_x * factor; ensures shorter morph
        center_x in -500.0f32..500.0,
        center_y in -500.0f32..500.0,
        block_min_x in -300.0f32..300.0,
        block_min_y in -300.0f32..300.0,
        block_size_x in 16.0f32..200.0,
        block_size_y in 16.0f32..200.0,
    ) {
        let base_size = Vec2::new(base_x, base_x * base_y_factor);
        let world = World::new(
            "one_block",
            Vec2::new(2_000.0, 2_000.0),
            Vec2::ZERO,
            vec![Block::solid(
                "block",
                Vec2::new(block_min_x, block_min_y),
                Vec2::new(block_size_x, block_size_y),
            )],
        );
        let standing = BodyMode::Standing.shape(base_size);
        let morph = BodyMode::MorphBall.shape(base_size);

        // Sanity: morph must be strictly smaller in both dims
        prop_assert!(morph.size.x < standing.size.x);
        prop_assert!(morph.size.y < standing.size.y);

        let center = Vec2::new(center_x, center_y);
        let standing_fits = standing.fits_at(center, &world, solid_predicate());
        let morph_fits = morph.fits_at(center, &world, solid_predicate());

        if standing_fits {
            prop_assert!(morph_fits);
        }
    }
}
