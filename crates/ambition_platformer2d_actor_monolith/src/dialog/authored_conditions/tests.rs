//! **Does a `.yarn` line actually reach a domain that published a condition —
//! through the real interpreter, with no edit to any binding?**
//!
//! ⭐⭐ **these tests publish their conditions FROM HERE**, under a domain name
//! nothing in the engine mentions, using only
//! [`PublishCondition`](ambition_platformer2d_shared_tangle::authored_logic::PublishCondition).
//! That is the whole claim: if asking required a bridge to learn a question's
//! name, a test crate could not have taught it one. ⛔ they deliberately do NOT
//! assert which conditions the engine ships — pinning that list would make every
//! new provider a failing test, which is the opposite of the property.
//!
//! ⚠ **and the Yarn runtime is REAL here.** A test that called
//! [`super::ask_condition`] directly would prove the function works and prove
//! nothing about whether authored `<<if>>` can reach it — the interpreter's
//! `call_with_world` path, its arity assertion and its value coercion are exactly
//! the parts that were believed impossible.

use bevy::prelude::*;
use bevy_yarnspinner::events::PresentLine;
use bevy_yarnspinner::prelude::*;

use ambition_platformer2d_shared_tangle::authored_logic::{
    ConditionArg, ConditionDescriptor, ConditionId, ConditionOutcome, ParamKind, ParamSpec,
    PublishCondition,
};
use ambition_platformer2d_shared_tangle::sim_id::SimId;

// ===== A domain that exists only in this file ===================

/// The state the test domain answers about. Any old resource — the point is that
/// the catalog has no idea what it is.
#[derive(Resource, Default)]
struct TownGossip {
    rumours: std::collections::HashSet<String>,
}

const RUMOUR: ParamSpec = ParamSpec {
    name: "rumour",
    kind: ParamKind::Name,
    summary: "the rumour being asked about",
};

fn heard_descriptor() -> ConditionDescriptor {
    ConditionDescriptor {
        id: ConditionId::new("gossip", "heard"),
        summary: "true once the town has heard the named rumour",
        params: &[RUMOUR],
    }
}

fn heard(world: &World, args: &[ConditionArg]) -> ConditionOutcome {
    let Some(rumour) = args[0].as_name() else {
        return ConditionOutcome::unanswerable("`rumour` must be a name");
    };
    let Some(gossip) = world.get_resource::<TownGossip>() else {
        return ConditionOutcome::unanswerable("no gossip domain is installed");
    };
    ConditionOutcome::from_bool(gossip.rumours.contains(rumour))
}

const CARRIED: ParamSpec = ParamSpec {
    name: "occurrence",
    kind: ParamKind::Reference,
    summary: "the occurrence being asked about",
};

fn carried_descriptor() -> ConditionDescriptor {
    ConditionDescriptor {
        id: ConditionId::new("gossip", "carried"),
        summary: "true while the named occurrence is being carried",
        params: &[CARRIED],
    }
}

/// ⚠ **this evaluator would say YES**, for `SimId::placement("axe")`, in the
/// world the reference test builds. It is the poison: the only way the authored
/// line can take the satisfied branch is if something coerced the quoted string
/// `"axe"` into an occurrence identity.
fn carried(world: &World, args: &[ConditionArg]) -> ConditionOutcome {
    let Some(wanted) = args[0].as_reference() else {
        return ConditionOutcome::unanswerable("`occurrence` must be a prepared reference");
    };
    let Some(mut query) = world.try_query::<&SimId>() else {
        return ConditionOutcome::unanswerable("nothing in this world has an identity");
    };
    ConditionOutcome::from_bool(query.iter(world).any(|sim_id| sim_id == wanted))
}

// ===== Harness ==================================================

/// Every line the interpreter presented, in order.
#[derive(Resource, Default)]
struct PresentedLines(Vec<String>);

fn record_line(event: On<PresentLine>, mut lines: ResMut<PresentedLines>) {
    lines.0.push(event.line.text.clone());
}

