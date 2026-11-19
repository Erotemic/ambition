//! A host that keeps a guest's room correct by rollback (SIMULATION-ISLANDS).
//!
//! Each guest's input is stamped with the host tick it was meant for
//! ([`GuestInput::tick`]) and reaches the host after that tick ran. With
//! [`RewindsGuestRooms`], the host drives each guest seat on every tick with
//! the input it holds for that tick, or else the latest held controls
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
use bevy::ecs::message::Message;
use bevy::prelude::*;

use ambition_combat::impact_hitstop::ImpactHitstop;
use ambition_characters::control::{DrivingParticipant, PlayerSlot, SlotInteractionState};
use ambition_platformer2d_core::{ConfirmedFrameBoundary, ControlFrame};
use ambition_platformer2d_runtime::rollback::room_snapshot::{RoomSnapshot, RoomSnapshotRows};
use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
use ambition_time::time_control::RequestedClockScale;
use ambition_time::{ClockState, DayClock, GameplayElapsed, SimTick, WorldTime};

use crate::GuestSeat;

/// The input age limit in ticks. Older network samples are rejected. An
/// admitted input without a saved tick applies from the next host tick.
pub const REWIND_HORIZON: u64 = 60;

pub use ambition_platformer2d_shared_tangle::lifecycle::RewindsGuestRooms;

/// What the room rollback host did, for a caller to read.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RoomRollbackStats {
    /// Room rewinds for a late input that differed from the one used.
    pub rewinds: u32,
    /// Ticks re-simulated by those rewinds.
    pub replayed: u32,
    /// Admitted late inputs that could not rewind and apply on the next tick.
    pub not_rewound: u32,
    /// The tick the latest rewind replayed from.
    pub last_rewound_from: Option<u64>,
}

/// The shared clock at the start of a tick: the timeline, the frame's time,
/// the day, the scale, the session's elapsed seconds, the impact freeze, and
/// the scale the clock is easing toward, and the seats' gestures. A slowdown
/// is a target the clock holds until another request: a request the replay
/// takes back must take its target back with it, or the restored scale eases
/// toward it again. A gesture (an interact buffer, a double tap) is timed on
/// the clock too; `rewind_room` keeps the live gestures of the seats outside
/// the room, which the replay does not run.
/// The freeze is an expiry against the timeline that stops the shared clock
/// while one room is live: a connect the replay takes back must take its
/// freeze back with it. Possession is each seat's too (the body it drives, the
/// hold that takes another), and `rewind_room` keeps it live for the seats
/// outside the room, as it keeps their gestures.
#[derive(Clone)]
pub(crate) struct Clock(
    SimTick,
    WorldTime,
    DayClock,
    ClockState,
    GameplayElapsed,
    ImpactHitstop,
    RequestedClockScale,
    SlotInteractionState,
    ambition_platformer2d_actor_monolith::control::possession::PossessionState,
);

pub(crate) fn clock(world: &World) -> Option<Clock> {
    Some(Clock(
        *world.get_resource::<SimTick>()?,
        *world.get_resource::<WorldTime>()?,
        *world.get_resource::<DayClock>()?,
        *world.get_resource::<ClockState>()?,
        *ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<GameplayElapsed>(world)?,
        world.get_resource::<ImpactHitstop>().copied().unwrap_or_default(),
        world.get_resource::<RequestedClockScale>().copied().unwrap_or_default(),
        world.get_resource::<SlotInteractionState>().copied().unwrap_or_default(),
        world.get_resource::<ambition_platformer2d_actor_monolith::control::possession::PossessionState>().cloned().unwrap_or_default(),
    ))
}

fn set_clock(world: &mut World, clock: Clock) {
    world.insert_resource(clock.0);
    world.insert_resource(clock.1);
    world.insert_resource(clock.2);
    world.insert_resource(clock.3);
    world.insert_resource(clock.5);
    if let Some(mut target) = world.get_resource_mut::<RequestedClockScale>() {
        *target = clock.6;
    }
    if let Some(mut gestures) = world.get_resource_mut::<SlotInteractionState>() {
        *gestures = clock.7;
    }
    if let Some(mut possession) = world.get_resource_mut::<ambition_platformer2d_actor_monolith::control::possession::PossessionState>() {
        *possession = clock.8;
    }
    if let Some(mut elapsed) =
        ambition_platformer2d_shared_tangle::lifecycle::session_world_component_mut::<GameplayElapsed>(world)
    {
        *elapsed = clock.4;
    }
}

