//! What can authored content tell this engine to do, and does telling work?
//!
//! The command contract's claim is the mirror of the condition one next door in
//! `the_engine_can_be_asked_questions`: a domain publishes its own verbs and
//! nothing central learns they exist. and it is
//! only worth anything about the composed engine — a contract wired up
//! nowhere is vocabulary nobody speaks.
//!
//! this file publishes no command of its own. The unit tests beside the
//! contract prove a stranger can publish one; this proves the engine actually
//! did, and that the request survives the whole road: the message channel, the
//! set, the dispatcher, the domain's own effect bus, and the save.
//!
//! the road is the point, not the flag. Writing `SetFlagRequested`
//! directly would set the same flag and prove nothing — every link this test
//! exists to check would be untested and still green.

use ambition_app::Platformer2dSimHarness;
use ambition_platformer2d::platformer::authored_logic::{
    AuthoredArg, CommandCatalog, CommandId, ConditionCatalog, ConditionId, ConditionOutcome,
    RunAuthoredCommand,
};

use crate::common::{base, fixed_60hz_room_sim};

const ROOM: &str = "blink_run";

/// THE COMPOSED ENGINE PUBLISHES A COMMAND, AND IT IS DISCOVERABLE.
///
/// deliberately thin about WHICH commands exist — pinning the catalog would
/// make every new provider a failing test, which is the opposite of the
/// property. What is pinned is that the resource exists in a real composition
/// and that the world-fact domain is in it.
#[test]
fn the_composed_engine_publishes_a_world_fact_command() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 4);

    let catalog = sim
        .world()
        .get_resource::<CommandCatalog>()
        .expect("the composed engine publishes at least one command")
        .clone();

    let set_flag = CommandId::new("world", "set_flag");
    let descriptor = catalog
        .describe(&set_flag)
        .expect("the world-fact domain publishes its verb from its own plugin");
    assert_eq!(
        descriptor.params.iter().map(|p| p.name).collect::<Vec<_>>(),
        ["flag", "on"],
        "the schema an author reads is the schema the catalog checks against"
    );
}

/// ⛔⛤ **AND THE VERB'S REFUSAL LANDS IN THE SAME RING AS THE QUESTION'S `no`,
/// WHICH IS THE PAIR THAT IS ACTUALLY THE DIAGNOSIS.**
///
/// *"The door did not open when I pressed it"* has two shapes and they need
/// different repairs: a condition answered no and the command never ran, or
/// the condition passed and the command refused for a reason of its own. Two
/// separate logs would make a reader join them by hand, and the join is the
/// answer. So `AuthoredVerdictLog` holds both, in the order they happened.
///
/// ⚠ **THE COMMAND SIDE'S ORDER IS TRUSTWORTHY AND THE CONDITION SIDE'S IS
/// NOT.** Commands run through one dispatcher holding `&mut World`, so they
/// serialise; conditions evaluate from `&World` and two systems may ask in
/// parallel. This arm reads a command and a condition it ITSELF issued, in one
/// thread, which is the only ordering claim the log supports.
#[test]
fn a_refused_verb_and_the_question_it_followed_are_in_one_readable_stream() {
    use ambition_platformer2d::platformer::authored_logic::{
        AuthoredVerdict, AuthoredVerdictLog,
    };

    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 4);
    sim.world_mut().insert_resource(AuthoredVerdictLog::default());

    let flag = "a_fact_no_authored_command_will_record";
    let flag_set = ConditionId::new("world", "flag_set");
    // The question first, then a verb the engine will refuse: the argument is
    // a Name where the schema wants a Truth, which no domain ever sees.
    let _ = sim.world().resource::<ConditionCatalog>().clone().evaluate(
        sim.world(),
        &flag_set,
        &[AuthoredArg::Name(flag.to_string())],
        &ambition_platformer2d::platformer::authored_logic::AuthoredAsk::new("probe", "a test"));
    let set_flag = CommandId::new("world", "set_flag");
    sim.world_mut().write_message(RunAuthoredCommand::new(
        set_flag.clone(),
        vec![
            AuthoredArg::Name(flag.to_string()),
            AuthoredArg::Name("yes-please".to_string()),
        ],
        ambition_platformer2d::platformer::authored_logic::AuthoredAsk::new("probe", "a test"),
    ));
    sim.step_n(base(), 1);

    let log = sim.world().resource::<AuthoredVerdictLog>();
    let refusal = log
        .refusal_of(
            &set_flag,
            &[
                AuthoredArg::Name(flag.to_string()),
                AuthoredArg::Name("yes-please".to_string()),
            ],
        )
        .expect("the engine refused a verb and kept no reason");
    assert!(
        refusal.contains("on") && refusal.contains("Truth"),
        "the refusal does not name the argument or the kind it wanted: {refusal}"
    );
    // ⚠ AND THE WORLD DID NOT CHANGE, which is what makes the refusal the
    // interesting record rather than a warning beside a mutation that happened
    // anyway.
    assert!(matches!(
        sim.world().resource::<ConditionCatalog>().clone().evaluate(
            sim.world(),
            &flag_set,
            &[AuthoredArg::Name(flag.to_string())],
        &ambition_platformer2d::platformer::authored_logic::AuthoredAsk::new("probe", "a test")),
        ConditionOutcome::NotSatisfied(_)
    ));

    // THE PAIR, read in order out of one stream.
    let stream = log.recent();
    let asked = stream
        .iter()
        .position(|entry| matches!(entry, AuthoredVerdict::Asked(v) if v.id == flag_set))
        .expect("the question is not in the log");
    let ran = stream
        .iter()
        .position(|entry| matches!(entry, AuthoredVerdict::Ran(v) if v.id == set_flag))
        .expect("the verb is not in the log");
    assert!(
        asked < ran,
        "the question was asked before the verb ran and the stream says \
         otherwise: {stream:?}"
    );
    assert!(
        stream[ran].to_string().contains("refused"),
        "a verdict renders without saying what happened: {}",
        stream[ran]
    );
}

