//! **`<<command "world.set_flag" "bob_field_survey_received" true>>` — authored
//! dialogue telling the engine to do something the engine already knows how to
//! do.**
//!
//! # The second authority this deletes
//!
//! [`authored_conditions`](super::authored_conditions) removed the second way
//! for a `.yarn` line to ASK the world something. This removes the second way
//! for one to TELL it:
//!
//! ```text
//! a domain      ->  its own typed request bus  ->  SetFlagRequested
//! a .yarn line  ->  a hand-written Bevy system ->  cmd_set_flag / cmd_clear_flag
//! ```
//!
//! Two mechanisms, one verb. Every gameplay-bearing verb an author could reach
//! was a Rust function in a game's vocabulary module, registered by name in a
//! second list, with its own conversion from Yarn's untyped text — so adding one
//! meant editing a game crate, and a verb the engine already published was
//! unreachable from content until somebody wrote the binding.
//!
//! Now there is one verb. A domain publishes a command from its own plugin — the
//! same three-line surface `world.set_flag` uses — and authored dialogue can ask
//! for it **with no edit here, in `ambition_dialog`, or in any game's vocabulary
//! module**. this file names no command, no domain and no flag; grep it and
//! see.
//!
//! # Why this is not the condition verb with a different name
//!
//! Three differences, all forced:
//!
//! **1. It cannot perform anything.** A condition verb evaluates and returns an
//! answer inside the runner's own exclusive system. A command verb must not
//! touch the world at all from here: the Yarn runner executes in `Update`,
//! outside the simulation and outside rollback, and a write from there is wiped
//! by the next rewind and never re-derived. So this **records a request** in the
//! [`NarrativeInputLedger`](crate::NarrativeInputLedger) stamped with the tick
//! it applies from, exactly as every other gameplay-bearing Yarn command did,
//! and the simulation performs it.
//!
//! **2. It is not limited to one argument.** The condition verb is, because
//! Yarn's VM *asserts* that a FUNCTION call's argument count equals the
//! registered parameter count. A **command** is dispatched by name with its
//! parameters as a list (`yarnspinner_runtime::command::Command::parse`), no
//! arity assertion anywhere, and `Option` parameters retrieve `None` when the
//! list runs out. **so the cap here is this file's own** — three arguments,
//! which is one more than any published command takes. Widening it is adding an
//! `Option` to a tuple, and nothing else.
//!
//! **3. Every authored argument arrives as TEXT.** Yarn types a function's
//! arguments; it does not type a command's — `Command::parse` splits the line
//! and maps every component through `YarnValue::from(String)`, so `true` reaches
//! this file as the string `"true"`. ⇒ this file parses, and it parses **against
//! the published descriptor's declared kind**. the alternative — guess the
//! kind from the text — is the lossy conversion that silently turns a flag named
//! `"1"` into a number.
//!
//! # A prepared REFERENCE is refused, on purpose
//!
//! Same rule as the condition verb, same reason: a [`ParamKind:Reference`] is a `SimId`, a
//! `.yarn` literal is a string, and coercing one into the other would perform a verb
//! confidently against whichever occurrence happens to share the spelling. **and the stake is
//! higher on this side** — a condition that guesses returns a wrong answer, a command that
//! guesses changes the wrong thing.

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
/// **a limit of this file, not of Yarn** — see the header. Stated as a
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
/// **every refusal is a `warn!` and nothing happening.** A Yarn command has no
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
    // **an unpublished id is refused HERE rather than passed through**, which
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
/// **the descriptor decides the kind; the authored text only has to fit.**
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
        // **exactly `true` / `false`, with no `1`, `yes` or `on`.** A verb that
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
