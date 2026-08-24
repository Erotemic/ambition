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

/// How much of a floor's own width, at each end, counts as its corner.
///
/// The fallback above is a pixel count and therefore a claim about one stage's
/// size. This is the same claim made relatively, which is what "lost its retreat
/// option" actually means: the outer sixth of whatever you are standing on. On
/// the Smash demo's 480px platform it is 72px a side, leaving the middle 70%
/// uncornered — against the absolute margin's middle 50%.
pub const CORNER_SHARE_OF_FLOOR: f32 = 0.15;

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

    // Recovery means outside the stage or airborne with published terrain and no
    // landing surface below. An empty terrain view means "unknown", not "void".
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

    // Cornering considers both room/blastzone bounds and the local floor edge.
    //
    // THE FLOOR TERM IS A SHARE OF THE FLOOR, not a pixel count. Measured
    // 2026-08-23 on the Smash demo: its platform is 480px wide and the absolute
    // margin was 120, so a body was "cornered" anywhere outside the middle 240 —
    // exactly HALF the stage it was standing on, permanently. 43% of every
    // decision the fighters made was answered from `Disadvantage`, and the
    // dodge that arm prices highest was the second most common decision in the
    // game. That is a claim about a stage size nobody re-checked, not a fact
    // about the fight.
    //
    // The doc on [`CORNER_MARGIN_PX`] says what this means — *"has lost its
    // retreat option"* — and losing your retreat is relative to the ground you
    // are standing on. A body with no floor under it keeps the absolute margin;
    // it has no width to take a share of.
    //
    // ⛔⛔ AND CORNERED IS A DIRECTION, NOT A DISTANCE. Both terms asked for the
    // NEAREST edge, so standing beside a ledge read the same as being backed
    // against it — and the ledge you stand beside to edge-guard is the nearest
    // edge there is. Measured 2026-08-24 on a 600px floor: a fighter walking
    // out to punish a hanging opponent flipped from `EdgeGuard` to
    // `Disadvantage` at 90px from the lip and RETREATED, every time. The
    // situation was unreachable from the only position it is played from.
    //
    // Retreat is away from the threat, so that is the direction the question is
    // asked in. Backed against the left ledge with the foe to the right is
    // still cornered; standing at the right ledge with the foe off it is a
    // whole stage of retreat and the strongest position in the genre.
    let retreat = -(foe.pos.x - me.pos.x).signum();
    let floor_edge = view.floor_ahead(retreat).unwrap_or(f32::INFINITY);
    let corner_margin = view
        .supporting_floor()
        .map(|floor| (floor.max.x - floor.min.x) * CORNER_SHARE_OF_FLOOR)
        .unwrap_or(CORNER_MARGIN_PX);
    let cornered =
        view.stage.room_toward(me.pos, retreat) < CORNER_MARGIN_PX || floor_edge < corner_margin;
    if me.phase == BodyPhase::Hitstun || cornered {
        return Situation::Disadvantage;
    }

    // 3. The opponent is offstage and has to come back through you.
    //
    // ⛔⛔ AND HANGING ON THE LEDGE IS THAT, which this missed entirely.
    // `actor_offstage` asks the ROOM's box — the blastzone envelope — so it is
    // true only once the opponent is nearly dead. A body on the ledge is inside
    // that box, carries `BodyPhase::Neutral`, and is not landing, so the single
    // most punishable state in the genre classified as ORDINARY NEUTRAL: the
    // brain approached at neutral weight and the hang was never a window.
    //
    // A hang is a commitment on a clock. The body cannot walk, cannot shield,
    // and every way out of it — climb, roll, getup attack, ledge jump, drop — is
    // an animation with a start. That is what an edge-guard is FOR.
    if view.actor_offstage(foe) || foe.ledge_hanging {
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
