//! Authored `encounter.signal` and `encounter.start` commands.
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
    app.publish_command(
        CommandDescriptor {
            id: CommandId::new("encounter", "start"),
            summary: "start a live encounter that is inactive; any other phase ignores it",
            params: &[START_TARGET],
        },
        start,
    );
}

const START_TARGET: ParamSpec = ParamSpec {
    name: "encounter",
    kind: ParamKind::Reference,
    summary: "the encounter occurrence to start, as `encounter:<id>`",
};

/// Start one live encounter.
///
/// It writes [`EncounterCommandKind::Start`](crate::lifecycle::EncounterCommandKind::Start),
/// which the reducer performs only from `Inactive`. So a caller that asks on
/// every tick (a room's `while_live` line) starts it once, and a completed
/// encounter is not started again. The occurrence rule is the one
/// [`signal`] uses.
fn start(world: &mut World, args: &[AuthoredArg]) -> CommandOutcome {
    let Some(target) = args[0].as_reference() else {
        return CommandOutcome::refused("`encounter.start` takes an occurrence reference");
    };
    let occurrences = resolve_encounter(world, target);
    let [(encounter_id, room)] = occurrences.as_slice() else {
        return CommandOutcome::refused(if occurrences.is_empty() {
            format!(
                "no live encounter occurrence `{target}` — either the room that spawns it \
                 is not active or the authored reference names something else"
            )
        } else {
            format!(
                "`{target}` has {} live occurrences (rooms {:?}), and the start names no \
                 room to tell them apart",
                occurrences.len(),
                occurrences.iter().map(|(_, room)| *room).collect::<Vec<_>>(),
            )
        });
    };
    world.write_message(EncounterCommand::new(encounter_id.clone(), crate::lifecycle::EncounterCommandKind::Start).in_room(*room));
    CommandOutcome::Done
}

/// Tell one live encounter that a fact it is waiting for has happened.
///
///  it writes [`EncounterCommand`] rather than touching a lifecycle, which
/// is the shape the command contract asks a runner for: ask the domain through
/// the bus the domain already owns, so the request is consumed on the same tick
/// by the same reducer that has always consumed it, and the rollback question is
/// answered by construction. Nothing new joins the wire.
///
///  it does not depend on query iteration order. The reference names an
/// authored encounter, and each live room that holds it holds an occurrence
/// of it (see [`crate::occurrence`]). The command carries no room, so it
/// signals the one occurrence in the room it stands in, and refuses when two
/// live rooms each hold one: which of them was meant is not written anywhere
/// it can read.
fn signal(world: &mut World, args: &[AuthoredArg]) -> CommandOutcome {
    let (Some(target), Some(key)) = (args[0].as_reference(), args[1].as_name()) else {
        return CommandOutcome::refused(
            "`encounter.signal` takes an occurrence reference and a signal key",
        );
    };
    let occurrences = resolve_encounter(world, target);
    let [(encounter_id, room)] = occurrences.as_slice() else {
        return CommandOutcome::refused(if occurrences.is_empty() {
            format!(
                "no live encounter occurrence `{target}` — either the room that spawns it \
                 is not active or the authored reference names something else"
            )
        } else {
            format!(
                "`{target}` has {} live occurrences (rooms {:?}), and the signal names no \
                 room to tell them apart",
                occurrences.len(),
                occurrences.iter().map(|(_, room)| *room).collect::<Vec<_>>(),
            )
        });
    };
    world.write_message(EncounterCommand::signal(encounter_id.clone(), key).in_room(*room));
    CommandOutcome::Done
}

/// The encounter id and live room of every occurrence a reference names.
fn resolve_encounter(
    world: &mut World,
    target: &SimId,
) -> Vec<(String, Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>)> {
    let mut occurrences = world.query::<(
        &SimId,
        &Encounter,
        Option<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
    )>();
    let mut found: Vec<_> = occurrences
        .iter(world)
        .filter(|(sim_id, ..)| *sim_id == target)
        .map(|(_, encounter, stamp)| (encounter.id.clone(), stamp.map(|stamp| stamp.0)))
        .collect();
    found.sort();
    found
}

#[cfg(test)]
mod tests;
