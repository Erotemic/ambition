//! Does a `.yarn` line actually reach a domain that published a condition —
//! through the real interpreter, with no edit to any binding?
//!
//! these tests publish their conditions FROM HERE, under a domain name
//! nothing in the engine mentions, using only
//! [`PublishCondition`](ambition_platformer2d_shared_tangle::authored_logic::PublishCondition).
//! That is the whole claim: if asking required a bridge to learn a question's
//! name, a test crate could not have taught it one. they deliberately do NOT
//! assert which conditions the engine ships — pinning that list would make every
//! new provider a failing test, which is the opposite of the property.
//!
//! and the Yarn runtime is REAL here. A test that called
//! [`super::ask_condition`] directly would prove the function works and prove
//! nothing about whether authored `<<if>>` can reach it — the interpreter's
//! `call_with_world` path, its arity assertion and its value coercion are exactly
//! the parts that were believed impossible.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::authored_logic::{
    AuthoredArg, ConditionDescriptor, ConditionId, ConditionOutcome, ParamKind, ParamSpec,
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

fn heard(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let Some(rumour) = args[0].as_name() else {
        return ConditionOutcome::unanswerable("`rumour` must be a name");
    };
    let Some(gossip) = world.get_resource::<TownGossip>() else {
        return ConditionOutcome::unanswerable("no gossip domain is installed");
    };
    ConditionOutcome::from_bool_unexplained(gossip.rumours.contains(rumour))
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

/// this evaluator would say YES, for `SimId::placement("axe")`, in the
/// world the reference test builds. It is the poison: the only way the authored
/// line can take the satisfied branch is if something coerced the quoted string
/// `"axe"` into an occurrence identity.
fn carried(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let Some(wanted) = args[0].as_reference() else {
        return ConditionOutcome::unanswerable("`occurrence` must be a prepared reference");
    };
    let Some(mut query) = world.try_query::<&SimId>() else {
        return ConditionOutcome::unanswerable("nothing in this world has an identity");
    };
    ConditionOutcome::from_bool_unexplained(query.iter(world).any(|sim_id| sim_id == wanted))
}

// ===== Harness ==================================================
//
// A second copy would have been a fork of the one piece of test code whose whole job is to be
// the production path.

use crate::dialog::yarn_harness::{advance, lines, start};

/// Build an app whose only dialogue vocabulary is the one generic condition verb.
fn app_running(source: &str) -> App {
    crate::dialog::yarn_harness::app_running(source, &[super::install_condition_binding])
}

// ===== The acceptance ===========================================

/// A CONDITION PUBLISHED BY A DOMAIN NOTHING NAMES IS ASKABLE FROM AUTHORED
/// `.yarn`, WITH NO EDIT TO ANY BINDING.
///
/// this is the behavioural test, and the thing it pins is an ABSENCE: no
/// vocabulary module, no installer, no mirror slice and no `add_function` call
/// learned the word `gossip.heard`. The only thing that happened is that a plugin
/// published it.
///
/// and the answer tracks live state. Asking twice across a mutation is what
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

    // the domain's own state changes MID-CONVERSATION. Nothing re-registers,
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

/// A QUESTION NOBODY PUBLISHED IS UNSATISFIED, NOT A CRASH AND NOT A YES.
///
/// the id is well-formed; only the publication is missing. `ConditionId::new`
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

/// THE HOST PLUGIN ACTUALLY PUSHES THE VERB.
///
/// this is the one failure in this file that would CRASH the shipped game
/// rather than close a gate. Authored `.yarn` now calls `condition(…)`; a Yarn
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

// Counts calls to `never_called`, so "refused before evaluation" and "evaluated
// and answered no" stop being the same observable.
//
// ⛔ THREAD-LOCAL, NOT A `static`: a shared counter is a channel between arms,
// and the arm below would then depend on no other arm having published this
// evaluator. The dialogue app is driven synchronously from the test thread, so
// the evaluator runs here.
thread_local! {
    static NEVER_CALLED_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Answers `Satisfied` for ANY argument, and records that it ran.
///
/// ⛔⛤ **NO ARM IN THIS FILE CAN WITNESS THE SURFACE'S REFUSAL, AND A POISON IS
/// HOW THAT WAS FOUND RATHER THAN AN ARGUMENT.** Deleting the
/// `ParamKind::Reference` arm from `prepare_argument` — the exact defect the test
/// above is NAMED for — leaves the whole crate GREEN. Twice over:
///
/// 1. `carried` itself answers `unanswerable` on a non-reference argument, so the
///    authored `<<else>>` prints "Refused." either way;
/// 2. and even an evaluator that says yes to everything is never reached, because
///    `ConditionCatalog::evaluate` compares `arg.kind()` against the descriptor's
///    `ParamKind` and refuses the mismatch BEFORE dispatching.
///
/// ⇒ **THE CATALOG OWNS THE SAFETY AND THE DIALOGUE SURFACE OWNS THE WORDING.**
/// The arm in `prepare_argument` is not a second guard; it is a better sentence,
/// written where the author's mistake is legible. That is worth keeping and worth
/// not mistaking for the thing that makes coercion impossible.
fn never_called(_world: &World, _args: &[AuthoredArg]) -> ConditionOutcome {
    NEVER_CALLED_CALLS.with(|calls| calls.set(calls.get() + 1));
    ConditionOutcome::from_bool_unexplained(true)
}

/// A PREPARED REFERENCE IS REFUSED RATHER THAN GUESSED.
///
/// the fixture is poisoned so that only the WRONG implementation can pass
/// it. The world really does contain `placement:axe`; the published evaluator
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
    // the identity the evaluator would say YES about, really present.
    app.world_mut().spawn(SimId::placement("axe"));

    start(&mut app, "Start");
    assert_eq!(
        lines(&app),
        vec!["Refused.".to_string()],
        "a quoted string is not an identity; coercing one would answer \
         confidently about whichever occurrence happened to share the spelling"
    );
}

/// NO AUTHORED STRING EVER REACHES AN EVALUATOR THAT DECLARED A REFERENCE.
///
/// ⛔ This is the SAFETY property, and it belongs to `ConditionCatalog::evaluate`
/// rather than to this surface — see [`never_called`]. The evaluator here says
/// YES to anything and counts its calls, so a coerced identity getting through
/// would be loud: the counter would be 1 and the line would read "Carried.".
/// Pinned here because the dialogue surface is the road an author actually takes,
/// and because deleting the surface's own refusal does NOT redden it — which is a
/// fact about where the guarantee lives, not a gap.
#[test]
fn a_reference_argument_never_reaches_the_evaluator_whatever_the_surface_does() {
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
    NEVER_CALLED_CALLS.with(|calls| calls.set(0));
    let mut app = app_running(SOURCE);
    app.publish_condition(carried_descriptor(), never_called);
    app.world_mut().spawn(SimId::placement("axe"));

    start(&mut app, "Start");
    assert_eq!(
        NEVER_CALLED_CALLS.with(std::cell::Cell::get),
        0,
        "the evaluator ran, so the reference was converted to SOMETHING and \
         handed over — the surface is supposed to refuse before this point"
    );
    assert_eq!(
        lines(&app),
        vec!["Refused.".to_string()],
        "an evaluator that says yes to everything still did not make this line \
         satisfied, which is only possible if it was never asked"
    );

    // ⛔ THE POSITIVE CONTROL FOR THE COUNTER, AND IT IS NOT OPTIONAL. Zero is
    // what this arm asserts AND what a counter nobody can reach reads — a
    // `thread_local!` observed from the wrong thread, an evaluator published
    // under the wrong id, a fixture that never starts. So publish the SAME
    // counting evaluator against a `ParamKind::Name` parameter, which the
    // surface does hand over, and require the counter to move.
    const REACHABLE: &str = "\
title: Start
---
<<if condition(\"gossip.heard\", \"the_bell\")>>
Heard.
<<else>>
Silent.
<<endif>>
===
";
    NEVER_CALLED_CALLS.with(|calls| calls.set(0));
    let mut reachable = app_running(REACHABLE);
    reachable.publish_condition(heard_descriptor(), never_called);
    start(&mut reachable, "Start");
    assert_eq!(
        NEVER_CALLED_CALLS.with(std::cell::Cell::get),
        1,
        "the counting evaluator was never reached even for a parameter the \
         surface DOES pass, so the zero asserted above is a property of the \
         counter rather than of the refusal"
    );
}
