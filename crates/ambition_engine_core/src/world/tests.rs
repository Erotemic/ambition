//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module with
//! private access via `use super::*;` (a direct sibling, so `super` depth is
//! unchanged).

use super::*;

#[test]
fn world_new_starts_without_water_or_climbable_regions() {
    let world = World::new(
        "test",
        Vec2::new(100.0, 80.0),
        Vec2::new(20.0, 20.0),
        Vec::new(),
    );
    // Engine `World` no longer carries authored entities — those
    // live on the sandbox-side `RoomSpec`. Pin that the engine
    // starts with empty region lists too so future authors don't
    // re-add an authored-entity Vec without thinking about the
    // sandbox/engine boundary.
    assert!(world.water_regions.is_empty());
    assert!(world.climbable_regions.is_empty());
}

#[test]
fn body_overlaps_any_uses_predicate() {
    let world = World::new(
        "test",
        Vec2::new(200.0, 200.0),
        Vec2::new(50.0, 50.0),
        vec![
            Block::solid("wall", Vec2::new(50.0, 50.0), Vec2::new(20.0, 20.0)),
            Block::hazard("spike", Vec2::new(100.0, 50.0), Vec2::new(20.0, 20.0)),
        ],
    );
    let body = Aabb::new(Vec2::new(60.0, 60.0), Vec2::new(5.0, 5.0));
    // Predicate matches the wall — overlap detected.
    assert!(world.body_overlaps_any(body, |b| matches!(b.kind, BlockKind::Solid)));
    // Predicate matches only hazards — no overlap because the body
    // is over the wall, not the hazard.
    assert!(!world.body_overlaps_any(body, |b| matches!(b.kind, BlockKind::Hazard)));
}

#[test]
fn first_body_sweep_picks_earliest_hit() {
    let world = World::new(
        "test",
        Vec2::new(500.0, 500.0),
        Vec2::new(10.0, 10.0),
        vec![
            Block::solid("near", Vec2::new(50.0, 0.0), Vec2::new(10.0, 100.0)),
            Block::solid("far", Vec2::new(200.0, 0.0), Vec2::new(10.0, 100.0)),
        ],
    );
    let body = Aabb::new(Vec2::new(20.0, 50.0), Vec2::new(5.0, 5.0));
    let hit = world.first_body_sweep(body, Vec2::new(300.0, 0.0), |_| true);
    let hit = hit.expect("sweep should hit something with two walls in path");
    assert_eq!(hit.block.name, "near");
    assert!(hit.time_of_impact >= 0.0 && hit.time_of_impact <= 1.0);
}

#[test]
fn chain_winding_matches_the_contact_convention() {
    // A floor authored left->right: tangent (1,0), normal (0,-1) = up in
    // y-down screen coordinates — identical to Contact::tangent's rule.
    let floor = SurfaceChain::open(
        "floor",
        vec![Vec2::new(0.0, 100.0), Vec2::new(200.0, 100.0)],
    );
    assert_eq!(floor.tangent(0), Vec2::new(1.0, 0.0));
    assert_eq!(floor.normal(0), Vec2::new(0.0, -1.0));
    assert_eq!(floor.total_length(), 200.0);
}

#[test]
fn chain_frame_at_wraps_closed_and_clamps_open() {
    // A 100x100 square loop traversed so its normals face the INTERIOR
    // (rideable inside): floor L->R, up the right wall, R->L along the
    // ceiling, down the left wall. Negative shoelace area by convention.
    let square = SurfaceChain::closed_loop(
        "loop",
        vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(100.0, 0.0),
            Vec2::new(100.0, -100.0),
            Vec2::new(0.0, -100.0),
        ],
    );
    assert_eq!(square.segment_count(), 4);
    assert_eq!(square.total_length(), 400.0);
    assert!(square.signed_area() < 0.0, "interior-rideable winding");
    // Interior-facing normals on every segment: floor up, right wall
    // leftward, ceiling down, left wall rightward.
    assert_eq!(square.normal(0), Vec2::new(0.0, -1.0));
    assert_eq!(square.normal(1), Vec2::new(-1.0, 0.0));
    assert_eq!(square.normal(2), Vec2::new(0.0, 1.0));
    assert_eq!(square.normal(3), Vec2::new(1.0, 0.0));
    // Arc length wraps: s = 450 is s = 50, halfway along the floor.
    let f = square.frame_at(450.0);
    assert_eq!(f.segment, 0);
    assert!((f.point - Vec2::new(50.0, 0.0)).length() < 1e-4);
    // Negative s wraps backward onto the left wall.
    let back = square.frame_at(-50.0);
    assert_eq!(back.segment, 3);
    // An open chain clamps instead.
    let open = SurfaceChain::open("ramp", vec![Vec2::new(0.0, 0.0), Vec2::new(100.0, -50.0)]);
    let end = open.frame_at(1.0e6);
    assert!((end.point - Vec2::new(100.0, -50.0)).length() < 1e-3);
}

