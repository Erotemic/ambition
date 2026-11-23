//! Exposes authored dialogue conditions through the shared `ConditionCatalog`.
//!
//! Yarn registers `condition(id, arg)` as a Bevy system and evaluates the condition against the
//! live world, so catalog-owned facts do not need a second `YarnStateMirror` copy. The mirror
//! remains only for facts the catalog cannot answer.
//!
//! The bridge supports exactly one authored argument because Yarn function arity is fixed. Raw
//! Yarn strings are not coerced into `ParamKind::Reference`. A `ParamKind::Individual` parameter
//! takes a character id or a role the conversation binds (`@talker`, `@initiator`).


use bevy::prelude::*;
use bevy_yarnspinner::prelude::{DialogueRunner, YarnValue};

use ambition_platformer2d_shared_tangle::authored_logic::{
    prepare_authored_arg, AuthoredArg, ConditionCatalog, ConditionId, ConditionOutcome, ParamKind,
    RoleBindings,
};

/// The name authored `.yarn` content spells this verb.
pub const YARN_FUNCTION_NAME: &str = "condition";

/// The verdict-valued verb: `verdict(id, arg)` names the answer instead of
/// collapsing it to a bool (O6b).
pub const VERDICT_FUNCTION_NAME: &str = "verdict";

/// The three words a verdict function returns. A gameplay gate compares
/// against [`SATISFIED`] for its positive arm and [`NOT_SATISFIED`] for its
/// negative arm, and refuses on [`UNANSWERABLE`]; `not condition(…)`,
/// `!= "satisfied"` and a catch-all `<<else>>` would each read an unanswered
/// question as an answered no.
pub const SATISFIED: &str = "satisfied";
pub const NOT_SATISFIED: &str = "not_satisfied";
pub const UNANSWERABLE: &str = "unanswerable";

/// The word a verdict function returns for `outcome`.
pub fn verdict_word(outcome: &ConditionOutcome) -> &'static str {
    match outcome {
        ConditionOutcome::Satisfied => SATISFIED,
        ConditionOutcome::NotSatisfied(_) => NOT_SATISFIED,
        ConditionOutcome::Unanswerable(_) => UNANSWERABLE,
    }
}

/// The answers each runner has bound in its current continuation.
///
/// A Yarn decision has no local variables, so its arms each call the verdict
/// function; this keeps those calls one question. A runner continues at most
/// once per frame (a line, an option or a command yields to the next update),
/// so an answer stamped with an earlier frame belongs to a finished decision
/// and is discarded. Ordinary Yarn variables would be shared by every runner
/// and outlive the decision.
#[derive(Resource, Default)]
pub struct DecisionVerdicts {
    frame: u32,
    bound: Vec<(Option<Entity>, String, &'static str)>,
}

/// Ask `question` once for the speaking runner's current decision: the first
/// call runs `ask`, later calls in the same continuation read its answer.
///
/// `question` must name everything the answer depends on (the condition id
/// and its argument). Without a frame clock (`FrameCount`) nothing tells one
/// continuation from the next, so every call asks.
pub fn decided(world: &mut World, question: String, ask: impl FnOnce(&mut World) -> ConditionOutcome) -> &'static str {
    let Some(frame) = world.get_resource::<bevy::diagnostic::FrameCount>().map(|frame| frame.0) else {
        return verdict_word(&ask(world));
    };
    let runner = world.get_resource::<ambition_dialog::SpeakingRunner>().and_then(|speaking| speaking.0);
    let mut verdicts = world.get_resource_or_insert_with(DecisionVerdicts::default);
    if verdicts.frame != frame {
        verdicts.frame = frame;
        verdicts.bound.clear();
    }
    if let Some((_, _, word)) = verdicts.bound.iter().find(|(by, asked, _)| *by == runner && *asked == question) {
        return word;
    }
    let word = verdict_word(&ask(world));
    world.resource_mut::<DecisionVerdicts>().bound.push((runner, question, word));
    word
}

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
    let verdict = commands.register_system(ask_verdict);
    runner.library_mut().add_function(VERDICT_FUNCTION_NAME, verdict);
}

/// `verdict(id, arg)` — the same question as `condition(id, arg)`, answered
/// with its verdict word, once per decision ([`decided`]).
fn ask_verdict(In((raw_id, raw_arg)): In<(String, YarnValue)>, world: &mut World) -> String {
    let question = format!("{raw_id} {raw_arg:?}");
    decided(world, question, |world| ask_outcome(&raw_id, raw_arg, world)).to_string()
}

