//! **ONE BODY MAY NOT MOVE FREELY THROUGH ANOTHER** — an opt-in constraint on
//! PROPOSED motion, applied before integration.
//!
//! ⭐ **it constrains, it never separates.** AVOID PUSHOUT is about GEOMETRY
//! REPAIR — nothing teleports out of an overlap it is already in — so a body
//! moving deeper into another simply gets less of the motion it asked for. Every
//! function here is monotone: the returned motion is never larger than the
//! motion proposed and never has the opposite sign.
//!
//! ⛔⛔ **AN ACCELERATION TERM CANNOT DO THIS.** The axis-swept kernel treats
//! `vel.x` as a velocity TARGET and overwrites it every tick, so anything summed
//! into it is erased before integration. **The only place a body can be told
//! about another body is where its motion is already being resolved.** (The
//! deleted force version and its eight vacuous tests are recorded where they can
//! still fail: `kernel::tests::a_grounded_body_walking_into_another_one_is_stopped_by_the_real_sweep`.)
//!
//! ⚠ **the vocabulary is deliberately genre-free.** This is not jostle, not
//! pushback and not a fighting-game term: it is one body's motion constrained by
//! the bodies it is touching. A platform fighter opts its cast into it and calls
//! the result jostle; a co-op platformer might opt two partners into it so they
//! can stand on a switch together without occupying one point.

use crate::{Aabb, AabbExt};

/// **THE OTHER BODIES THIS STEP MAY NOT MOVE FREELY THROUGH**, and how hard they
/// resist.
///
/// ⚠ **`Default` is the identity and that is load-bearing.** Every body that
/// never opted in carries an empty field, so the constraint below returns the
/// proposed motion unchanged and the kernel behaves byte-for-byte as it did.
/// Body contact is a capability a composition grants, never a term every body in
/// the engine pays for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyContactBlocker {
    /// Where this body is, in the common pre-integration snapshot.
    pub aabb: Aabb,
    /// **How fast it was travelling in that same snapshot** — the evidence that
    /// it is coming the other way, and the only thing that lets a mover tell
    /// "the gap is mine to spend" from "we are both spending it".
    ///
    /// ⚠ **its ENTRY velocity, and it is not a proposed step.** The snapshot is
    /// taken before any body has resolved its controller, so this is last
    /// tick's answer: exact for a body already walking, and silent about a body
    /// that is about to start. What makes that safe is that BOTH halves of a
    /// pair read the same silence, so [`constrain_motion`] can answer it
    /// symmetrically rather than guess.
    pub velocity: crate::Vec2,
}

impl BodyContactBlocker {
    pub fn new(aabb: Aabb, velocity: crate::Vec2) -> Self {
        Self { aabb, velocity }
    }

    /// This body's snapshot speed along one axis, counted only when it points
    /// the way the mover is going — a blocker fleeing is not spending the gap.
    fn approach(&self, horizontal: bool, moving_positive: bool) -> f32 {
        let along = if horizontal {
            self.velocity.x
        } else {
            self.velocity.y
        };
        // The mover travels `moving_positive`; a blocker CLOSING on it travels
        // the other way.
        let toward = if moving_positive { -along } else { along };
        toward.max(0.0)
    }
}

#[derive(Clone, Copy, Default)]
pub struct BodyContactField<'a> {
    /// The OTHER opted-in bodies, sampled from a COMMON pre-integration
    /// snapshot.
    ///
    /// ⛔ **the snapshot is why this is a slice and not a query.** Two bodies
    /// resolved in sequence would each see the other at a different pose — the
    /// first at its entry pose and the second at the first's already-integrated
    /// one — so who moved first would decide who won. Sampling once, before any
    /// body integrates, makes the pass order-independent.
    pub blockers: &'a [BodyContactBlocker],
    /// How hard they resist, `0.0` (not at all) to `1.0` (a solid wall).
    ///
    /// ⭐ **the knob is here because the GENRE differs and the games differ**:
    /// Smash-like fighters push through each other slowly, a beat-em-up may want
    /// a hard stop, and neither is more correct. `1.0` stops the body at contact;
    /// `0.25` lets it keep a quarter of the motion that would take it deeper.
    pub resistance: f32,
    /// **THIS body's own velocity in that same snapshot.**
    ///
    /// ⛔⛔ **both halves of a pair must divide one gap the same way, and that
    /// is only possible from numbers they both see.** Splitting by each body's
    /// ACTUAL proposed step would have each computing its share from a figure
    /// the other cannot read, and two shares derived from different arithmetic
    /// do not add up to the gap. See [`constrain_motion`].
    pub own_velocity: crate::Vec2,
}

impl<'a> BodyContactField<'a> {
    /// A body nothing resists — the state of every body in every composition
    /// that has not opted in.
    pub const NONE: Self = Self {
        blockers: &[],
        resistance: 0.0,
        own_velocity: crate::Vec2::ZERO,
    };

