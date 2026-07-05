//! Shared collision-semantics kernel: the gravity-relative support/surface
//! truths every actor body agrees on.
//!
//! Two sweeps consume these primitives:
//!
//! - [`crate::movement::collision`] — the controlled-body movement sweep, with
//!   jump-buffer / dash / blink / climb / wall-state affordances layered on top.
//! - `ambition_platformer_primitives::kinematic` — the generic enemy/NPC/actor
//!   sweep.
//!
//! Both used to carry private copies of these helpers. The copies were *almost*
//! identical, which is the dangerous kind of duplication: the two bodies agreed
//! at the design level while being free to drift at the implementation level
//! (one-way landing eligibility, support-face tolerances, non-down gravity).
//! This module is the single source of truth for the low-level semantic kernel
//! so every controlled/scripted/AI/remote actor collides against the same rules.
//! The richer *affordances* (depenetration strategy, wall-cling, climb passage,
//! ability tuning) stay in each sweep — only the pure classification/geometry
//! truths live here.
//!
//! Everything here is a pure function of `(BlockKind, Aabb, gravity_dir, …)` —
//! no `World`, no ECS, no per-frame state — so it is trivially testable across
//! all four cardinal gravity directions (see the `tests` module).

use crate::geometry::{Aabb, AabbExt};
use crate::world::{Block, BlockKind, World};
use crate::Vec2;

/// Resting contact tolerance along the gravity (feet) axis, in pixels. A body
/// whose feet are within this distance of a support face counts as resting on
/// it.
pub const CONTACT_SLOP: f32 = 4.0;

/// One-way landing crossing tolerance, in pixels. A body may land on a one-way
/// surface only if its previous feet coordinate was within this slack of the
/// surface's anti-gravity face — handling discrete timesteps near the surface.
pub const ONE_WAY_CROSSING_SLOP: f32 = 8.0;

/// Minimum motion (along an axis or toward the feet) treated as non-zero.
pub const MOTION_EPS: f32 = 1.0e-5;

/// Required real overlap on the axis perpendicular to gravity, in pixels, before
/// a body counts as resting on a surface (so a sliver overhang does not). See
/// [`perpendicular_overlap`].
pub const EDGE_OVERLAP_SLOP: f32 = 1.0;

/// A world axis. The world is axis-aligned, so sweeps and penetration repair
/// step one world axis at a time even though support/wall *decisions* are
/// expressed in gravity-relative (feet/head/side) terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
}

impl Axis {
    pub fn perpendicular(self) -> Self {
        match self {
            Axis::X => Axis::Y,
            Axis::Y => Axis::X,
        }
    }
}

/// Whether a world axis currently plays the gravity (feet/head) role or the
/// side (wall) role, given the body's gravity direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisRole {
    Gravity,
    Side,
}

/// The world axis gravity currently runs along (cardinal `gravity_dir`).
pub fn gravity_axis(gravity_dir: Vec2) -> Axis {
    if gravity_dir.x.abs() > gravity_dir.y.abs() {
        Axis::X
    } else {
        Axis::Y
    }
}

/// Classify a world axis as the gravity axis or a side axis for this gravity.
pub fn axis_role(axis: Axis, gravity_dir: Vec2) -> AxisRole {
    if axis == gravity_axis(gravity_dir) {
        AxisRole::Gravity
    } else {
        AxisRole::Side
    }
}

/// True when `delta` carries the body toward its feet (the +gravity direction).
pub fn moving_toward_feet(delta: Vec2, gravity_dir: Vec2) -> bool {
    delta.dot(gravity_dir) > MOTION_EPS
}

/// Surfaces a body can rest on: full solids, blink walls, and one-ways.
pub fn is_support_surface(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Solid | BlockKind::BlinkWall { .. } | BlockKind::OneWay
    )
}

/// Surfaces that block both axes unconditionally (solids and blink walls).
pub fn is_full_collision_surface(kind: BlockKind) -> bool {
    matches!(kind, BlockKind::Solid | BlockKind::BlinkWall { .. })
}

/// Whether `kind` is a collision surface for `axis` under this gravity.
///
/// Full solids/blink walls block both axes. One-way surfaces are collision
/// surfaces only on the current gravity axis (their passability is then decided
/// by the one-way landing rule); they never block on a side axis. Hazards, pogo
/// orbs, and rebound blocks are handled by gameplay logic, not collision.
pub fn is_solid_for_axis(kind: BlockKind, axis: Axis, gravity_dir: Vec2) -> bool {
    match kind {
        BlockKind::Solid | BlockKind::BlinkWall { .. } => true,
        BlockKind::OneWay => axis_role(axis, gravity_dir) == AxisRole::Gravity,
        BlockKind::Hazard | BlockKind::PogoOrb | BlockKind::Rebound { .. } => false,
    }
}

