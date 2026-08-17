//! **An authored `Switch` decides for itself what it does.**
//!
//! # What this replaces, and why the replacement is the point
//!
//! This was the switch half of `drive_symmetry_attunement` in
//! `ambition_content`, and the pairing it walked:
//!
//! ```ignore
//! const KERNEL_FACES: [(&str, &str); 4] = [
//!     ("kernel_switch_down", "gravity_down"),
//!     ("kernel_switch_left", "gravity_left"),
//!     // …
//! ];
//! ```
//!
//! ⭐⭐ **the switch was in the level and what it did was in the compiler** —
//! the same sentence the [`gated_lock_walls`](super::gated_lock_walls) sibling
//! was written for, one contract half later. An author adding a fifth kernel
//! face had to edit Rust in another crate, in a table matched by string, and an
//! agent reading the room saw four switches whose only distinguishing fact was
//! an `action` field the puzzle deliberately ignores.
//!
//! Now a `Switch` carries an `on_activate` line naming a published command and
//! its arguments, and pressing it asks for exactly that.
//!
//! ⭐ **and the capability generalised on the way out**, again: this is an
//! engine system, so any game on this engine wires a switch to any domain's verb
//! with no Rust at all.
//!
//! # ⭐⭐ The prepared half, which is M2's whole subject
//!
//! The authored text is turned into a
//! [`PreparedCommand`](ambition_platformer2d_shared_tangle::authored_logic::PreparedCommand)
//! **once**, when the room's rules are first read — the id checked against the
//! published catalog, the arity against the descriptor, every argument against
//! its declared kind, and a reference minted through `SimId`'s own constructor.
//! A line that is wrong is refused *there*, with a warning naming the switch and
//! the reason.
//!
//! ⇒ **what the tick holds is a command id and a list of prepared values.** The
//! authored string is not stored, so there is nothing on the simulation path to
//! parse even in principle. See the preparation module's header for why each of
//! those properties is structural rather than a promise.
//!
//! # ⚠ The store is DERIVED, like its sibling's
//!
//! [`AuthoredSwitchCommands`] is a pure function of (LDtk project, active room),
//! which is the same argument [`GatedLockWallCache`](super::gated_lock_walls)
//! makes and is what lets it be declared derived to rollback rather than
//! registered: neither input can move inside a rollback window — a room
//! transition commits only on a confirmed frame and a project swap is a hot
//! reload.
//!
//! ⛔ **there is no cursor, no step index and no program counter anywhere in
//! here**, which is the cheapest possible answer to the census finding that the
//! tree ships three different opinions about whether a program counter is
//! rollback state. A switch is pressed; a prepared call is requested; nothing
//! remembers where it was.
//!
//! # ⚠ Two systems, not one, and the reason is the borrow
//!
//! Preparing needs the catalog and the LDtk project; requesting needs the
//! activation channel. Both are ordinary systems — ⭐ notably *not* the exclusive
//! shape [`gated_lock_walls`](super::gated_lock_walls) needs, because asking a
//! condition takes `&World` and asking for a command does not.

use std::collections::BTreeMap;

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::authored_logic::{
    AuthoredCommandSet, CommandCatalog, PreparedCommand, RunAuthoredCommand,
};

/// **The authored field a `Switch` spells its verb in.**
///
/// ⚠ optional by design: the switches that arm an encounter, reset a fight or
/// drive a sand sim carry none, and must keep working.
pub const ON_ACTIVATE_FIELD: &str = "on_activate";

/// One authored switch that names a verb: its activation id and the line it
/// authored.
///
/// ⭐ **pure text at this stage on purpose.** Reading the level and validating
/// against the catalog are two different failures with two different fixes, and
/// keeping them apart is what lets the LDtk walk stay a pure function testable
/// without an ECS — the good part inherited from the sibling system.
#[derive(Clone, Debug, PartialEq)]
pub struct AuthoredSwitchCommand {
    pub switch_id: String,
    pub line: String,
}

/// **Every `Switch` in `active_room_id` that authors an `on_activate`.**
pub fn authored_switch_commands(
    project: &ambition_platformer2d_ldtk::LdtkProject,
    active_room_id: &str,
) -> Vec<AuthoredSwitchCommand> {
    let mut out = Vec::new();
    for level in &project.levels {
        if level.active_area() != active_room_id {
            continue;
        }
        for entity in level.all_entity_instances() {
            if entity.identifier != "Switch" {
                continue;
            }
            let Some(switch_id) = ambition_platformer2d_ldtk::field_string(entity, "id") else {
                continue;
            };
            let Some(line) = ambition_platformer2d_ldtk::field_string(entity, ON_ACTIVATE_FIELD)
            else {
                continue;
            };
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            out.push(AuthoredSwitchCommand {
                switch_id: switch_id.trim().to_string(),
                line: line.to_string(),
            });
        }
    }
    out
}

/// **The active room's prepared switch verbs.**
///
/// ⚠ **derived, not rollback state** — see the module header. ⛔ and the values
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

/// **Prepare the active room's authored switch verbs.** (sim)
///
/// Refreshes when either input moves. ⚠ **a line that does not prepare is
/// dropped with a warning naming the switch** — the alternative is a switch that
/// silently does nothing, which is how an author spends an afternoon on a typo.
pub fn prepare_authored_switch_commands(
    rooms: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<crate::rooms::RoomSet>,
    project: Option<Res<ambition_platformer2d_ldtk::ActiveLdtkProject>>,
    catalog: Option<Res<CommandCatalog>>,
    mut prepared: ResMut<AuthoredSwitchCommands>,
) {
    let Some(catalog) = catalog else {
        return;
    };
    let active_room_id = rooms.active_spec().id.clone();
    let project_changed = project.as_ref().is_some_and(|project| project.is_changed());
    let stale = prepared.room.as_deref() != Some(active_room_id.as_str());
    if !project_changed && !stale {
        return;
    }

    let authored = project
        .map(|project| authored_switch_commands(&project.0, &active_room_id))
        .unwrap_or_default();
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

/// **Ask for the verb an activated switch authored.** (sim)
///
/// ⚠ **it reads rather than drains**, unlike the authored-command dispatcher:
/// `SwitchActivated` has other consumers (the encounter reset path, the sand
/// sim) and a drain here would eat their facts. The cursor is `Local` state a
/// rewind does not restore, which is the shape the content system this replaces
/// already had — and the channel is cleared on rollback, which bounds it.
pub fn request_authored_switch_commands(
    prepared: Res<AuthoredSwitchCommands>,
    mut activations: MessageReader<crate::encounter::SwitchActivated>,
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
/// ⭐ **it publishes no command**, the same separation every other participant in
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
                // ⚠ **both pins are real and both are needed.** `SwitchActivated`
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
