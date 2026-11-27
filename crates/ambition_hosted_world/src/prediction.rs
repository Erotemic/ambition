//! A guest that predicts its own body (NA-3), in production.
//!
//! With [`PredictsOwnBody`] a guest's simulation stays awake once it is
//! connected: its own seat moves on the frame its devices move it, instead
//! of a round trip later. Its own tick is the tick it stamps its input with:
//! each tick sends one [`GuestInput`] for that tick, so the host runs the
//! input on the tick the guest ran it. Each tick it saves its own room at the
//! start (`RoomSnapshotRows`, with the clock, as the host's room rollback
//! does) and notes where its own body ended the tick before.
//!
//! The host stamps the guest's own body with the tick its rows were written
//! at ([`RowTick`]). When that body arrives somewhere other than where the
//! guest predicted it for that tick (another body hit it, or an input
//! arrived late), the guest puts its room back to the save after that tick,
//! writes every body it is sent as the host had it, and replays its own
//! inputs since: the replay is the host's room rewind
//! (`room_rollback::replay_room`) run on the guest. The other bodies are the
//! host's, written when their rows arrive and moved by the guest's own
//! simulation between.
//!
//! The guest's tick follows its estimate of the host tick plus its run-ahead
//! ([`crate::input_clock`]): on its first predicted tick it takes that tick,
//! and after it it jumps ahead when it falls behind (the host predicts the
//! ticks it skipped, and the next correction replays them). A guest whose
//! lead shrinks stays ahead: its inputs arrive earlier than they need to.
//!
//! The measurements are in
//! `docs/planning/engine/hosted-world-networking-experiment.md`.

use std::collections::BTreeMap;

use bevy::prelude::*;

use ambition_characters::control::{DrivingParticipant, PlayerSlot, SeatRawFrames, SlotControlLatches};
use ambition_platformer2d_core::{BodyKinematics, ControlFrame};
use ambition_platformer2d_runtime::rollback::room_snapshot::{RoomSnapshot, RoomSnapshotRows};
use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
use ambition_time::SimTick;

use crate::room_rollback::{self, Clock, REWIND_HORIZON};
use crate::{BodyRow, GuestInput, HostTick, InputFromSeat, LifeRow, LocalSeat, RowTick};

/// On a guest: predict its own body, and correct it by replaying its room
/// from the host's word (see the module docs). Without it a guest's
/// simulation sleeps once it is connected and it draws its own body where
/// the host's rows put it.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct PredictsOwnBody;

/// What a predicting guest did, for a caller to read.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct GuestPredictionStats {
    /// The host's rows of the guest's own body compared with its prediction.
    pub compared: u32,
    /// Of those, the ones that differed and were corrected by a replay.
    pub corrected: u32,
    /// Ticks re-simulated by those replays.
    pub replayed: u32,
    /// Ones that differed and could not replay (no save of that tick, a
    /// crossing under way, or a refusal such as a match).
    pub not_corrected: u32,
    /// The sum and the largest of the differences (world units, its place).
    pub correction_sum: f32,
    pub largest_correction: f32,
    /// Times the guest's tick jumped ahead to its estimate.
    pub jumps: u32,
}

/// A predicting guest's record: its own inputs and its room's saves, by tick.
#[derive(Resource, Default)]
pub(crate) struct GuestPrediction {
    /// The seat this record is for; another seat starts a new record.
    seat: Option<PlayerSlot>,
    /// The room the saves hold; another room starts a new record.
    room: Option<LiveRoomInstance>,
    /// Each tick's own input, as sent.
    inputs: BTreeMap<u64, ControlFrame>,
    /// The own room at the start of each tick.
    saved: BTreeMap<u64, (RoomSnapshot, Clock)>,
    /// The own body at the end of each tick.
    after: BTreeMap<u64, BodyRow>,
    /// While a correction replays: inputs come from the record.
    replaying: bool,
    /// The newest host tick whose rows were compared.
    compared_through: u64,
}

impl GuestPrediction {
    /// The input for `tick`: the one sent for it, or else the held controls
    /// of the latest before it (as the host predicts a tick with no input).
    fn input_for(&self, tick: u64) -> ControlFrame {
        crate::input_history::predicted_input(&self.inputs, tick)
    }