    /// **THE FIELD A BODY IS RESOLVED AGAINST**: who is in its way, how hard
    /// they resist, and the velocity the snapshot recorded for the body itself.
    ///
    /// ⛔⛔ **there is no constructor that omits the last one, and there used to
    /// be.** `new` built a field whose own velocity was zero on the documented
    /// promise that "every share it computes is the whole gap" — true only while
    /// the no-evidence case was *also* the whole gap. It has no production
    /// caller and never had one (`delta_along` is `vel * dt`, so a body
    /// proposing motion always has the velocity that produced it); what it had
    /// were four unit tests describing a stationary body asking for thirty units
    /// of motion, and between them they kept the branch that decides what
    /// happens when NEITHER body is moving completely unexercised.
    pub fn moving(
        blockers: &'a [BodyContactBlocker],
        resistance: f32,
        own_velocity: crate::Vec2,
    ) -> Self {
        Self {
            blockers,
            resistance,
            own_velocity,
        }
    }

    /// Whether this field can constrain anything at all.
    pub fn is_inert(&self) -> bool {
        self.blockers.is_empty() || self.resistance <= 0.0
    }
}

/// Two boxes overlap on the axis this motion is NOT along, so motion along it
/// can actually reach one from the other.
///
/// ⚠ **`>` not `>=`**: two bodies standing exactly edge to edge on the cross
/// axis are not in each other's way, which is the same strict-overlap rule
/// `AabbExt::strict_intersects` states for the world.
fn overlaps_across(mover: Aabb, blocker: Aabb, horizontal: bool) -> bool {
    if horizontal {
        mover.min.y < blocker.max.y && blocker.min.y < mover.max.y
    } else {
        mover.min.x < blocker.max.x && blocker.min.x < mover.max.x
    }
}

/// The two boxes' extents along this axis, as `(low, high)` pairs.
fn spans(mover: Aabb, blocker: Aabb, horizontal: bool) -> (f32, f32, f32, f32) {
    if horizontal {
        (mover.min.x, mover.max.x, blocker.min.x, blocker.max.x)
    } else {
        (mover.min.y, mover.max.y, blocker.min.y, blocker.max.y)
    }
}

/// How much free space lies between the two boxes along this axis in the
/// direction of travel. Negative when they already overlap.
fn gap_along(mover: Aabb, blocker: Aabb, horizontal: bool, moving_positive: bool) -> f32 {
    let (mover_low, mover_high, blocker_low, blocker_high) = spans(mover, blocker, horizontal);
    if moving_positive {
        blocker_low - mover_high
    } else {
        mover_low - blocker_high
    }
}

/// How far the two boxes overlap along this axis, zero when they do not.
fn overlap_along(mover: Aabb, blocker: Aabb, horizontal: bool) -> f32 {
    let (mover_low, mover_high, blocker_low, blocker_high) = spans(mover, blocker, horizontal);
    (mover_high.min(blocker_high) - mover_low.max(blocker_low)).max(0.0)
}

/// **IS THIS DIRECTION GOING DEEPER INTO THAT BODY?**
///
/// ⛔⛔ **it does not have to GUESS which way "out" is**, which is the objection
/// the first version conceded to by resisting every direction: an infinitesimal
/// step either increases the axis overlap or it does not. Without this,
/// four fighters spawning on one point could not walk apart.
///
/// ⚠ **and declining to resist a body that is LEAVING is not a pushout.** Nothing
/// here moves anybody. It only stops taking motion away from a body that is
/// already resolving the situation itself.
fn deepens(mover: Aabb, blocker: Aabb, horizontal: bool, moving_positive: bool) -> bool {
    const PROBE: f32 = 1.0e-3;
    let step = if moving_positive { PROBE } else { -PROBE };
    let moved = mover.translated(if horizontal {
        crate::Vec2::new(step, 0.0)
    } else {
        crate::Vec2::new(0.0, step)
    });
    overlap_along(moved, blocker, horizontal) > overlap_along(mover, blocker, horizontal)
}