/// Signed separation between the body's feet face and the surface's head face
/// along the gravity axis. Zero at perfect rest; positive when the feet are
/// past (penetrating) the support face.
pub fn support_face_separation(body: Aabb, surface: Aabb, gravity_dir: Vec2) -> f32 {
    body.feet_coord(gravity_dir) - surface.head_coord(gravity_dir)
}

/// True when the body's center is on the support side of the surface (the side
/// gravity pulls it onto), i.e. it could be resting on rather than under it.
pub fn body_on_support_side(body: Aabb, surface: Aabb, gravity_dir: Vec2) -> bool {
    body.center().dot(gravity_dir) <= surface.center().dot(gravity_dir)
}

/// The position delta that snaps the body's feet face exactly onto the
/// surface's head face along the gravity axis.
pub fn snap_feet_to_surface(body: Aabb, surface: Aabb, gravity_dir: Vec2) -> Vec2 {
    gravity_dir * (surface.head_coord(gravity_dir) - body.feet_coord(gravity_dir))
}

/// True when a penetration snap/push is a genuine small contact correction
/// rather than a pushout-teleport. A legitimate resting/contact resolution moves
/// the body at most a contact-slop distance; a move larger than the body's own
/// half-extent means the matched surface is one the body is deeply penetrating
/// (its nearest in-axis exit is the FAR face), and shoving its feet/edge to that
/// far surface would fling the body clear across — or out of — the world.
///
/// Deep overlap is depenetration's bounded job: leave the body and let its own
/// velocity carry it out the near face over subsequent frames. This is the
/// engine's no-artificial-pushout invariant ([[feedback_avoid_pushout]]),
/// shared by the controlled-body sweep and the generic kinematic primitive so
/// neither path can single-tick teleport an embedded actor out of the world.
/// Caught twice by the actor OOB trace: the mockingbird arena (2026-06-21) and
/// the central hub under sideways gravity (2026-06-25).
pub fn is_contact_range_snap(snap: Vec2, body: Aabb) -> bool {
    snap.length() <= body.half_size().length()
}

/// Overlap on the axis PERPENDICULAR to gravity — the "width" a body must share
/// with a surface to rest on it (the X span under vertical gravity, the Y span
/// under wall-walking). Requires [`EDGE_OVERLAP_SLOP`] of real overlap on each
/// side, so a body hanging off an edge by a sliver does not count as resting.
///
/// Canonical unification (2026-06-25): the controlled-body sweep required this
/// slack ("strict-touch contract"); the generic kinematic sweep used none. The
/// slack is the more conservative, tuned rule, so it now applies to every actor.
pub fn perpendicular_overlap(body: Aabb, surface: Aabb, gravity_dir: Vec2) -> bool {
    if gravity_dir.y.abs() >= gravity_dir.x.abs() {
        body.right() > surface.left() + EDGE_OVERLAP_SLOP
            && body.left() < surface.right() - EDGE_OVERLAP_SLOP
    } else {
        body.bottom() > surface.top() + EDGE_OVERLAP_SLOP
            && body.top() < surface.bottom() - EDGE_OVERLAP_SLOP
    }
}

/// Whether a body may LAND on a one-way surface this step: it must be moving
/// toward its feet, have started on the passable (anti-gravity) side within
/// [`ONE_WAY_CROSSING_SLOP`], and share perpendicular overlap. `drop_through`
/// — or absent gravity — suppresses the landing so the body falls through.
///
/// Canonical unification (2026-06-25): includes the `gravity_dir == ZERO` guard
/// (a one-way "landing" is meaningless without a gravity direction) that the
/// controlled-body sweep had and the kinematic sweep lacked.
pub fn one_way_landing_from_previous_feet(
    body: Aabb,
    block: Aabb,
    delta: Vec2,
    gravity_dir: Vec2,
    drop_through: bool,
    prev_feet_coord: f32,
) -> bool {
    if drop_through || gravity_dir == Vec2::ZERO {
        return false;
    }
    moving_toward_feet(delta, gravity_dir)
        && prev_feet_coord <= block.head_coord(gravity_dir) + ONE_WAY_CROSSING_SLOP
        && perpendicular_overlap(body, block, gravity_dir)
}

