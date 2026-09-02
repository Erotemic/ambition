//! What the item domain lets authored content ask about custody.
//!
//! the domain answers; the rule asks. Nothing outside this file needs to
//! know that custody is spelled [`ItemCustody`], that it has two variants, or
//! that a held object keeps its entity — all of which this domain is entitled to
//! change. An author writes `custody.is_held(<occurrence>)` and gets an answer.
//!
//! `Unanswerable` is doing real work here. *"Is the axe held?"* asked about
//! an occurrence this world never authored is not `false`: false would mean the
//! axe exists and nobody has it, and a wall that opens on the negation would
//! stand open in a level that has no axe at all. That distinction is the whole
//! reason the outcome is not a `bool`.

use ambition_platformer2d_shared_tangle::authored_logic::{
    AuthoredArg, ConditionDescriptor, ConditionId, ConditionOutcome, ParamKind, ParamSpec,
};
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::World;

use super::ItemCustody;

/// The domain segment every condition in this file is published under.
pub const DOMAIN: &str = "custody";

const OCCURRENCE: ParamSpec = ParamSpec {
    name: "occurrence",
    kind: ParamKind::Reference,
    summary: "the authored occurrence being asked about",
};

/// `custody.is_held(occurrence)` — is this object in somebody's hands?
pub fn is_held_descriptor() -> ConditionDescriptor {
    ConditionDescriptor {
        id: ConditionId::new(DOMAIN, "is_held"),
        summary: "true while the named occurrence is carried by any body",
        params: &[OCCURRENCE],
    }
}

/// `custody.is_held` — see [`is_held_descriptor`].
///
/// it answers about the OCCURRENCE, not about a hand, so an object passed
/// between bodies never flickers false. A condition phrased "is body B holding
/// it" would be a different question and would need a second parameter; this one
/// deliberately does not ask who, because most gates do not care.
pub fn is_held(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let Some(wanted) = args[0].as_reference() else {
        return ConditionOutcome::unanswerable("`occurrence` must be a prepared reference");
    };
    // `try_query` rather than `query`: a composition with no item plugin
    // installed has never registered `ItemCustody`, and that is genuinely
    // unanswerable rather than false.
    let Some(mut objects) = world.try_query::<(&SimId, &ItemCustody)>() else {
        return ConditionOutcome::unanswerable(
            "no item domain is installed in this composition, so nothing has custody",
        );
    };
    // NOT `.any()` on a match — the question is about ONE identity, and
    // finding it is what separates "not held" from "not there". Iteration order
    // is irrelevant because at most one live occurrence carries an identity;
    // that invariant is the occurrence ledger's, and it is why this can stop at
    // the first hit without depending on archetype order.
    let found = objects
        .iter(world)
        .find(|(sim_id, _)| *sim_id == wanted)
        .map(|(_, custody)| *custody);
    match found {
        Some(custody) => ConditionOutcome::from_bool(!custody.in_world(), || {
            ambition_platformer2d_shared_tangle::authored_logic::WhyNot::new(
                "item.is_held",
                wanted.as_str(),
                "the occurrence is lying in the world, in nobody's custody",
            )
        }),
        None => ConditionOutcome::unanswerable(format!(
            "no live occurrence `{wanted}` — it may be authored in an unloaded room, \
             consumed, or never authored at all"
        )),
    }
}
