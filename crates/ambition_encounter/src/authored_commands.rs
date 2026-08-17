//! **`encounter.signal` — the encounter domain's authored verb.**
//!
//! The lifecycle reducer has always been driven by one generic message
//! ([`EncounterCommand`]), and every emitter of that message so far has been a
//! hand-written Rust system holding a `&'static str` encounter id. This
//! publishes the same verb into the shared command catalog, so authored content
//! can name it without a system and without an id in a compiler.
//!
//! ⭐ **it is the SECOND provider of the command contract, and it edits nothing
//! central to be one.** `world.set_flag` lives in the world-facts domain; this
//! lives here, beside the reducer it feeds. Neither names the other.
//!
//! # ⭐⭐ Why the target is a prepared REFERENCE and not a name
//!
//! [`EncounterCommand`] carries an encounter id as a `String`, and the shortest
//! version of this verb would have taken `ParamKind::Name` and passed it
//! straight through. It takes a [`SimId`] instead, and the difference is
//! visible in behaviour rather than in taste:
//!
//! - a **name** that matches nothing produces an `EncounterCommand` addressed to
//!   an encounter that does not exist. The reducer skips it, silently, forever —
//!   the exact failure mode of a typo'd id in a const table.
//! - a **reference** is resolved here, against the live occurrences, and an
//!   unresolvable one is [`CommandOutcome::Refused`] with a reason naming the
//!   identity. An author gets a sentence instead of a puzzle.
//!
//! ⚠ **and the id the reducer receives is read off the resolved occurrence**,
//! never recovered from the reference's spelling. [`SimId`]'s own docs say the
//! string is *"not parsed"* and that nothing may recover a fact from it; this
//! obeys that by looking the occurrence up and asking it for its own id.

use bevy::prelude::{App, World};

use ambition_platformer2d_shared_tangle::authored_logic::{
    AuthoredArg, CommandDescriptor, CommandId, CommandOutcome, ParamKind, ParamSpec, PublishCommand,
};
use ambition_platformer2d_shared_tangle::sim_id::SimId;

use crate::entity::Encounter;
use crate::lifecycle::EncounterCommand;

const TARGET: ParamSpec = ParamSpec {
    name: "encounter",
    kind: ParamKind::Reference,
    summary: "the encounter occurrence to signal, as `encounter:<id>`",
};

const KEY: ParamSpec = ParamSpec {
    name: "key",
    kind: ParamKind::Name,
    summary: "the stable signal key its objective consumes",
};

/// Publish this domain's authored verbs. Called by
/// [`EncounterRegistryPlugin`](crate::EncounterRegistryPlugin).
pub(crate) fn publish_authored_commands(app: &mut App) {
    app.publish_command(
        CommandDescriptor {
            id: CommandId::new("encounter", "signal"),
            summary: "record a stable signal key against a live encounter's objective",
            params: &[TARGET, KEY],
        },
        signal,
    );
}

/// **Tell one live encounter that a fact it is waiting for has happened.**
///
/// ⚠ **it writes [`EncounterCommand`] rather than touching a lifecycle**, which
/// is the shape the command contract asks a runner for: ask the domain through
/// the bus the domain already owns, so the request is consumed on the same tick
/// by the same reducer that has always consumed it, and the rollback question is
/// answered by construction. Nothing new joins the wire.
///
/// ⛔ **it does not depend on query iteration order.** A [`SimId`] names at most
/// one occurrence, so the search below has one answer or none regardless of the
/// order the archetypes happen to be walked.
fn signal(world: &mut World, args: &[AuthoredArg]) -> CommandOutcome {
    let (Some(target), Some(key)) = (args[0].as_reference(), args[1].as_name()) else {
        return CommandOutcome::refused(
            "`encounter.signal` takes an occurrence reference and a signal key",
        );
    };
    let Some(encounter_id) = resolve_encounter(world, target) else {
        return CommandOutcome::refused(format!(
            "no live encounter occurrence `{target}` — either the room that spawns it \
             is not active or the authored reference names something else"
        ));
    };
    world.write_message(EncounterCommand::signal(encounter_id, key));
    CommandOutcome::Done
}

/// The encounter id belonging to the occurrence a reference names.
fn resolve_encounter(world: &mut World, target: &SimId) -> Option<String> {
    let mut occurrences = world.query::<(&SimId, &Encounter)>();
    occurrences
        .iter(world)
        .find(|(sim_id, _)| *sim_id == target)
        .map(|(_, encounter)| encounter.id.clone())
}

#[cfg(test)]
mod tests;
