//! **`<<if condition("world.flag_set", "bob_field_survey_received")>>` — authored
//! dialogue asking the engine a question the engine already knows how to
//! answer.**
//!
//! # The second authority this deletes
//!
//! Until this module there were **two** unrelated ways for authored content to
//! ask the world something:
//!
//! ```text
//! a lock wall  ->  ConditionCatalog        ->  world.flag_set(<gated_by>)
//! a .yarn line ->  a hand-written closure  ->  flag("<id>") over YarnStateMirror
//! ```
//!
//! Two mechanisms, one question. Adding a gate-able fact meant publishing a
//! condition **and** writing a mirror slice **and** writing a Yarn closure, and
//! the two could disagree about what the answer meant without anything noticing.
//! **that is the "second authority" shape this project refuses elsewhere**, and
//! it was live here.
//!
//! Now there is one verb. A domain publishes a condition from its own plugin —
//! the same three-line surface `custody.is_held` and `world.flag_set` already use
//! — and authored dialogue can ask it **with no edit here, in `ambition_dialog`,
//! or in any game's vocabulary module**. this file names no question, no
//! domain, and no flag; grep it and see.
//!
//! # A Yarn function CAN be a Bevy system, and the tree said otherwise
//!
//! The reason a `flag(…)` closure existed at all was a documented belief, written
//! into `ambition_content::yarn_vocabulary`'s header and into `ambition_dialog`'s:
//!
//! > *"Functions can't be Bevy systems — they're called synchronously from the
//! > runtime interpreter — so they read save state through a shared
//! > `YarnStateMirror` refreshed each frame."*
//!
//! **that was true of the crate at some version and is not true of the one in
//! the lockfile.** `bevy_yarnspinner` advances the interpreter from
//! `continue_runtime`, an **exclusive** system, and threads its `&mut World` all
//! the way down: `Dialogue::continue_with_world` → `YarnFn::call_with_world` →
//! `World::run_system_with`. `SystemId<In<P>, O>` implements `YarnFn`, so a
//! registered Bevy system — including an exclusive one — is a legal Yarn
//! function and receives the live world.
//!
//! ⇒ a condition evaluator's `&World` requirement, which looked like the reason
//! the mirror had to exist, is satisfied directly. **No projection, no staleness,
//! no second copy of the answer.**
//!
//! # The mirror is not dead, and what is left of it is a projection
//!
//! [`ambition_dialog::YarnStateMirror`] still carries what the catalog cannot yet
//! answer (boss/quest state, visit counts, wallet, content `extras`). **it is
//! downstream, not a peer**: a fact the catalog can answer must be asked, never
//! mirrored, or the two mechanisms are back. See
//! `docs/planning/engine/authored-gameplay-logic-and-orchestration.md`.
//!
//! # Why exactly ONE argument, which is a real limit and not a taste
//!
//! `condition(id, arg)` takes an id and one argument, so it can ask any published
//! condition whose descriptor declares exactly one parameter. That is not a
//! design preference — Yarn's VM **asserts** that a call's argument count equals
//! the registered function's parameter count
//! (`yarnspinner_runtime::virtual_machine`), and `Option` parameters are counted
//! too, so a variadic bridge is not expressible. A zero- or two-parameter
//! condition needs a sibling registration here.
//!
//! **that sibling is deliberately not written.** Every condition published at
//! HEAD takes exactly one argument; a `condition2` with no caller would be
//! vocabulary nobody speaks, and the arity mismatch is already reported as an
//! [`ConditionOutcome::Unanswerable`] naming the real arity rather than answering
//! wrongly.
//!
//! # A prepared REFERENCE is refused, on purpose
//!
//! `ParamKind::Reference` is a [`SimId`](ambition_platformer2d_shared_tangle::sim_id::SimId) —
//! *"never a raw string and never an `Entity`"*, in the contract's own words,
//! because a string reference is un-renameable and un-validatable. A `.yarn`
//! literal is a string. Coercing one into the other would let
//! `condition("custody.is_held", "axe")` answer confidently about
//! `placement:axe` — and answer *false-shaped* nonsense the day the occurrence is
//! named anything else.
//!
//! ⇒ this refuses, with a reason. Prepared references from authored source are job, and a
//! refusal is a thing can replace; a wrong answer is a thing would have to find first.

use bevy::prelude::*;
use bevy_yarnspinner::prelude::{DialogueRunner, YarnValue};

use ambition_platformer2d_shared_tangle::authored_logic::{
    AuthoredArg, ConditionCatalog, ConditionId, ConditionOutcome, ParamKind,
};

/// The name authored `.yarn` content spells this verb.
pub const YARN_FUNCTION_NAME: &str = "condition";

