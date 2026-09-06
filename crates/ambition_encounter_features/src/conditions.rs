//! Authored ENCOUNTER conditions — "what became of this arena?"
//!
//! The encounter capability publishing its own route-facing vocabulary, from
//! its own plugin. Nothing central learns that encounters can be asked about;
//! a composition that installs encounters gets the question with them, and one
//! that does not never sees it.
//!
//! ⛔ IT IS NOT `world.switch_on`, and the difference is which fact is being
//! asked for. A wave encounter's completion latches every switch linked to it
//! (`systems.rs`), so the two answers usually agree — but the switch is the
//! MECHANISM's state, flippable by a reset switch the player can walk up to,
//! and this is the OUTCOME the save recorded. A route gated on "you cleared
//! this arena" must not reopen because somebody reset the lights.

use ambition_persistence::save_data::PersistedEncounterState;
use ambition_platformer2d_shared_tangle::authored_logic::{
    AuthoredArg, ConditionDescriptor, ConditionId, ConditionOutcome, ParamKind, ParamSpec, WhyNot,
};
use bevy::prelude::World;

/// The domain segment every condition in this file is published under.
pub const DOMAIN: &str = "encounter";

const ENCOUNTER: ParamSpec = ParamSpec {
    name: "encounter",
    kind: ParamKind::Name,
    summary: "the encounter id, as the authored level and the save spell it",
};

/// `encounter.cleared(encounter)` — did the player finish this one?
pub fn cleared_descriptor() -> ConditionDescriptor {
    ConditionDescriptor {
        id: ConditionId::new(DOMAIN, "cleared"),
        summary: "true once the named encounter has been cleared, as the save records it",
        params: &[ENCOUNTER],
    }
}

/// `encounter.cleared` — see [`cleared_descriptor`].
///
/// ⭐ THE FIRST CONDITION OVER A NON-BOOLEAN DURABLE FACT, and it publishes ONE
/// of the three states rather than a `state_is(encounter, state)` that takes the
/// state as an argument. `Cleared` is the question a route actually has;
/// `Failed` and `Untouched` are both "not cleared" to a door, and they differ in
/// a way only a design that wants them would need. A generic accessor here would
/// be the key-value fact database the world-facts program refuses, arriving one
/// enum at a time — so a second state becomes a second NAMED question, published
/// when something wants it.
///
/// ⚠ AN UNRECORDED ENCOUNTER IS `NotSatisfied`, not `Unanswerable`. The save's
/// own accessor reconstructs a missing row as `Untouched` and says why — "not
/// usually written to disk; missing entries reconstruct to this value" — so
/// absence is a real state here rather than a missing subject. What IS
/// unanswerable is having no save layer, because then nothing recorded anything.
pub fn cleared(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let Some(encounter) = args[0].as_name() else {
        return ConditionOutcome::unanswerable("`encounter` must be a name");
    };
    let Some(save) = world.get_resource::<ambition_persistence::save::AmbitionGameSave>() else {
        return ConditionOutcome::unanswerable(
            "no save layer is installed in this composition, so no encounter outcome is recorded",
        );
    };
    let state = save.data().encounter(encounter);
    ConditionOutcome::from_bool(state == PersistedEncounterState::Cleared, || {
        WhyNot::new(
            "encounter.cleared",
            encounter,
            match state {
                PersistedEncounterState::Untouched => "it has never been finished",
                PersistedEncounterState::Failed => "the last attempt ended in a death",
                PersistedEncounterState::Cleared => unreachable!("that is the satisfied arm"),
            },
        )
    })
}

/// Publishes the encounter domain's conditions.
///
/// One plugin for one registration line, matching the world-fact, inventory and
/// body domains: composition adds it, and nothing else in the engine learns
/// that an encounter can be asked what became of it.
pub struct EncounterConditionsPlugin;

impl bevy::prelude::Plugin for EncounterConditionsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use ambition_platformer2d_shared_tangle::authored_logic::PublishCondition;
        app.publish_condition(cleared_descriptor(), cleared);
    }
}

#[cfg(test)]
#[path = "conditions_tests.rs"]
mod conditions_tests;