/// The host's record of each guest seat's inputs and each guest room's saves.
#[derive(Resource, Default)]
pub(crate) struct GuestInputs {
    last_sequence: BTreeMap<PlayerSlot, u64>,
    owned_since: BTreeMap<PlayerSlot, u64>,
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
    /// The live checkpoint commits read so far.
    commits: bevy::ecs::message::MessageCursor<ambition_platformer2d_shared_tangle::lifecycle::CheckpointCommitted>,
}

impl GuestInputs {
    pub(crate) fn record(&mut self, seat: PlayerSlot, input: crate::GuestInput, now: u64) -> Option<(u64, ControlFrame)> {
        if input.tick > now.saturating_add(crate::input_clock::MAX_INPUT_LEAD + crate::input_clock::MAX_RUN_AHEAD)
            || now.saturating_sub(input.tick) > REWIND_HORIZON
            || self.last_sequence.get(&seat).is_some_and(|last| input.sequence <= *last)
        { return None; }
        self.last_sequence.insert(seat, input.sequence);
        // A stamp never goes back past the seat's latest: the confirmed-frame
        // boundary (`publish_live_boundary`) confirms each tick before it.
        let latest = self.arrived.get(&seat).and_then(|arrived| arrived.keys().next_back()).copied().unwrap_or(0);
        let tick = input.tick.max(self.owned_since.get(&seat).copied().unwrap_or(0)).max(latest);
        let frame = self.arrived.entry(seat).or_default().entry(tick).or_default();
        *frame = frame.merge_sample(input.frame);
        let frame = *frame;
        self.forget_before(now);
        Some((tick, frame))
    }

    /// The newest tick no late input can make the host replay, at the head of
    /// `tick`: for each guest seat, the tick before its latest stamped input
    /// (its stamps never go back), or before its ownership began when none
    /// has come; a tick past the rewind horizon is final whatever arrives.
    fn confirmed_through(&self, tick: u64) -> i64 {
        let now = tick as i64 - 1;
        let floor = now - REWIND_HORIZON as i64 - 1;
        self.owned_since
            .iter()
            .map(|(seat, since)| match self.arrived.get(seat).and_then(|arrived| arrived.keys().next_back()) {
                Some(latest) => *latest as i64 - 1,
                None => *since as i64 - 1,
            })
            .min()
            .map_or(tick as i64, |confirmed| confirmed.max(floor).min(tick as i64))
    }

    pub(crate) fn begin_ownership(&mut self, seat: PlayerSlot, tick: u64) {
        self.forget_seat(seat);
        self.owned_since.insert(seat, tick);
    }

    pub(crate) fn forget_seat(&mut self, seat: PlayerSlot) {
        self.arrived.remove(&seat);
        self.used.remove(&seat);
        self.local.remove(&seat);
        self.last_sequence.remove(&seat);
        self.owned_since.remove(&seat);
    }