/// Install the one generic condition verb on a freshly built runner.
///
/// Pushed into [`ambition_dialog::YarnContentBindings`] by
/// [`super::YarnBindingsPlugin`]. the mirror argument is unused and that is the
/// whole point of this module: this verb reads the live world, not a snapshot of
/// it.
pub fn install_condition_binding(
    commands: &mut Commands,
    runner: &mut DialogueRunner,
    _mirror: &ambition_dialog::YarnStateMirror,
) {
    let ask = commands.register_system(ask_condition);
    runner.library_mut().add_function(YARN_FUNCTION_NAME, ask);
}

/// `condition(id, arg)` — ask whichever domain published `id`.
///
/// **EXCLUSIVE, and it has to be.** A condition evaluator takes `&World`
/// because the catalog cannot know which domain's state answers which question;
/// a system that took `&World` could not also hold the catalog as a `Res`. The
/// cost is nothing here — this runs inside `continue_runtime`, which is already
/// an exclusive system, so no schedule sync point is added.
///
/// **every refusal returns `false` and says why in a `warn!`.** Yarn's `<<if>>`
/// needs a bool, so the third answer has to collapse somewhere; it collapses the
/// way [`ConditionOutcome::is_satisfied`] specifies — *unanswerable is not
/// satisfied* — which leaves an unanswerable gate CLOSED. the other direction
/// would open a door in exactly the world where the question is least understood.
fn ask_condition(In((raw_id, raw_arg)): In<(String, YarnValue)>, world: &mut World) -> bool {
    let Some(id) = ConditionId::parse(&raw_id) else {
        warn!(
            target: "ambition_conversation::dialog::authored_conditions",
            "condition({raw_id:?}, …): not a `domain.question` id; nothing was asked",
        );
        return false;
    };
    if !world.contains_resource::<ConditionCatalog>() {
        warn!(
            target: "ambition_conversation::dialog::authored_conditions",
            "condition({raw_id:?}, …): no domain in this composition has published \
             any condition, so there is nothing to ask",
        );
        return false;
    }
    let outcome = world.resource_scope::<ConditionCatalog, _>(|world, catalog| {
        let args = match prepare_argument(&catalog, &id, raw_arg) {
            Ok(args) => args,
            Err(refusal) => return refusal,
        };
        catalog.evaluate(world, &id, &args)
    });
    match &outcome {
        ConditionOutcome::Unanswerable(reason) => {
            warn!(
                target: "ambition_conversation::dialog::authored_conditions",
                "condition({raw_id:?}, …) is unanswerable: {reason}",
            );
        }
        ConditionOutcome::Satisfied | ConditionOutcome::NotSatisfied => {}
    }
    outcome.is_satisfied()
}

/// Turn one Yarn value into the [`AuthoredArg`] the published descriptor
/// declares, or refuse.
///
/// **the descriptor decides the kind; the Yarn value only has to fit.** The
/// obvious alternative — infer the kind from the Yarn value's own type — is the
/// lossy conversion that answers the wrong question: Yarn stores every number as
/// `f32` and has no reference type at all, so inference would silently turn
/// *"which occurrence"* into *"which name"* and hand the domain an argument that
/// type-checks and means something else.
fn prepare_argument(
    catalog: &ConditionCatalog,
    id: &ConditionId,
    value: YarnValue,
) -> Result<Vec<AuthoredArg>, ConditionOutcome> {
    // Re-writing that message here would be a second, worse copy of it.
    let Some(descriptor) = catalog.describe(id) else {
        return Ok(Vec::new());
    };
    let [param] = descriptor.params else {
        return Err(ConditionOutcome::unanswerable(format!(
            "`{id}` takes {} argument(s); authored dialogue's `condition(id, arg)` \
             can only ask a condition that takes exactly one",
            descriptor.params.len()
        )));
    };
    let arg = match (param.kind, value) {
        (ParamKind::Name, YarnValue::String(name)) => AuthoredArg::Name(name),
        (ParamKind::Number, YarnValue::Number(number)) => AuthoredArg::Number(number as f64),
        (ParamKind::Truth, YarnValue::Boolean(truth)) => AuthoredArg::Truth(truth),
        (ParamKind::Reference, _) => {
            return Err(ConditionOutcome::unanswerable(format!(
                "`{id}` argument `{}` is a prepared reference to an occurrence, and \
                 authored dialogue can only pass names, numbers and truths — a \
                 quoted string is not an identity",
                param.name
            )));
        }
        (kind, other) => {
            return Err(ConditionOutcome::unanswerable(format!(
                "`{id}` argument `{}` is a {kind:?}, and the authored value {other:?} \
                 is not one",
                param.name
            )));
        }
    };
    Ok(vec![arg])
}

#[cfg(test)]
mod tests;
