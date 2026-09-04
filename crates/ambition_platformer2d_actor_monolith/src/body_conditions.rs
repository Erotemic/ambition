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
use ambition_platformer2d_core::body_clusters::BodyAbilities;
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
    let player_can = world
        .try_query_filtered::<&BodyAbilities, bevy::prelude::With<
            ambition_platformer2d_shared_tangle::markers::PlayerEntity,
        >>()
        .is_some_and(|mut bodies| bodies.iter(world).any(enabled));
    let driven_can = world
        .try_query_filtered::<&BodyAbilities, bevy::prelude::With<
            ambition_characters::control::DrivingParticipant,
        >>()
        .is_some_and(|mut bodies| bodies.iter(world).any(enabled));
    ConditionOutcome::from_bool(player_can || driven_can, || {
        WhyNot::new("body.can", verb, "no player or driven body has it")
    })
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
    }
}

#[cfg(test)]
mod tests;
