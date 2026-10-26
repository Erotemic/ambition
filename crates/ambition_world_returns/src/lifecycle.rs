//! Apply admitted replay and restore boundaries to return records.

use super::*;

/// An admitted replay is a fresh attempt at its room: its breakables and
/// pickups are built whole, so the room's records go before the rebuild reads
/// them, and every countdown of the room stops (a breakable's respawn, a
/// pickup's regrowth).
///
/// ⚠ AND THE OLD ATTEMPT'S TIMERS STOP. The old broken breakables and
/// collected pickups live on until the rebuild, one frame or more later. With
/// a running timer and no record, [`mirror_breakable_respawns`] recorded them
/// again, and the replayed room was built with the platform broken. They stay
/// gone until the rebuild retires them. The replay chain is in the `PlayerInput` phase,
/// which runs before the mirror's phase (`FeatureInteraction`) in the same
/// tick, and the phase boundary applies the removal before the mirror runs.
///
/// ⛔ Do not order this system after `FeatureInteractionSet::WorldObjects`.
/// `RoomReplayConsequences` follows this system's set and is in `PlayerInput`,
/// so that edge is a cycle, and the schedule build does not finish (every
/// test of a composed app stops before its first frame).
pub fn forget_scheduled_returns_on_replay(
    mut replays: ambition_combat::events::AdmittedReplays,
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    running: Query<Entity, (With<RespawnTimer>, With<FeatureSimEntity>)>,
    mut schedule: SessionWorldMut<WorldTimeSchedule>,
    mut commands: Commands,
) {
    for replay in replays.read() {
        let replayed = replay.subject.as_ref().and_then(|subject| subject.room);
        let Some(definition) = rooms.definition_named(replayed) else {
            continue;
        };
        for entity in &running {
            if rooms.definition_of(entity) == Some(definition) {
                commands.entity(entity).remove::<RespawnTimer>();
            }
        }
        let room = rooms.rooms().spec(definition).id.clone();
        schedule.forget_room(&room);
    }
}

/// An admitted checkpoint restore keeps only the horizons of the
/// participants it spares (Q151): the dying participant leaves every record's
/// owners, and a record nobody else owns goes back. A New Game spares nobody.
///
/// At the admission, not at the commit, because the admission names who is
/// spared and the commit does not.
pub fn disown_scheduled_returns_on_restore(
    mut replays: ambition_combat::events::AdmittedReplays,
    mut schedule: SessionWorldMut<WorldTimeSchedule>,
) {
    for replay in replays.read() {
        if replay.to_checkpoint {
            schedule.keep_only_owners(&replay.spared_participants);
        }
    }
}

/// The commit of a checkpoint restore: a record of a room that is not live
/// stays while it has an owner (see [`disown_scheduled_returns_on_restore`]),
/// because nothing else remembers it. The records of the live rooms are
/// forgotten: a room the restore rebuilds is built whole, and a room that
/// stays live records its running timers again on the next tick. A New Game
/// forgets every record. (checkpoint reducer, in `CheckpointDomainApply`)
pub fn forget_scheduled_returns_on_restore(
    inputs: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::CheckpointRestoreInputs>>,
    fresh: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::FreshRunRestore>>,
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    mut schedule: SessionWorldMut<WorldTimeSchedule>,
) {
    if inputs.is_none() {
        return;
    }
    if fresh.is_some() {
        schedule.forget_all();
        return;
    }
    let live: Vec<String> = rooms
        .live_definitions()
        .map(|definition| rooms.rooms().spec(definition).id.clone())
        .collect();
    schedule.forget_live_and_unowned(&live);
}
