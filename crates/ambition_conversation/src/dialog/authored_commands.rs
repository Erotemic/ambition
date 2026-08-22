//! Generic Yarn bridge for authored engine commands.
//!
//! `<<command ...>>` resolves the command through [`CommandCatalog`] and records a
//! request in [`NarrativeInputLedger`](crate::NarrativeInputLedger). The Yarn
//! runner executes outside rollback simulation, so it never mutates gameplay state
//! directly. Yarn command arguments arrive as text and are parsed according to the
//! published descriptor. Raw Yarn strings cannot satisfy `ParamKind::Reference`;
//! references require prepared `SimId` values.

use bevy::prelude::*;
use bevy_yarnspinner::prelude::DialogueRunner;

use ambition_platformer2d_shared_tangle::authored_logic::{
    AuthoredArg, CommandCatalog, CommandDescriptor, CommandId, ParamKind, ParamSpec,
    RunAuthoredCommand,
};

use crate::ledger::NarrativeInputWriter;

/// The name authored `.yarn` content spells this verb.
pub const YARN_COMMAND_NAME: &str = "command";

/// How many arguments one authored call may carry after the id.
///
/// a limit of this file, not of Yarn — see the header. Stated as a
/// constant so the refusal message and the tuple below cannot disagree.
pub const MAX_AUTHORED_ARGS: usize = 3;

/// Install the one generic command verb on a freshly built runner.
///
/// Pushed into [`ambition_dialog::YarnContentBindings`] beside the condition
/// verb. the mirror argument is unused, for the same reason it is unused
/// there: nothing here reads a projection of the world.
pub fn install_command_binding(
    commands: &mut Commands,
    runner: &mut DialogueRunner,
    _mirror: &ambition_dialog::YarnStateMirror,
) {
    let run = commands.register_system(request_authored_command);
    runner.commands_mut().add_command(YARN_COMMAND_NAME, run);
}

/// `<<command "domain.verb" arg…>>` — ask whichever domain published the verb.
///
/// every refusal is a `warn!` and nothing happening. A Yarn command has no
/// return value, so there is nowhere for an outcome to go; the alternative to
/// logging is a silent no-op, which is how an author spends an afternoon on a
/// typo.
fn request_authored_command(
    In((raw_id, a0, a1, a2)): In<(String, Option<String>, Option<String>, Option<String>)>,
    catalog: Option<Res<CommandCatalog>>,
    mut narrative: NarrativeInputWriter<RunAuthoredCommand>,
) {
    let Some(id) = CommandId::parse(&raw_id) else {
        warn!(
            target: "ambition_conversation::dialog::authored_commands",
            "command({raw_id:?}, …): not a `domain.verb` id; nothing was asked for",
        );
        return;
    };
    let Some(catalog) = catalog else {
        warn!(
            target: "ambition_conversation::dialog::authored_commands",
            "command({raw_id:?}, …): no domain in this composition has published \
             any command, so there is nothing to ask for",
        );
        return;
    };
    // an unpublished id is refused HERE rather than passed through, which
    // is the opposite of what the condition verb does — and the difference is
    // the ledger. A condition's refusal is produced by the catalog at the moment
    // it is asked, so passing through gets the catalog's own better message. A
    // command's request is stamped for a FUTURE tick, so passing an unpublished
    // id through would log the refusal a tick later, out of any authored
    // context, with no line number and no conversation.
    let Some(descriptor) = catalog.describe(&id) else {
        warn!(
            target: "ambition_conversation::dialog::authored_commands",
            "command({raw_id:?}, …): no such command is published; the installed \
             engine knows {} others",
            catalog.len(),
        );
        return;
    };
    let authored = [a0, a1, a2];
    match prepare_arguments(descriptor, &authored) {
        Ok(args) => narrative.write(RunAuthoredCommand::new(id, args)),
        Err(refusal) => warn!(
            target: "ambition_conversation::dialog::authored_commands",
            "command({raw_id:?}, …) was refused: {refusal}",
        ),
    }
}

/// Turn the authored text into the [`AuthoredArg`]s the published descriptor
/// declares, or refuse with a reason an author can act on.
///
/// the descriptor decides the kind; the authored text only has to fit.
/// See the module header: Yarn hands a command its parameters untyped, so there
/// is no value-side type to infer from even if inferring were a good idea.
fn prepare_arguments(
    descriptor: &CommandDescriptor,
    authored: &[Option<String>; MAX_AUTHORED_ARGS],
) -> Result<Vec<AuthoredArg>, String> {
    let given = authored.iter().filter(|slot| slot.is_some()).count();
    if descriptor.params.len() > MAX_AUTHORED_ARGS {
        return Err(format!(
            "`{}` takes {} arguments; authored dialogue's `command` verb can \
             carry at most {MAX_AUTHORED_ARGS}",
            descriptor.id,
            descriptor.params.len(),
        ));
    }
    if given != descriptor.params.len() {
        return Err(format!(
            "`{}` takes {} argument(s) ({}), got {given}",
            descriptor.id,
            descriptor.params.len(),
            descriptor
                .params
                .iter()
                .map(|p| p.name)
                .collect::<Vec<_>>()
                .join(", "),
        ));
    }
    descriptor
        .params
        .iter()
        .zip(authored.iter().flatten())
        .map(|(param, text)| prepare_one(&descriptor.id, param, text))
        .collect()
}

fn prepare_one(id: &CommandId, param: &ParamSpec, text: &str) -> Result<AuthoredArg, String> {
    match param.kind {
        ParamKind::Name => Ok(AuthoredArg::Name(text.to_string())),
        ParamKind::Number => text.parse::<f64>().map(AuthoredArg::Number).map_err(|_| {
            format!(
                "`{id}` argument `{}` is a Number, and the authored value {text:?} \
                 is not one",
                param.name
            )
        }),
        // exactly `true` / `false`, with no `1`, `yes` or `on`. A verb that
        // accepted four spellings of truth would accept a fifth by accident, and
        // a mistyped one would read as `false` — which is a flag being CLEARED
        // when the author meant to set it.
        ParamKind::Truth => match text {
            "true" => Ok(AuthoredArg::Truth(true)),
            "false" => Ok(AuthoredArg::Truth(false)),
            other => Err(format!(
                "`{id}` argument `{}` is a Truth, and the authored value {other:?} \
                 is neither `true` nor `false`",
                param.name
            )),
        },
        ParamKind::Reference => Err(format!(
            "`{id}` argument `{}` is a prepared reference to an occurrence, and \
             authored dialogue can only pass names, numbers and truths — a quoted \
             string is not an identity",
            param.name
        )),
    }
}

#[cfg(test)]
mod tests;
