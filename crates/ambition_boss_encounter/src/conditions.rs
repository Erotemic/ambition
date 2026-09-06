//! Authored BOSS conditions — "did the player beat this one?"
//!
//! The boss capability publishing its own route- and dialogue-facing
//! vocabulary, from its own plugin, beside the systems that write the fact.
//!
//! ⛔⛔ THIS RETIRES A SECOND AUTHORITY RATHER THAN ADDING A VERB, and that is
//! the whole justification. `YarnStateMirrorData::bosses_cleared`
//! (`ambition_dialog/src/bindings.rs`) held a per-frame projection of exactly
//! this fact so a bespoke Yarn function `boss_cleared(id)` could answer it
//! synchronously — and both modules already named that as the thing this
//! project refuses: *"the mirror remains only for facts the catalog cannot
//! answer"* (`authored_conditions.rs`) and *"Two mechanisms answering one
//! question is exactly the second authority this project refuses elsewhere"*
//! (`ambition_content/src/yarn_vocabulary.rs`).
//!
//! ⭐ THE MIGRATION HAS A PRECEDENT IN THE SAME FILE. The mirror's FLAG slice is
//! already gone, because `world.flag_set` answers it live. This is the same move
//! for the next fact, and `quest.active` is its sibling.
//!
//! ⚠ NOT `encounter.cleared`. `encounters` (`PersistedEncounter`) and `bosses`
//! (`PersistedBossDefeat`) are separate save fields (`save_data.rs:313`, `:317`)
//! with separate accessors, so the existing encounter condition does not answer
//! this question. Checked rather than assumed — "a boss is an encounter" is the
//! plausible reading that would have made this look already-done.

use ambition_persistence::save_data::PersistedEncounterState;
use ambition_platformer2d_shared_tangle::authored_logic::{
    AuthoredArg, ConditionDescriptor, ConditionId, ConditionOutcome, ParamKind, ParamSpec, WhyNot,
};
use bevy::prelude::World;

/// The domain segment every condition in this file is published under.
pub const DOMAIN: &str = "boss";

const BOSS: ParamSpec = ParamSpec {
    name: "boss",
    kind: ParamKind::Name,
    summary: "the boss id, as the authored placement and the save spell it",
};

/// `boss.cleared(boss)` — has this boss been beaten?
pub fn cleared_descriptor() -> ConditionDescriptor {
    ConditionDescriptor {
        id: ConditionId::new(DOMAIN, "cleared"),
        summary: "true once the named boss has been defeated, as the save records it",
        params: &[BOSS],
    }
}

/// `boss.cleared` — see [`cleared_descriptor`].
///
/// ⭐ ONE NAMED STATE, not `state_is(boss, state)`, for the reason
/// `encounter.cleared` records: `Failed` and `Untouched` are both "not beaten"
/// to a door or a line of dialogue, and a generic accessor would be the
/// key-value fact database the world-facts program refuses, arriving one enum at
/// a time. A second state becomes a second named question when something wants
/// it.
///
/// ⚠ AN UNRECORDED BOSS IS `NotSatisfied`, not `Unanswerable` — the save's own
/// accessor reconstructs a missing row as `Untouched`, so absence is a real
/// state rather than a missing subject. What IS unanswerable is having no save
/// layer, because then nothing recorded anything.
pub fn cleared(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let Some(boss) = args[0].as_name() else {
        return ConditionOutcome::unanswerable("`boss` must be a name");
    };
    let Some(save) = world.get_resource::<ambition_persistence::save::AmbitionGameSave>() else {
        return ConditionOutcome::unanswerable(
            "no save layer is installed in this composition, so no boss outcome is recorded",
        );
    };
    let state = save.data().boss(boss);
    ConditionOutcome::from_bool(state == PersistedEncounterState::Cleared, || {
        WhyNot::new(
            "boss.cleared",
            boss,
            match state {
                PersistedEncounterState::Untouched => "it has never been beaten",
                PersistedEncounterState::Failed => "the last attempt ended in a death",
                PersistedEncounterState::Cleared => unreachable!("that is the satisfied arm"),
            },
        )
    })
}

/// Publishes the boss domain's conditions.
///
/// One plugin for one registration line, matching the world-fact, inventory,
/// body and encounter domains: composition adds it, and nothing else in the
/// engine learns that a boss can be asked whether it was beaten.
pub struct BossConditionsPlugin;

impl bevy::prelude::Plugin for BossConditionsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use ambition_platformer2d_shared_tangle::authored_logic::PublishCondition;
        app.publish_condition(cleared_descriptor(), cleared);
    }
}

#[cfg(test)]
mod tests;
