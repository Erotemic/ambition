//! Does a `<<command …>>` line actually reach a domain that published a
//! command — through the real interpreter, with no edit to any binding?
//!
//! these tests publish their command FROM HERE, under a domain name
//! nothing in the engine mentions, using only
//! [`PublishCommand`](ambition_platformer2d_shared_tangle::authored_logic::PublishCommand).
//! That is the whole claim: if asking for a verb required a bridge to learn its
//! name, a test crate could not have taught it one. they deliberately do NOT
//! assert which commands the engine ships — pinning that list would make every
//! new provider a failing test.
//!
//! what a Yarn-level test can and cannot see. The verb's job ends at the
//! narrative ledger; the simulation performs the command a tick later. So these
//! assert on the LEDGER's release, which is the seam the verb actually writes,
//! and the contract module's own tests assert that a released request performs
//! the command.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::authored_logic::{
    AuthoredArg, CommandDescriptor, CommandId, CommandOutcome, ParamKind, ParamSpec, PublishCommand,
    RunAuthoredCommand,
};

use crate::dialog::yarn_harness::{app_running as harness_app, start};

// ===== A domain that exists only in this file ===================

const RUMOUR: ParamSpec = ParamSpec {
    name: "rumour",
    kind: ParamKind::Name,
    summary: "the rumour to spread",
};

const LOUDLY: ParamSpec = ParamSpec {
    name: "loudly",
    kind: ParamKind::Truth,
    summary: "whether the whole town hears it",
};

fn spread_descriptor() -> CommandDescriptor {
    CommandDescriptor {
        id: CommandId::new("gossip", "spread"),
        summary: "start a rumour",
        params: &[RUMOUR, LOUDLY],
    }
}

fn spread(_world: &mut World, _args: &[AuthoredArg]) -> CommandOutcome {
    CommandOutcome::Done
}

const OCCURRENCE: ParamSpec = ParamSpec {
    name: "occurrence",
    kind: ParamKind::Reference,
    summary: "the occurrence to nudge",
};

fn nudge_descriptor() -> CommandDescriptor {
    CommandDescriptor {
        id: CommandId::new("gossip", "nudge"),
        summary: "nudge one occurrence",
        params: &[OCCURRENCE],
    }
}

// ===== Harness ==================================================

const SOURCE: &str = "\
title: Start
---
<<command \"gossip.spread\" \"the baker is a spy\" true>>
Told.
===
";

/// An app running `source` with the command verb installed and everything the
/// narrative ledger needs.
fn app(source: &str) -> App {
    let mut app = harness_app(source, &[super::install_command_binding]);
    app.init_resource::<crate::ActiveConversation>();
    app.init_resource::<crate::NarrativeInputLedger<RunAuthoredCommand>>();
    app.add_message::<RunAuthoredCommand>();
    app
}

/// Put a conversation on the record so the ledger has an instance to stamp
/// against, then release whatever the verb wrote into the message channel.
///
/// this is the production release system, not a test shortcut — a request
/// the ledger refuses to release is a request the simulation never sees.
fn released(app: &mut App) -> Vec<RunAuthoredCommand> {
    app.world_mut()
        .run_system_once(crate::release_narrative_inputs::<RunAuthoredCommand>)
        .expect("release");
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<RunAuthoredCommand>>()
        .drain()
        .collect()
}

use bevy::ecs::system::RunSystemOnce as _;

fn talking(app: &mut App) {
    app.world_mut()
        .resource_mut::<crate::ActiveConversation>()
        .open(crate::LiveConversation::for_test(
            None,
            None,
            "Start",
            crate::ConversationInputOwner::Primary,
        ));
}

// ===== The acceptance ===========================================