    fn forget_before(&mut self, tick: u64) {
        let keep = tick.saturating_sub(REWIND_HORIZON);
        crate::input_history::retain_input_from(&mut self.inputs, keep);
        self.saved.retain(|at, _| *at >= keep);
        self.after.retain(|at, _| *at >= keep);
    }
}

/// The body `seat` drives, and its room.
fn own_body(world: &mut World, seat: PlayerSlot) -> Option<(Entity, Option<LiveRoomInstance>)> {
    let mut drivers = world.query::<(Entity, &DrivingParticipant, Option<&InRoomInstance>)>();
    let mut found: Vec<(Entity, Option<LiveRoomInstance>)> = drivers
        .iter(world)
        .filter(|(_, driver, _)| driver.0 == seat)
        .map(|(entity, _, room)| (entity, room.map(|room| room.0)))
        .collect();
    found.sort_by_key(|(entity, _)| *entity);
    found.into_iter().next()
}

/// At the head of each tick of a predicting guest (before the clock): take
/// this tick's number from the input clock, drive the own seat with what the
/// devices gave (or, in a replay, with the record), send it stamped with the
/// tick, note where the own body ended the tick before, and save the room.
pub(crate) fn predict_own_tick(world: &mut World) {
    if !world.contains_resource::<PredictsOwnBody>() {
        return;
    }
    let Some(seat) = world.get_resource::<LocalSeat>().map(|seat| seat.0) else { return };
    let host = world.query::<&HostTick>().iter(world).next().map(|tick| tick.0);
    let Some(host) = host else { return };
    let mut prediction = world.remove_resource::<GuestPrediction>().unwrap_or_default();
    if prediction.seat != Some(seat) {
        prediction = GuestPrediction { seat: Some(seat), ..Default::default() };
    }
    let from = world.get_resource::<InputFromSeat>().map_or(seat, |from| from.0);
    let mut tick = world.resource::<SimTick>().0 + 1;
    if prediction.replaying {
        let recorded = prediction.input_for(tick);
        if let Some(mut latches) = world.get_resource_mut::<SlotControlLatches>() {
            latches.reset(seat);
            latches.reset(from);
        }
        if from != seat {
            if let Some(mut raw) = world.get_resource_mut::<SeatRawFrames>() {
                raw.set(from, ControlFrame::default());
            }
        }
        ambition_platformer2d_runtime::input_drive::drive_slot_frame(world, seat, recorded);
    } else {
        let elapsed = world.resource::<Time<Real>>().elapsed();
        let step = world.resource::<Time<Fixed>>().timestep();
        let (estimate, wanted) = world.resource_mut::<crate::input_clock::GuestInputClock>().wanted(host, elapsed, step);
        // The first predicted tick takes the wanted tick whichever way it
        // lies; after it the tick only jumps ahead, since a stamp the host
        // already holds would merge into it.
        let first = prediction.inputs.is_empty();
        if tick < wanted || (first && tick != wanted) {
            world.resource_mut::<SimTick>().0 = wanted.saturating_sub(1);
            tick = wanted;
            if !first {
                world.resource_mut::<GuestPredictionStats>().jumps += 1;
            }
        }
        // The devices' seat drives the seat the host gave this guest, and
        // nothing drives the devices' own seat (on the host it is someone
        // else's).
        let given = match world.get_resource::<SlotControlLatches>() {
            Some(latches) => latches.peek(from),
            None => world.get_resource::<SeatRawFrames>().map(|raw| raw.get(from)).unwrap_or_default(),
        };
        if from != seat {
            if let Some(mut latches) = world.get_resource_mut::<SlotControlLatches>() {
                latches.reset(from);
            }
            if let Some(mut raw) = world.get_resource_mut::<SeatRawFrames>() {
                raw.set(from, ControlFrame::default());
            }
            ambition_platformer2d_runtime::input_drive::drive_slot_frame(world, seat, given);
        }
        prediction.inputs.insert(tick, given);
        let sequence = world.resource_mut::<crate::input_clock::GuestInputClock>().sample_at(tick, estimate);
        world.write_message(GuestInput { sequence, tick, frame: given });
    }
    if let Some((body, room)) = own_body(world, seat) {
        if let Some(kinematics) = world.get::<BodyKinematics>(body) {
            prediction.after.insert(tick - 1, BodyRow::of(kinematics));
        }
        if prediction.room != room {
            prediction.saved.clear();
            prediction.after.clear();
            prediction.room = room;
        }
        // A crossing moves or rebuilds what a save holds (as on the host).
        if room_rollback::a_crossing_is_under_way(world) {
            prediction.saved.clear();
        } else if let (Some(room), Some(rows), Some(clock)) =
            (room, world.remove_resource::<RoomSnapshotRows>(), room_rollback::clock(world))
        {
            let entities = ambition_platformer2d_shared_tangle::lifecycle::room_scope_members(world, room);
            prediction.saved.insert(tick, (rows.save(world, &entities), clock));
            world.insert_resource(rows);
        }
    }
    prediction.forget_before(tick);
    world.insert_resource(prediction);
}

