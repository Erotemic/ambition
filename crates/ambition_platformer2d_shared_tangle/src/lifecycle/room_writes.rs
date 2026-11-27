//! World records written from a room, and their retraction when the room
//! rewinds (SIMULATION-ISLANDS ISL-2d,
//! `docs/planning/engine/island-rollback-and-compact-peer-input.md`).
//!
//! What lives on a room's entities rewinds with the room. A world store a
//! room writes to (the record of grants since the checkpoint, a save flag)
//! does not: replayed, the room writes it again. So such a store stamps each
//! record with the room and the tick it was written on ([`RoomWrite`]), and
//! registers a retractor. Whatever rewinds a room to a tick calls
//! [`retract_room_writes`] with the tick it saved at; each store drops that
//! room's records written after it, and the replay writes them again.
//!
//! The tick is `SimTick`, which advances at the head of each step: a room
//! saved at tick `t` writes its next frame at `t + 1`.

use std::ops::DerefMut;

use bevy::prelude::*;

use super::LiveRoomInstance;

/// The room a world record was written from, and the tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RoomWrite {
    pub room: LiveRoomInstance,
    pub tick: u64,
}

/// How far back, in ticks, a room may rewind: a store that keeps its room
/// writes for no other reason (a checkpoint does not clear it) drops the ones
/// older than this, so the log stays bounded. The room rollback host must
/// not rewind a room further (ten seconds at 60 Hz; a late input is a few
/// frames).
pub const ROOM_REWIND_HORIZON_TICKS: u64 = 600;

impl RoomWrite {
    /// Whether a rewind of `room` to `saved_at` takes this record back.
    pub fn retracted_by(&self, room: LiveRoomInstance, saved_at: u64) -> bool {
        self.room == room && self.tick > saved_at
    }

    /// Whether no rewind can take this record back any longer, `now` being
    /// the current tick ([`ROOM_REWIND_HORIZON_TICKS`]).
    pub fn beyond_the_horizon(&self, now: u64) -> bool {
        tick_beyond_the_horizon(self.tick, now)
    }
}

fn tick_beyond_the_horizon(tick: u64, now: u64) -> bool {
    tick.saturating_add(ROOM_REWIND_HORIZON_TICKS) < now
}

// The log helpers below take the log as `impl DerefMut<Target = Vec<T>>`, so
// a caller can pass a `Mut` projection of a resource. They write through it
// only when they take a record: a rewind that takes nothing does not mark the
// store changed.

/// Take out of `log` the records that a rewind of `room` to `saved_at` takes
/// back, newest first, which is the order to undo them in. The records that
/// stay keep their order. `stamp` gives the room write of a record; a record
/// with no room write (`None`) belongs to no room and stays.
pub fn take_rewound<T, S: Into<Option<RoomWrite>>>(
    mut log: impl DerefMut<Target = Vec<T>>,
    room: LiveRoomInstance,
    saved_at: u64,
    stamp: impl Fn(&T) -> S,
) -> Vec<T> {
    let rewound = |entry: &T| stamp(entry).into().is_some_and(|written| written.retracted_by(room, saved_at));
    if !log.iter().any(&rewound) {
        return Vec::new();
    }
    let mut taken: Vec<T> = log.extract_if(.., |entry| rewound(entry)).collect();
    taken.reverse();
    taken
}

/// As [`take_rewound`], for a log whose records overwrite a shared value:
/// each taken record comes with whether a later record of the same key
/// (`same_key(later, taken)`) stays in the log. Then the value keeps that
/// later write, and the owner must not put back the value the taken record
/// replaced.
pub fn take_rewound_superseded<T, S: Into<Option<RoomWrite>>>(
    mut log: impl DerefMut<Target = Vec<T>>,
    room: LiveRoomInstance,
    saved_at: u64,
    stamp: impl Fn(&T) -> S,
    same_key: impl Fn(&T, &T) -> bool,
) -> Vec<(T, bool)> {
    let mut taken = Vec::new();
    for at in (0..log.len()).rev() {
        if !stamp(&log[at]).into().is_some_and(|written| written.retracted_by(room, saved_at)) {
            continue;
        }
        let entry = log.remove(at);
        let superseded = log[at..].iter().any(|later| same_key(later, &entry));
        taken.push((entry, superseded));
    }
    taken
}

/// Take out of `log` the records that no rewind can reach at tick `now`
/// ([`ROOM_REWIND_HORIZON_TICKS`]), in write order. `written_at` gives the
/// tick a record was written on.
pub fn take_beyond_the_horizon<T>(
    mut log: impl DerefMut<Target = Vec<T>>,
    now: u64,
    written_at: impl Fn(&T) -> u64,
) -> Vec<T> {
    let expired = |entry: &T| tick_beyond_the_horizon(written_at(entry), now);
    if !log.iter().any(&expired) {
        return Vec::new();
    }
    log.extract_if(.., |entry| expired(entry)).collect()
}