#[test]
fn chain_project_reports_arc_and_rideable_side() {
    let floor = SurfaceChain::open(
        "floor",
        vec![Vec2::new(0.0, 100.0), Vec2::new(200.0, 100.0)],
    );
    // A point ABOVE the floor (y < 100 in y-down coords) is on the
    // rideable +normal side.
    let (s, d) = floor.project(Vec2::new(50.0, 90.0));
    assert!((s - 50.0).abs() < 1e-4);
    assert!(d > 0.0, "above the floor is the rideable side (d = {d})");
    let (_, below) = floor.project(Vec2::new(50.0, 110.0));
    assert!(below < 0.0, "below the floor is the solid side");
}

#[test]
fn chain_validate_catches_authoring_hazards() {
    // Too few points.
    assert!(!SurfaceChain::open("p", vec![Vec2::ZERO])
        .validate()
        .is_empty());
    // Degenerate segment (duplicated join vertex).
    let dup = SurfaceChain::open(
        "dup",
        vec![
            Vec2::ZERO,
            Vec2::new(50.0, 0.0),
            Vec2::new(50.0, 0.0),
            Vec2::new(100.0, 0.0),
        ],
    );
    assert!(dup.validate().iter().any(|p| p.contains("degenerate")));
    // Closed chain duplicating its first point at the end.
    let closed_dup = SurfaceChain::closed_loop(
        "ring",
        vec![
            Vec2::ZERO,
            Vec2::new(100.0, 0.0),
            Vec2::new(100.0, -100.0),
            Vec2::ZERO,
        ],
    );
    assert!(closed_dup
        .validate()
        .iter()
        .any(|p| p.contains("closing segment is implicit") || p.contains("degenerate")));
    // Self-intersection (a bowtie).
    let bowtie = SurfaceChain::open(
        "bowtie",
        vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(100.0, -100.0),
            Vec2::new(100.0, 0.0),
            Vec2::new(0.0, -100.0),
        ],
    );
    assert!(bowtie.validate().iter().any(|p| p.contains("cross")));
    let malformed_depths = SurfaceChain::open(
        "bad-depth-count",
        vec![Vec2::ZERO, Vec2::X, Vec2::new(2.0, 0.0)],
    )
    .with_segment_depths(vec![0]);
    assert!(
        malformed_depths
            .validate()
            .iter()
            .any(|problem| problem.contains("depth_lanes")),
        "authors must provide exactly one depth lane per segment"
    );
    let malformed_junction = SurfaceChain::open(
        "bad-junction",
        vec![Vec2::ZERO, Vec2::X, Vec2::new(2.0, 0.0)],
    )
    .with_junctions(vec![SurfaceJunction::new(vec![0, 2])]);
    assert!(
        malformed_junction
            .validate()
            .iter()
            .any(|problem| problem.contains("not coincident")),
        "a route switch connects coincident topological occurrences, not arbitrary points"
    );
    let valid_junction = SurfaceChain::open(
        "valid-junction",
        vec![
            Vec2::new(-1.0, 0.0),
            Vec2::ZERO,
            Vec2::new(0.0, -1.0),
            Vec2::ZERO,
            Vec2::X,
        ],
    )
    .with_junctions(vec![SurfaceJunction::new(vec![1, 3])]);
    assert!(
        valid_junction.validate().is_empty(),
        "coincident route occurrences are valid: {:?}",
        valid_junction.validate()
    );
    let cross_chain_junction =
        SurfaceChain::open("cross-chain-owner", vec![Vec2::ZERO, Vec2::X]).with_junctions(vec![
            SurfaceJunction::across(vec![SurfacePort::local(0), SurfacePort::chain(1, 0)]),
        ]);
    assert!(
        cross_chain_junction.validate().is_empty(),
        "chain-local validation accepts an external route port; world validation owns the referenced index"
    );
    let route_world = World::new("route-world", Vec2::new(10.0, 10.0), Vec2::ZERO, Vec::new())
        .with_chains(vec![
            cross_chain_junction,
            SurfaceChain::open("target", vec![Vec2::ZERO, Vec2::Y]),
        ]);
    assert!(
        route_world.validate_surface_junctions().is_empty(),
        "coincident cross-chain ports validate in their owning world: {:?}",
        route_world.validate_surface_junctions()
    );
    // A healthy ramp validates clean.
    let ramp = SurfaceChain::open(
        "ramp",
        vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(100.0, -30.0),
            Vec2::new(200.0, -40.0),
        ],
    );
    assert!(ramp.validate().is_empty(), "{:?}", ramp.validate());
}

