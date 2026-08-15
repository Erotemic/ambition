//! Shared collision-semantics kernel: the gravity-relative support/surface
//! truths every actor body agrees on.
//!
//! ⭐ **ONE sweep consumes these primitives** — [`crate::movement::collision`],
//! the body movement sweep, with jump-buffer / dash / blink / climb / wall-state
//! affordances layered on top. **Every actor goes through it**: controlled,
//! scripted, AI and remote alike.
//!
//! ⛔ **this header used to say "two sweeps" and name
//! `ambition_platformer2d_shared_tangle::kinematic` as the generic enemy/NPC one.
//! That module was DELETED on 2026-08-14 (D126.2)** — it had zero production
//! callers and 26 test invocations, all inside its own test file, so its only
//! caller was the thing testing it. The two sweeps really did once carry private,
//! *almost* identical copies of these helpers, which is the dangerous kind of
//! duplication: they agreed at the design level while free to drift at the
//! implementation level (one-way landing eligibility, support-face tolerances,
//! non-down gravity). Extracting them here removed the drift; deleting the second
//! sweep removed the fork. ⚠ **the "both sweeps" phrasing survived the second
//! event and is what made a dead path read as live** — if you find another
//! sentence claiming two sweeps, it is stale.
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
///
/// ⛔ **`BonkOnly` is deliberately absent, and that absence IS the feature.** It
/// is solid only against a head coming up into it, so nothing ever rests on one
/// — a hidden block that supported a body would be an invisible floor, which is
/// the bug it was added to remove.
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
        // Both directional kinds are collision surfaces on the GRAVITY axis
        // only; which way each one blocks is decided by its own landing rule.
        BlockKind::OneWay | BlockKind::BonkOnly => {
            axis_role(axis, gravity_dir) == AxisRole::Gravity
        }
        BlockKind::Hazard | BlockKind::PogoOrb | BlockKind::Rebound { .. } => false,
    }
}

