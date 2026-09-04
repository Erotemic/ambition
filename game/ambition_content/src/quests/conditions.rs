//! Authored QUEST conditions — "is this quest under way?"
//!
//! ⭐⭐ THE FIRST CONDITION PUBLISHED BY THE GAME RATHER THAN AN ENGINE CRATE,
//! and that is the correct home rather than a compromise. Quests are Ambition
//! content: the engine has no quest crate, the roster lives in
//! `crate::quest::default_quest_specs`, and the pump that advances them is
//! registered by [`super::AmbitionQuestContentPlugin`]. ⇒ *"A domain owns its
//! own publication"* puts this here, and it demonstrates that the condition
//! catalog is extensible by a GAME and not only by the engine — a composition
//! without Ambition's quests simply never sees the question.
//!
//! ⛔⛔ IT RETIRES A SECOND AUTHORITY. `YarnStateMirrorData::quests_active` held
//! a per-frame projection of exactly this fact so a bespoke Yarn function
//! `quest_active(id)` could answer it synchronously. That is the shape both
//! `authored_conditions.rs` and `yarn_vocabulary.rs` already name as refused,
//! and it is the third slice to leave the mirror — after `flag` (when
//! `world.flag_set` landed) and `bosses_cleared` (when `boss.cleared` did).

use ambition_persistence::save_data::PersistedQuestState;
use ambition_platformer2d_shared_tangle::authored_logic::{
    AuthoredArg, ConditionDescriptor, ConditionId, ConditionOutcome, ParamKind, ParamSpec, WhyNot,
};
use bevy::prelude::World;

/// The domain segment every condition in this file is published under.
pub const DOMAIN: &str = "quest";

const QUEST: ParamSpec = ParamSpec {
    name: "quest",
    kind: ParamKind::Name,
    summary: "the quest id, as the authored roster and the save spell it",
};

/// `quest.active(quest)` — is the player in the middle of this one?
pub fn active_descriptor() -> ConditionDescriptor {
    ConditionDescriptor {
        id: ConditionId::new(DOMAIN, "active"),
        summary: "true while the named quest is in progress, as the save records it",
        params: &[QUEST],
    }
}

/// `quest.active` — see [`active_descriptor`].
///
/// ⭐ `InProgress` AND NOTHING ELSE, deliberately, and this one has FOUR states
/// rather than the three `encounter.cleared` and `boss.cleared` face —
/// `NotStarted`, `InProgress`, `Completed`, `Failed`. ⛔ Which makes the naming
/// matter more, not less: *"active"* is a question a line of dialogue actually
/// has ("are you still looking for it?"), and it is NOT the same question as
/// *"did you finish it"*. A `quest.completed` is a second NAMED question,
/// published when something wants it — not a `state_is(quest, state)` accessor,
/// which would be the key-value fact database the world-facts program refuses,
/// arriving one enum at a time.
///
/// ⚠ AN UNRECORDED QUEST IS `NotSatisfied`, not `Unanswerable`: the save's
/// accessor reconstructs a missing row as `NotStarted`, so absence is a real
/// state. What IS unanswerable is having no save layer at all.
pub fn active(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let Some(quest) = args[0].as_name() else {
        return ConditionOutcome::unanswerable("`quest` must be a name");
    };
    let Some(save) = world.get_resource::<ambition_persistence::save::AmbitionGameSave>() else {
        return ConditionOutcome::unanswerable(
            "no save layer is installed in this composition, so no quest progress is recorded",
        );
    };
    let (state, _step) = save.data().quest(quest);
    ConditionOutcome::from_bool(state == PersistedQuestState::InProgress, || {
        WhyNot::new(
            "quest.active",
            quest,
            match state {
                PersistedQuestState::NotStarted => "it has not been started",
                PersistedQuestState::Completed => "it is already finished",
                PersistedQuestState::Failed => "it ended in failure",
                PersistedQuestState::InProgress => unreachable!("that is the satisfied arm"),
            },
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_persistence::save::AmbitionGameSave;
    use bevy::prelude::App;

    fn ask(world: &World, quest: &str) -> ConditionOutcome {
        active(world, &[AuthoredArg::Name(quest.to_string())])
    }

    fn world_with(quest: &str, state: PersistedQuestState) -> App {
        let mut app = App::new();
        app.insert_resource(AmbitionGameSave::default());
        app.world_mut()
            .resource_mut::<AmbitionGameSave>()
            .data_mut()
            .set_quest(quest.to_string(), state, 0);
        app
    }

    /// ⛔ ONLY `InProgress` IS ACTIVE, and BOTH of the ways a quest stops being
    /// active are asserted. A predicate that meant "started" would pass the
    /// `Completed` arm; one that meant "not finished" would pass `NotStarted`.
    #[test]
    fn a_quest_is_active_only_while_it_is_in_progress() {
        let app = world_with("pirate_treasure", PersistedQuestState::InProgress);
        assert_eq!(ask(app.world(), "pirate_treasure"), ConditionOutcome::Satisfied);

        for finished in [PersistedQuestState::Completed, PersistedQuestState::Failed] {
            let app = world_with("pirate_treasure", finished);
            assert!(
                matches!(ask(app.world(), "pirate_treasure"), ConditionOutcome::NotSatisfied(_)),
                "a {finished:?} quest is no longer active"
            );
        }
    }

    /// ⚠ AND AN UNSTARTED QUEST IS NOT ACTIVE EITHER — the why-not says which of
    /// the three it is, in the domain's words, so a reader is not left guessing
    /// whether the quest was finished or never begun.
    #[test]
    fn an_unrecorded_quest_is_not_active_and_says_which_state_it_is_in() {
        let app = world_with("pirate_treasure", PersistedQuestState::InProgress);
        let outcome = ask(app.world(), "a_quest_nobody_offered");
        let why = match &outcome {
            ConditionOutcome::NotSatisfied(why) => why,
            other => panic!("an unrecorded quest is not active, not {other:?}"),
        };
        assert_eq!(why.term, "quest.active");
        assert_eq!(why.subject, "a_quest_nobody_offered");
        assert!(
            why.observed.contains("not been started"),
            "the why-not names the state: {}",
            why.observed
        );
    }

    /// ⛔ NO SAVE LAYER IS `Unanswerable`, not `false` — a composition with no
    /// save recorded nothing, and "not active" would be a confident claim about
    /// a world with no memory.
    #[test]
    fn a_composition_with_no_save_layer_cannot_answer() {
        let app = App::new();
        assert!(
            matches!(ask(app.world(), "pirate_treasure"), ConditionOutcome::Unanswerable(_)),
            "with no save layer nothing recorded progress"
        );
    }
}