#[test]
fn first_body_sweep_returns_none_when_predicate_filters_all() {
    let world = World::new(
        "test",
        Vec2::new(500.0, 500.0),
        Vec2::new(10.0, 10.0),
        vec![Block::solid(
            "wall",
            Vec2::new(50.0, 0.0),
            Vec2::new(10.0, 100.0),
        )],
    );
    let body = Aabb::new(Vec2::new(20.0, 50.0), Vec2::new(5.0, 5.0));
    // Predicate rejects every block — sweep finds nothing.
    let hit = world.first_body_sweep(body, Vec2::new(300.0, 0.0), |_| false);
    assert!(hit.is_none());
}

#[test]
fn water_at_returns_none_outside_any_region() {
    let world = World::new(
        "test",
        Vec2::new(500.0, 500.0),
        Vec2::new(10.0, 10.0),
        Vec::new(),
    );
    let body = Aabb::new(Vec2::new(50.0, 50.0), Vec2::new(5.0, 5.0));
    assert!(world.water_at(body).is_none());
}

#[test]
fn water_at_reports_full_submersion_for_a_body_below_the_surface() {
    // Aabb::new is (center, half_size). Water region: center
    // (200, 200), half (100, 100) → min=(100,100), max=(300,300).
    // top()=100. Body: center (200, 200), half (10, 10) →
    // top=190. depth = 190 - 100 = 90. Body height = 20.
    // submersion = 90 / 20 = 4.5, clamps to 1.0.
    let mut world = World::new(
        "test",
        Vec2::new(500.0, 500.0),
        Vec2::new(10.0, 10.0),
        Vec::new(),
    );
    world.water_regions.push(WaterRegion::new(
        Aabb::new(Vec2::new(200.0, 200.0), Vec2::new(100.0, 100.0)),
        WaterKind::Clear,
        WaterVolumeSpec::default(),
    ));
    let body = Aabb::new(Vec2::new(200.0, 200.0), Vec2::new(10.0, 10.0));
    let contact = world.water_at(body).expect("body sits inside water");
    assert!(
        (contact.submersion - 1.0).abs() < 1e-3,
        "expected full submersion clamp; got {}",
        contact.submersion
    );
    assert_eq!(contact.kind, WaterKind::Clear);
}

#[test]
fn water_at_reports_zero_submersion_at_the_surface() {
    // Water region top at y=100 (center 200, half 100). Body
    // centered (200, 110), half (10, 10) → top=100 (exactly at
    // the surface), bottom=120. depth = 0, submersion = 0.
    let mut world = World::new(
        "test",
        Vec2::new(500.0, 500.0),
        Vec2::new(10.0, 10.0),
        Vec::new(),
    );
    world.water_regions.push(WaterRegion::new(
        Aabb::new(Vec2::new(200.0, 200.0), Vec2::new(100.0, 100.0)),
        WaterKind::Clear,
        WaterVolumeSpec::default(),
    ));
    let body = Aabb::new(Vec2::new(200.0, 110.0), Vec2::new(10.0, 10.0));
    let contact = world.water_at(body).expect("body straddles surface");
    assert!(
        (contact.submersion - 0.0).abs() < 1e-3,
        "expected zero submersion at surface; got {}",
        contact.submersion
    );
}

#[test]
fn climbable_at_returns_none_outside_any_region() {
    let world = World::new(
        "test",
        Vec2::new(500.0, 500.0),
        Vec2::new(10.0, 10.0),
        Vec::new(),
    );
    let body = Aabb::new(Vec2::new(50.0, 50.0), Vec2::new(5.0, 5.0));
    assert!(world.climbable_at(body).is_none());
}

