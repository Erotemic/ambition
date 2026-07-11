//! **FB3 — L1, the situation classifier.**
//!
//! `docs/planning/engine/fighter-brain.md` §1:
//!
//! > *"L1 — Situation classifier (cheap, every tick): derives the tactical state
//! > from the view — `Neutral`, `Advantage` (opponent in hitstun/landing),
//! > `Disadvantage` (self in hitstun/shield-broken/cornered), `Recovery`
//! > (offstage/knocked out of arena), `EdgeGuard` (opponent recovering). Pure
//! > function of the view; unit-tested per scenario fixture."*
//!
//! It is a pure function of a [`Perceived`] view and nothing else. That is the no-cheat
//! contract's first clause, and it is why FB1's audit had to come first: before
//! it, the view carried no move phase, no damage meter, and no stage geometry, so
//! three of these five states were not derivable at all.
//!
//! ## Precedence, and why it is not a scoring function
//!
//! Two facts can hold at once — you can be offstage AND in hitstun, or juggling an
//! opponent while cornered. L1 answers ONE question: *what is this tick about?*
//! So the states are ranked, and the rank is the design:
//!
//! 1. [`Recovery`](Situation::Recovery) — **self is offstage.** Nothing else
//!    matters; a stock lost to the blastzone is not repaid by a punish.
//! 2. [`Disadvantage`](Situation::Disadvantage) — **self is in hitstun, or
//!    cornered.** You are the one who has to solve something.
//! 3. [`EdgeGuard`](Situation::EdgeGuard) — **the opponent is offstage.** The
//!    single highest-value window in the game, and it expires.
//! 4. [`Advantage`](Situation::Advantage) — the opponent is punishable: in
//!    hitstun, in an attack's startup or recovery, or committed to a landing.
//! 5. [`Neutral`](Situation::Neutral) — nobody has anything.
//!
//! Disadvantage outranks EdgeGuard on purpose: a player who chases an offstage
//! opponent while himself in hitstun is not edge-guarding, he is being carried.
//!
//! ## What "cornered" and "landing" mean, concretely
//!
//! Both are thresholds, and both are here rather than in a profile, because they
//! are facts about the STAGE and the KIT, not about difficulty. A level-1 CPU and
//! a level-9 CPU agree about whether they are cornered; they disagree about what
//! to do next, and that is L2's job (§1).

use ambition_engine_core as ae;

#[cfg(test)]
use crate::perception::WorldView;
use crate::perception::{BodyPhase, Perceived, PerceivedActor};

/// How close to a blastzone counts as cornered, in world px. A body with less
/// than this much stage behind it has lost its retreat option, which is what
/// "cornered" means — not that it is about to die.
pub const CORNER_MARGIN_PX: f32 = 120.0;

/// A body is "landing" when it is airborne, moving toward the ground fast enough
/// that it cannot change its mind, and low enough that it is committed. Landing
/// lag is the most reliable punish window in a platform fighter.
pub const LANDING_SPEED_PX_S: f32 = 60.0;

/// The tactical state of one tick, from one body's point of view.
///
/// Ordered by the precedence above: a larger variant OUTRANKS a smaller one, so
/// `max` over the facts that hold is the classification. That is a property the
/// tests lean on, and the reason the derive is not decorative.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Situation {
    /// Nobody has anything. Where a fight actually lives.
    Neutral,
    /// The opponent is punishable — hitstun, attack startup or recovery, or a
    /// committed landing.
    Advantage,
    /// The opponent is offstage. The highest-value window in the game.
    EdgeGuard,
    /// Self is in hitstun, or cornered against a blastzone.
    Disadvantage,
    /// Self is offstage. Everything else waits.
    Recovery,
}

/// Is this actor committed to a landing? Airborne, descending, and past the point
/// of changing its mind.
///
/// Gravity-relative, because a fight under rotated gravity is the same fight. The
/// view carries `gravity_down` for exactly this.
pub fn is_landing(actor: &PerceivedActor, gravity_down: ae::Vec2) -> bool {
    !actor.on_ground && actor.vel.dot(gravity_down) > LANDING_SPEED_PX_S
}

/// Is this actor punishable RIGHT NOW — the thing L2 will price?
///
/// [`BodyPhase::is_punishable`] covers hitstun, attack startup, and attack
/// recovery. Active frames are deliberately NOT punishable: that is where the
/// hitbox is, and walking into it is not a punish. A committed landing is added
/// here because landing lag is not a `BodyPhase` — it is a kinematic fact.
pub fn is_punishable(actor: &PerceivedActor, gravity_down: ae::Vec2) -> bool {
    actor.alive && (actor.phase.is_punishable() || is_landing(actor, gravity_down))
}

/// **L1.** Classify the tick from this view's own point of view.
///
/// The opponent is the nearest hostile, which is the same body every other layer
/// of the brain targets — L1 does not get a private query, and a body with no
/// hostile in view is in [`Neutral`] however cornered it is, because "cornered"
/// only means something relative to someone.
pub fn classify(view: Perceived<'_>) -> Situation {
    let me = &view.self_view;
    let gravity_down = me.gravity_down;

    // 1. Self offstage. Nothing else matters.
    if view.self_offstage() {
        return Situation::Recovery;
    }

    // A body with nobody to fight is in neutral, however uncomfortable its
    // position. "Cornered" against an empty stage is just standing near an edge.
    let Some(foe) = view.nearest_hostile() else {
        return if me.phase == BodyPhase::Hitstun {
            // ...unless it is being hit by something that is not an actor: a
            // hazard, a boss volume, a stray projectile. Reeling is reeling.
            Situation::Disadvantage
        } else {
            Situation::Neutral
        };
    };

    // 2. Self is the one with a problem.
    let cornered = view.stage.distance_to_edge(me.pos) < CORNER_MARGIN_PX;
    if me.phase == BodyPhase::Hitstun || cornered {
        return Situation::Disadvantage;
    }

    // 3. The opponent is offstage and has to come back through you.
    if view.actor_offstage(foe) {
        return Situation::EdgeGuard;
    }

    // 4. The opponent is committed to something.
    if is_punishable(foe, gravity_down) {
        return Situation::Advantage;
    }

    Situation::Neutral
}

#[cfg(test)]
mod tests;
