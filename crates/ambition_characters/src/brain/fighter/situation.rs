//! Pure fighter tactical-state classifier.
//!
//! Classification uses only [`Perceived`] state. Precedence is `Recovery > Disadvantage
//! > EdgeGuard > Advantage > Neutral`, so self-preservation outranks punish windows.
//! Corner and landing thresholds describe stage/kit facts and do not vary by CPU
//! difficulty; difficulty affects later option selection.

use ambition_platformer2d_core as ae;

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

/// L1. Classify the tick from this view's own point of view.
///
/// The opponent is the nearest hostile, which is the same body every other layer
/// of the brain targets — L1 does not get a private query, and a body with no
/// hostile in view is in [`Neutral`] however cornered it is, because "cornered"
/// only means something relative to someone.
pub fn classify(view: Perceived<'_>) -> Situation {
    let me = &view.self_view;
    let gravity_down = me.gravity_down;

    // 1. Self offstage. Nothing else matters.
    //
    // "offstage" is TWO facts on a platform stage, and this asked one.
    // `StageView` is the ROOM, so a fighter that walked off the lip of a 420px
    // platform in a 640px room was still *inside the stage* for another hundred
    // pixels of falling: L1 answered `Neutral`, L2 kept offering `Retreat` — the
    // verb pointing further out — and `Recover`, the one verb that means "get
    // back", was not on the list until the body had left the ROOM.
    //
    // So the question recovery is actually about is whether there is anywhere to
    // LAND. A body with the room around it and nothing underneath it is
    // recovering, whatever the room says.
    //
    // and it asks whether terrain was BUILT first. A view with no solids
    // at all is not a body over an abyss, it is a composition that does not
    // publish terrain — the `juggle_escape` scenario fixture is exactly that,
    // and without this clause it read as `Recovery` the moment this landed.
    // "I cannot see the floor" must never mean "the floor ends here";
    // `floor_ahead` carries the same warning three screens up.
    let nothing_to_land_on =
        !view.terrain.is_empty() && !view.self_view.on_ground && view.ground_below().is_none();
    if view.self_offstage() || nothing_to_land_on {
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
    //
    // cornered is about the FLOOR as well as the room. It was
    // only ever asked of `stage.distance_to_edge`, and on an enclosed room the
    // room's edge and the floor's edge are the same line — which is why nothing
    // needed the distinction until the smash stage, the first room in this
    // engine you can walk out of. On a platform stage a fighter standing on the
    // very lip of the floor is still 110px from the room boundary, so this
    // answered "not cornered" while the body was one step from a self-KO.
    //
    // A fighter that loses stocks to the floor has no difficulty curve, which
    // makes this the first thing the ladder needs to be true.
    let floor_edge = view.floor_edge_distance().unwrap_or(f32::INFINITY);
    let cornered =
        view.stage.distance_to_edge(me.pos) < CORNER_MARGIN_PX || floor_edge < CORNER_MARGIN_PX;
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
