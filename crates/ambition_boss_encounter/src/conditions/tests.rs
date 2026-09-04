use super::*;
use ambition_persistence::save::AmbitionGameSave;
use ambition_persistence::save_data::PersistedEncounterState;
use bevy::prelude::App;

fn ask(world: &World, boss: &str) -> ConditionOutcome {
    cleared(world, &[AuthoredArg::Name(boss.to_string())])
}

/// A world with a save and one boss in the given state.
fn world_with(boss: &str, state: PersistedEncounterState) -> App {
    let mut app = App::new();
    app.insert_resource(AmbitionGameSave::default());
    app.world_mut()
        .resource_mut::<AmbitionGameSave>()
        .data_mut()
        .set_boss(boss.to_string(), state);
    app
}

/// THE BOSS ANSWERS ONLY WHEN IT WAS BEATEN — and the closed arm is asserted
/// first, from a boss the save KNOWS about: "never fought" and "lost" are
/// different answers and both must stay shut.
#[test]
fn only_a_defeated_boss_satisfies_the_condition() {
    let app = world_with("mockingbird", PersistedEncounterState::Failed);
    assert!(
        matches!(ask(app.world(), "mockingbird"), ConditionOutcome::NotSatisfied(_)),
        "a boss whose last attempt ended in a death has not been beaten"
    );

    let app = world_with("mockingbird", PersistedEncounterState::Cleared);
    assert_eq!(ask(app.world(), "mockingbird"), ConditionOutcome::Satisfied);
}

/// ⚠ AN UNRECORDED BOSS IS `NotSatisfied`, NOT `Unanswerable`. The save
/// reconstructs a missing row as `Untouched`, so absence is a real state — and
/// the why-not must say so in the domain's own words rather than leaving a
/// reader to guess that nobody asked.
#[test]
fn a_boss_the_save_has_never_heard_of_is_simply_not_beaten() {
    let app = world_with("mockingbird", PersistedEncounterState::Cleared);
    let outcome = ask(app.world(), "a_boss_nobody_has_met");
    let why = match &outcome {
        ConditionOutcome::NotSatisfied(why) => why,
        other => panic!("an unrecorded boss is not beaten, not {other:?}"),
    };
    assert_eq!(why.term, "boss.cleared");
    assert_eq!(why.subject, "a_boss_nobody_has_met");
    assert!(
        why.observed.contains("never been beaten"),
        "the why-not says which state it is in: {}",
        why.observed
    );
}

/// ⛔ NO SAVE LAYER IS `Unanswerable`, and that is a different answer from
/// `false`. A composition with no save recorded nothing; reporting "not beaten"
/// there would be a confident claim about a world that has no memory.
#[test]
fn a_composition_with_no_save_layer_cannot_answer() {
    let app = App::new();
    assert!(
        matches!(ask(app.world(), "mockingbird"), ConditionOutcome::Unanswerable(_)),
        "with no save layer nothing recorded an outcome, so there is nothing to report"
    );
}