/// A REQUESTED COMMAND TRAVELS THE WHOLE ROAD AND CHANGES THE WORLD.
///
/// the acceptance for the command half, in the composed app. The request
/// is written into the ordinary message channel — the only public road to
/// `CommandCatalog::run`, which is private — and the assertion is made through
/// the CONDITION half, so what is checked is that the world-fact domain agrees
/// with itself about the fact.
///
/// both terms are observed: the flag is asked before and after. A test
/// that only checked the end state would pass against an engine that set the
/// flag for some other reason.
#[test]
fn a_requested_command_reaches_the_domain_and_the_save() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 4);

    let flag = "a_fact_only_an_authored_command_records";
    let flag_set = ConditionId::new("world", "flag_set");
    let ask = |sim: &Platformer2dSimHarness| {
        sim.world().resource::<ConditionCatalog>().clone().evaluate(
            sim.world(),
            &flag_set,
            &[AuthoredArg::Name(flag.to_string())],
        &ambition_platformer2d::platformer::authored_logic::AuthoredAsk::new("probe", "a test"))
    };

    assert!(
        matches!(ask(&sim), ConditionOutcome::NotSatisfied(_)),
        "nothing had recorded this fact yet, so the test below has something to \
         prove"
    );

    sim.world_mut().write_message(RunAuthoredCommand::new(
        CommandId::new("world", "set_flag"),
        vec![
            AuthoredArg::Name(flag.to_string()),
            AuthoredArg::Truth(true),
        ],
        ambition_platformer2d::platformer::authored_logic::AuthoredAsk::new("probe", "a test"),
    ));
    // One tick: the dispatcher runs in `AuthoredCommandSet`, which is ordered
    // before `GameplayEffects` precisely so the `SetFlagRequested` it writes is
    // applied by `apply_flag_effects` on the SAME tick.
    sim.step_n(base(), 1);

    assert_eq!(
        ask(&sim),
        ConditionOutcome::Satisfied,
        "the authored verb did not reach the save — check that \
         `AuthoredCommandPlugin` is composed and that its set is still ordered \
         before `GameplayEffects`"
    );

    // and it happens ONCE. The dispatcher drains rather than reading with
    // a cursor, so a second tick finds nothing. Clearing the flag by hand and
    // stepping again is how that is observable at all.
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .data_mut()
        .set_flag(flag, false);
    sim.step_n(base(), 2);
    assert!(
        matches!(ask(&sim), ConditionOutcome::NotSatisfied(_)),
        "the request was performed again on a later tick; a grant is not \
         idempotent and this is how one becomes two"
    );
}

/// AN UNPUBLISHED COMMAND CHANGES NOTHING AND DOES NOT PANIC.
///
/// authored content names a verb by string. A typo is a diagnostic, not a
/// crashed game — the same rule `ConditionId::parse` exists for.
#[test]
fn a_command_nobody_published_is_refused_by_the_running_engine() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 4);

    sim.world_mut().write_message(RunAuthoredCommand::new(
        CommandId::new("world", "unset_flag"),
        vec![AuthoredArg::Name("whatever".to_string())],
        ambition_platformer2d::platformer::authored_logic::AuthoredAsk::new("probe", "a test"),
    ));
    sim.step_n(base(), 2);
}