    /// The input for `seat` on `tick`: the one it sent for that tick, or else
    /// its latest before it (the prediction).
    fn input_for(&self, seat: PlayerSlot, tick: u64) -> ControlFrame {
        self.arrived
            .get(&seat)
            .and_then(|arrived| arrived.range(..=tick).next_back())
            .map(|(at, frame)| if *at == tick { *frame } else { frame.levels_only() })
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

/// At the head of each live tick on a host that rewinds guest rooms: publish
/// the confirmed-frame boundary, so the irreversible observers (checkpoints,
/// deaths, passages, the conversation ledger, host intents, persistence) act
/// on a tick only once no late guest input can make the host replay it. A
/// replay keeps its own boundary (`rewind_room`).
pub(crate) fn publish_live_boundary(world: &mut World) {
    if !world.contains_resource::<RewindsGuestRooms>() {
        return;
    }
    let Some(inputs) = world.get_resource::<GuestInputs>() else { return };
    if inputs.replaying {
        return;
    }
    let tick = world.resource::<SimTick>().0 + 1;
    let confirmed = inputs.confirmed_through(tick);
    let session = ambition_platformer2d_shared_tangle::lifecycle::live_session_scope(world).map_or(0, |scope| scope.0);
    let boundary = ConfirmedFrameBoundary { current: tick as i32, confirmed: confirmed as i32, session };
    if world.get_resource::<ConfirmedFrameBoundary>() != Some(&boundary) {
        world.insert_resource(boundary);
    }
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
    } else if let Some(commits) =
        world.get_resource::<bevy::ecs::message::Messages<ambition_platformer2d_shared_tangle::lifecycle::CheckpointCommitted>>()
    {
        // A checkpoint commit is the world's agreement on every room at once,
        // and every domain takes its baseline from it: a rewind to before it
        // would run the commit again with the other rooms hidden. A live
        // commit drops every room's history, so a late input for an earlier
        // tick drives its seat from the next one. A replay commits nothing:
        // the shrine and the authored checkpoint hold theirs for a live tick.
        if inputs.commits.read(commits).count() > 0 {
            inputs.saved.clear();
        }
    }
    // A lifecycle operation (a crossing, a room's reset, a return) moves or
    // rebuilds what the saves hold. One a replay records commits on the first
    // live tick after it (the boundary `rewind_room` publishes); while one is
    // recorded or loading the host keeps no history, so no later replay runs
    // it a second time.
    if !inputs.replaying && a_crossing_is_under_way(world) {
        inputs.saved.clear();
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
            let entities = ambition_platformer2d_shared_tangle::lifecycle::room_scope_members(world, room);
            let snapshot = rows.save(world, &entities);
            inputs.saved.entry(room).or_default().insert(tick, (snapshot, now.clone()));
        }
        // Hidden rooms still have guests and rollback history during replay.
        if !inputs.replaying {
            inputs.saved.retain(|room, _| rooms.values().any(|at| at == room));
        }
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
    let mut unseated = BTreeMap::<PlayerSlot, ControlFrame>::new();
    for (seat, tick, frame) in arrived {
        if !rooms.contains_key(&seat) {
            let pending = unseated.entry(seat).or_default();
            *pending = pending.merge_sample(frame);
        }
        // A seat without a room, or a tick at the oldest boundary, has no
        // saved state to replay. Its input applies from the next tick.
        let rewindable = rooms.contains_key(&seat) && now.saturating_sub(tick) < REWIND_HORIZON;
        if !rewindable && tick <= now {
            stats.not_rewound += 1;
        }
        let tick = if rewindable { tick } else { tick.max(now + 1) };
        if !rewindable {
            let applied = inputs.arrived.entry(seat).or_default().entry(tick).or_default();
            *applied = applied.merge_sample(frame);
        }
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
    for (seat, frame) in unseated {
        ambition_platformer2d_runtime::input_drive::drive_slot_frame(world, seat, frame);
    }
    // A crossing spans two rooms, and a replay hides every room but one: the
    // crossing's plan would not find the room it joins and would build a
    // second one. While a crossing is recorded or loading, or a registered
    // refusal holds (a match), a late input drives the seat from the next
    // tick instead.
    let refused = a_crossing_is_under_way(world)
        || ambition_platformer2d_shared_tangle::lifecycle::room_replay_refusal(world).is_some();
    for (room, tick) in rewind {
        if refused {
            stats.not_rewound += 1;
            continue;
        }
        let replayed = rewind_room(world, room, tick, now);
        match replayed {
            Some(ticks) => {
                stats.rewinds += 1;
                stats.replayed += ticks;
                stats.last_rewound_from = Some(tick);
            }
            None => stats.not_rewound += 1,
        }
    }
    world.insert_resource(stats);
}

/// A room crossing recorded and not yet begun, or begun and not yet
/// committed.
pub(crate) fn a_crossing_is_under_way(world: &World) -> bool {
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
    if !world.contains_resource::<RoomSnapshotRows>() {
        return None;
    }
    let replaying = |world: &mut World, on: bool| world.resource_mut::<GuestInputs>().replaying = on;
    Some(replay_room(world, room, &mut snapshot, at, tick, now, replaying, |_| {}))
}

/// Put `room` back to `snapshot` (its state at the start of `tick`, with the
/// clock `at`), let `correct` write what is known better than the save, and
/// replay the room alone to `now`: the other rooms hidden, its world writes
/// since the save retracted, the input tables as they stood put back after.
/// `replaying` marks the replay for the caller's own tick head. Returns the
/// ticks replayed.
#[allow(clippy::too_many_arguments)]
pub(crate) fn replay_room(
    world: &mut World,
    room: LiveRoomInstance,
    snapshot: &mut RoomSnapshot,
    at: Clock,
    tick: u64,
    now: u64,
    replaying: fn(&mut World, bool),
    correct: impl FnOnce(&mut World),
) -> u32 {
    let rows = world.remove_resource::<RoomSnapshotRows>().expect("the caller checked the room rows");
    let entities = ambition_platformer2d_shared_tangle::lifecycle::room_scope_members(world, room);
    rows.restore(world, snapshot, &entities);
    world.insert_resource(rows);
    correct(world);
    ambition_platformer2d_shared_tangle::lifecycle::begin_room_replay(world, room);
    let live_gestures = world.get_resource::<SlotInteractionState>().copied();
    let live_possession = world.get_resource::<ambition_platformer2d_actor_monolith::control::possession::PossessionState>().cloned();
    let saved_at = at.0 .0;
    set_clock(world, at);
    ambition_platformer2d_shared_tangle::lifecycle::retract_room_writes(world, room, saved_at);
    let others: Vec<LiveRoomInstance> = world
        .query::<&LiveRoomInstance>()
        .iter(world)
        .copied()
        .filter(|other| *other != room)
        .collect();
    let mut hidden = Vec::new();
    for other in others {
        hidden.extend(ambition_platformer2d_shared_tangle::lifecycle::room_scope_members(world, other));
    }
    for entity in &hidden {
        world.entity_mut(*entity).insert(Disabled);
    }
    // The replay drives the host seats with their records; the frame being
    // run keeps what its devices gave it.
    let latches = world.get_resource::<ambition_characters::control::SlotControlLatches>().copied();
    let raw = world.get_resource::<ambition_characters::control::SeatRawFrames>().copied();
    let slots = world.get_resource::<ambition_characters::control::SlotControls>().copied();
    replaying(world, true);
    // The replayed ticks are not confirmed while they run: an irreversible
    // act the replay records (a crossing, which needs the rooms hidden here)
    // waits for the first live tick after it. Nor is anything the live
    // boundary had not confirmed: another guest's input can still be late.
    let session = ambition_platformer2d_shared_tangle::lifecycle::live_session_scope(world).map_or(0, |scope| scope.0);
    let live = world.remove_resource::<ConfirmedFrameBoundary>();
    let confirmed = live.map_or(tick as i32 - 1, |live| live.confirmed.min(tick as i32 - 1));
    world.insert_resource(ConfirmedFrameBoundary { current: tick as i32, confirmed, session });
    let mut ticks = 0;
    while world.resource::<SimTick>().0 < now {
        run_one_tick(world);
        ticks += 1;
    }
    replaying(world, false);
    world.remove_resource::<ConfirmedFrameBoundary>();
    match live {
        // The live boundary stands again; the next live tick publishes anew.
        Some(live) => world.insert_resource(live),
        // What the replay's verdicts were stamped with is settled now.
        None => {
            if let Some(log) = world.get_resource::<ambition_platformer2d_shared_tangle::authored_logic::AuthoredVerdictLog>() {
                log.confirm_through(session, now as i32);
            }
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
    // The seats outside the room did not run the replayed ticks: their
    // gestures and possession are the live ones.
    let inside: Vec<PlayerSlot> = world
        .query::<(&DrivingParticipant, &InRoomInstance)>()
        .iter(world)
        .filter(|(_, at)| at.0 == room)
        .map(|(driver, _)| driver.0)
        .collect();
    if let (Some(live), Some(mut possession)) = (live_possession, world.get_resource_mut::<ambition_platformer2d_actor_monolith::control::possession::PossessionState>()) {
        for index in 0..ambition_characters::control::SlotControls::MAX_SLOTS {
            let seat = PlayerSlot(index as u8);
            if !inside.contains(&seat) && possession.get(seat) != live.get(seat) {
                *possession.get_mut(seat) = *live.get(seat);
            }
        }
    }
    if let Some(live) = live_gestures {
        if let Some(mut gestures) = world.get_resource_mut::<SlotInteractionState>() {
            for index in 0..ambition_characters::control::SlotControls::MAX_SLOTS {
                let seat = PlayerSlot(index as u8);
                if inside.contains(&seat) {
                    continue;
                }
                if let Some(row) = gestures.get_mut(seat) {
                    *row = live.get(seat);
                }
            }
        }
    }
    for entity in &hidden {
        if let Ok(mut entity) = world.get_entity_mut(*entity) {
            entity.remove::<Disabled>();
        }
    }
    ambition_platformer2d_shared_tangle::lifecycle::end_room_replay(world);
    ticks
}

/// Presentation output stays at the live boundary while a room replays.
pub(crate) fn register_presentation_replay_channels(app: &mut App) {
    struct Register;
    impl ambition_platformer2d_runtime::external_effects::PresentationEffectVisitor for Register {
        fn visit<M: Message>(&mut self, app: &mut App) {
            ambition_platformer2d_shared_tangle::lifecycle::register_room_replay_message::<M>(app);
        }
    }
    ambition_platformer2d_runtime::external_effects::for_each_presentation_effect(app, &mut Register);
}

/// Run one simulation step, as the host's own loop runs it.
pub(crate) fn run_one_tick(world: &mut World) {
    let label = world.resource::<ambition_platformer2d_shared_tangle::schedule::SimSchedule>().peek();
    if label == bevy::ecs::schedule::ScheduleLabel::intern(&FixedUpdate) {
        *world.resource_mut::<Time>() = world.resource::<Time<Fixed>>().as_generic();
        world.run_schedule(bevy::app::FixedMain);
        *world.resource_mut::<Time>() = world.resource::<Time<Virtual>>().as_generic();
    } else {
        world.run_schedule(label);
    }
}

#[cfg(test)]
mod input_tests {
    use super::*;
    use crate::GuestInput;

    #[test]
    fn samples_share_a_tick_without_losing_edges_or_predicting_them_again() {
        let mut inputs = GuestInputs::default();
        let seat = PlayerSlot(1);
        inputs.record(seat, GuestInput { sequence: 1, tick: 100, frame: ControlFrame {
            jump_pressed: true, jump_held: true, axis_x: 1.0, ..Default::default()
        } }, 100).unwrap();
        inputs.record(seat, GuestInput { sequence: 2, tick: 100, frame: ControlFrame {
            jump_released: true, ..Default::default()
        } }, 100).unwrap();
        let frame = inputs.input_for(seat, 100);
        assert!(frame.jump_pressed && frame.jump_released);
        assert!(!frame.jump_held);
        assert_eq!(frame.axis_x, 0.0);
        assert!(!inputs.input_for(seat, 101).jump_pressed);
        assert!(!inputs.input_for(seat, 101).jump_released);
    }

    #[test]
    fn an_invalid_timestamp_cannot_allocate_input_history_or_overflow() {
        for now in [0, 100, u64::MAX - 2] {
            let mut inputs = GuestInputs::default();
            let sample = GuestInput { sequence: 1, tick: u64::MAX, frame: Default::default() };
            let accepted = inputs.record(PlayerSlot(1), sample, now).is_some();
            assert_eq!(accepted, now == u64::MAX - 2);
            assert_eq!(inputs.arrived.len(), usize::from(accepted));
        }
        let mut inputs = GuestInputs::default();
        assert!(inputs.record(PlayerSlot(1), GuestInput { sequence: 1, tick: 1, frame: Default::default() }, 100).is_none());
        assert!(inputs.arrived.is_empty() && inputs.last_sequence.is_empty());
    }

    #[test]
    fn seat_retirement_drops_prediction_and_accepts_the_new_owners_sequence() {
        let mut inputs = GuestInputs::default();
        let seat = PlayerSlot(1);
        inputs.record(seat, GuestInput { sequence: 200, tick: 100, frame: ControlFrame { axis_x: 1.0, ..Default::default() } }, 100).unwrap();
        inputs.forget_before(200);
        assert_eq!(inputs.input_for(seat, 201).axis_x, 1.0);
        inputs.begin_ownership(seat, 201);
        assert_eq!(inputs.input_for(seat, 201), ControlFrame::default());
        let (tick, _) = inputs.record(seat, GuestInput { sequence: 1, tick: 199, frame: Default::default() }, 200).unwrap();
        assert_eq!(tick, 201, "a new owner cannot rewind the previous owner's frames");
    }
}
