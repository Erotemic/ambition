//! these build a bare `App` with this domain's plugin and nothing else, which
//! is the point: the verb has to be reachable from a composition that knows
//! about encounters and nothing about switches, dialogue or LDtk.

use super::*;
use bevy::prelude::App;

use ambition_platformer2d_shared_tangle::authored_logic::{
    CommandCatalog, PreparedCommand, RunAuthoredCommand,
};

use crate::lifecycle::EncounterCommandKind;

const ATTUNEMENT: &str = "symmetry_attunement";

fn app_with_one_encounter() -> App {
    let mut app = App::new();
    app.add_message::<EncounterCommand>();
    publish_authored_commands(&mut app);
    app.world_mut()
        .spawn((Encounter::new(ATTUNEMENT), SimId::encounter(ATTUNEMENT)));
    app
}

fn prepare(app: &App, line: &str) -> Result<PreparedCommand, String> {
    app.world()
        .resource::<CommandCatalog>()
        .prepare_line(line)
        .map_err(|error| error.to_string())
}

/// through `RunAuthoredCommand::prepared`, so the arguments these tests
/// perform are the ones a real request would carry rather than a hand-built
/// vector that could disagree with preparation.
fn perform(app: &mut App, call: &PreparedCommand) -> CommandOutcome {
    let request = RunAuthoredCommand::prepared(call);
    signal(app.world_mut(), &request.args)
}

/// AN AUTHORED LINE SIGNALS A LIVE ENCOUNTER, AND THE REDUCER RECEIVES THE
/// OCCURRENCE'S OWN ID.
///
/// both terms are observed: the command bus is empty before the verb runs and
/// carries exactly one signal after, so this cannot pass with `signal` gutted.
#[test]
fn an_authored_line_signals_the_encounter_it_references() {
    let mut app = app_with_one_encounter();
    let call = prepare(
        &app,
        "encounter.signal encounter:symmetry_attunement gravity_down",
    )
    .expect("the line names the published verb");

    assert!(
        drain(&mut app).is_empty(),
        "something wrote an encounter command before the verb ran"
    );

    assert_eq!(perform(&mut app, &call), CommandOutcome::Done);

    let commands = drain(&mut app);
    assert_eq!(
        commands.len(),
        1,
        "one authored call, one lifecycle command"
    );
    assert_eq!(commands[0].encounter, ATTUNEMENT);
    assert_eq!(
        commands[0].kind,
        EncounterCommandKind::Signal("gravity_down".to_string()),
    );
}

/// A REFERENCE THAT NAMES NO LIVE OCCURRENCE IS REFUSED WITH A SENTENCE.
///
/// this is the whole reason the parameter is a reference rather than a name.
/// A name would have produced a perfectly well-formed `EncounterCommand`
/// addressed to nothing, and the reducer would have dropped it in silence.
#[test]
fn a_reference_to_no_live_occurrence_is_refused_rather_than_addressed_to_nothing() {
    let mut app = app_with_one_encounter();
    let call = prepare(
        &app,
        "encounter.signal encounter:no_such_puzzle gravity_down",
    )
    .expect("preparation cannot know which occurrences are live; that is runtime");

    let CommandOutcome::Refused(reason) = perform(&mut app, &call) else {
        panic!("an unresolvable reference must be refused");
    };
    assert!(reason.contains("encounter:no_such_puzzle"), "{reason}");
    assert!(
        drain(&mut app).is_empty(),
        "a refused signal still reached the lifecycle bus"
    );
}

/// THE PLACEMENT NAMESPACE IS NOT THE ENCOUNTER NAMESPACE.
///
/// a boss WRAP's encounter id is also its body's placement id, so these two
/// spellings of the same word name two different rows — which is exactly why the
/// authored surface makes the author say which.
#[test]
fn a_placement_reference_does_not_resolve_to_the_encounter_of_the_same_name() {
    let mut app = app_with_one_encounter();
    let call = prepare(
        &app,
        "encounter.signal placement:symmetry_attunement gravity_down",
    )
    .expect("`placement:` is an authorable namespace");

    assert!(matches!(
        perform(&mut app, &call),
        CommandOutcome::Refused(_)
    ));
}

fn drain(app: &mut App) -> Vec<EncounterCommand> {
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<EncounterCommand>>()
        .drain()
        .collect()
}
