//! Authored `encounter.signal` command.
//!
//! The command publishes the encounter domain's existing [`EncounterCommand`]
//! through the shared authored-command catalog. Its target uses a prepared
//! [`SimId`] reference rather than a free-form name: unresolved targets are
//! refused during command preparation, and execution reads the encounter id from
//! the resolved occurrence rather than parsing identity back out of the string.

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

/// Tell one live encounter that a fact it is waiting for has happened.
///
///  it writes [`EncounterCommand`] rather than touching a lifecycle, which
/// is the shape the command contract asks a runner for: ask the domain through
/// the bus the domain already owns, so the request is consumed on the same tick
/// by the same reducer that has always consumed it, and the rollback question is
/// answered by construction. Nothing new joins the wire.
///
///  it does not depend on query iteration order. A [`SimId`] names at most
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
