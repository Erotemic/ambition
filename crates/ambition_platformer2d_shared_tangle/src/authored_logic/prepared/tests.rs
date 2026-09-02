//! these tests build their own `App`, the same shape the command
//! contract's own tests use and with the same honest limit: they are about
//! PREPARATION — its refusals, its one door, and the fact that a prepared call
//! carries no text. That real content prepares real rules is proved by
//! `world::authored_switch_commands` and by the app fixture that drives the
//! composed game.

use super::*;
use bevy::prelude::{App, Resource};

use crate::authored_logic::{
    commands::run_requested_authored_commands, CommandDescriptor, CommandOutcome,
    ConditionDescriptor, ParamKind, ParamSpec, PublishCommand, PublishCondition,
};

/// Whatever state the test's own domain owns. a domain nothing in this
/// engine has ever heard of, which is the point: if preparing an authored call
/// needed a central list of kinds, this module could not be written outside it.
#[derive(Resource, Default)]
struct Gossip(Vec<String>);

const ABOUT: ParamSpec = ParamSpec {
    name: "about",
    kind: ParamKind::Reference,
    summary: "the occurrence the rumour is about",
};
const LOUDLY: ParamSpec = ParamSpec {
    name: "loudly",
    kind: ParamKind::Truth,
    summary: "whether it is shouted",
};

fn spread(world: &mut World, args: &[AuthoredArg]) -> CommandOutcome {
    let Some(about) = args[0].as_reference() else {
        return CommandOutcome::refused("argument was not a reference");
    };
    let loudly = matches!(args[1], AuthoredArg::Truth(true));
    let Some(mut gossip) = world.get_resource_mut::<Gossip>() else {
        return CommandOutcome::refused("nobody in this world gossips");
    };
    gossip
        .0
        .push(format!("{about}{}", if loudly { "!" } else { "" }));
    CommandOutcome::Done
}

fn spread_descriptor() -> CommandDescriptor {
    CommandDescriptor {
        id: CommandId::new("gossip", "spread"),
        summary: "spread a rumour about an occurrence",
        params: &[ABOUT, LOUDLY],
    }
}

fn is_rumoured(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let Some(about) = args[0].as_reference() else {
        return ConditionOutcome::unanswerable("argument was not a reference");
    };
    let Some(gossip) = world.get_resource::<Gossip>() else {
        return ConditionOutcome::unanswerable("nobody in this world gossips");
    };
    ConditionOutcome::from_bool_unexplained(
        gossip.0.iter().any(|line| line.starts_with(about.as_str())),
    )
}

fn is_rumoured_descriptor() -> ConditionDescriptor {
    ConditionDescriptor {
        id: ConditionId::new("gossip", "is_rumoured"),
        summary: "whether an occurrence has been gossiped about",
        params: &[ABOUT],
    }
}

fn app_with_gossip() -> App {
    let mut app = App::new();
    app.add_message::<RunAuthoredCommand>()
        .init_resource::<Gossip>()
        .publish_command(spread_descriptor(), spread)
        .publish_condition(is_rumoured_descriptor(), is_rumoured);
    app
}

/// AUTHORED TEXT FROM A DOMAIN THE ENGINE NEVER HEARD OF BECOMES A CALL THE
/// SIMULATION MAKES — AND THE VALIDATION HAPPENS BEFORE THE TICK.
///
/// this is behavioural acceptance, and it is three claims in one test because any one of them
/// alone would pass with the others broken.
///
/// 1. Nothing central was edited. This module publishes `gossip.spread` and
///    prepares a call naming it, using only the public surface. A central
///    registry of authorable kinds would make this uncompilable.
/// 2. Preparation is not performance. The rumour is silent while the line is
///    only PREPARED, and again while the request is only WRITTEN. both terms
///    are observed, so this cannot pass with the dispatcher deleted.
/// 3. The prepared call carries no text. What reaches the dispatcher is an
///    id and two [`AuthoredArg`]s; the authored line is gone.  there is nothing
///    left for a tick to parse.
#[test]
fn an_authored_line_from_a_foreign_domain_prepares_then_runs() {
    let mut app = app_with_gossip();
    let catalog = app.world().resource::<CommandCatalog>().clone();

    let call = catalog
        .prepare_line("gossip.spread encounter:town_square true")
        .expect("the line names a published verb with well-typed arguments");

    assert_eq!(
        call.args(),
        [
            AuthoredArg::Reference(SimId::encounter("town_square")),
            AuthoredArg::Truth(true),
        ],
        "preparation produced the values the descriptor declares, not the text"
    );
    assert!(
        app.world().resource::<Gossip>().0.is_empty(),
        "PREPARING a call performed it; then preparation is not a separate step \
         and nothing here is proved about ordering"
    );

    app.world_mut()
        .write_message(RunAuthoredCommand::prepared(&call));
    assert!(
        app.world().resource::<Gossip>().0.is_empty(),
        "a REQUESTED command had already happened; then the dispatcher is not what \
         performs one"
    );

    run_requested_authored_commands(app.world_mut());
    assert_eq!(
        app.world().resource::<Gossip>().0,
        ["encounter:town_square!"]
    );
}

