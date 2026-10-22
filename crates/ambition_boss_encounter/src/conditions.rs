//! Authored boss conditions: "did the player beat this one?"
//!
//! The boss capability publishes its own route- and dialogue-facing
//! vocabulary, from its own plugin, beside the systems that write the fact.
//!
//! This condition reads the save directly and is the only answer to the
//! question. Do not add a per-frame mirror of this fact for a bespoke Yarn
//! function: two mechanisms answering one question is a second authority.
//! (`world.flag_set` and `quest.active` follow the same rule.)
//!
//! This is not `encounter.cleared`. `encounters` (`PersistedEncounter`) and
//! `bosses` (`PersistedBossDefeat`) are separate save fields with separate
//! accessors.

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
/// One named state, not `state_is(boss, state)`: `Failed` and `Untouched`
/// are both "not beaten" to a door or a line of dialogue, and a generic
/// accessor would become a key-value fact database. Add another named
/// question when something needs another state.
///
/// "Not recorded" is also what a nonexistent boss id looks like, so a typo
/// would be `NotSatisfied` forever. Boss progress is keyed only by stable
/// authored encounter/placement ids, and content validation guards this:
/// `every_authored_boss_cleared_call_names_a_real_boss_placement` resolves
/// each authored argument through `boss_placement_id` (the production function
/// `convert_boss_spawn` uses), so a behavior id fails; a Python alias guard
/// catches other typos.
///
/// So this evaluator is permissive on purpose: the roster is in the authored
/// worlds, not in any resource this `&World` can reach, and the check runs
/// where the roster is.
///
/// An unrecorded boss is `NotSatisfied`, not `Unanswerable`: the save
/// reconstructs a missing row as `Untouched`, so absence is a real state. Only
/// a missing save layer is unanswerable.
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

/// `boss.engaged(boss)` — has this boss come out of its sleep and its intro?
pub fn engaged_descriptor() -> ConditionDescriptor {
    ConditionDescriptor {
        id: ConditionId::new(DOMAIN, "engaged"),
        summary: "true once the named boss fights (it has left its sleep and its intro) or has been defeated",
        params: &[BOSS],
    }
}

/// `boss.engaged` — see [`engaged_descriptor`]. A wall the boss breaks open
/// when it starts to fight (the Inugami's, out of the throne room) waits on
/// it: it stands while the boss sleeps or plays its intro, and stays open once
/// the boss is beaten. A room built again builds the boss asleep, so the wall
/// stands again.
///
/// The fight is the boss's live body (`BossEncounter`) in a phase after its
/// intro; the defeat is the save's record, as `boss.cleared` reads it.
pub fn engaged(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let Some(boss) = args[0].as_name() else {
        return ConditionOutcome::unanswerable("`boss` must be a name");
    };
    let beaten = world
        .get_resource::<ambition_persistence::save::AmbitionGameSave>()
        .is_some_and(|save| save.data().boss(boss) == PersistedEncounterState::Cleared);
    let fighting = world.try_query::<(&crate::BossConfig, &crate::BossEncounter)>().is_some_and(|mut bosses| {
        bosses.iter(world).any(|(config, status)| {
            config.id == boss
                && !matches!(
                    status.encounter_phase(),
                    crate::BossEncounterPhase::Dormant | crate::BossEncounterPhase::Intro
                )
        })
    });
    ConditionOutcome::from_bool(beaten || fighting, || WhyNot::new("boss.engaged", boss, "it sleeps, or plays its intro"))
}

/// `boss.past_first_phase(boss)` — has this boss been driven out of its first
/// phase?
pub fn past_first_phase_descriptor() -> ConditionDescriptor {
    ConditionDescriptor {
        id: ConditionId::new(DOMAIN, "past_first_phase"),
        summary: "true once the named boss fights its second phase or later, or has been defeated",
        params: &[BOSS],
    }
}

/// `boss.past_first_phase` — see [`past_first_phase_descriptor`]. A wall the
/// boss breaks open when you have hurt it enough (the Inugami's, out of the
/// throne room: Jon, 2026-10-22, "You shouldn't be able to move into a chase
/// zone until you damage him enough in phase 1") waits on it. It stands through
/// the boss's transition, which is the boss breaking it, and opens when the
/// second phase starts; it stays open once the boss is beaten.
pub fn past_first_phase(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let Some(boss) = args[0].as_name() else {
        return ConditionOutcome::unanswerable("`boss` must be a name");
    };
    let beaten = world
        .get_resource::<ambition_persistence::save::AmbitionGameSave>()
        .is_some_and(|save| save.data().boss(boss) == PersistedEncounterState::Cleared);
    let past = world.try_query::<(&crate::BossConfig, &crate::BossEncounter)>().is_some_and(|mut bosses| {
        bosses.iter(world).any(|(config, status)| {
            config.id == boss
                && matches!(
                    status.encounter_phase(),
                    crate::BossEncounterPhase::Phase2
                        | crate::BossEncounterPhase::Stagger
                        | crate::BossEncounterPhase::Enrage
                        | crate::BossEncounterPhase::Death
                )
        })
    });
    ConditionOutcome::from_bool(beaten || past, || {
        WhyNot::new("boss.past_first_phase", boss, "it is asleep, or in its first phase or its transition")
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
        app.publish_condition(engaged_descriptor(), engaged);
        app.publish_condition(past_first_phase_descriptor(), past_first_phase);
    }
}

#[cfg(test)]
mod tests;