/// **THE CONSTRAINT.** Return the motion this body is actually allowed along
/// `delta` this step, given the bodies it may not move freely through.
///
/// The rule, once, for both cases the pass has to survive:
///
/// - **not walking** — a step longer than one tick of this body's own walk is a
///   launch, a blink, a scripted throw; it passes through untouched. See
///   `walk_budget`.
/// - **approaching** — the body travels its SHARE of the free gap at full speed
///   and keeps only `1 - resistance` of whatever is left over. At
///   `resistance == 1.0` it stops exactly at contact, which is a solid.
/// - **already overlapping and going DEEPER** — the free gap is zero, so the
///   whole motion is scaled. Going the other way is not resisted at all: a body
///   resolving an overlap itself is not something this pass has any business
///   slowing down, and declining to resist it moves nobody, so it is not a
///   pushout. ⛔ what the rule forbids is TELEPORTING bodies apart, and nothing
///   here ever writes a position.
///
/// ⛔ **it never returns more motion than it was given and never flips the
/// sign.** That property is what makes this composable with the world sweep that
/// runs after it: shortening a proposed delta can only ever produce a pose the
/// world sweep would already have accepted.
///
/// ⛔⛔ **TWO MOVERS MAY NOT BOTH SPEND ONE GAP.** ⇒ the gap is DIVIDED, in
/// proportion to how fast each body is closing it, so the shares sum to the gap
/// across the pair — for two movers and for four — and a body whose neighbours
/// are all standing still has the whole of it.
///
/// ⛔ **not by halving.** Halving looks equivalent and is not: it takes half the
/// gap from a body walking at a stationary neighbour, who should have all of it.
/// Proportion collapses to that answer; halving does not.
///
/// ⚠ **both halves divide by SNAPSHOT velocities, never by their own proposed
/// step.** Two bodies deriving shares from figures the other cannot read
/// produce two shares that do not add up to the gap — the same order-dependence
/// the snapshot exists to remove, wearing arithmetic instead of query order.
///
/// ⛔⛔ **AND SILENCE IS NOT PERMISSION.** The snapshot precedes every
/// controller, so two bodies that both begin walking on one tick read zero for
/// each other AND for themselves; granting the whole gap on no evidence let each
/// spend all of it, permanently. An equal share is the only division that cannot
/// over-spend when nobody has evidence, and it costs one tick to a body starting
/// from rest already within a step of a neighbour.
pub fn constrain_motion(
    mover: Aabb,
    delta_along: f32,
    horizontal: bool,
    // **ONE TICK OF THIS BODY'S OWN WALK**, and the rule is not a budget — it is
    // a QUESTION: is this body walking?
    //
    // ⛔⛔ **WITHOUT IT, BODY CONTACT EATS A KNOCKBACK LAUNCH**, and a
    // "take at most a walk's worth per tick" version does not fix it. Two
    // fighters walking into each other stall where they meet; a launched fighter
    // passes through everybody, which is also the genre's answer. Held by
    // `tests::contact_only_resists_a_body_that_is_walking`, which carries the
    // measurement.
    walk_budget: f32,
    // The step this tick spans, so a snapshot VELOCITY can be compared against a
    // proposed DISTANCE. See [`BodyContactBlocker::velocity`].
    dt: f32,
    field: BodyContactField<'_>,
) -> f32 {
    if field.is_inert() || delta_along == 0.0 || !delta_along.is_finite() {
        return delta_along;
    }
    // Faster than this body can walk ⇒ it is not walking. See `walk_budget`.
    if delta_along.abs() > walk_budget.max(0.0) {
        return delta_along;
    }
    let resistance = field.resistance.clamp(0.0, 1.0);
    let moving_positive = delta_along > 0.0;
    let asked = delta_along.abs();
    // This body's own closing speed in the SAME snapshot both halves read.
    let mine = {
        let along = if horizontal {
            field.own_velocity.x
        } else {
            field.own_velocity.y
        };
        let toward = if moving_positive { along } else { -along };
        toward.max(0.0) * dt
    };
    let mut allowed = asked;
    for blocker in field.blockers {
        if !overlaps_across(mover, blocker.aabb, horizontal) {
            continue;
        }
        let gap = gap_along(mover, blocker.aabb, horizontal, moving_positive).max(0.0);
        // **HOW MUCH OF THAT GAP IS THIS BODY'S TO SPEND.** The other body is
        // closing too, and the two shares must sum to the gap rather than each
        // being the whole of it.
        //
        // ⚠ **NO EVIDENCE MEANS AN EQUAL SHARE, and it may not mean the whole
        // gap.** A snapshot in which neither body was moving carries nothing to
        // divide by — and both bodies read that same nothing, so granting each
        // the whole gap let a pair starting from REST spend it twice. Halves are
        // the only division that cannot over-spend when the evidence is absent.
        //
        // ⭐ **this costs one tick and only to a body that starts from rest
        // already within one step of a neighbour.** Further away, the whole step
        // fits in the gap and nothing here is consulted; and from the second
        // tick the mover's own velocity is evidence, so a body walking at
        // somebody merely standing there gets the whole gap exactly as before.
        let theirs = blocker.approach(horizontal, moving_positive) * dt;
        let closing = mine + theirs;
        let free = if closing > 0.0 {
            gap * (mine / closing)
        } else {
            gap * 0.5
        };
        if asked <= free {
            continue;
        }
        // Already in contact and heading OUT: this body is resolving the overlap
        // itself and nothing here has any business slowing it down.
        //
        // ⚠ **asked of the real GAP, not of this body's share.** A share can be
        // zero with daylight still between the boxes, and reading that as
        // "already overlapping" would send a body that is merely being out-paced
        // down the leaving-an-overlap path.
        if gap <= 0.0 && !deepens(mover, blocker.aabb, horizontal, moving_positive) {
            continue;
        }
        // The part of the step that would take this body deeper.
        let deeper = asked - free;
        allowed = allowed.min(free + deeper * (1.0 - resistance));
    }
    if moving_positive {
        allowed
    } else {
        -allowed
    }
}

#[cfg(test)]
#[path = "body_contact/tests.rs"]
mod tests;
