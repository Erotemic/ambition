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
        self.tick.saturating_add(ROOM_REWIND_HORIZON_TICKS) < now
    }
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

/// Begin the room's simulation scope and suspend live message streams.
pub fn begin_room_replay(world: &mut World, room: LiveRoomInstance) {
    assert!(!world.contains_resource::<RoomReplay>(), "room replays cannot nest");
    world.insert_resource(RoomReplay { room });
    super::session_messages::isolate_channels_for_room_replay(world);
}

/// Restore live message streams and leave the room's simulation scope.
pub fn end_room_replay(world: &mut World) {
    super::session_messages::restore_channels_after_room_replay(world);
    world.remove_resource::<RoomReplay>();
}
