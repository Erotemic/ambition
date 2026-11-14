//! Runs the command line a room authors in its `while_live` level field.
//!
//! The line is prepared once per live room against the published
//! `CommandCatalog`, and asked for on each tick while that room is live. It is
//! a condition on the live rooms, not an edge, so it keeps no memory of which
//! rooms were live before, and a rewind has nothing of it to restore. The verb
//! must do nothing when its work is already done; `encounter.start` ignores an
//! encounter that is not inactive.
//!
//! A room may hold its line back with a condition line in `while_live_when`:
//! the line is asked for only on a tick that question answers yes. The
//! question is asked each tick, so this keeps no memory either.

use std::collections::BTreeMap;

use bevy_app::prelude::*;
use bevy_log::warn;
use bevy_ecs::prelude::*;

use ambition_platformer2d_shared_tangle::authored_logic::{
    AuthoredAsk, AuthoredCommandSet, CommandCatalog, ConditionCatalog, PreparedCommand, PreparedCondition,
    RunAuthoredCommand,
};

/// The level field a room spells its line in.
pub const WHILE_LIVE_FIELD: &str = "while_live";
/// The level field that holds the line back until its question is yes.
pub const WHILE_LIVE_WHEN_FIELD: &str = "while_live_when";

/// A room's prepared line, and the question that holds it back, if any.
#[derive(Clone, Debug)]
pub struct RoomLine {
    pub call: PreparedCommand,
    pub when: Option<PreparedCondition>,
}

/// Each live room's prepared `while_live` line, by room id. Derived from the
/// room set and the catalog, so it is not rollback state.
#[derive(Resource, Default)]
pub struct AuthoredRoomCommands {
    /// Every live room has an entry, also one with no line, so that a room
    /// that becomes live is seen as a change.
    rooms: BTreeMap<String, Option<RoomLine>>,
}

impl AuthoredRoomCommands {
    /// The prepared line of room `room_id`, if it authored one that prepared.
    pub fn get(&self, room_id: &str) -> Option<&RoomLine> {
        self.rooms.get(room_id)?.as_ref()
    }
}

/// Prepare each live room's `while_live` line. (sim)
///
/// A line that does not prepare is dropped with a warning that names the
/// room, because the other result is a room that does nothing and says
/// nothing.
pub fn prepare_authored_room_commands(
    rooms: crate::rooms::LiveRoomSpecs,
    catalog: Option<Res<CommandCatalog>>,
    conditions: Option<Res<ConditionCatalog>>,
    mut prepared: ResMut<AuthoredRoomCommands>,
) {
    let Some(catalog) = catalog else {
        return;
    };
    let live: BTreeMap<String, (Option<String>, Option<String>)> = rooms
        .live_definitions()
        .map(|definition| {
            let spec = rooms.rooms().spec(definition);
            (spec.id.clone(), (spec.metadata.while_live.clone(), spec.metadata.while_live_when.clone()))
        })
        .collect();
    let stale = !prepared.rooms.keys().eq(live.keys());
    if !rooms.is_changed() && !stale {
        return;
    }
    let refuse = |room_id: &str, field: &str, error: &dyn std::fmt::Display| {
        warn!(
            target: "crate::world::authored_room_commands",
            "room `{room_id}` authors a `{field}` this composition cannot perform: {error}",
        );
    };
    prepared.rooms = live
        .into_iter()
        .map(|(room_id, (line, when))| {
            let call = line.and_then(|line| match catalog.prepare_line(&line) {
                Ok(call) => Some(call),
                Err(error) => {
                    refuse(&room_id, WHILE_LIVE_FIELD, &error);
                    None
                }
            });
            // A line held back by a question that cannot be asked is dropped,
            // not run on every tick: the author meant it to wait.
            let line = call.and_then(|call| match when {
                None => Some(RoomLine { call, when: None }),
                Some(when) => match conditions.as_deref().map(|conditions| conditions.prepare_line(&when)) {
                    Some(Ok(when)) => Some(RoomLine { call, when: Some(when) }),
                    Some(Err(error)) => {
                        refuse(&room_id, WHILE_LIVE_WHEN_FIELD, &error);
                        None
                    }
                    None => {
                        refuse(&room_id, WHILE_LIVE_WHEN_FIELD, &"no domain has published any condition");
                        None
                    }
                },
            });
            (room_id, line)
        })
        .collect();
}

/// Ask for each live room's line, in that room. (sim)
///
/// In room id order, then live room order, so a resimulation asks in the same
/// order. Two live rooms of one id each ask, each in its own room: the line is
/// the room's, and a verb that acts on one occurrence (`encounter.start`)
/// acts on that room's (ISL-1b).
pub fn request_authored_room_commands(
    world: &mut World,
    rooms: &mut bevy_ecs::system::SystemState<(Res<AuthoredRoomCommands>, crate::rooms::LiveRoomSpecs)>,
) {
    let asks: Vec<(String, ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance, RoomLine)> = {
        let Ok((prepared, rooms)) = rooms.get(world) else {
            return;
        };
        let mut live: Vec<_> =
            rooms.live_specs().into_iter().map(|(room, spec)| (spec.id.clone(), room)).collect();
        live.sort();
        live.into_iter()
            .filter_map(|(room_id, room)| prepared.get(&room_id).cloned().map(|line| (room_id, room, line)))
            .collect()
    };
    for (room_id, room, line) in asks {
        let ask = AuthoredAsk::new("room", room_id.as_str());
        let held = line.when.as_ref().is_some_and(|when| {
            !world
                .get_resource::<ConditionCatalog>()
                .is_some_and(|conditions| conditions.ask(world, when, &ask).is_satisfied())
        });
        if !held {
            world.write_message(RunAuthoredCommand::prepared(&line.call, ask).in_room(Some(room)));
        }
    }
}

/// Installs the store and its two systems. It publishes no command.
pub struct AuthoredRoomCommandPlugin;

impl Plugin for AuthoredRoomCommandPlugin {
    fn build(&self, app: &mut App) {
        use ambition_platformer2d_shared_tangle::schedule::{
            GameplaySimulationRoot, Platformer2dSimulationPhase, SimScheduleExt as _,
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
                .after(Platformer2dSimulationPhase::CoreSimulation)
                .before(AuthoredCommandSet),
        );
    }
}

#[cfg(test)]
mod tests;
