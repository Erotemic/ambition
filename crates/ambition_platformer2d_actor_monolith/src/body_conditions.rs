//! Authored BODY-CAPABILITY conditions — "can the body do this verb at all?"
//!
//! `body.can(verb)` is the route-facing reader of [`AbilitySet`], the fifth of
//! the gate families in
//! `docs/planning/engine/capability-progression-and-world-gating.md` to become
//! reachable from a route. It exists because a gated lock wall may now name its
//! own condition (`gated_by = "body.can wall_climb"`); publishing this before
//! that landed would have been a condition no author could ask.
//!
//! ⭐ IT READS THE EFFECTIVE SET, NOT THE AUTHORED BASE. [`BodyAbilities`] is
//! what the movement kernel actually reads, and
//! [`AbilityBase`](ambition_platformer2d_core::body_clusters::AbilityBase) is
//! the intrinsic kit it derives from — so a session mask or a story lockout that
//! turns a verb off must close the route it opens, or the wall and the world
//! disagree about what the player can do. A gate that asked the base would open
//! for a body that cannot make the jump.
//!
//! ⛔ IT IS NOT `ActionSet::gated_by`. That narrows a brain's ACTION set on
//! `attack`/`shield` — what a body may attempt this tick. This answers what the
//! world should let it reach. The two read the same vocabulary deliberately and
//! must not come to disagree about what a field MEANS; they are not sharing a
//! predicate because they are not asking the same question.

use ambition_platformer2d_core::abilities::AbilitySet;
use ambition_platformer2d_core::body_clusters::{BodyAbilities, BodyKinematics};
use ambition_platformer2d_shared_tangle::authored_logic::{
    AuthoredArg, ConditionDescriptor, ConditionId, ConditionOutcome, ParamKind, ParamSpec, WhyNot,
};
use bevy::prelude::World;

/// The domain segment every condition in this file is published under.
pub const DOMAIN: &str = "body";

const VERB: ParamSpec = ParamSpec {
    name: "verb",
    kind: ParamKind::Name,
    summary: "an `AbilitySet` field name, spelled exactly (`wall_climb`, `double_jump`, `fly`)",
};

/// `body.can(verb)` — may the player's body use this verb at all?
pub fn can_descriptor() -> ConditionDescriptor {
    ConditionDescriptor {
        id: ConditionId::new(DOMAIN, "can"),
        summary: "true while the body the player is driving has this capability enabled",
        params: &[VERB],
    }
}

/// `body.can` — see [`can_descriptor`]. An unknown verb is `Unanswerable`; a
/// known verb the body does not have is `NotSatisfied`.
///
/// ⚠ ANY PLAYER OR DRIVEN BODY SATISFIES IT, which is `inventory.holds`' rule
/// and is the same choice for the same reason: possession moves the participant
/// between bodies, so "the player" is a population and not an entity. ⛔ A
/// consequence worth stating: while a participant drives a vessel that can
/// climb, a wall gated on climbing opens — and that is the intent, because the
/// body meeting the route is the one that would climb it.
pub fn can(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let Some(verb) = args[0].as_name() else {
        return ConditionOutcome::unanswerable("`verb` must be a name");
    };
    // Asked against the DEFAULT set first, so an unknown verb is a content
    // diagnostic even in a composition with no body in it. Resolving the body
    // first would report "nothing is driving" for a misspelling.
    if ability_named(&AbilitySet::default(), verb).is_none() {
        return ConditionOutcome::unanswerable(format!(
            "no ability is spelled `{verb}`; `body.can` reads `AbilitySet` field names exactly"
        ));
    }
    let enabled = |set: &BodyAbilities| ability_named(&set.abilities, verb) == Some(true);
    ConditionOutcome::from_bool(driven_bodies(world, enabled), || {
        WhyNot::new("body.can", verb, "no body a participant is driving has it")
    })
}