/// `condition(id, arg)` — ask whichever domain published `id`.
///
/// EXCLUSIVE, and it has to be. A condition evaluator takes `&World`
/// because the catalog cannot know which domain's state answers which question;
/// a system that took `&World` could not also hold the catalog as a `Res`. The
/// cost is nothing here — this runs inside `continue_runtime`, which is already
/// an exclusive system, so no schedule sync point is added.
///
/// every refusal returns `false` and says why in a `warn!`. Yarn's `<<if>>`
/// needs a bool, so the third answer has to collapse somewhere; it collapses the
/// way [`ConditionOutcome::is_satisfied`] specifies — *unanswerable is not
/// satisfied* — which leaves an unanswerable gate CLOSED. the other direction
/// would open a door in exactly the world where the question is least understood.
///
/// The Boolean form is for positive gates only: a gate with an answered-negative
/// arm asks [`ask_verdict`] instead.
fn ask_condition(In((raw_id, raw_arg)): In<(String, YarnValue)>, world: &mut World) -> bool {
    ask_outcome(&raw_id, raw_arg, world).is_satisfied()
}

/// Ask the catalog, logging a refusal or a no.
fn ask_outcome(raw_id: &str, raw_arg: YarnValue, world: &mut World) -> ConditionOutcome {
    let Some(id) = ConditionId::parse(raw_id) else {
        warn!(
            target: "crate::dialog::authored_conditions",
            "condition({raw_id:?}, …): not a `domain.question` id; nothing was asked",
        );
        return ConditionOutcome::unanswerable(format!("{raw_id:?} is not a `domain.question` id"));
    };
    if !world.contains_resource::<ConditionCatalog>() {
        warn!(
            target: "crate::dialog::authored_conditions",
            "condition({raw_id:?}, …): no domain in this composition has published \
             any condition, so there is nothing to ask",
        );
        return ConditionOutcome::unanswerable("no domain in this composition has published any condition");
    }
    // ⭐ **WHICH NODE ASKED, read from the live conversation rather than
    // invented.** The verdict ring records who asked, and *"a dialogue node"*
    // is not a source an author can find in a script. ⚠ A Yarn function
    // running with no live conversation is a fixture, not a shipped road; it
    // gets a subject that SAYS so instead of a plausible-looking blank.
    let asked_by = ambition_platformer2d_shared_tangle::authored_logic::AuthoredAsk::new(
        "dialogue",
        // The runner asked, so it is the presented conversation's node.
        world
            .get_resource::<crate::ActiveConversation>()
            .zip(crate::speaking_conversation_instance(world))
            .and_then(|(active, instance)| active.of_instance(&instance))
            .map_or("<no live conversation>", |live| live.instance.node()),
    );
    let roles = crate::dialog::conversation_roles(crate::speaking_conversation_instance(world).as_ref());
    let outcome = world.resource_scope::<ConditionCatalog, _>(|world, catalog| {
        let args = match prepare_argument(&catalog, &id, raw_arg, &roles) {
            Ok(args) => args,
            Err(refusal) => return refusal,
        };
        catalog.evaluate(world, &id, &args, &asked_by)
    });
    match &outcome {
        ConditionOutcome::Unanswerable(reason) => {
            warn!(
                target: "crate::dialog::authored_conditions",
                "condition({raw_id:?}, …) is unanswerable: {reason}",
            );
        }
        ConditionOutcome::NotSatisfied(why) => {
            // The branch did not open, and this is the structured reason an
            // author or an agent reads instead of a bare `false` (M5).
            bevy::log::debug!(
                target: "crate::dialog::authored_conditions",
                "condition({raw_id:?}, …) is not satisfied: {why}",
            );
        }
        ConditionOutcome::Satisfied => {}
    }
    outcome
}

/// Turn one Yarn value into the [`AuthoredArg`] the published descriptor
/// declares, or refuse.
///
/// the descriptor decides the kind; the Yarn value only has to fit. The
/// obvious alternative — infer the kind from the Yarn value's own type — is the
/// lossy conversion that answers the wrong question: Yarn stores every number as
/// `f32` and has no reference type at all, so inference would silently turn
/// *"which occurrence"* into *"which name"* and hand the domain an argument that
/// type-checks and means something else.
fn prepare_argument(
    catalog: &ConditionCatalog,
    id: &ConditionId,
    value: YarnValue,
    roles: &RoleBindings,
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
        (ParamKind::Individual, YarnValue::String(text)) => {
            let arg = prepare_authored_arg(id.as_str(), param, &text).map_err(ConditionOutcome::unanswerable)?;
            return roles.bind_args(&[arg]).map_err(ConditionOutcome::unanswerable);
        }
        (ParamKind::Reference, _) => {
            return Err(ConditionOutcome::unanswerable(
                crate::dialog::reference_is_not_authorable_by_dialogue(
                    id.as_str(),
                    param.name,
                ),
            ));
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