/// Build an app whose only dialogue vocabulary is the one generic verb.
///
/// ⚠ it installs through [`super::install_condition_binding`] — the exact
/// function `YarnBindingsPlugin` pushes into the installer seam — rather than
/// calling `add_function` itself, so a change that broke the production
/// installation would break this too.
fn app_running(source: &str) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin {
        watch_for_changes_override: Some(false),
        ..default()
    });
    app.add_plugins(YarnSpinnerPlugin::with_yarn_source(
        YarnFileSource::InMemory(YarnFile::new("authored_conditions_test.yarn", source)),
    ));
    app.init_resource::<PresentedLines>();
    app.add_observer(record_line);
    app
}

fn runner_entity(app: &mut App) -> Entity {
    if let Some(entity) = app
        .world_mut()
        .query_filtered::<Entity, With<DialogueRunner>>()
        .iter(app.world())
        .next()
    {
        return entity;
    }
    while !app.world().contains_resource::<YarnProject>() {
        app.update();
    }
    let mirror = ambition_dialog::YarnStateMirror::default();
    let mut system_state: bevy::ecs::system::SystemState<(Commands, Res<YarnProject>)> =
        bevy::ecs::system::SystemState::new(app.world_mut());
    let (mut commands, project) = system_state.get_mut(app.world_mut());
    let mut runner = project.create_dialogue_runner(&mut commands);
    super::install_condition_binding(&mut commands, &mut runner, &mirror);
    system_state.apply(app.world_mut());
    app.world_mut().spawn(runner).id()
}

/// Spawn the runner if needed, install the verb, and run the node to its first
/// line.
fn start(app: &mut App, node: &str) {
    let entity = runner_entity(app);
    let mut runner = app
        .world_mut()
        .get_mut::<DialogueRunner>(entity)
        .expect("runner");
    if runner.is_running() {
        runner.stop();
    }
    runner.start_node(node);
    app.update();
}

/// Advance one beat.
fn advance(app: &mut App) {
    let entity = runner_entity(app);
    app.world_mut()
        .get_mut::<DialogueRunner>(entity)
        .expect("runner")
        .continue_in_next_update();
    app.update();
}

fn lines(app: &App) -> Vec<String> {
    app.world().resource::<PresentedLines>().0.clone()
}

// ===== The acceptance ===========================================