/// Whether `surface` supports `body` at rest under this gravity: a support kind,
/// perpendicular overlap, the body's center on the support side, and the feet
/// face within [`CONTACT_SLOP`] of the surface head. A one-way does not support
/// a body that is dropping through.
///
/// Canonical unification (2026-06-25): includes the `body_on_support_side`
/// requirement (you are not resting ON something your center has passed) that
/// the kinematic sweep had and the controlled-body sweep lacked.
pub fn surface_supports_body_at_rest(
    kind: BlockKind,
    body: Aabb,
    surface: Aabb,
    gravity_dir: Vec2,
    drop_through: bool,
) -> bool {
    if !is_support_surface(kind) || !perpendicular_overlap(body, surface, gravity_dir) {
        return false;
    }
    if matches!(kind, BlockKind::OneWay) && drop_through {
        return false;
    }
    body_on_support_side(body, surface, gravity_dir)
        && support_face_separation(body, surface, gravity_dir).abs() <= CONTACT_SLOP
}

/// The first world block that supports `body` at rest under this gravity, if any.
pub fn supporting_block<'a>(
    world: &'a World,
    body: Aabb,
    gravity_dir: Vec2,
    drop_through: bool,
) -> Option<&'a Block> {
    world.blocks.iter().find(|block| {
        surface_supports_body_at_rest(block.kind, body, block.aabb, gravity_dir, drop_through)
    })
}

// --- The contact vocabulary (fable review 2026-07-05, AJ10 layer 1) ---
//
// The lingua franca between the world's geometry and a body's interpretation
// of it: "the world exposes coherent contact information; bodies decide what
// that contact means." Both sweeps POPULATE contacts (the player sweep into
// `FrameEvents.contacts`, the kinematic sweep through
// `step_kinematic_observed`); resolution itself is unchanged — the AABB path
// still acts on axis faces, and the surface-follower solver consumes the same
// vocabulary for chains. Observability first, byte-identical.

/// What a contact was made against.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ContactSource {
    /// An axis-aligned world block.
    Block { kind: BlockKind },
    /// A segment of a [`crate::world::SurfaceChain`] (`chain` indexes
    /// `World::chains`, `segment` the chain's segment list).
    Chain { chain: u32, segment: u32 },
}

/// One resolved world contact for a moving body this step.
///
/// Conventions (shared with the surface-follower solver):
/// - `normal` is the unit OUTWARD normal of the SURFACE — it points away from
///   the surface, toward the body. (Parry's `normal1` is the outward normal of
///   the MOVING shape, i.e. the opposite sign — negate it at the boundary.)
/// - `tangent()` is `normal` rotated so that for a floor under down-gravity
///   (`normal == (0,-1)` with y growing downward) the tangent is `(1,0)` —
///   "rightward along the surface". `t = (-n.y, n.x)`, `n = (t.y, -t.x)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contact {
    /// Approximate contact point on the surface (face midpoint of the
    /// perpendicular overlap for block faces).
    pub point: Vec2,
    /// Unit outward normal of the surface, pointing toward the body.
    pub normal: Vec2,
    /// Normalized time along the attempted step delta in `[0,1]`; `0.0` for
    /// repair/rest contacts that did not come from a swept cast.
    pub toi: f32,
    /// The surface's own frame motion this step (a moving platform's
    /// `Block.velocity`; `ZERO` for static geometry).
    pub surface_velocity: Vec2,
    pub source: ContactSource,
}

impl Contact {
    /// Surface tangent, consistently wound: `(-n.y, n.x)`.
    pub fn tangent(&self) -> Vec2 {
        Vec2::new(-self.normal.y, self.normal.x)
    }
}

/// Build a [`Contact`] for a body touching an axis-aligned face of `block`.
/// `normal` is the SURFACE outward normal (cardinal for block faces).
pub fn block_face_contact(body: Aabb, block: &Block, normal: Vec2, toi: f32) -> Contact {
    Contact {
        point: face_contact_point(body, block.aabb, normal),
        normal,
        toi,
        surface_velocity: block.velocity,
        source: ContactSource::Block { kind: block.kind },
    }
}

/// Approximate contact point on an axis-aligned surface face: the face
/// coordinate along the normal axis, the midpoint of the body/surface overlap
/// on the perpendicular axis.
fn face_contact_point(body: Aabb, surface: Aabb, normal: Vec2) -> Vec2 {
    if normal.x.abs() > normal.y.abs() {
        let x = if normal.x > 0.0 {
            surface.right()
        } else {
            surface.left()
        };
        let y0 = body.top().max(surface.top());
        let y1 = body.bottom().min(surface.bottom());
        Vec2::new(x, 0.5 * (y0 + y1))
    } else if normal.y != 0.0 {
        let y = if normal.y > 0.0 {
            surface.bottom()
        } else {
            surface.top()
        };
        let x0 = body.left().max(surface.left());
        let x1 = body.right().min(surface.right());
        Vec2::new(0.5 * (x0 + x1), y)
    } else {
        body.center()
    }
}

#[cfg(test)]
mod tests {
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
}
