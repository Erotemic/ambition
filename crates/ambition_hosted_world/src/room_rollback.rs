//! A host that keeps a guest's room correct by rollback (SIMULATION-ISLANDS).
//!
//! Each guest's input is stamped with the host tick it was meant for
//! ([`GuestInput::tick`]) and reaches the host after that tick ran. With
//! [`RewindsGuestRooms`], the host drives each guest seat on every tick with
//! the input it holds for that tick, or else the guest's last arrived input
//! (its prediction), and saves each room only guests stand in before the
//! tick. When an input arrives for a tick already simulated and differs from
//! what was used, that room alone goes back to its save, the other rooms are
//! hidden (`Disabled`), and the simulation replays to now with the inputs as
//! known: the world records the room wrote are retracted first, so the replay
//! writes them again (ISL-2d).
//!
//! A host seat in a guest's room is replayed with what its devices handed
//! each tick: the head records the seat's latch (or, on a host without
//! latches, its raw row) for the tick, and a replay drives the seat with that
//! record. The input tables as they stood are put back after the replay, so
//! what the devices gave this frame still reaches the next tick. The impact
//! freeze is saved with the clock: while one room is live it stops the shared
//! clock, and a connect the replay takes back takes its freeze back too.
//! The measurements and witnesses are in
//! `docs/planning/engine/island-rollback-and-compact-peer-input.md`.

use std::collections::BTreeMap;

use bevy::ecs::entity_disabling::Disabled;
use bevy::ecs::message::{Message, Messages};
use bevy::prelude::*;

use ambition_combat::impact_hitstop::ImpactHitstop;
use ambition_characters::control::{DrivingParticipant, PlayerSlot};
use ambition_platformer2d_core::{ConfirmedFrameBoundary, ControlFrame};
use ambition_platformer2d_runtime::rollback::room_snapshot::{RoomSnapshot, RoomSnapshotRows};
use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
use ambition_time::{ClockState, DayClock, GameplayElapsed, SimTick, WorldTime};

use crate::GuestSeat;

/// How many ticks back a late input can still be rewound to. An input older
/// than this is applied as the guest's latest and its tick is not replayed.
pub const REWIND_HORIZON: u64 = 60;

/// On a host: predict each guest's input and rewind the guest's room when its
/// input for a past tick arrives and differs (see the module docs). Without
/// it, a guest's input drives its seat when it arrives.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct RewindsGuestRooms;

/// What the room rollback host did, for a caller to read.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RoomRollbackStats {
    /// Room rewinds for a late input that differed from the one used.
    pub rewinds: u32,
    /// Ticks re-simulated by those rewinds.
    pub replayed: u32,
    /// Late inputs older than the horizon, applied from the next tick.
    pub not_rewound: u32,
}

/// The shared clock at the start of a tick: the timeline, the frame's time,
/// the day, the scale, the session's elapsed seconds, and the impact freeze.
/// The freeze is an expiry against the timeline that stops the shared clock
/// while one room is live: a connect the replay takes back must take its
/// freeze back with it.
#[derive(Clone, Copy, Debug)]
struct Clock(SimTick, WorldTime, DayClock, ClockState, GameplayElapsed, ImpactHitstop);

fn clock(world: &World) -> Option<Clock> {
    Some(Clock(
        *world.get_resource::<SimTick>()?,
        *world.get_resource::<WorldTime>()?,
        *world.get_resource::<DayClock>()?,
        *world.get_resource::<ClockState>()?,
        *ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<GameplayElapsed>(world)?,
        world.get_resource::<ImpactHitstop>().copied().unwrap_or_default(),
    ))
}

fn set_clock(world: &mut World, clock: Clock) {
    world.insert_resource(clock.0);
    world.insert_resource(clock.1);
    world.insert_resource(clock.2);
    world.insert_resource(clock.3);
    world.insert_resource(clock.5);
    if let Some(mut elapsed) =
        ambition_platformer2d_shared_tangle::lifecycle::session_world_component_mut::<GameplayElapsed>(world)
    {
        *elapsed = clock.4;
    }
}

/// The host's record of each guest seat's inputs and each guest room's saves.
#[derive(Resource, Default)]
pub(crate) struct GuestInputs {
    /// Each seat's arrived inputs, by the tick they name.
    arrived: BTreeMap<PlayerSlot, BTreeMap<u64, ControlFrame>>,
    /// The input each guest seat was driven with, by tick.
    used: BTreeMap<PlayerSlot, BTreeMap<u64, ControlFrame>>,
    /// What each host seat in a guest's room was handed, by tick.
    local: BTreeMap<PlayerSlot, BTreeMap<u64, ControlFrame>>,
    /// While a rewind replays: host seats take their records.
    replaying: bool,
    /// Each guest room's state at the start of a tick, by tick.
    saved: BTreeMap<LiveRoomInstance, BTreeMap<u64, (RoomSnapshot, Clock)>>,
}

