//! these tests build their own `App`, which is exactly the shape that can
//! pass with the production wiring absent. They are about the CONTRACT — its
//! privacy, its one road, its arity checking, its refusals. The claim that real
//! domains publish real commands is proved by the integration fixture that
//! drives the composed app, not here.

use super::*;
use bevy::prelude::{Component, Resource};

use crate::authored_logic::{ParamKind, ParamSpec};

const RUNG: ParamSpec = ParamSpec {
    name: "note",
    kind: ParamKind::Name,
    summary: "which note to ring",
};

/// Whatever real state a domain would own. A resource rather than a component so
/// the test can assert on it without a query.
#[derive(Resource, Default)]
struct Bell(Vec<String>);

/// Present only so `is_carried`'s shape has a mirror here — a runner that has to
/// find something in the world rather than in a resource.
#[derive(Component)]
struct Struck;

fn ring(world: &mut World, args: &[AuthoredArg]) -> CommandOutcome {
    let Some(note) = args[0].as_name() else {
        return CommandOutcome::refused("argument was not a name");
    };
    let Some(mut bell) = world.get_resource_mut::<Bell>() else {
        return CommandOutcome::refused("nothing in this world has a bell to ring");
    };
    bell.0.push(note.to_string());
    world.spawn(Struck);
    CommandOutcome::Done
}

fn descriptor(domain: &str, verb: &str) -> CommandDescriptor {
    CommandDescriptor {
        id: CommandId::new(domain, verb),
        summary: "a test command",
        params: &[RUNG],
    }
}

fn request(app: &mut App, id: &CommandId, args: Vec<AuthoredArg>) {
    app.world_mut()
        .write_message(RunAuthoredCommand::new(id.clone(), args));
}

/// A CRATE THAT IS NOT THE ENGINE CAN PUBLISH A COMMAND, AND THE ONLY ROAD TO
/// PERFORMING ONE IS THE REQUEST CHANNEL.
///
/// this is the milestone's behavioural acceptance for the command half,
/// and it is two claims in one test because the second is what makes the first
/// mean anything.
///
/// 1. This module names no other domain, edits no enum and touches no
///    registration table — it calls `publish_command` and nothing else. If a
///    central list of command kinds existed, this could not compile.
/// 2. and the test cannot pass by accident: it asserts the bell is silent
///    while the request is only WRITTEN, and rings only once the dispatcher has
///    run. Both terms are observed. A version of this that asserted only the
///    end state would still pass if `run_requested_authored_commands` were
///    deleted and something else performed the effect.
#[test]
fn a_provider_that_names_no_other_domain_can_publish_and_be_performed() {
    let mut app = App::new();
    app.add_message::<RunAuthoredCommand>()
        .init_resource::<Bell>()
        .publish_command(descriptor("bystander", "ring"), ring);

    let id = CommandId::new("bystander", "ring");
    request(&mut app, &id, vec![AuthoredArg::Name("C".to_string())]);

    assert!(
        app.world().resource::<Bell>().0.is_empty(),
        "a REQUESTED command had already happened; then the dispatcher is not \
         what performs one and this test proves nothing about ordering"
    );

    run_requested_authored_commands(app.world_mut());

    assert_eq!(app.world().resource::<Bell>().0, ["C"]);
}

/// A COMMAND HAPPENS ONCE PER REQUEST.
///
/// A grant is not idempotent.
#[test]
fn a_request_is_performed_once_and_leaves_the_buffer_empty() {
    let mut app = App::new();
    app.add_message::<RunAuthoredCommand>()
        .init_resource::<Bell>()
        .publish_command(descriptor("bystander", "ring"), ring);

    let id = CommandId::new("bystander", "ring");
    request(&mut app, &id, vec![AuthoredArg::Name("C".to_string())]);

    run_requested_authored_commands(app.world_mut());
    run_requested_authored_commands(app.world_mut());

    assert_eq!(
        app.world().resource::<Bell>().0,
        ["C"],
        "the note rang twice for one request"
    );
}

/// TWO domains coexist and each is discoverable on its own.
#[test]
fn the_catalog_composes_domains_without_either_naming_the_other() {
    let mut app = App::new();
    app.publish_command(descriptor("world", "ring"), ring);
    app.publish_command(descriptor("weather", "ring"), ring);

    let catalog = app.world().resource::<CommandCatalog>();
    assert_eq!(catalog.len(), 2);
    assert_eq!(
        catalog
            .describe_all()
            .map(|d| d.id.to_string())
            .collect::<Vec<_>>(),
        ["weather.ring", "world.ring"],
        "the listing is id-ordered, so a diagnostic that prints it is stable"
    );
    assert_eq!(catalog.describe_domain("weather").count(), 1);
}