/// EVERY WAY AN AUTHORED LINE CAN BE WRONG IS REFUSED AT PREPARE TIME, WITH A
/// REASON NAMING THE PART THAT IS WRONG.
///
/// the failure this pins is a preparer that cannot say no. A `prepare` that returned `Ok`
/// for everything would satisfy every other test in this file — the call would still run, the
/// argument would still be an `AuthoredArg`, and the wrongness would surface on a tick as a
/// catalog refusal, which is exactly where says it must not surface.
///
/// the good line is prepared in the same test on purpose: a preparer that
/// refused EVERYTHING would pass the refusal half and be just as broken.
#[test]
fn preparation_refuses_before_the_tick_what_a_tick_would_otherwise_discover() {
    let app = app_with_gossip();
    let catalog = app.world().resource::<CommandCatalog>().clone();

    assert!(
        catalog
            .prepare_line("gossip.spread encounter:town_square true")
            .is_ok(),
        "the well-formed line must prepare, or the refusals below prove nothing"
    );

    for (line, expected) in [
        ("gossip.mutter encounter:town_square true", "knows 1 others"),
        ("gossip.spread encounter:town_square", "takes 2 argument(s)"),
        (
            "gossip.spread encounter:town_square true loudly",
            "takes 2 argument(s)",
        ),
        ("gossip.spread encounter:town_square yes", "loudly"),
        ("gossip.spread town_square true", "names no namespace"),
        ("gossip.spread rumour:town_square true", "not an authorable"),
        ("gossip.spread encounter: true", "and nothing else"),
        ("gossipspread encounter:town_square true", "domain.verb"),
        ("", "this line is blank"),
    ] {
        let error = catalog
            .prepare_line(line)
            .expect_err("this authored line is wrong and must be refused");
        assert!(
            error.reason().contains(expected),
            "refusing {line:?} said {:?}, which does not name {expected:?}",
            error.reason(),
        );
        assert_eq!(
            error.source(),
            line,
            "the diagnostic lost the authored line"
        );
    }

    assert!(
        app.world().resource::<Gossip>().0.is_empty(),
        "a refused line reached the domain's runner"
    );
}

/// A PREPARED REFERENCE IS A `SimId` MINTED BY `SimId`'s OWN CONSTRUCTORS.
///
/// the shape this refuses is `SimId::from_snapshot(text)` — the one
/// constructor that takes a raw string, and the one that skips the escaping the
/// id encoding's injectivity depends on. An authored id containing the
/// separator proves which road was taken: through the constructor it is escaped,
/// and around it, it is not.
#[test]
fn an_authored_reference_goes_through_the_identity_vocabulary() {
    let app = app_with_gossip();
    let catalog = app.world().resource::<CommandCatalog>().clone();

    let prepared = |text: &str| {
        catalog
            .prepare_line(&format!("gossip.spread {text} false"))
            .map(|call| call.args()[0].clone())
    };

    assert_eq!(
        prepared("encounter:symmetry_attunement").unwrap(),
        AuthoredArg::Reference(SimId::encounter("symmetry_attunement")),
    );
    assert_eq!(
        prepared("placement:symmetry_attunement").unwrap(),
        AuthoredArg::Reference(SimId::placement("symmetry_attunement")),
        "the namespace the author wrote is the namespace the id gets",
    );
    assert_ne!(
        prepared("encounter:kernel").unwrap(),
        prepared("placement:kernel").unwrap(),
        "two namespaces, two identities — which is why the author names one",
    );

    let AuthoredArg::Reference(nested) = prepared("encounter:a:b").unwrap() else {
        panic!("a reference param prepares to a reference");
    };
    assert_eq!(
        nested,
        SimId::encounter("a:b"),
        "the body is escaped by the constructor; a raw string would have produced \
         `encounter:a:b`, which is a DIFFERENT identity from `encounter:a%3Ab`",
    );
    assert_eq!(nested.as_str(), "encounter:a%3Ab");
}

/// A PREPARED QUESTION IS VALIDATED ONCE AND ASKED EVERY TICK.
///
/// The wall that pays for this is `world:gated_lock_walls`.
#[test]
fn a_prepared_question_is_validated_once_and_asked_without_reassembly() {
    let mut app = app_with_gossip();
    let conditions = app.world().resource::<ConditionCatalog>().clone();

    let gate = conditions
        .prepare(
            ConditionId::new("gossip", "is_rumoured"),
            &["encounter:town_square"],
        )
        .expect("a published question with a well-typed argument");

    assert!(
        matches!(
            conditions.ask(app.world(), &gate),
            ConditionOutcome::NotSatisfied(_)
        ),
        "nobody has gossiped yet"
    );

    app.world_mut()
        .resource_mut::<Gossip>()
        .0
        .push("encounter:town_square".to_string());

    assert_eq!(
        conditions.ask(app.world(), &gate),
        ConditionOutcome::Satisfied,
        "the SAME prepared question answers differently as the world moves — which \
         is what makes preparing it once legitimate"
    );

    // and the condition half refuses at prepare time too, or "prepared" would
    // mean something different on each side of the contract.
    assert!(conditions
        .prepare(ConditionId::new("gossip", "is_admired"), &["encounter:x"])
        .is_err());
    assert!(conditions
        .prepare(ConditionId::new("gossip", "is_rumoured"), &[])
        .is_err());
}