/// The host declares whether its live rooms can rewind independently. The
/// predicate reads host mode, not the confirmation state of one current frame.
#[derive(Resource, Clone, Copy)]
pub struct SimulationIslands(pub fn(&World) -> bool);

/// One native rollback authority. Allocation rooms do not define membership.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimulationIsland {
    Session(super::SessionScopeId),
    Room(super::SessionScopeId, LiveRoomInstance),
}

pub fn simulation_island(world: &World, entity: Entity) -> Option<SimulationIsland> {
    let session = world.get::<super::SessionScopedEntity>(entity)?.0;
    if world.get_resource::<SimulationIslands>().is_some_and(|policy| (policy.0)(world)) {
        if let Some(room) = super::object_scope_room(world, entity) {
            return Some(SimulationIsland::Room(session, room));
        }
    }
    Some(SimulationIsland::Session(session))
}

/// Direct delivery requires one rollback authority and one session.
pub fn shares_simulation_island(world: &World, source: Entity, receiver: Entity) -> bool {
    simulation_island(world, source).zip(simulation_island(world, receiver))
        .is_some_and(|(source, receiver)| source == receiver)
}

type Retractor = fn(&mut World, LiveRoomInstance, u64);

/// Every store that takes room writes, with its retractor.
#[derive(Resource, Default)]
pub struct RoomWriteRetractors {
    retractors: Vec<(&'static str, Retractor)>,
}

impl RoomWriteRetractors {
    /// The stores, by name, in registration order.
    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.retractors.iter().map(|(name, _)| *name)
    }
}

/// Register `retract` as the retractor of the store `name`. A second
/// registration under one name replaces the first.
pub fn register_room_write_retractor(app: &mut App, name: &'static str, retract: Retractor) {
    let mut retractors = app.world_mut().get_resource_or_insert_with(RoomWriteRetractors::default);
    if let Some(slot) = retractors.retractors.iter_mut().find(|(known, _)| *known == name) {
        slot.1 = retract;
    } else {
        retractors.retractors.push((name, retract));
    }
}

/// Install `L`, a log that holds a store's room writes and nothing else, empty
/// at each session's activation.
///
/// A new session counts its live rooms and its ticks from the start again, so
/// a record the last session kept names a room and a tick of this one: a
/// rewind of this session's first room would take back the last session's
/// writes onto it, and the horizon would never drop them.
pub fn register_room_write_log<L: Resource + Component<Mutability = bevy::ecs::component::Mutable> + Default>(app: &mut App) {
    app.init_resource::<L>();
    app.add_message::<super::SessionScopeActivated>();
    app.add_systems(
        Update,
        forget_room_write_log::<L>
            .in_set(super::SessionScopeSet::Activate)
            .run_if(on_message::<super::SessionScopeActivated>),
    );
}

fn forget_room_write_log<L: Resource + Component<Mutability = bevy::ecs::component::Mutable> + Default>(mut log: ResMut<L>) {
    *log = L::default();
}

/// `room` rewinds to the tick it was saved at: every store drops the records
/// that room wrote after it.
pub fn retract_room_writes(world: &mut World, room: LiveRoomInstance, saved_at: u64) {
    let retractors: Vec<Retractor> = world
        .get_resource::<RoomWriteRetractors>()
        .map(|known| known.retractors.iter().map(|(_, retract)| *retract).collect())
        .unwrap_or_default();
    for retract in retractors {
        retract(world, room, saved_at);
    }
}

/// The only room whose simulation can write during a replay. A resource
/// writer uses its owner's room to take part in this scope. World-owned
/// operations wait for the host's next normal tick.
#[derive(Resource, Clone, Copy)]
pub struct RoomReplay {
    pub room: LiveRoomInstance,
}

impl RoomReplay {
    pub fn allows(&self, owner: Option<LiveRoomInstance>) -> bool {
        owner == Some(self.room)
    }
}

type Refusal = fn(&World) -> bool;

/// The states in which no room may replay alone: each is one contest or one
/// operation across rooms, whose state a single room's replay would run a
/// second time without taking the first back. The host drives a late input
/// from its next tick instead.
#[derive(Resource, Default)]
pub struct RoomReplayRefusals {
    refusals: Vec<(&'static str, Refusal)>,
}

/// Register `refuses` under `name`. A second registration under one name
/// replaces the first.
pub fn register_room_replay_refusal(app: &mut App, name: &'static str, refuses: Refusal) {
    let mut known = app.world_mut().get_resource_or_insert_with(RoomReplayRefusals::default);
    if let Some(slot) = known.refusals.iter_mut().find(|(known, _)| *known == name) {
        slot.1 = refuses;
    } else {
        known.refusals.push((name, refuses));
    }
}

/// The first registered refusal that holds now, by name.
pub fn room_replay_refusal(world: &World) -> Option<&'static str> {
    let known = world.get_resource::<RoomReplayRefusals>()?;
    known.refusals.iter().find(|(_, refuses)| refuses(world)).map(|(name, _)| *name)
}