impl GuestInputs {
    /// The input for `seat` on `tick`: the one it sent for that tick, or else
    /// its latest before it (the prediction).
    fn input_for(&self, seat: PlayerSlot, tick: u64) -> ControlFrame {
        self.arrived
            .get(&seat)
            .and_then(|arrived| arrived.range(..=tick).next_back())
            .map(|(_, frame)| *frame)
            .unwrap_or_default()
    }

    fn forget_before(&mut self, tick: u64) {
        let keep = tick.saturating_sub(REWIND_HORIZON);
        for history in self.local.values_mut() {
            history.retain(|at, _| *at >= keep);
        }
        for history in self.arrived.values_mut().chain(self.used.values_mut()) {
            // The latest arrived input before the horizon stays: it is the
            // prediction for every tick after it.
            let last_before = history.range(..keep).next_back().map(|(tick, _)| *tick);
            history.retain(|at, _| *at >= keep || Some(*at) == last_before);
        }
        for saves in self.saved.values_mut() {
            saves.retain(|at, _| *at >= keep);
        }
    }
}

fn of_room(world: &mut World, room: LiveRoomInstance) -> Vec<Entity> {
    let mut entities: Vec<Entity> = world
        .query::<(Entity, &InRoomInstance)>()
        .iter(world)
        .filter(|(_, at)| at.0 == room)
        .map(|(entity, _)| entity)
        .collect();
    entities.extend(
        world
            .query::<(Entity, &LiveRoomInstance)>()
            .iter(world)
            .filter(|(_, root)| **root == room)
            .map(|(entity, _)| entity),
    );
    entities.sort();
    entities
}

/// Each guest seat's room, and the host seats standing in those rooms.
fn guest_rooms(world: &mut World) -> (BTreeMap<PlayerSlot, LiveRoomInstance>, Vec<PlayerSlot>) {
    let guests: Vec<PlayerSlot> = world.query::<&GuestSeat>().iter(world).map(|seat| seat.0).collect();
    let seated: Vec<(PlayerSlot, LiveRoomInstance)> = world
        .query::<(&DrivingParticipant, &InRoomInstance)>()
        .iter(world)
        .map(|(driver, room)| (driver.0, room.0))
        .collect();
    let rooms: BTreeMap<PlayerSlot, LiveRoomInstance> =
        seated.iter().filter(|(seat, _)| guests.contains(seat)).copied().collect();
    let local = seated
        .iter()
        .filter(|(seat, room)| !guests.contains(seat) && rooms.values().any(|at| at == room))
        .map(|(seat, _)| *seat)
        .collect();
    (rooms, local)
}

/// At the head of each tick (before the clock): drive every guest seat with
/// its input for this tick, and save each room only guests stand in.
pub(crate) fn drive_guests_and_save_rooms(world: &mut World) {
    if !world.contains_resource::<RewindsGuestRooms>() {
        return;
    }
    let tick = world.resource::<SimTick>().0 + 1;
    let (rooms, local) = guest_rooms(world);
    let mut inputs = world.remove_resource::<GuestInputs>().unwrap_or_default();
    if inputs.replaying {
        if let Some(mut boundary) = world.get_resource_mut::<ConfirmedFrameBoundary>() {
            boundary.current = tick as i32;
        }
    }
    for seat in rooms.keys() {
        let frame = inputs.input_for(*seat, tick);
        inputs.used.entry(*seat).or_default().insert(tick, frame);
        ambition_platformer2d_runtime::input_drive::drive_slot_frame(world, *seat, frame);
    }
    // A host seat in a guest's room: record what its devices gave this tick,
    // and give it that again when the tick is replayed. A device host folds
    // presses into latches; a host without them (headless drivers) writes the
    // seat's raw row.
    for seat in &local {
        use ambition_characters::control::{SeatRawFrames, SlotControlLatches};
        if inputs.replaying {
            let recorded = inputs.local.get(seat).and_then(|record| record.get(&tick)).copied().unwrap_or_default();
            if let Some(mut latches) = world.get_resource_mut::<SlotControlLatches>() {
                latches.reset(*seat);
            }
            ambition_platformer2d_runtime::input_drive::drive_slot_frame(world, *seat, recorded);
        } else {
            let given = match world.get_resource::<SlotControlLatches>() {
                Some(latches) => latches.peek(*seat),
                None => world.get_resource::<SeatRawFrames>().map(|raw| raw.get(*seat)).unwrap_or_default(),
            };
            inputs.local.entry(*seat).or_default().insert(tick, given);
        }
    }
    if let (Some(rows), Some(now)) = (world.remove_resource::<RoomSnapshotRows>(), clock(world)) {
        let mut guest_rooms: Vec<LiveRoomInstance> = rooms.values().copied().collect();
        guest_rooms.sort();
        guest_rooms.dedup();
        for room in guest_rooms {
            let entities = of_room(world, room);
            let snapshot = rows.save(world, &entities);
            inputs.saved.entry(room).or_default().insert(tick, (snapshot, now));
        }
        inputs.saved.retain(|room, _| rooms.values().any(|at| at == room));
        world.insert_resource(rows);
    }
    inputs.forget_before(tick);
    world.insert_resource(inputs);
}

