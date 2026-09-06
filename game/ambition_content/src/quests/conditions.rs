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
///
/// ⛔⛔ BUT "UNRECORDED" AND "NOT A QUEST" ARE TWO ABSENCES, AND THE SAVE CANNOT
/// TELL THEM APART. `save.data().quest(id)` reconstructs ANY string as
/// `NotStarted`, so before this check existed a misspelt authored id was
/// `NotSatisfied` forever — a dialogue branch or a gate that never fires, with
/// no diagnostic anywhere, indistinguishable from a quest the player simply has
/// not begun. That is the permissive default [`ParamKind::Name`] warns about:
/// preparation cannot refuse an unknown name because it holds no `World`, so the
/// refusal has to happen here.
///
/// ⭐ THE ROSTER IS THE `QuestRegistry`, NOT `default_quest_specs()`, and the
/// difference is load-bearing: the registry is what a composition actually ran,
/// including anything it added through `ensure`, while the static list is only
/// what this crate ships. Validating against the static list would reject a
/// legitimately registered quest.
///
/// ⚠ AND IT IS CONSULTED ONLY WHEN `initialized`, because the registry is
/// populated by a SYSTEM (`populate_quest_registry`). An empty registry on a
/// frame before that system runs is "the roster is not known yet", not "no such
/// quest" — trusting it would turn every real quest into a diagnostic for one
/// frame. No roster ⇒ this falls through and answers from the save exactly as
/// before, which is what a save-only composition (a dialogue test) gets.
pub fn active(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let Some(quest) = args[0].as_name() else {
        return ConditionOutcome::unanswerable("`quest` must be a name");
    };
    // The roster BEFORE the save, for the reason `body.can` states about its
    // verb: resolving progress first would answer "it has not been started" for
    // a misspelling, which is the wrong sentence about the wrong subject.
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
/// ⭐ SEPARATED FROM THE PUMP, mirroring `BossConditionsPlugin`. The publication
/// is one line and the roster/progression systems are many, but they are
/// different capabilities: a composition may want to ASK whether a quest is
/// active without running the content quest pump — a dialogue test is exactly
/// that composition, and so is any host that carries the save but not the
/// progression schedule.
///
/// ⚠ THE REASON IS A SECOND-AUTHORITY ONE, not tidiness. While the publication
/// lived inline in `AmbitionQuestContentPlugin::build`, anything that needed
/// only the question had to re-derive it by calling `publish_condition` with
/// `active_descriptor()` itself — a second place that decides what the quest
/// domain publishes, which drifts the moment a second question is added.
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

    /// A world that also carries a POPULATED roster, so the misspelling check is
    /// live. `initialized` is set by hand rather than by running the content
    /// pump: the pump is a different capability, and this file tests the
    /// question, not the progression.
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

    /// ⛔⛔ THE ABSENCE THAT ANSWERED TWO QUESTIONS, split. A quest id the roster
    /// has never heard of is `Unanswerable` — a content diagnostic — and NOT the
    /// `NotSatisfied` the save alone would produce, because `save.data().quest`
    /// reconstructs any string at all as `NotStarted`. Without this the only
    /// symptom of a misspelt authored id is a branch that never fires.
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

    /// ⚠ AND THE DIAGNOSTIC MUST NOT SWALLOW THE REAL NO. A quest the roster
    /// KNOWS, which the player has genuinely not begun, is still a plain
    /// `NotSatisfied` — the check above would be worthless if it turned every
    /// unstarted quest into an error.
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

    /// ⛔ AN UNPOPULATED ROSTER IS "NOT KNOWN YET", NOT "NO SUCH QUEST". The
    /// registry is filled by a system, so on any frame before it runs every real
    /// quest is missing from it. Trusting it then would make the whole roster
    /// unanswerable for a frame; this asserts the fall-through, which is the arm
    /// a poison on the `initialized` guard would otherwise leave green.
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
    ///
    /// ⛔ SCOPE: this fixture carries NO `QuestRegistry`, and that is why an id
    /// nobody offered still answers `NotSatisfied` here. It pins the save-only
    /// composition. Once a roster is present the same id is `Unanswerable` —
    /// see `a_quest_the_roster_never_heard_of_is_a_diagnostic_not_a_no`.
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
