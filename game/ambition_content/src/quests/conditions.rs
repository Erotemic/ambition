//! Authored quest conditions: "is this quest under way?"
//!
//! The game publishes this condition, not an engine crate. Quests are Ambition
//! content: the engine has no quest crate, the roster lives in
//! `crate::quest::default_quest_specs`, and
//! [`super::AmbitionQuestContentPlugin`] registers the pump that advances them.
//! A domain owns its own publication, so the condition catalog is extensible
//! by a game. A composition without Ambition's quests never sees the
//! question.
//!
//! It retires a second authority. `YarnStateMirrorData::quests_active` held <!-- cite-ok: records the second authority this retired; the name is gone by design -->
//! a per-frame projection of this fact so a bespoke Yarn function
//! `quest_active(id)` could answer it synchronously. `authored_conditions.rs`
//! and `yarn_vocabulary.rs` both refuse that shape.

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
/// `InProgress` and nothing else. A quest has four states (`NotStarted`,
/// `InProgress`, `Completed`, `Failed`). *"Active"* is a question dialogue
/// asks ("are you still looking for it?"), and it is not *"did you finish
/// it"*. A `quest.completed` would be a second named question, published when
/// needed, not a `state_is(quest, state)` accessor: that would be the
/// key-value fact database the world-facts program refuses.
///
/// An unrecorded quest is `NotSatisfied`, not `Unanswerable`: the save's
/// accessor reconstructs a missing row as `NotStarted`, so absence is a real
/// state. Having no save layer at all is unanswerable.
///
/// But "unrecorded" and "not a quest" are two absences the save cannot tell
/// apart: `save.data().quest(id)` reconstructs any string as `NotStarted`.
/// Without this check a misspelt authored id would be `NotSatisfied` forever,
/// with no diagnostic. That is the permissive default [`ParamKind::Name`]
/// warns about: preparation holds no `World`, so the refusal happens here.
///
/// The roster is the `QuestRegistry`, not `default_quest_specs()`: the
/// registry is what the composition ran, including quests added through
/// `ensure`. Validating against the static list would reject a registered
/// quest.
///
/// It is consulted only when `initialized`, because a system
/// (`populate_quest_registry`) fills the registry. An empty registry before
/// that is "not known yet", not "no such quest". Without a roster this answers
/// from the save, which is what a save-only composition (a dialogue test)
/// gets.
pub fn active(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let Some(quest) = args[0].as_name() else {
        return ConditionOutcome::unanswerable("`quest` must be a name");
    };
    // The roster before the save, as `body.can` does for its verb: resolving
    // progress first would answer "not started" for a misspelling.
    if let Some(registry) = world.get_resource::<crate::quest::QuestRegistry>() {
        if registry.initialized && registry.get(quest).is_none() {
            return ConditionOutcome::unanswerable(format!(
                "no quest is spelled `{quest}`; the registry knows {}",
                registry.quests.len()
            ));
        }
    }
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

/// Publishes the quest domain's questions, and nothing else.
///
/// Separate from the pump, like `BossConditionsPlugin`. A composition may ask
/// whether a quest is active without running the quest pump: a dialogue test
/// does, and so does a host with the save but not the progression schedule.
///
/// Otherwise, anything that needed only the question would re-derive it with
/// `publish_condition(active_descriptor())`: a second place deciding what the
/// quest domain publishes.
pub struct QuestConditionsPlugin;

impl bevy::prelude::Plugin for QuestConditionsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use ambition_platformer2d_shared_tangle::authored_logic::PublishCondition;
        app.publish_condition(active_descriptor(), active);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_persistence::save::AmbitionGameSave;
    use bevy::prelude::App;

    fn ask(world: &World, quest: &str) -> ConditionOutcome {
        active(world, &[AuthoredArg::Name(quest.to_string())])
    }

    /// A world that also has a populated roster, so the misspelling check is
    /// live. `initialized` is set by hand: this file tests the question, not the
    /// pump.
    fn world_with_roster(quest: &str, state: PersistedQuestState, known: &[&str]) -> App {
        let mut app = world_with(quest, state);
        let mut registry = crate::quest::QuestRegistry::default();
        for id in known {
            registry.ensure(ambition_persistence::quest::QuestSpec::new(
                *id,
                "a title",
                "a summary",
                Vec::new(),
            ));
        }
        registry.initialized = true;
        app.insert_resource(registry);
        app
    }

    /// A quest id the roster does not know is `Unanswerable` (a content
    /// diagnostic), not the `NotSatisfied` the save alone would give, because
    /// `save.data().quest` reconstructs any string as `NotStarted`.
    #[test]
    fn a_quest_the_roster_never_heard_of_is_a_diagnostic_not_a_no() {
        let app = world_with_roster(
            "pirate_treasure",
            PersistedQuestState::InProgress,
            &["pirate_treasure"],
        );
        assert!(
            matches!(ask(app.world(), "pirat_treasure"), ConditionOutcome::Unanswerable(_)),
            "a misspelling must not be reported as a quest that has not been started"
        );
    }

    /// The diagnostic must not swallow the real no. A known quest the player has
    /// not begun is still `NotSatisfied`.
    #[test]
    fn a_real_quest_that_is_merely_unstarted_is_still_a_plain_no() {
        let app = world_with_roster(
            "pirate_treasure",
            PersistedQuestState::NotStarted,
            &["pirate_treasure"],
        );
        assert!(matches!(
            ask(app.world(), "pirate_treasure"),
            ConditionOutcome::NotSatisfied(_)
        ));
    }

    /// An unpopulated roster is "not known yet", not "no such quest". A system
    /// fills the registry, so before it runs every real quest is missing. This
    /// asserts the fall-through that the `initialized` guard protects.
    #[test]
    fn an_uninitialized_roster_rejects_nothing() {
        let mut app = world_with("pirate_treasure", PersistedQuestState::InProgress);
        app.insert_resource(crate::quest::QuestRegistry::default());
        assert_eq!(ask(app.world(), "pirate_treasure"), ConditionOutcome::Satisfied);
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

    /// Only `InProgress` is active, and both ways out are asserted. A predicate
    /// meaning "started" would pass `Completed`; one meaning "not finished" would
    /// pass `NotStarted`.
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

    /// An unstarted quest is not active either. The why-not names which state it
    /// is, in the domain's words.
    ///
    /// Scope: this fixture has no `QuestRegistry`, so an unknown id still answers
    /// `NotSatisfied` here. With a roster the same id is `Unanswerable`; see
    /// `a_quest_the_roster_never_heard_of_is_a_diagnostic_not_a_no`.
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

    /// No save layer is `Unanswerable`, not `false`: "not active" would be a
    /// confident claim about a world with no memory.
    #[test]
    fn a_composition_with_no_save_layer_cannot_answer() {
        let app = App::new();
        assert!(
            matches!(ask(app.world(), "pirate_treasure"), ConditionOutcome::Unanswerable(_)),
            "with no save layer nothing recorded progress"
        );
    }
}