/// EVERY BODY A PARTICIPANT IS ACTUALLY DRIVING — the population both conditions
/// in this file ask, and the one thing they must not get wrong.
///
/// ⛔⛔ IT IS NOT `PlayerEntity`. Possession MOVES control:
/// `control/authority.rs:39` removes `DrivingParticipant` from the home avatar
/// and inserts it on the possessed target, and the home avatar KEEPS
/// `PlayerEntity` throughout. So a predicate that accepted either marker
/// answered from the resting body — a home avatar that can wall-climb would
/// open a climbing route for a vessel that cannot, and a 30-unit home would open
/// a crawlspace for an 80-unit vessel. Measured against the possession authority
/// rather than assumed.
///
/// ⭐ These are `ambition_held_items::DrivenBodies`' semantics — the controlled
/// subject plus real seat holders — reached without that `SystemParam`, because
/// an authored condition is a plain `fn(&World, &[AuthoredArg])` and cannot take
/// one. The AUTHORITY is shared even though the access is not; if that struct's
/// definition of driven changes, this must follow it.
///
/// ⚠ IT IS STILL EXISTENTIAL, and that is a deliberate non-decision. With one
/// participant the population is one body. What "any driven body qualifies"
/// should mean when several participants drive different bodies is the co-op
/// gate question `capability-progression-and-world-gating.md` leaves open, and
/// answering it here by accident would settle it in the wrong place.
fn driven_bodies<'w, T: bevy::prelude::Component>(
    world: &'w World,
    mut predicate: impl FnMut(&T) -> bool,
) -> bool {
    let controlled = world
        .get_resource::<ambition_platformer2d_shared_tangle::markers::ControlledSubject>()
        .and_then(|subject| subject.0)
        .and_then(|entity| world.get_entity(entity).ok())
        .and_then(|entity| entity.get::<T>())
        .is_some_and(&mut predicate);
    if controlled {
        return true;
    }
    // ⛔⛔ THE FALLBACK IS AN EXISTENTIAL, AND IN CO-OP THAT IS A RULING NOBODY
    // TOOK. With one participant it names the one driven body and there is
    // nothing to decide. With two seats it answers YES when EITHER driver's body
    // satisfies the condition — so a wall gated on climbing opens for the seat
    // that cannot climb, because the other one can.
    //
    // ⭐ THAT IS EXACTLY THE OPEN DESIGN QUESTION this page's owner plan lists
    // ("how should co-op gates behave when one participant can traverse and
    // another cannot") — see awaiting-maintainer-decision.md #54. It is named
    // here rather than settled here, because a gate solid is a property of the
    // WORLD and a per-participant answer would need the wall to stop being one:
    // that is a mechanism change, not a predicate change.
    //
    // ⚠ Widening the population is what made this live: while the predicate read
    // `PlayerEntity`, the same OR was there and was equally a ruling. The bug it
    // hid was worse, so this is not a regression — it is a latent decision that
    // moving to the driven population made visible.
    world
        .try_query_filtered::<&T, bevy::prelude::With<
            ambition_characters::control::DrivingParticipant,
        >>()
        .is_some_and(|mut bodies| bodies.iter(world).any(predicate))
}


/// One `AbilitySet` field, by its authored name — `None` for a name the set has
/// no field for.
///
/// ⛔⛔ THE DESTRUCTURE IS THE GUARD, AND `deny(unused_variables)` IS WHAT MAKES
/// IT ONE. A hand-kept list of 29 field names goes stale the first time somebody
/// adds a capability, and a stale one fails SILENTLY: the new verb is simply
/// unaskable, and an author who names it gets "no ability is spelled that" for a
/// field that exists. Binding every field by name and denying unused bindings
/// turns adding a field into a compile error here, which is the only moment the
/// author of the new capability is still looking.
#[deny(unused_variables)]
fn ability_named(set: &AbilitySet, verb: &str) -> Option<bool> {
    let AbilitySet {
        move_horizontal,
        jump,
        variable_jump,
        double_jump,
        fast_fall,
        wall_jump,
        wall_cling,
        wall_climb,
        dash,
        double_dash,
        fly,
        fly_toggle,
        blink,
        precision_blink,
        blink_through_soft_walls,
        blink_through_hard_walls,
        attack,
        pogo,
        directional_primary,
        directional_special,
        rebound,
        reset,
        ledge_grab,
        swim,
        glide,
        dodge,
        shield,
        grab,
        interact,
    } = *set;
    Some(match verb {
        "move_horizontal" => move_horizontal,
        "jump" => jump,
        "variable_jump" => variable_jump,
        "double_jump" => double_jump,
        "fast_fall" => fast_fall,
        "wall_jump" => wall_jump,
        "wall_cling" => wall_cling,
        "wall_climb" => wall_climb,
        "dash" => dash,
        "double_dash" => double_dash,
        "fly" => fly,
        "fly_toggle" => fly_toggle,
        "blink" => blink,
        "precision_blink" => precision_blink,
        "blink_through_soft_walls" => blink_through_soft_walls,
        "blink_through_hard_walls" => blink_through_hard_walls,
        "attack" => attack,
        "pogo" => pogo,
        "directional_primary" => directional_primary,
        "directional_special" => directional_special,
        "rebound" => rebound,
        "reset" => reset,
        "ledge_grab" => ledge_grab,
        "swim" => swim,
        "glide" => glide,
        "dodge" => dodge,
        "shield" => shield,
        "grab" => grab,
        "interact" => interact,
        _ => return None,
    })
}