#[test]
fn climbable_at_reports_first_intersecting_region() {
    // Two ladders side-by-side. Body sits inside the second
    // (`right`); query should return that region's metrics, not
    // the first.
    let left = ClimbableRegion::ladder(Aabb::new(Vec2::new(100.0, 200.0), Vec2::new(20.0, 100.0)));
    let right = ClimbableRegion::ladder(Aabb::new(Vec2::new(300.0, 200.0), Vec2::new(20.0, 100.0)));
    let world = World::new(
        "test",
        Vec2::new(500.0, 500.0),
        Vec2::new(10.0, 10.0),
        Vec::new(),
    )
    .with_climbable_regions(vec![left, right]);
    let body = Aabb::new(Vec2::new(305.0, 220.0), Vec2::new(10.0, 16.0));
    let contact = world.climbable_at(body).expect("body inside right ladder");
    assert!(
        (contact.center_x - 300.0).abs() < f32::EPSILON,
        "expected right-ladder center_x=300, got {}",
        contact.center_x
    );
    assert!(
        (contact.top_y - 100.0).abs() < f32::EPSILON,
        "expected top_y=100 (center 200 - half 100), got {}",
        contact.top_y
    );
    assert!(
        (contact.bottom_y - 300.0).abs() < f32::EPSILON,
        "expected bottom_y=300 (center 200 + half 100), got {}",
        contact.bottom_y
    );
    assert_eq!(contact.kind, ClimbableKind::Ladder);
}

#[test]
fn thin_region_warnings_flags_tunnelable_regions_and_passes_thick_ones() {
    // A thick water pool and a thin climbable strip (a 6px-wide vertical
    // vine — under the 26px floor a fast body tunnels it in one frame).
    let world = World::new(
        "test",
        Vec2::new(500.0, 500.0),
        Vec2::new(10.0, 10.0),
        Vec::new(),
    )
    .with_water_regions(vec![WaterRegion::new(
        Aabb::new(Vec2::new(200.0, 200.0), Vec2::new(100.0, 40.0)),
        WaterKind::Clear,
        WaterVolumeSpec::default(),
    )])
    .with_climbable_regions(vec![ClimbableRegion::ladder(Aabb::new(
        Vec2::new(300.0, 200.0),
        Vec2::new(3.0, 100.0),
    ))]);
    let warnings = world.thin_region_warnings();
    assert_eq!(warnings.len(), 1, "only the thin vine warns: {warnings:?}");
    assert!(
        warnings[0].contains("climbable") && warnings[0].contains("tunnel"),
        "the warning names the thin climbable region: {warnings:?}"
    );
    // The floor is derived from the max expected body speed at 60 Hz.
    assert!((MIN_STATE_REGION_THICKNESS - 26.0).abs() < 1.0);
}

#[test]
fn climbable_kind_supports_ladder_wall_vine_variants() {
    // Compile-time check that all three kinds can be constructed
    // and round-trip through ClimbableRegion::new. The variants
    // exist so future authoring layers can drop in without a
    // breaking enum change.
    let aabb = Aabb::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
    let ladder = ClimbableRegion::new(aabb, ClimbableKind::Ladder, ClimbableSpec::default());
    let wall = ClimbableRegion::new(aabb, ClimbableKind::Wall, ClimbableSpec::default());
    let vine = ClimbableRegion::new(aabb, ClimbableKind::Vine, ClimbableSpec::default());
    assert_eq!(ladder.kind, ClimbableKind::Ladder);
    assert_eq!(wall.kind, ClimbableKind::Wall);
    assert_eq!(vine.kind, ClimbableKind::Vine);
}

#[test]
fn climbable_spec_defaults_match_design_intent() {
    // Default spec: 180 px/sec climb, 0.25 strafe factor.
    // Ladder is faster than fall (32 px/16ms ≈ 2 frames) but
    // slower than walk (~360 px/sec) so the player can plausibly
    // beat a falling enemy to the next rung but can't speed-run
    // ladders.
    let spec = ClimbableSpec::default();
    assert!(
        (spec.climb_speed - 180.0).abs() < f32::EPSILON,
        "default climb_speed should be 180 (got {})",
        spec.climb_speed
    );
    assert!(
        (spec.strafe_factor - 0.25).abs() < f32::EPSILON,
        "default strafe_factor should be 0.25 (got {})",
        spec.strafe_factor
    );
}
