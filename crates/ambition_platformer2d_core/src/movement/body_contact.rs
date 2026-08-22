//! ONE BODY MAY NOT MOVE FREELY THROUGH ANOTHER — an opt-in constraint on
//! PROPOSED motion, applied before integration.
//!
//!  it constrains, it never separates. AVOID PUSHOUT is about GEOMETRY
//! REPAIR — nothing teleports out of an overlap it is already in — so a body
//! moving deeper into another simply gets less of the motion it asked for. Every
//! function here is monotone: the returned motion is never larger than the
//! motion proposed and never has the opposite sign.
//!
//!  the vocabulary is deliberately genre-free. This is not jostle, not
//! pushback and not a fighting-game term: it is one body's motion constrained by
//! the bodies it is touching. A platform fighter opts its cast into it and calls
//! the result jostle; a co-op platformer might opt two partners into it so they
//! can stand on a switch together without occupying one point.

use crate::{Aabb, AabbExt};

/// THE OTHER BODIES THIS STEP MAY NOT MOVE FREELY THROUGH, and how hard they
/// resist.
///
///  `Default` is the identity and that is load-bearing. Every body that
/// never opted in carries an empty field, so the constraint below returns the
/// proposed motion unchanged and the kernel behaves byte-for-byte as it did.
/// Body contact is a capability a composition grants, never a term every body in
/// the engine pays for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyContactBlocker {
    /// Where this body is, in the common pre-integration snapshot.
    pub aabb: Aabb,
    /// Velocity at contact-snapshot time. Both bodies use the same entry-state
    /// pair when dividing a closing gap, so the conservative shares remain symmetric.
    pub entry_velocity: crate::Vec2,
}

impl BodyContactBlocker {
    pub fn new(aabb: Aabb, entry_velocity: crate::Vec2) -> Self {
        Self {
            aabb,
            entry_velocity,
        }
    }

    /// This body's snapshot speed along one axis, counted only when it points
    /// the way the mover is going — a blocker fleeing is not spending the gap.
    fn approach(&self, horizontal: bool, moving_positive: bool) -> f32 {
        let along = if horizontal {
            self.entry_velocity.x
        } else {
            self.entry_velocity.y
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
    ///  the snapshot is why this is a slice and not a query. Two bodies
    /// resolved in sequence would each see the other at a different pose — the
    /// first at its entry pose and the second at the first's already-integrated
    /// one — so who moved first would decide who won. Sampling once, before any
    /// body integrates, makes the pass order-independent.
    pub blockers: &'a [BodyContactBlocker],
    /// How hard they resist, `0.0` (not at all) to `1.0` (a solid wall).
    ///
    ///  the knob is here because the GENRE differs and the games differ:
    /// Smash-like fighters push through each other slowly, a beat-em-up may want
    /// a hard stop, and neither is more correct. `1.0` stops the body at contact;
    /// `0.25` lets it keep a quarter of the motion that would take it deeper.
    pub resistance: f32,
    /// This body's velocity from the same contact snapshot. Pairwise gap shares
    /// must be derived from values visible to both bodies.
    pub own_entry_velocity: crate::Vec2,
}

impl<'a> BodyContactField<'a> {
    /// A body nothing resists — the state of every body in every composition
    /// that has not opted in.
    pub const NONE: Self = Self {
        blockers: &[],
        resistance: 0.0,
        own_entry_velocity: crate::Vec2::ZERO,
    };

    /// THE FIELD A BODY IS RESOLVED AGAINST: who is in its way, how hard
    /// they resist, and the velocity the snapshot recorded for the body itself.
    ///
    /// It has no production caller and never had one (`delta_along` is `vel * dt`, so a body
    /// proposing motion always has the velocity that produced it); what it had were four unit tests
    /// describing a stationary body asking for thirty units of motion, and between them they kept
    /// the branch that decides what happens when NEITHER body is moving completely unexercised.
    pub fn moving(
        blockers: &'a [BodyContactBlocker],
        resistance: f32,
        own_entry_velocity: crate::Vec2,
    ) -> Self {
        Self {
            blockers,
            resistance,
            own_entry_velocity,
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
///  `>` not `>=`: two bodies standing exactly edge to edge on the cross
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

/// Whether an infinitesimal step in this direction increases overlap with `blocker`.
/// Motion that is already leaving an overlap is not resisted; this function never moves
/// either body.
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

/// Constrain one axis of walking motion against body-contact blockers.
///
/// Non-walking displacement (launches/blinks/scripted throws) passes through unchanged.
/// Approaching walkers spend their share of the free gap, then apply contact resistance
/// to any remaining motion. Existing overlap is resisted only when motion deepens it.
/// Two moving bodies divide a shared gap in proportion to snapshot closing velocities;
/// when neither has velocity evidence they divide it equally. The result never exceeds or
/// reverses the proposed motion.
pub fn constrain_motion(
    mover: Aabb,
    delta_along: f32,
    horizontal: bool,
    // ONE TICK OF THIS BODY'S OWN WALK, and the rule is not a budget — it is
    // a QUESTION: is this body walking?
    //
    //  WITHOUT IT, BODY CONTACT EATS A KNOCKBACK LAUNCH, and a
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
    // Faster than this body can walk  it is not walking. See `walk_budget`.
    if delta_along.abs() > walk_budget.max(0.0) {
        return delta_along;
    }
    let resistance = field.resistance.clamp(0.0, 1.0);
    let moving_positive = delta_along > 0.0;
    let asked = delta_along.abs();
    // This body's own closing speed in the SAME snapshot both halves read.
    let mine = {
        let along = if horizontal {
            field.own_entry_velocity.x
        } else {
            field.own_entry_velocity.y
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
        // HOW MUCH OF THAT GAP IS THIS BODY'S TO SPEND. The other body is
        // closing too, and the two shares must sum to the gap rather than each
        // being the whole of it.
        //
        //  NO EVIDENCE MEANS AN EQUAL SHARE, and it may not mean the whole
        // gap. A snapshot in which neither body was moving carries nothing to
        // divide by — and both bodies read that same nothing, so granting each
        // the whole gap let a pair starting from REST spend it twice. Halves are
        // the only division that cannot over-spend when the evidence is absent.
        //
        //  this costs one tick and only to a body that starts from rest
        // already within one step of a neighbour. Further away, the whole step
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
        //  asked of the real GAP, not of this body's share. A share can be
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