/// A COMMAND PUBLISHED BY A DOMAIN NOTHING NAMES IS REACHABLE FROM AUTHORED
/// `.yarn`, WITH NO EDIT TO ANY BINDING.
///
/// the milestone's consumer-side acceptance. `gossip.spread` exists only
/// in this file; no bridge, no vocabulary table and no game crate learned its
/// name. And the arguments arrive PREPARED and TYPED — the `true` that Yarn
/// hands over as the string `"true"` is an [`AuthoredArg::Truth`] by the time it
/// reaches the request.
#[test]
fn a_command_published_by_a_foreign_domain_is_requestable_from_authored_yarn() {
    let mut app = app(SOURCE);
    app.publish_command(spread_descriptor(), spread);
    talking(&mut app);

    assert!(
        released(&mut app).is_empty(),
        "something was already requested before the line ran; then this test says \
         nothing about whether the authored line did it"
    );

    start(&mut app, "Start");

    assert_eq!(
        released(&mut app),
        vec![RunAuthoredCommand::new(
            CommandId::new("gossip", "spread"),
            vec![
                AuthoredArg::Name("the baker is a spy".to_string()),
                AuthoredArg::Truth(true),
            ],
        )],
    );
}

/// AN UNPUBLISHED COMMAND ASKS FOR NOTHING, AND DOES NOT CRASH.
///
/// and note what is NOT tested here: that a bad verb name panics. It must not — `<<command>>`
/// is one registered Yarn command whatever id follows it, so an unknown domain never reaches
/// yarnspinner's `CommandNotFound` path at all.
#[test]
fn a_command_nobody_published_requests_nothing() {
    let mut app = app("\
title: Start
---
<<command \"gossip.invent\" \"x\">>
Told.
===
");
    app.publish_command(spread_descriptor(), spread);
    talking(&mut app);
    start(&mut app, "Start");

    assert!(released(&mut app).is_empty());
}

/// THE ARGUMENT COUNT IS CHECKED AGAINST THE PUBLISHED DESCRIPTOR.
#[test]
fn the_wrong_number_of_arguments_requests_nothing() {
    let mut app = app("\
title: Start
---
<<command \"gossip.spread\" \"only one\">>
Told.
===
");
    app.publish_command(spread_descriptor(), spread);
    talking(&mut app);
    start(&mut app, "Start");

    assert!(
        released(&mut app).is_empty(),
        "a one-argument call reached a two-argument command; the missing `loudly` \
         would have had to be invented, and inventing `false` CLEARS a flag the \
         author meant to set"
    );
}

/// A TRUTH IS EXACTLY `true` OR `false`.
///
/// this is the poison, and it is pointed at the direction that loses
/// data. Yarn hands a command every parameter as text, so `"yes"` and `"1"`
/// are indistinguishable from `"true"` at the type level. A lenient parse would
/// map an unrecognised spelling to `false` — and `false` on `world.set_flag` is
/// not a no-op, it is a flag being CLEARED.
#[test]
fn an_unrecognised_truth_is_refused_rather_than_read_as_false() {
    let mut app = app("\
title: Start
---
<<command \"gossip.spread\" \"the baker is a spy\" yes>>
Told.
===
");
    app.publish_command(spread_descriptor(), spread);
    talking(&mut app);
    start(&mut app, "Start");

    assert!(
        released(&mut app).is_empty(),
        "`yes` was accepted; whichever way it was read, the author's meaning was \
         guessed at"
    );
}

/// A PREPARED REFERENCE IS REFUSED RATHER THAN GUESSED.
///
/// the same refusal the condition verb makes, and the stake is higher: a
/// condition that guesses returns a wrong answer, a command that guesses changes
/// the wrong thing.
#[test]
fn a_reference_argument_is_refused_rather_than_coerced_from_a_quoted_string() {
    let mut app = app("\
title: Start
---
<<command \"gossip.nudge\" \"axe\">>
Told.
===
");
    app.publish_command(nudge_descriptor(), spread);
    talking(&mut app);
    start(&mut app, "Start");

    assert!(
        released(&mut app).is_empty(),
        "a quoted string became an occurrence identity"
    );
}

/// THE HOST PLUGIN ACTUALLY PUSHES THE VERB.
///
/// Everything else here proves the verb works once installed — this proves it gets installed.
#[test]
fn the_bindings_plugin_installs_the_command_verb() {
    let mut app = App::new();
    app.add_plugins(super::super::YarnBindingsPlugin);
    let installers = &app
        .world()
        .resource::<ambition_dialog::YarnContentBindings>()
        .installers;
    assert!(
        installers.contains(&(super::install_command_binding as _)),
        "the engine's command verb must reach every runner this plugin composes; \
         found {} other installer(s)",
        installers.len()
    );
}