/// Take each arrived input into the record, and rewind each guest room whose
/// past input arrived different from the one used.
pub(crate) fn take_arrived_inputs(world: &mut World, arrived: Vec<(PlayerSlot, u64, ControlFrame)>) {
    let now = world.resource::<SimTick>().0;
    let (rooms, _) = guest_rooms(world);
    let mut inputs = world.remove_resource::<GuestInputs>().unwrap_or_default();
    let mut stats = world.remove_resource::<RoomRollbackStats>().unwrap_or_default();
    let mut rewind: BTreeMap<LiveRoomInstance, u64> = BTreeMap::new();
    for (seat, tick, frame) in arrived {
        // An input for a tick before the horizon has no save to go back to:
        // it is the guest's input from the next tick on.
        let rewindable = rooms.contains_key(&seat) && tick + REWIND_HORIZON > now;
        if !rewindable && tick <= now {
            stats.not_rewound += 1;
        }
        let tick = if rewindable { tick } else { tick.max(now + 1) };
        inputs.arrived.entry(seat).or_default().insert(tick, frame);
        if tick > now {
            continue;
        }
        let used = inputs.used.get(&seat).and_then(|used| used.get(&tick)).copied();
        if used.is_none_or(|used| used == frame) {
            continue;
        }
        if let Some(room) = rooms.get(&seat) {
            let earliest = rewind.entry(*room).or_insert(tick);
            *earliest = (*earliest).min(tick);
        }
    }
    world.insert_resource(inputs);
    // A crossing spans two rooms, and a replay hides every room but one: the
    // crossing's plan would not find the room it joins and would build a
    // second one. While a crossing is recorded or loading, a late input
    // drives the seat from the next tick instead.
    let crossing = a_crossing_is_under_way(world);
    for (room, tick) in rewind {
        if crossing {
            stats.not_rewound += 1;
            continue;
        }
        let replayed = rewind_room(world, room, tick, now);
        match replayed {
            Some(ticks) => {
                stats.rewinds += 1;
                stats.replayed += ticks;
            }
            None => stats.not_rewound += 1,
        }
    }
    world.insert_resource(stats);
}

/// A room crossing recorded and not yet begun, or begun and not yet
/// committed.
fn a_crossing_is_under_way(world: &World) -> bool {
    world
        .get_resource::<ambition_platformer2d_session::lifecycle_commit::PendingLifecycleCommit>()
        .is_some_and(|pending| pending.peek().is_some())
        || world
            .get_resource::<ambition_platformer2d_runtime::room_transition::RoomTransitionLoadState>()
            .is_some_and(|load| load.active.is_some())
}

