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
        )
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
    ));
    sim.step_n(base(), 2);
}
