//! Runs the command line a room authors in its `while_live` level field.
//!
//! The line is prepared once per live room against the published
//! `CommandCatalog`, and asked for on each tick while that room is live. It is
//! a condition on the live rooms, not an edge, so it keeps no memory of which
//! rooms were live before, and a rewind has nothing of it to restore. The verb
//! must do nothing when its work is already done; `encounter.start` ignores an
//! encounter that is not inactive.

use std::collections::BTreeMap;

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::authored_logic::{
    AuthoredAsk, AuthoredCommandSet, CommandCatalog, PreparedCommand, RunAuthoredCommand,
};

/// The level field a room spells its line in.
pub const WHILE_LIVE_FIELD: &str = "while_live";

/// Each live room's prepared `while_live` line, by room id. Derived from the
/// room set and the catalog, so it is not rollback state.
#[derive(Resource, Default)]
pub struct AuthoredRoomCommands {
    /// Every live room has an entry, also one with no line, so that a room
    /// that becomes live is seen as a change.
    rooms: BTreeMap<String, Option<PreparedCommand>>,
}

impl AuthoredRoomCommands {
    /// The prepared line of room `room_id`, if it authored one that prepared.
    pub fn get(&self, room_id: &str) -> Option<&PreparedCommand> {
        self.rooms.get(room_id)?.as_ref()
    }
}

/// Prepare each live room's `while_live` line. (sim)
///
/// A line that does not prepare is dropped with a warning that names the
/// room, because the other result is a room that does nothing and says
/// nothing.
pub fn prepare_authored_room_commands(
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    catalog: Option<Res<CommandCatalog>>,
    mut prepared: ResMut<AuthoredRoomCommands>,
) {
    let Some(catalog) = catalog else {
        return;
    };
    let live: BTreeMap<String, Option<String>> = rooms
        .live_definitions()
        .map(|definition| {
            let spec = rooms.rooms().spec(definition);
            (spec.id.clone(), spec.metadata.while_live.clone())
        })
        .collect();
    let stale = !prepared.rooms.keys().eq(live.keys());
    if !rooms.is_changed() && !stale {
        return;
    }
    prepared.rooms = live
        .into_iter()
        .map(|(room_id, line)| {
            let call = line.and_then(|line| match catalog.prepare_line(&line) {
                Ok(call) => Some(call),
                Err(error) => {
                    warn!(
                        target: "crate::world::authored_room_commands",
                        "room `{room_id}` authors a `{WHILE_LIVE_FIELD}` this composition \
                         cannot perform: {error}",
                    );
                    None
                }
            });
            (room_id, call)
        })
        .collect();
}

/// Ask for each live room's line. (sim)
///
/// In room id order, so a resimulation asks in the same order. Two live
/// rooms of one id ask once: the line is the room's, not the instance's.
pub fn request_authored_room_commands(
    prepared: Res<AuthoredRoomCommands>,
    mut requests: MessageWriter<RunAuthoredCommand>,
) {
    for (room_id, call) in &prepared.rooms {
        if let Some(call) = call {
            requests.write(RunAuthoredCommand::prepared(
                call,
                AuthoredAsk::new("room", room_id.as_str()),
            ));
        }
    }
}

/// Installs the store and its two systems. It publishes no command.
pub struct AuthoredRoomCommandPlugin;

impl Plugin for AuthoredRoomCommandPlugin {
    fn build(&self, app: &mut App) {
        use ambition_platformer2d_shared_tangle::schedule::{
            GameplaySimulationRoot, Platformer2dSimulationPhaseMonolith, SimScheduleExt as _,
        };

        let sim = app.sim_schedule();
        app.init_resource::<AuthoredRoomCommands>().add_systems(
            sim,
            (prepare_authored_room_commands, request_authored_room_commands)
                .chain()
                // In the root set, so a session frozen at a title or loading
                // route asks for nothing, and before the runner so a request
                // is performed on the tick it is made.
                .in_set(GameplaySimulationRoot)
                .after(Platformer2dSimulationPhaseMonolith::CoreSimulation)
                .before(AuthoredCommandSet),
        );
    }
}

#[cfg(test)]
mod tests;
