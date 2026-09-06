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
        impact_speed: 0.0,
        involuntary: false,
        kind: ContactKind::Support,
        point: Vec2::ZERO,
        normal: Vec2::new(0.0, -1.0),
        toi: 0.0,
        surface_velocity: Vec2::ZERO,
        source: ContactSource::Block {
            kind: BlockKind::Solid,
            id: crate::geo_id::GeoId::anon(),
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
    let c = block_face_contact(
        body,
        &block,
        Vec2::new(0.0, -1.0),
        0.25,
        ContactKind::Support,
        0.0,
    );
    assert!((c.point.y - 100.0).abs() < 1e-4, "on the top face");
    assert!((c.point.x - 40.0).abs() < 1e-4, "midpoint of x overlap");
    assert_eq!(c.toi, 0.25);
    assert_eq!(c.surface_velocity, Vec2::ZERO);
    assert_eq!(
        c.source,
        ContactSource::Block {
            kind: BlockKind::Solid,
            // `Block::solid` fixtures default to an anonymous geometry id.
            id: crate::geo_id::GeoId::anon(),
        }
    );
    // Body pressed against the block's left face (normal pointing -x).
    let side_body = aabb_from_min_size(Vec2::new(-20.0, 105.0), Vec2::new(20.0, 10.0));
    let side = block_face_contact(
        side_body,
        &block,
        Vec2::new(-1.0, 0.0),
        0.0,
        ContactKind::Side,
        0.0,
    );
    assert!((side.point.x - 0.0).abs() < 1e-4, "on the left face");
    assert!((side.point.y - 110.0).abs() < 1e-4, "midpoint of y overlap");
    // A moving block stamps its velocity onto the contact.
    let mut mover = block.clone();
    mover.velocity = Vec2::new(3.0, 0.0);
    let carried = block_face_contact(
        body,
        &mover,
        Vec2::new(0.0, -1.0),
        0.0,
        ContactKind::Support,
        0.0,
    );
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

/// The whole reason `BonkOnly` exists, stated where the predicate lives: a body
/// resting exactly on its head face is NOT supported, where the same body on the
/// same rectangle marked `Solid` or `OneWay` is.
#[test]
fn nothing_ever_rests_on_a_bonk_only_block() {
    let block = aabb_from_min_size(Vec2::new(0.0, 100.0), Vec2::new(32.0, 32.0));
    let body = Aabb::new(Vec2::new(16.0, 90.0), Vec2::new(10.0, 10.0));
    let down = Vec2::new(0.0, 1.0);

    // The control: this body IS resting, and both other kinds say so.
    for kind in [BlockKind::Solid, BlockKind::OneWay] {
        assert!(
            surface_supports_body_at_rest(kind, body, block, down, false),
            "the fixture is wrong: a {kind:?} does not support this body either, \
             so the assertion below proves nothing"
        );
    }
    assert!(
        !surface_supports_body_at_rest(BlockKind::BonkOnly, body, block, down, false),
        "a bonk-only block held a body up — which is an invisible floor, the \
         exact thing the kind was added to stop"
    );
    assert!(!is_support_surface(BlockKind::BonkOnly));
}

/// And it IS solid to a head coming up into it, or the reward it hides can
/// never be struck.
#[test]
fn a_bonk_only_block_is_struck_from_below_and_only_from_below() {
    let block = aabb_from_min_size(Vec2::new(0.0, 100.0), Vec2::new(32.0, 32.0));
    let down = Vec2::new(0.0, 1.0);
    // A body just under the block, moving UP into it.
    let rising = Aabb::new(Vec2::new(16.0, 142.0), Vec2::new(10.0, 10.0));
    assert!(
        bonk_strike_from_head(rising, block, Vec2::new(0.0, -6.0), down),
        "a head rising into the block did not strike it, so the reward is \
         unreachable"
    );
    // The same body FALLING through the same space strikes nothing.
    assert!(
        !bonk_strike_from_head(rising, block, Vec2::new(0.0, 6.0), down),
        "falling through counted as a strike"
    );
    // Walking sideways past it strikes nothing.
    assert!(
        !bonk_strike_from_head(rising, block, Vec2::new(6.0, 0.0), down),
        "walking past counted as a strike"
    );
    // And a body nowhere near it, rising, strikes nothing.
    let elsewhere = Aabb::new(Vec2::new(300.0, 142.0), Vec2::new(10.0, 10.0));
    assert!(!bonk_strike_from_head(
        elsewhere,
        block,
        Vec2::new(0.0, -6.0),
        down
    ));
}

/// It is a collision surface on the gravity axis only — the same statement
/// `OneWay` makes, so a body never clips its SIDES either.
#[test]
fn a_bonk_only_block_never_blocks_a_side_axis() {
    let down = Vec2::new(0.0, 1.0);
    assert!(is_solid_for_axis(BlockKind::BonkOnly, Axis::Y, down));
    assert!(!is_solid_for_axis(BlockKind::BonkOnly, Axis::X, down));
}

/// the rule, as an assertion: you cannot stand on an invisible block.
///
/// the vocabulary is the fix and this is the line that says so. `BonkOnly` is the MIRROR of
/// `OneWay`: `OneWay` is *"solid when crossed from above"*, `BonkOnly` is *"solid only against a
/// head coming up into it"*. Two did: the controlled body's penetration repair and the generic
/// kinematic sweep.
///
///  not "just make it non-solid" — the reward IS a `ContactKind::Head`
/// contact the collision system produces, so a block with nothing to hit cannot
/// be struck and the coin disappears with the ledge.
#[test]
fn an_invisible_block_is_not_a_floor_but_is_still_strikeable() {
    assert!(
        !is_support_surface(BlockKind::BonkOnly),
        "an invisible block reports itself as standable, which is the whole bug"
    );
    // Its one-way sibling IS, which is what makes this a vocabulary and not a
    // special case — the two differ in exactly one direction.
    assert!(is_support_surface(BlockKind::OneWay));

    //  and it is STILL a gravity-axis collision surface, deliberately: that is
    // what stops a rising head and pays the coin. A caller reading this alone
    // sees a floor, which is why `blocks_only_a_rising_head` exists.
    assert!(is_solid_for_axis(
        BlockKind::BonkOnly,
        Axis::Y,
        Vec2::new(0.0, 1.0)
    ));
    assert!(blocks_only_a_rising_head(BlockKind::BonkOnly));
    assert!(
        !blocks_only_a_rising_head(BlockKind::OneWay),
        "a one-way platform is a real floor; only the hidden block is air to feet"
    );
    // Never a side-axis wall — you cannot bump into an invisible block sideways.
    assert!(!is_solid_for_axis(
        BlockKind::BonkOnly,
        Axis::X,
        Vec2::new(0.0, 1.0)
    ));
}

/// FEET ON A HEAD IS THE SAME QUESTION UNDER EVERY GRAVITY.
///
///  the poison it pins is a hand-rolled `feet.y >= head.y`: the identical
/// stack, rotated into each cardinal frame, must give the identical answer, and
/// a screen-space test passes exactly one of the four.
#[test]
fn a_stack_reads_as_feet_on_a_head_in_every_cardinal_frame() {
    const BAND: f32 = 14.0;
    let size = Vec2::new(24.0, 40.0);
    let mut seen_true = 0;
    for dir in CARDINALS {
        let half = crate::AccelerationFrame::new(dir).to_world_half(size * 0.5);
        let victim = Aabb::new(Vec2::ZERO, half);
        let stomper = Aabb::new(-dir * size.y, half);
        assert!(
            feet_on_head(stomper, victim, dir, BAND),
            "a body resting on a head was not standing on it under gravity {dir:?}"
        );
        seen_true += 1;

        // The mirror: the SAME two bodies with the stomper underneath.
        let below = Aabb::new(dir * size.y, half);
        assert!(
            !feet_on_head(below, victim, dir, BAND),
            "a body UNDER a head was reported as standing on it under gravity {dir:?}"
        );
    }
    assert_eq!(seen_true, 4, "the frame sweep read no subjects at all");
}

/// THE BAND IS PENETRATION TOLERANCE, NOT REACH.
#[test]
fn hovering_above_a_head_is_not_standing_on_it() {
    const BAND: f32 = 14.0;
    let down = Vec2::new(0.0, 1.0);
    let size = Vec2::new(24.0, 40.0);
    let victim = Aabb::new(Vec2::ZERO, size * 0.5);

    // Feet 8px INTO the head: inside the band.
    let sunk = Aabb::new(Vec2::new(0.0, -size.y + 8.0), size * 0.5);
    assert!(feet_on_head(sunk, victim, down, BAND));

    let hovering = Aabb::new(Vec2::new(0.0, -size.y - 8.0), size * 0.5);
    assert!(
        !feet_on_head(hovering, victim, down, BAND),
        "a body hovering above another was standing on it"
    );

    // And past the band on the far side is through it, not on it.
    let through = Aabb::new(Vec2::new(0.0, -size.y + BAND + 4.0), size * 0.5);
    assert!(!feet_on_head(through, victim, down, BAND));
}

/// AND IT NEEDS A REAL LATERAL SHARE, THROUGH THE SHARED PRIMITIVE.
#[test]
fn a_body_beside_a_head_is_not_on_it() {
    const BAND: f32 = 14.0;
    let down = Vec2::new(0.0, 1.0);
    let size = Vec2::new(24.0, 40.0);
    let victim = Aabb::new(Vec2::ZERO, size * 0.5);
    let beside = Aabb::new(Vec2::new(size.x * 1.5, -size.y), size * 0.5);
    assert!(!feet_on_head(beside, victim, down, BAND));
}
