//! Exposes authored dialogue conditions through the shared `ConditionCatalog`.
//!
//! Yarn registers `condition(id, arg)` as a Bevy system and evaluates the condition against the
//! live world, so catalog-owned facts do not need a second `YarnStateMirror` copy. The mirror
//! remains only for facts the catalog cannot answer.
//!
//! The bridge supports exactly one authored argument because Yarn function arity is fixed. Raw
//! Yarn strings are not coerced into `ParamKind::Reference`; reference conditions are refused
//! until an authored reference representation can produce a validated `SimId`.

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
fn ask_condition(In((raw_id, raw_arg)): In<(String, YarnValue)>, world: &mut World) -> bool {
    let Some(id) = ConditionId::parse(&raw_id) else {
        warn!(
            target: "crate::dialog::authored_conditions",
            "condition({raw_id:?}, …): not a `domain.question` id; nothing was asked",
        );
        return false;
    };
    if !world.contains_resource::<ConditionCatalog>() {
        warn!(
            target: "crate::dialog::authored_conditions",
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
                target: "crate::dialog::authored_conditions",
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
