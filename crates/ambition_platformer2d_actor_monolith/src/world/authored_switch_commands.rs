//! Runs commands authored directly on `Switch` entities.
//!
//! `on_activate` text is prepared once against the published `CommandCatalog`; the simulation
//! stores only the validated command id and prepared arguments. The cache is derived from the
//! active room and LDtk project and therefore contains no rollback cursor or program counter.
//! When a switch activates, the system requests its prepared command through the shared authored
//! command runner.

use std::collections::BTreeMap;

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::authored_logic::{
    AuthoredCommandSet, CommandCatalog, PreparedCommand, RunAuthoredCommand,
};

/// The authored field a `Switch` spells its verb in.
///
/// optional by design: the switches that arm an encounter, reset a fight or
/// drive a sand sim carry none, and must keep working.
pub const ON_ACTIVATE_FIELD: &str = "on_activate";

/// One authored switch that names a verb: its activation id and the line it
/// authored.
///
/// pure text at this stage on purpose. Reading the level and validating
/// against the catalog are two different failures with two different fixes, and
/// keeping them apart is what lets the LDtk walk stay a pure function testable
/// without an ECS — the good part inherited from the sibling system.
#[derive(Clone, Debug, PartialEq)]
pub struct AuthoredSwitchCommand {
    pub switch_id: String,
    pub line: String,
}

/// Every `Switch` in `room` that authors an `on_activate`.
pub fn authored_switch_commands(
    room: &ambition_platformer2d_world::rooms::RoomSpec,
) -> Vec<AuthoredSwitchCommand> {
    room.switch_commands
        .iter()
        .map(|command| AuthoredSwitchCommand {
            switch_id: command.switch_id.trim().to_string(),
            line: command.line.clone(),
        })
        .collect()
}

/// The active room's prepared switch verbs.
///
/// derived, not rollback state — see the module header. and the values
/// inside it cannot be edited by anything that holds it: a
/// [`PreparedCommand`] has no mutator at all, so the only thing a `ResMut` can
/// do is replace the whole room's set with another validated one.
#[derive(Resource, Default)]
pub struct AuthoredSwitchCommands {
    room: Option<String>,
    calls: BTreeMap<String, PreparedCommand>,
}

impl AuthoredSwitchCommands {
    /// The verb a switch id asks for, if it authored one that prepared.
    pub fn get(&self, switch_id: &str) -> Option<&PreparedCommand> {
        self.calls.get(switch_id)
    }

    pub fn len(&self) -> usize {
        self.calls.len()
    }

    pub fn is_empty(&self) -> bool {
        self.calls.is_empty()
    }
}

/// Prepare the active room's authored switch verbs. (sim)
///
/// Refreshes when either input moves. a line that does not prepare is
/// dropped with a warning naming the switch — the alternative is a switch that
/// silently does nothing, which is how an author spends an afternoon on a typo.
pub fn prepare_authored_switch_commands(
    rooms: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_world::rooms::RoomSet,
    >,
    catalog: Option<Res<CommandCatalog>>,
    mut prepared: ResMut<AuthoredSwitchCommands>,
) {
    let Some(catalog) = catalog else {
        return;
    };
    let active_room_id = rooms.active_spec().id.clone();
    // It watched `ActiveLdtkProject:is_changed`; the command lines come off the room set now,
    // so that is what has to be watched — a hot reload that rebuilds rooms under an UNCHANGED
    // room id would otherwise keep serving prepared calls from content that is no longer
    // loaded, which is the case the old signal existed for.
    let rooms_changed = rooms.is_changed();
    let stale = prepared.room.as_deref() != Some(active_room_id.as_str());
    if !rooms_changed && !stale {
        return;
    }

    let authored = authored_switch_commands(rooms.active_spec());
    prepared.calls.clear();
    prepared.room = Some(active_room_id.clone());
    for AuthoredSwitchCommand { switch_id, line } in authored {
        match catalog.prepare_line(&line) {
            Ok(call) => {
                prepared.calls.insert(switch_id, call);
            }
            Err(error) => warn!(
                target: "ambition_platformer2d_actor_monolith::world::authored_switch_commands",
                "switch `{switch_id}` in room `{active_room_id}` authors an \
                 `{ON_ACTIVATE_FIELD}` this composition cannot perform: {error}",
            ),
        }
    }
}

/// Ask for the verb an activated switch authored. (sim)
///
/// it reads rather than drains, unlike the authored-command dispatcher:
/// `SwitchActivated` has other consumers (the encounter reset path, the sand
/// sim) and a drain here would eat their facts. The cursor is `Local` state a
/// rewind does not restore, which is the shape the content system this replaces
/// already had — and the channel is cleared on rollback, which bounds it.
pub fn request_authored_switch_commands(
    prepared: Res<AuthoredSwitchCommands>,
    mut activations: MessageReader<ambition_encounter::switches::SwitchActivated>,
    mut requests: MessageWriter<RunAuthoredCommand>,
) {
    for activation in activations.read() {
        if let Some(call) = prepared.get(activation.activation.id.as_str()) {
            requests.write(RunAuthoredCommand::prepared(call));
        }
    }
}

/// Installs the store and its two systems.
///
/// it publishes no command, the same separation every other participant in
/// this contract keeps: this is a consumer of whatever the installed domains
/// published, and it names none of them.
pub struct AuthoredSwitchCommandPlugin;

impl Plugin for AuthoredSwitchCommandPlugin {
    fn build(&self, app: &mut App) {
        use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt as _;

        let sim = app.sim_schedule();
        app.init_resource::<AuthoredSwitchCommands>().add_systems(
            sim,
            (
                prepare_authored_switch_commands,
                request_authored_switch_commands,
            )
                .chain()
                // both pins are real and both are needed. `SwitchActivated`
                // is written in `FeatureInteraction`; the dispatcher that
                // performs the request runs in `AuthoredCommandSet`, which is
                // pinned only after `CoreSimulation` and before
                // `GameplayEffects` — so without the first pin an activation
                // could miss its own frame, and without the second the request
                // would wait for the next one. Both sets live in this schedule,
                // so neither is the silently-vacuous cross-schedule kind.
                .after(crate::schedule::Platformer2dSimulationPhaseMonolith::FeatureInteraction)
                .before(AuthoredCommandSet),
        );
    }
}

#[cfg(test)]
mod tests;
