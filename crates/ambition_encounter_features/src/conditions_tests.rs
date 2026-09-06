use crate::conditions::*;
use ambition_persistence::save_data::PersistedEncounterState;
use ambition_platformer2d_shared_tangle::authored_logic::{AuthoredArg, ConditionOutcome};
use bevy::prelude::*;

fn world_with_save() -> App {
    let mut app = App::new();
    app.insert_resource(ambition_persistence::save::AmbitionGameSave::default());
    app
}

fn ask(app: &App, encounter: &str) -> ConditionOutcome {
    cleared(app.world(), &[AuthoredArg::Name(encounter.to_string())])
}

/// ALL THREE RECORDED STATES ARE ASSERTED, and only one of them opens a route.
///
/// `Failed` is the arm worth pinning: a door gated on clearing an arena must
/// not open because the player fought it and died, which a `!= Untouched`
/// reading would do.
#[test]
fn only_a_cleared_encounter_satisfies_the_question() {
    let mut app = world_with_save();

    assert!(
        matches!(ask(&app, "goblin_encounter"), ConditionOutcome::NotSatisfied(_)),
        "an encounter nobody has touched is not cleared"
    );

    fn set(app: &mut App, state: PersistedEncounterState) {
        app.world_mut()
            .resource_mut::<ambition_persistence::save::AmbitionGameSave>()
            .data_mut()
            .set_encounter("goblin_encounter", state);
    }

    set(&mut app, PersistedEncounterState::Failed);
    assert!(
        matches!(ask(&app, "goblin_encounter"), ConditionOutcome::NotSatisfied(_)),
        "a death is not a clear, and a `!= Untouched` reading would say it was"
    );

    set(&mut app, PersistedEncounterState::Cleared);
    assert_eq!(ask(&app, "goblin_encounter"), ConditionOutcome::Satisfied);
}

/// THE THREE OUTCOMES ARE THREE, and a composition with no save layer gets the
/// third rather than a confident `false`.
///
/// A world that records nothing has no subject for the question. Answering
/// `NotSatisfied` would leave every encounter-gated route shut with nothing
/// said about why, which is the failure the enum's third variant exists for.
#[test]
fn without_a_save_layer_the_question_has_no_subject() {
    let app = App::new();
    assert!(matches!(
        ask(&app, "goblin_encounter"),
        ConditionOutcome::Unanswerable(_)
    ));
}

/// THE REFUSAL SAYS WHICH STATE IT SAW, because a gate that will not open is
/// the hardest thing in a level to diagnose from the outside.
#[test]
fn the_refusal_names_the_state_it_found() {
    let mut app = world_with_save();
    app.world_mut()
        .resource_mut::<ambition_persistence::save::AmbitionGameSave>()
        .data_mut()
        .set_encounter("goblin_encounter", PersistedEncounterState::Failed);

    let ConditionOutcome::NotSatisfied(why) = ask(&app, "goblin_encounter") else {
        panic!("a failed encounter is not cleared");
    };
    assert!(
        why.to_string().contains("death"),
        "the diagnostic must distinguish a loss from never having fought: {why}"
    );
}