/// Whether two rows of one body agree, to the float noise of the wire.
fn agree(a: &BodyRow, b: &BodyRow) -> bool {
    const NOISE: f32 = 1e-4;
    (a.pos[0] - b.pos[0]).abs() <= NOISE
        && (a.pos[1] - b.pos[1]).abs() <= NOISE
        && (a.vel[0] - b.vel[0]).abs() <= NOISE
        && (a.vel[1] - b.vel[1]).abs() <= NOISE
        && a.facing == b.facing
}

/// After the host's rows are applied: compare the own body's row with the
/// prediction for the tick it was written at, and on a difference put the
/// room back to the end of that tick, write every body as the host has it
/// and replay the own inputs since.
pub(crate) fn reconcile_own_body(world: &mut World) {
    if !world.contains_resource::<PredictsOwnBody>() {
        return;
    }
    let Some(seat) = world.get_resource::<LocalSeat>().map(|seat| seat.0) else { return };
    let Some((body, Some(room))) = own_body(world, seat) else { return };
    let (Some(row), Some(at)) = (world.get::<BodyRow>(body).copied(), world.get::<RowTick>(body).map(|at| at.0)) else {
        return;
    };
    let Some(mut prediction) = world.get_resource_mut::<GuestPrediction>() else { return };
    if at <= prediction.compared_through || prediction.room != Some(room) {
        return;
    }
    let Some(predicted) = prediction.after.get(&at).copied() else { return };
    prediction.compared_through = at;
    let error = Vec2::from_array(row.pos).distance(Vec2::from_array(predicted.pos));
    let agrees = agree(&row, &predicted);
    let saved = if agrees {
        None
    } else {
        prediction.saved.remove(&(at + 1)).inspect(|_| {
            // The replay saves and notes each tick again as it runs it.
            prediction.saved.retain(|tick, _| *tick <= at);
            prediction.after.retain(|tick, _| *tick <= at);
        })
    };
    let mut stats = world.resource_mut::<GuestPredictionStats>();
    stats.compared += 1;
    if agrees {
        return;
    }
    let refused = room_rollback::a_crossing_is_under_way(world)
        || ambition_platformer2d_shared_tangle::lifecycle::room_replay_refusal(world).is_some()
        || !world.contains_resource::<RoomSnapshotRows>();
    let Some((mut snapshot, clock)) = saved.filter(|_| !refused) else {
        world.resource_mut::<GuestPredictionStats>().not_corrected += 1;
        return;
    };
    let now = world.resource::<SimTick>().0;
    let replaying = |world: &mut World, on: bool| world.resource_mut::<GuestPrediction>().replaying = on;
    let ticks = room_rollback::replay_room(world, room, &mut snapshot, clock, at + 1, now, replaying, |world| {
        // Every body the host sends, as the host had it at `at`.
        let mut sent = world.query::<(&BodyRow, &mut BodyKinematics, Option<&LifeRow>, Option<&mut ambition_characters::actor::BodyHealth>)>();
        for (row, mut kinematics, life, health) in sent.iter_mut(world) {
            row.write_into(&mut kinematics);
            if let (Some(life), Some(mut health)) = (life, health) {
                life.write_into(&mut health, None);
            }
        }
    });
    let mut stats = world.resource_mut::<GuestPredictionStats>();
    stats.corrected += 1;
    stats.replayed += ticks;
    stats.correction_sum += error;
    stats.largest_correction = stats.largest_correction.max(error);
}