const OPENING: ParamSpec = ParamSpec {
    name: "height",
    kind: ParamKind::Number,
    summary: "the vertical opening in world units; the body fits when it is no taller",
};

/// `body.fits(height)` — is the body short enough to pass through this opening?
///
/// The second body-property gate family, and the one Ambition's goal names
/// first: *"gate routes through body size."*
pub fn fits_descriptor() -> ConditionDescriptor {
    ConditionDescriptor {
        id: ConditionId::new(DOMAIN, "fits"),
        summary: "true while the body the player is driving is no taller than this opening",
        params: &[OPENING],
    }
}

/// `body.fits` — see [`fits_descriptor`].
///
/// ⭐ IT READS THE BODY'S CURRENT SIZE, NOT ITS STANDING BASELINE, and that is
/// the same choice [`can`] makes for the same reason. `BodyKinematics::size` is
/// what the stances write; `BodyBaseSize` is the authored standing baseline they
/// derive FROM. A gate that asked the baseline would refuse a body that
/// physically fits — the world disagreeing with itself about a hole in it — so
/// crouching, morphing and any future stance count.
///
/// ⛔ AND IT IS THE BODY'S OWN-FRAME HEIGHT, WHICH IS NOT THE SAME AS ITS
/// WORLD-SPACE ONE. `BodyKinematics::size` is in the body's frame;
/// `aabb_oriented(gravity_dir)` — what the collision doctrine actually sweeps —
/// SWAPS width and height under sideways gravity, because the body lies along
/// the wall. So under flipped or sideways gravity this reads a different number
/// than the collision footprint does, deliberately.
///
/// ⭐ Gravity-independent is the right rule for a ROUTE, and the alternative is
/// worse than it sounds: a world-space reading would make one authored wall open
/// and close as gravity flipped, so a level author could not say what their own
/// crawlspace means without knowing which way gravity was pointing when the
/// player arrived. The body and the passage rotate together; "how tall is this
/// creature" does not.
///
/// ⛔ ONE PARAMETER, DELIBERATELY. A width question is a different condition,
/// not a second argument: in a side-on platformer you traverse an opening
/// horizontally, so "am I short enough" is the physical question and "am I
/// narrow enough" is a different route shape that should have to be published
/// and named before it can be authored.
///
/// ⚠ A NEGATIVE OR ZERO OPENING IS UNANSWERABLE rather than false. No body has
/// a non-positive height, so `false` would be the right answer for the wrong
/// reason and would hide an authoring mistake behind a wall that correctly
/// never opens.
pub fn fits(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let Some(opening) = args[0].as_number() else {
        return ConditionOutcome::unanswerable("`height` must be a number");
    };
    if !(opening > 0.0) {
        return ConditionOutcome::unanswerable(format!(
            "`{opening}` is not an opening; `body.fits` takes a positive height in world units"
        ));
    }
    let opening = opening as f32;
    let short_enough = |body: &BodyKinematics| body.size.y <= opening;
    // The DRIVEN population, for the reason [`driven_bodies`] states: a
    // possessed vessel's height is the one that has to fit, not the resting
    // home avatar's.
    ConditionOutcome::from_bool(driven_bodies(world, short_enough), || {
        WhyNot::new(
            "body.fits",
            format!("{opening}"),
            "no body a participant is driving is short enough",
        )
    })
}

/// Publishes the body domain's conditions.
///
/// One plugin for one registration line, matching
/// [`WorldFactConditionsPlugin`](crate::world_facts::WorldFactConditionsPlugin)
/// and the inventory's: composition adds it, and nothing else in the engine
/// learns that a body can be asked what it is capable of.
pub struct BodyCapabilityConditionsPlugin;

impl bevy::prelude::Plugin for BodyCapabilityConditionsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use ambition_platformer2d_shared_tangle::authored_logic::PublishCondition;
        app.publish_condition(can_descriptor(), can);
        app.publish_condition(fits_descriptor(), fits);
    }
}

#[cfg(test)]
mod tests;