/// Put `room` back to the start of `tick` and replay it to `now`, the other
/// rooms hidden. `None` when the room holds no save for `tick`.
fn rewind_room(world: &mut World, room: LiveRoomInstance, tick: u64, now: u64) -> Option<u32> {
    let (mut snapshot, at) = {
        let mut inputs = world.resource_mut::<GuestInputs>();
        let saves = inputs.saved.get_mut(&room)?;
        let saved = saves.remove(&tick)?;
        // The replay saves each tick again as it runs it.
        saves.retain(|at, _| *at < tick);
        saved
    };
    let rows = world.remove_resource::<RoomSnapshotRows>()?;
    let entities = of_room(world, room);
    rows.restore(world, &mut snapshot, &entities);
    world.insert_resource(rows);
    ambition_platformer2d_shared_tangle::lifecycle::clear_channels_for_a_rewind(world);
    set_clock(world, at);
    ambition_platformer2d_shared_tangle::lifecycle::retract_room_writes(world, room, at.0 .0);
    let others: Vec<LiveRoomInstance> = world
        .query::<&LiveRoomInstance>()
        .iter(world)
        .copied()
        .filter(|other| *other != room)
        .collect();
    let mut hidden = Vec::new();
    for other in others {
        hidden.extend(of_room(world, other));
    }
    for entity in &hidden {
        world.entity_mut(*entity).insert(Disabled);
    }
    // The replay drives the host seats with their records; the frame being
    // run keeps what its devices gave it.
    let latches = world.get_resource::<ambition_characters::control::SlotControlLatches>().copied();
    let raw = world.get_resource::<ambition_characters::control::SeatRawFrames>().copied();
    let slots = world.get_resource::<ambition_characters::control::SlotControls>().copied();
    world.resource_mut::<GuestInputs>().replaying = true;
    // The replayed ticks are not confirmed while they run: an irreversible
    // act the replay records (a crossing, which needs the rooms hidden here)
    // waits for the first tick after it. Outside a replay the host publishes
    // no boundary and every tick is confirmed on arrival, as before.
    let session = ambition_platformer2d_shared_tangle::lifecycle::live_session_scope(world).map_or(0, |scope| scope.0);
    let published = !world.contains_resource::<ConfirmedFrameBoundary>();
    if published {
        world.insert_resource(ConfirmedFrameBoundary { current: tick as i32, confirmed: tick as i32 - 1, session });
    }
    // What the replayed ticks present was presented when they first ran.
    let effects = world.get_resource::<ReplayHidesEffects>().map(|hides| hides.0.clone()).unwrap_or_default();
    for (lift, _) in &effects {
        lift(world);
    }
    let mut ticks = 0;
    while world.resource::<SimTick>().0 < now {
        run_one_tick(world);
        ticks += 1;
    }
    for (_, put_back) in &effects {
        put_back(world);
    }
    world.resource_mut::<GuestInputs>().replaying = false;
    if published {
        world.remove_resource::<ConfirmedFrameBoundary>();
        // What the replay's verdicts were stamped with is settled now.
        if let Some(log) = world.get_resource::<ambition_platformer2d_shared_tangle::authored_logic::AuthoredVerdictLog>() {
            log.confirm_through(session, now as i32);
        }
    }
    if let Some(latches) = latches {
        world.insert_resource(latches);
    }
    if let Some(raw) = raw {
        world.insert_resource(raw);
    }
    if let Some(slots) = slots {
        world.insert_resource(slots);
    }
    for entity in &hidden {
        if let Ok(mut entity) = world.get_entity_mut(*entity) {
            entity.remove::<Disabled>();
        }
    }
    Some(ticks)
}

/// The presentation channels (sounds, visuals, shakes: the families
/// `for_each_presentation_effect` names) a replay writes nowhere. Each
/// family's lift puts an empty channel in and the real one aside, and its
/// put-back restores the real one, so readers' cursors still match it. The
/// replayed ticks were presented when they first ran; a replay that
/// presented them again played a rewound room's sounds and visuals twice.
#[derive(Resource, Default)]
pub(crate) struct ReplayHidesEffects(Vec<(fn(&mut World), fn(&mut World))>);

impl ReplayHidesEffects {
    pub(crate) fn of_app(app: &mut App) -> Self {
        struct Collect(Vec<(fn(&mut World), fn(&mut World))>);
        impl ambition_platformer2d_runtime::external_effects::PresentationEffectVisitor for Collect {
            fn visit<M: Message>(&mut self, _app: &mut App) {
                self.0.push((lift::<M>, put_back::<M>));
            }
        }
        let mut collect = Collect(Vec::new());
        ambition_platformer2d_runtime::external_effects::for_each_presentation_effect(app, &mut collect);
        Self(collect.0)
    }
}

#[derive(Resource)]
struct Lifted<M: Message>(Messages<M>);

fn lift<M: Message>(world: &mut World) {
    if let Some(mut channel) = world.get_resource_mut::<Messages<M>>() {
        let real = std::mem::take(&mut *channel);
        world.insert_resource(Lifted(real));
    }
}

fn put_back<M: Message>(world: &mut World) {
    if let Some(Lifted(real)) = world.remove_resource::<Lifted<M>>() {
        *world.resource_mut::<Messages<M>>() = real;
    }
}

/// Run one simulation step, as the host's own loop runs it.
fn run_one_tick(world: &mut World) {
    let label = world.resource::<ambition_platformer2d_shared_tangle::schedule::SimSchedule>().peek();
    if label == bevy::ecs::schedule::ScheduleLabel::intern(&FixedUpdate) {
        *world.resource_mut::<Time>() = world.resource::<Time<Fixed>>().as_generic();
        world.run_schedule(bevy::app::FixedMain);
        *world.resource_mut::<Time>() = world.resource::<Time<Virtual>>().as_generic();
    } else {
        world.run_schedule(label);
    }
}