/// AN UNPUBLISHED COMMAND IS REFUSED WITH A COUNT, NOT SILENTLY DROPPED.
///
/// the count is the sentence that tells an author whether they typo'd a verb
/// or forgot a plugin.
#[test]
fn asking_for_an_unpublished_command_is_refused_with_a_reason() {
    let mut app = App::new();
    app.publish_command(descriptor("world", "ring"), ring);
    let catalog = app.world().resource::<CommandCatalog>().clone();

    let CommandOutcome::Refused(reason) = catalog.run(
        app.world_mut(),
        &CommandId::new("nobody", "cares"),
        &[AuthoredArg::Name("C".to_string())],
    ) else {
        panic!("an unpublished command must be refused");
    };
    assert!(reason.contains("knows 1 others"), "{reason}");
}

/// ARITY AND KIND ARE CHECKED ONCE, CENTRALLY, AND THE REASON NAMES THE
/// PARAMETER.
#[test]
fn a_mistyped_argument_is_refused_with_a_reason_an_author_can_act_on() {
    let mut app = App::new();
    app.init_resource::<Bell>()
        .publish_command(descriptor("world", "ring"), ring);
    let catalog = app.world().resource::<CommandCatalog>().clone();
    let id = CommandId::new("world", "ring");

    let CommandOutcome::Refused(too_few) = catalog.run(app.world_mut(), &id, &[]) else {
        panic!("no arguments must be refused");
    };
    assert!(too_few.contains("note"), "{too_few}");

    let CommandOutcome::Refused(wrong_kind) =
        catalog.run(app.world_mut(), &id, &[AuthoredArg::Number(1.0)])
    else {
        panic!("a Number where a Name belongs must be refused");
    };
    assert!(wrong_kind.contains("note"), "{wrong_kind}");

    assert!(
        app.world().resource::<Bell>().0.is_empty(),
        "a refused command still reached the domain's runner"
    );
}

/// A DOMAIN THAT CANNOT PERFORM ITS OWN VERB REFUSES RATHER THAN PANICKING.
///
/// a composition without the state a command needs is a real composition — a
/// headless fixture, a menu route — not a broken one.
#[test]
fn a_composition_missing_the_domains_state_refuses() {
    let mut app = App::new();
    app.add_message::<RunAuthoredCommand>()
        .publish_command(descriptor("world", "ring"), ring);
    let id = CommandId::new("world", "ring");
    request(&mut app, &id, vec![AuthoredArg::Name("C".to_string())]);

    // No `Bell` resource: the runner refuses, the dispatcher logs, nothing panics.
    run_requested_authored_commands(app.world_mut());
    assert!(app.world().get_resource::<Bell>().is_none());
}

/// TWO DOMAINS CANNOT OWN ONE ID, AND IT FAILS AT STARTUP.
#[test]
#[should_panic(expected = "already published")]
fn publishing_one_id_twice_panics_rather_than_letting_the_last_plugin_win() {
    let mut app = App::new();
    app.publish_command(descriptor("world", "ring"), ring);
    app.publish_command(descriptor("world", "ring"), ring);
}

/// AN ID CANNOT BE SPELLED TWO WAYS, and the panic says `command`/`verb`
/// rather than a generic complaint about segments.
#[test]
#[should_panic(expected = "a command id's segments")]
fn a_dot_inside_a_segment_is_refused() {
    let _ = CommandId::new("world", "set.flag");
}

/// AUTHORED CONTENT NAMES A COMMAND BY STRING, AND A TYPO IS A DIAGNOSTIC
/// RATHER THAN A PANIC.
#[test]
fn an_id_read_back_from_authored_text_refuses_instead_of_panicking() {
    assert_eq!(
        CommandId::parse("world.set_flag"),
        Some(CommandId::new("world", "set_flag"))
    );
    // every one of these would have PANICKED through `new`.
    assert_eq!(CommandId::parse("set_flag"), None, "no domain at all");
    assert_eq!(CommandId::parse(".set_flag"), None, "empty domain");
    assert_eq!(CommandId::parse("world."), None, "empty verb");
    assert_eq!(CommandId::parse("a.b.c"), None, "ambiguous segments");
    assert_eq!(CommandId::parse(""), None);
    // and it never repairs.
    assert_ne!(
        CommandId::parse(" world.set_flag"),
        Some(CommandId::new("world", "set_flag")),
        "trimming would make the authored name and the published name two \
         spellings of one id"
    );
}