/// Start replay when the caller drives a full App update. Pair with
/// end_room_replay. Use with_room_replay for a World-only replay.
pub fn begin_room_replay(world: &mut World, room: LiveRoomInstance) {
    assert!(!world.contains_resource::<RoomReplay>(), "room replays cannot nest");
    super::session_messages::isolate_channels_for_room_replay(world);
    world.insert_resource(RoomReplay { room });
}

/// Restore the live streams and remove the room replay scope.
pub fn end_room_replay(world: &mut World) {
    super::session_messages::restore_channels_after_room_replay(world);
    world.remove_resource::<RoomReplay>();
}

/// Run with private message streams for a room replay. Restore live streams
/// before returning or propagating a panic. The caller owns simulation state.
pub fn with_room_replay<R>(
    world: &mut World,
    room: LiveRoomInstance,
    run: impl FnOnce(&mut World) -> R,
) -> R {
    use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

    begin_room_replay(world, room);
    let result = catch_unwind(AssertUnwindSafe(|| run(world)));
    end_room_replay(world);
    result.unwrap_or_else(|panic| resume_unwind(panic))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a() -> LiveRoomInstance {
        LiveRoomInstance::from_ordinal(1)
    }

    fn b() -> LiveRoomInstance {
        LiveRoomInstance::from_ordinal(2)
    }

    /// One write to `key` from `room` on `tick`.
    fn write(room: LiveRoomInstance, tick: u64, key: &'static str) -> (RoomWrite, &'static str) {
        (RoomWrite { room, tick }, key)
    }

    fn stamp(entry: &(RoomWrite, &'static str)) -> RoomWrite {
        entry.0
    }

    fn same_key(later: &(RoomWrite, &'static str), taken: &(RoomWrite, &'static str)) -> bool {
        later.1 == taken.1
    }

    /// Rooms A and B write one key in turns. The rewind of A takes the later
    /// writes of A newest first, and leaves the writes of B and the earlier
    /// write of A in their order. A replay of A takes nothing twice.
    #[test]
    fn a_rewind_takes_its_rooms_later_writes_newest_first() {
        let (a, b) = (a(), b());
        let mut log = vec![write(a, 3, "k"), write(b, 4, "k"), write(a, 5, "k"), write(b, 6, "k"), write(a, 7, "k")];
        assert_eq!(take_rewound(&mut log, a, 4, stamp), vec![write(a, 7, "k"), write(a, 5, "k")]);
        assert_eq!(log, vec![write(a, 3, "k"), write(b, 4, "k"), write(b, 6, "k")]);
        assert!(take_rewound(&mut log, a, 4, stamp).is_empty());
    }

    /// A record with no room write is never taken.
    #[test]
    fn a_record_of_no_room_stays() {
        let written = Some(RoomWrite { room: a(), tick: 9 });
        let mut log = vec![(None, "k"), (written, "k")];
        assert_eq!(take_rewound(&mut log, a(), 0, |entry| entry.0), vec![(written, "k")]);
        assert_eq!(log, vec![(None, "k")]);
    }

    /// A later write of B to the same key supersedes the taken write of A. A
    /// write to another key does not, and a later write that the same rewind
    /// takes does not.
    #[test]
    fn a_later_write_of_another_room_supersedes() {
        let (a, b) = (a(), b());
        let mut log = vec![write(a, 5, "k"), write(a, 6, "j"), write(b, 7, "k"), write(a, 8, "j")];
        assert_eq!(
            take_rewound_superseded(&mut log, a, 4, stamp, same_key),
            vec![(write(a, 8, "j"), false), (write(a, 6, "j"), false), (write(a, 5, "k"), true)],
        );
        assert_eq!(log, vec![write(b, 7, "k")]);
    }

    /// The replay of A takes only the writes of A. A rewind of B after it
    /// finds its own writes as they were.
    #[test]
    fn a_replay_of_one_room_leaves_the_other() {
        let (a, b) = (a(), b());
        let mut log = vec![write(b, 5, "k"), write(a, 6, "k"), write(b, 7, "k")];
        assert_eq!(take_rewound_superseded(&mut log, a, 0, stamp, same_key), vec![(write(a, 6, "k"), true)]);
        assert_eq!(take_rewound_superseded(&mut log, b, 5, stamp, same_key), vec![(write(b, 7, "k"), false)]);
        assert_eq!(log, vec![write(b, 5, "k")]);
    }

    /// The horizon takes the writes older than it, in write order, and keeps
    /// the write exactly at it.
    #[test]
    fn the_horizon_takes_the_old_writes() {
        let (a, b) = (a(), b());
        let now = 1_000;
        let edge = now - ROOM_REWIND_HORIZON_TICKS;
        let mut log = vec![write(a, edge - 2, "k"), write(b, edge - 1, "k"), write(a, edge, "k"), write(b, now, "k")];
        assert_eq!(
            take_beyond_the_horizon(&mut log, now, |entry| entry.0.tick),
            vec![write(a, edge - 2, "k"), write(b, edge - 1, "k")],
        );
        assert_eq!(log, vec![write(a, edge, "k"), write(b, now, "k")]);
    }
}