/// **Kinds whose ONLY solidity is a rising head strike — air to everything else.**
///
/// ⛔ **`is_solid_for_axis` is not enough on its own, and that gap shipped.**
/// `BonkOnly` answers `true` there on the gravity axis, because a head coming up
/// into one must be stopped — so every OTHER query that filters candidates with
/// that predicate and then forgets to ask [`bonk_strike_from_head`] treats a
/// hidden block as an ordinary floor. Two did: the controlled body's
/// penetration REPAIR, and the generic kinematic sweep enemies use. The result
/// was one block kind meaning different things depending on which movement
/// engine drove the body (GPT 5.6, review through `d46a0f7` — correct).
///
/// ⭐ **so the rule is stated once, here, and every path that is not the swept
/// head-strike test skips these outright.** A query that genuinely handles the
/// strike (`movement::collision`'s sweep) asks `bonk_strike_from_head` instead.
pub fn blocks_only_a_rising_head(kind: BlockKind) -> bool {
    matches!(kind, BlockKind::BonkOnly)
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

/// **The mirror of [`one_way_landing_from_previous_feet`]: does this body's HEAD
/// come up into the block?**
///
/// A `BonkOnly` block is solid in exactly this case and in no other, so a body
/// walks through it, falls through it, and stands where it is as if it were not
/// there — until it jumps into it from underneath.
///
/// ⚠ **`drop_through` is not consulted**, and the asymmetry is real rather than
/// an omission: dropping through is a request to stop being held UP, and nothing
/// is holding you up here. The one-way rule needs it; this one has nothing to
/// waive.
pub fn bonk_strike_from_head(body: Aabb, block: Aabb, delta: Vec2, gravity_dir: Vec2) -> bool {
    if gravity_dir == Vec2::ZERO {
        return false;
    }
    // Moving AGAINST gravity — toward the head — and the head was still below
    // the block's underside before this step, so the block is being entered
    // from beneath rather than already overlapped.
    delta.dot(gravity_dir) < -MOTION_EPS
        && body.head_coord(gravity_dir) >= block.feet_coord(gravity_dir) - ONE_WAY_CROSSING_SLOP
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
// that contact means." The ONE sweep populates contacts into
// `FrameEvents.contacts`; resolution itself is unchanged — the AABB path still
// acts on axis faces, and the surface-follower solver consumes the same
// vocabulary for chains. Observability first, byte-identical.
//
// ⭐ **this said "both sweeps" and named a second one.** A duplicate
// enemies/NPCs sweep (`step_kinematic` / `step_kinematic_observed`, in
// `ambition_platformer2d_shared_tangle::kinematic`) really did exist and really
// did consume this vocabulary — until the actor unification routed every body
// through `step_motion` and left it with nothing but its own tests calling it.
// It is deleted (D126.2). This vocabulary is shared between the world and the
// bodies that read it, not between two resolvers.

/// The SEMANTIC role a resolved contact plays for the body, judged against the
/// body's CURRENT resolved frame (never world Y) at the moment the contact is
/// generated — the sweep knows which pass produced it, an attached policy knows
/// its adhesion. Consumers read this instead of re-deriving meaning from
/// contact-list ordering or normal comparisons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContactKind {
    /// A feet-side resting/landing contact: the surface currently holding the
    /// body up against its frame's pull. THE source of the published support
    /// normal.
    Support,
    /// An anti-feet (head) blocking contact.
    Head,
    /// A lateral blocking contact along the frame's side axis.
    Side,
    /// A policy-owned adhesion contact (the crawler's cling): support-like, but
    /// its normal is the policy's attachment, deliberately independent of the
    /// frame's pull.
    Attachment,
}

/// Frame-relative fallback classification for generators without structural
/// knowledge of which pass produced the hit (swept TOI hits on chains). The
/// axis sweep classifies structurally instead (its pass + direction already
/// know the role).
pub fn classify_contact_normal(normal: Vec2, frame_down: Vec2) -> ContactKind {
    let support_dot = normal.dot(-frame_down);
    if support_dot >= 0.5 {
        ContactKind::Support
    } else if support_dot <= -0.5 {
        ContactKind::Head
    } else {
        ContactKind::Side
    }
}

/// What a contact was made against.
///
/// `Block` carries the struck block's durable [`GeoId`](crate::geo_id::GeoId) —
/// the same identity `WorldDelta` ops and traces use — so a gameplay reader can
/// answer "*which* authored block did I touch" (a ?-block bonk, brick-break, a
/// coin block, a rider's platform) without point-matching against the world.
/// That `GeoId` owns a `String`, so this enum (and [`Contact`]) is `Clone` but
/// deliberately not `Copy`: identity is worth one small clone per block contact.
#[derive(Clone, Debug, PartialEq)]
pub enum ContactSource {
    /// An axis-aligned world block, tagged with its durable geometry identity.
    Block {
        kind: BlockKind,
        id: crate::geo_id::GeoId,
    },
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
#[derive(Clone, Debug, PartialEq)]
pub struct Contact {
    /// The semantic role this contact plays for the body (frame-relative,
    /// assigned at generation).
    pub kind: ContactKind,
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

/// **NO POSITION ON THIS AXIS SATISFIES EVERY SOLID CLAIMING IT** — the body is
/// over-constrained (crushed) between two surfaces closing on it.
///
/// Penetration repair treats each intersecting solid as a CONSTRAINT on the
/// body's centre coordinate along one axis: a surface the body is resting on
/// demands a minimum, a surface pressing on its head demands a maximum. The
/// feasible set is their intersection — an interval — and the resolved position
/// is a function of that interval, which is why it cannot depend on the order
/// the blocks happen to sit in `World::blocks`. When the interval is EMPTY
/// there is no position to resolve to, and inventing one (obeying the deepest
/// claim, obeying the last claim, splitting the difference) would be a
/// deterministic wrong answer that hides a real physical condition. So the
/// resolver reports this instead, and leaves the body where it is.
///
/// ⭐ **the consequence is deliberately NOT decided here, and this is the
/// engine's standing split** (the same one [`crate::movement::ResetCause`]
/// states in its own doc): the reusable mechanics layer reports WHAT PHYSICALLY
/// HAPPENED, game policy decides what it MEANS. Damage, death, a stock, a
/// respawn, a forced displacement, crush immunity, a rider being squashed by
/// the platform carrying it, or nothing at all are all Ambition policy
/// decisions, and none of them belongs in the kernel. **Nothing in this crate
/// consumes this report** — it rides out on `FrameEvents` for an owner to
/// interpret, exactly like `ResetCause` does.
///
/// ⚠ **a policy that LATCHES this across frames owns new rollback state.** The
/// report itself is per-step output, rebuilt from world geometry every tick and
/// never read back by the simulation, so it adds nothing to the rollback
/// surface. A crush TIMER, a "was crushed" flag, or a death latch derived from
/// it is simulation state and owes registration + rollback clearing like any
/// other.
#[derive(Clone, Debug, PartialEq)]
pub struct AxisConstraintConflict {
    /// The world axis whose feasible interval came out empty.
    pub axis: Axis,
    /// The LOWEST centre coordinate the +axis-side claimant will accept.
    pub min_center: f32,
    /// The HIGHEST centre coordinate the -axis-side claimant will accept.
    /// `max_center < min_center` — that inversion is the conflict.
    pub max_center: f32,
    /// The surface demanding `min_center` (it pushes the body toward +axis).
    pub min_source: ContactSource,
    /// The surface demanding `max_center` (it pushes the body toward -axis).
    pub max_source: ContactSource,
}

impl AxisConstraintConflict {
    /// How far apart the two demands are, in pixels: the penetration that NO
    /// position can remove. Positive by construction, and the natural severity
    /// for a policy that wants to distinguish a graze from a squash.
    pub fn overconstraint(&self) -> f32 {
        self.min_center - self.max_center
    }
}

/// Build a [`Contact`] for a body touching an axis-aligned face of `block`.
/// `normal` is the SURFACE outward normal (cardinal for block faces); `kind`
/// is the generator's structural classification of the contact's role.
pub fn block_face_contact(
    body: Aabb,
    block: &Block,
    normal: Vec2,
    toi: f32,
    kind: ContactKind,
) -> Contact {
    Contact {
        kind,
        point: face_contact_point(body, block.aabb, normal),
        normal,
        toi,
        surface_velocity: block.velocity,
        source: ContactSource::Block {
            kind: block.kind,
            id: block.id.clone(),
        },
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
mod tests;