/// **A CONDITION PUBLISHED BY A DOMAIN NOTHING NAMES IS ASKABLE FROM AUTHORED
/// `.yarn`, WITH NO EDIT TO ANY BINDING.**
///
/// ⭐⭐ this is the behavioural test, and the thing it pins is an ABSENCE: no
/// vocabulary module, no installer, no mirror slice and no `add_function` call
/// learned the word `gossip.heard`. The only thing that happened is that a plugin
/// published it.
///
/// ⚠ **and the answer tracks live state.** Asking twice across a mutation is what
/// separates *"the catalog was consulted"* from *"a value was copied at
/// startup"* — the mirror this replaces could have passed the first half.
#[test]
fn a_condition_published_by_a_stranger_is_askable_from_authored_yarn() {
    const SOURCE: &str = "\
title: Start
---
<<if condition(\"gossip.heard\", \"the_bell\")>>
first: they are talking about the bell.
<<else>>
first: nobody has mentioned the bell.
<<endif>>
<<if condition(\"gossip.heard\", \"the_bell\")>>
second: they are talking about the bell.
<<else>>
second: nobody has mentioned the bell.
<<endif>>
===
";
    let mut app = app_running(SOURCE);
    app.init_resource::<TownGossip>();
    app.publish_condition(heard_descriptor(), heard);

    start(&mut app, "Start");
    assert_eq!(
        lines(&app),
        vec!["first: nobody has mentioned the bell.".to_string()],
        "the rumour has not been heard, and the authored else-branch says so"
    );

    // ⭐ the domain's own state changes MID-CONVERSATION. Nothing re-registers,
    // re-mirrors, refreshes or restarts anything — the next `<<if>>` is the same
    // authored expression in the same running interpreter.
    app.world_mut()
        .resource_mut::<TownGossip>()
        .rumours
        .insert("the_bell".to_string());
    advance(&mut app);

    assert_eq!(
        lines(&app),
        vec![
            "first: nobody has mentioned the bell.".to_string(),
            "second: they are talking about the bell.".to_string(),
        ],
        "the identical authored expression takes the other branch one beat later, \
         because the verb asks the live world rather than reading a snapshot"
    );
}

/// **A QUESTION NOBODY PUBLISHED IS UNSATISFIED, NOT A CRASH AND NOT A YES.**
///
/// ⚠ the id is well-formed; only the publication is missing. ⛔ `ConditionId::new`
/// would have PANICKED on the malformed one below, which is why authored content
/// goes through `parse`.
#[test]
fn an_unpublished_or_malformed_question_leaves_the_gate_closed() {
    const SOURCE: &str = "\
title: Start
---
<<if condition(\"gossip.never_published\", \"x\")>>
Answered.
<<else>>
Refused.
<<endif>>
<<if condition(\"not_an_id_at_all\", \"x\")>>
Answered.
<<else>>
Refused.
<<endif>>
===
";
    let mut app = app_running(SOURCE);
    app.init_resource::<TownGossip>();
    app.publish_condition(heard_descriptor(), heard);

    start(&mut app, "Start");
    // Advance past the first line to reach the second `<<if>>`.
    advance(&mut app);

    assert_eq!(
        lines(&app),
        vec!["Refused.".to_string(), "Refused.".to_string()],
        "an unanswerable condition is NOT satisfied — a gate that opened because \
         nobody could answer would open in the world least understood"
    );
}

/// **THE HOST PLUGIN ACTUALLY PUSHES THE VERB.**
///
/// ⛔ **this is the one failure in this file that would CRASH the shipped game
/// rather than close a gate.** Authored `.yarn` now calls `condition(…)`; a Yarn
/// call to a function no runner registered is `FunctionNotFound`, and
/// `bevy_yarnspinner` pipes that into `panic_on_err`. Everything else here proves
/// the verb works once installed — this proves it gets installed.
#[test]
fn the_bindings_plugin_installs_the_condition_verb() {
    let mut app = App::new();
    app.add_plugins(super::super::YarnBindingsPlugin);
    let installers = &app
        .world()
        .resource::<ambition_dialog::YarnContentBindings>()
        .installers;
    assert!(
        installers.contains(&(super::install_condition_binding as _)),
        "the engine's condition verb must reach every runner this plugin composes; \
         found {} other installer(s)",
        installers.len()
    );
}

/// **A PREPARED REFERENCE IS REFUSED RATHER THAN GUESSED.**
///
/// ⭐⭐ **the fixture is poisoned so that only the WRONG implementation can pass
/// it.** The world really does contain `placement:axe`; the published evaluator
/// really would answer `Satisfied` if it were handed that identity. The only way
/// the authored line can print "Carried." is if something turned the quoted
/// string `"axe"` into a [`SimId`] — which is precisely the un-renameable string
/// reference the contract forbids.
#[test]
fn a_reference_argument_is_refused_rather_than_coerced_from_a_quoted_string() {
    const SOURCE: &str = "\
title: Start
---
<<if condition(\"gossip.carried\", \"axe\")>>
Carried.
<<else>>
Refused.
<<endif>>
===
";
    let mut app = app_running(SOURCE);
    app.publish_condition(carried_descriptor(), carried);
    // ⚠ the identity the evaluator would say YES about, really present.
    app.world_mut().spawn(SimId::placement("axe"));

    start(&mut app, "Start");
    assert_eq!(
        lines(&app),
        vec!["Refused.".to_string()],
        "a quoted string is not an identity; coercing one would answer \
         confidently about whichever occurrence happened to share the spelling"
    );
}
