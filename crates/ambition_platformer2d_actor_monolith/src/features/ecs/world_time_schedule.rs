//! OW5: an authored occurrence that went away comes back at a time on the
//! world clock, also while its room is not live.
//!
//! Two customers share the mechanism, one schedule and one clock (Q152: world
//! time is an engine mechanism, not a breakable one):
//!
//! * a broken breakable authored `AfterSeconds(n)` respawns;
//! * a collected pickup authored `AfterSeconds(n)` regrows.
//!
//! The rule (docs/planning/engine/open-world-runtime-and-residency.md,
//! "Activity policy and deterministic inputs"): the return is due at a time on
//! an admitted logical clock, [`GameplayElapsed`], the session's sum of the
//! scaled simulation dt. [`WorldTimeSchedule`] keeps that due time per
//! occurrence, keyed by its room definition and its authored id, at the session
//! lifetime:
//!
//! * while the room is live, the occurrence's live `RespawnTimer` is the
//!   authority, and [`mirror_breakable_respawns`] and [`regrow_pickups`] record
//!   its due time when it starts and forget it when the occurrence is whole
//!   again;
//! * when the room retires, the record stays: it is the dormant next-event
//!   state, and nothing ticks it;
//! * when the room is built again, construction reads the record (the commit
//!   facts, `PersistedFates`): an occurrence whose return is not yet due is
//!   built gone (a breakable broken with the time that remains, a pickup
//!   collected), and one that is due is built whole.
//!
//! No physics runs for a room that is not live. A replay of a room is a fresh
//! attempt, so it forgets that room's records ([`forget_scheduled_returns_on_replay`]);
//! a checkpoint restore and the session edge forget them all, because the save
//! holds no broken breakable and no collected pickup.
//!
//! ⚠ Keyed by room DEFINITION, because a record must outlive the instance it
//! was made in. A room has at most one live room (`DefinitionAlreadyLive`), so
//! two live rooms never share a key. Authored ids are unique within a room, so
//! a breakable and a pickup never share a key either.

use std::collections::BTreeMap;
use std::sync::Arc;

use bevy::prelude::*;

use ambition_combat::components::{BreakableFeature, Collected, FeatureId, PickupFeature, RespawnTimer};
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;

use crate::features::GameplayElapsed;

/// When each gone occurrence comes back, on [`GameplayElapsed`], by (room
/// definition id, authored id): a broken breakable's respawn and a collected
/// pickup's regrowth.
///
/// Rollback state: a break, a collection, a respawn and a regrowth write it on
/// a tick, and a rewind across that tick must take the record back with the
/// occurrence.
///
/// The records are SHARED, NOT COPIED (OW3, as M2 for the occurrence ledger).
/// The rollback host clones, hashes and compares this every frame, and the
/// records of every room that is not live stay here. A clone copies a pointer,
/// a write copies the records only when a snapshot shares them, and the
/// checksum is kept with the allocation it was computed from.
#[derive(Resource, Clone, Debug, Default)]
pub struct WorldTimeSchedule {
    due: Arc<Records>,
}

type Records = BTreeMap<(String, String), f32>;

/// Equal records, with the same allocation first: a snapshot and the live
/// schedule share their records while nothing changed.
impl PartialEq for WorldTimeSchedule {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.due, &other.due) || self.due == other.due
    }
}

impl WorldTimeSchedule {
    /// When the occurrence `feature` of room `room` comes back, if it is gone.
    pub fn due(&self, room: &str, feature: &str) -> Option<f32> {
        self.due
            .get(&(room.to_string(), feature.to_string()))
            .copied()
    }

    /// How long the occurrence `feature` of room `room` stays gone at time
    /// `now`: `Some` while its return is not yet due, `None` when it is whole
    /// or due.
    pub fn remaining(&self, room: &str, feature: &str, now: f32) -> Option<f32> {
        self.due(room, feature)
            .map(|due| due - now)
            .filter(|remaining| *remaining > 0.0)
    }

    /// The records, in key order.
    pub fn records(&self) -> impl Iterator<Item = (&(String, String), &f32)> {
        self.due.iter()
    }

    /// The occurrence `feature` of room `room` comes back at `due`. Keeps an
    /// existing record: the first departure decides the time.
    pub fn record(&mut self, room: &str, feature: &str, due: f32) {
        let key = (room.to_string(), feature.to_string());
        if !self.due.contains_key(&key) {
            Arc::make_mut(&mut self.due).insert(key, due);
        }
    }

    /// Forget the record of the occurrence `feature` of room `room`.
    pub fn forget(&mut self, room: &str, feature: &str) {
        let key = (room.to_string(), feature.to_string());
        if self.due.contains_key(&key) {
            Arc::make_mut(&mut self.due).remove(&key);
        }
    }

    /// Forget every record of room `room`.
    pub fn forget_room(&mut self, room: &str) {
        if self.due.keys().any(|(record_room, _)| record_room == room) {
            Arc::make_mut(&mut self.due).retain(|(record_room, _), _| record_room != room);
        }
    }

    /// Forget every record.
    pub fn forget_all(&mut self) {
        if !self.due.is_empty() {
            self.due = Arc::default();
        }
    }

    /// Entity-free value projection: two peers that disagree about when an
    /// occurrence comes back have diverged.
    ///
    /// The fold of the records, kept with the `Arc` it was computed from. The
    /// slot holds a clone of that `Arc`, so the allocation cannot be written in
    /// place, and the same allocation holds the same records. Any other
    /// allocation is folded again, so two peers agree whatever their slots hold.
    pub fn checksum(&self) -> u64 {
        static KEPT: std::sync::Mutex<Option<(Arc<Records>, u64)>> = std::sync::Mutex::new(None);
        if self.due.is_empty() {
            return self.fold();
        }
        let mut kept = KEPT.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((held, sum)) = kept.as_ref() {
            if Arc::ptr_eq(held, &self.due) {
                return *sum;
            }
        }
        let sum = self.fold();
        *kept = Some((self.due.clone(), sum));
        sum
    }

    fn fold(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::{checksum_bytes, put_str, put_u64};
        let mut bytes = Vec::new();
        put_u64(&mut bytes, self.due.len() as u64);
        for ((room, feature), due) in self.due.iter() {
            put_str(&mut bytes, room);
            put_str(&mut bytes, feature);
            put_u64(&mut bytes, u64::from(due.to_bits()));
        }
        checksum_bytes(&bytes)
    }
}

/// The time each gone occurrence stays gone as of now, by (room definition
/// id, authored id): the records whose return is not yet due. A room commit
/// reads it (`PersistedFates::with_scheduled_returns`).
pub fn remaining_scheduled_returns(world: &World) -> BTreeMap<(String, String), f32> {
    let (Some(schedule), Some(now)) = (
        world.get_resource::<WorldTimeSchedule>(),
        world.get_resource::<GameplayElapsed>(),
    ) else {
        return BTreeMap::new();
    };
    schedule
        .due
        .iter()
        .map(|(key, due)| (key.clone(), due - now.0))
        .filter(|(_, remaining)| *remaining > 0.0)
        .collect()
}

/// Keep each live breakable's record in step with its live timer: a broken
/// breakable whose timer runs gets its due time when it has none, and a whole
/// breakable loses its record. Writes only on those two edges.
///
/// ⛔ NO CHANGE-TICK GATE (`Added<RespawnTimer>`) for the start edge. A
/// rollback load inserts the timer again, so a gate would open on a load
/// frame, and only on a peer that loads. The rule is on values: no record,
/// and a timer runs.
pub fn mirror_breakable_respawns(
    elapsed: Res<GameplayElapsed>,
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    breakables: Query<
        (Entity, &FeatureId, &BreakableFeature, Option<&RespawnTimer>),
        With<FeatureSimEntity>,
    >,
    mut schedule: ResMut<WorldTimeSchedule>,
) {
    for (entity, feature, breakable, timer) in &breakables {
        let Some(definition) = rooms.definition_of(entity) else {
            continue;
        };
        let room = &rooms.rooms().spec(definition).id;
        match (breakable.broken(), timer) {
            (true, Some(timer)) => schedule.record(room, feature.as_str(), elapsed.0 + timer.0),
            (false, _) => schedule.forget(room, feature.as_str()),
            // Broken for good (`Never`, `OnRoomReload`): no respawn to schedule.
            (true, None) => {}
        }
    }
}

/// A collected pickup authored `AfterSeconds(n)` counts its regrowth down on
/// its `RespawnTimer` (inserted at its collection, or at the rebuild of its
/// room while the regrowth is not yet due) and is collectable again when the
/// timer ends. The schedule mirrors the timer as [`mirror_breakable_respawns`]
/// mirrors a breakable's: a record when the timer runs and there is none, and
/// no record once the pickup is collectable.
///
/// ⛔ NO CHANGE-TICK GATE, for the reason the breakable mirror gives.
pub fn regrow_pickups(
    world_time: Res<ambition_time::WorldTime>,
    elapsed: Res<GameplayElapsed>,
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    mut pickups: Query<
        (Entity, &FeatureId, Has<Collected>, Option<&mut RespawnTimer>),
        (With<PickupFeature>, With<FeatureSimEntity>),
    >,
    mut schedule: ResMut<WorldTimeSchedule>,
    mut commands: Commands,
) {
    // The sim clock, as a breakable's respawn: the regrowth freezes in
    // bullet time with everything else (ADR 0010).
    let dt = world_time.sim_dt();
    for (entity, feature, collected, timer) in &mut pickups {
        let Some(definition) = rooms.definition_of(entity) else {
            continue;
        };
        let room = &rooms.rooms().spec(definition).id;
        match (collected, timer) {
            (true, Some(mut timer)) => {
                timer.0 = (timer.0 - dt).max(0.0);
                if timer.0 <= 0.0 {
                    commands.entity(entity).remove::<(Collected, RespawnTimer)>();
                    schedule.forget(room, feature.as_str());
                } else {
                    schedule.record(room, feature.as_str(), elapsed.0 + timer.0);
                }
            }
            (false, _) => schedule.forget(room, feature.as_str()),
            // Collected for good (`Never`, `OnRoomReload`): nothing to schedule.
            (true, None) => {}
        }
    }
}

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
    mut replays: MessageReader<ambition_combat::events::RoomReplayAdmitted>,
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    running: Query<Entity, (With<RespawnTimer>, With<FeatureSimEntity>)>,
    mut schedule: ResMut<WorldTimeSchedule>,
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

/// A checkpoint restore rebuilds from the save, which holds no broken
/// breakable and no collected pickup. (checkpoint reducer, in
/// `CheckpointDomainApply`)
pub fn forget_scheduled_returns_on_restore(
    inputs: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::CheckpointRestoreInputs>>,
    schedule: Option<ResMut<WorldTimeSchedule>>,
) {
    if let (Some(_), Some(mut schedule)) = (inputs, schedule) {
        schedule.forget_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A record answers how long a breakable stays broken, and nothing once
    /// it is due.
    #[test]
    fn a_record_says_how_long_a_breakable_stays_broken() {
        let mut schedule = WorldTimeSchedule::default();
        Arc::make_mut(&mut schedule.due).insert(("basement".into(), "platform".into()), 10.0);
        assert_eq!(
            [7.5, 10.0, 12.0].map(|now| schedule.remaining("basement", "platform", now)),
            [Some(2.5), None, None]
        );
        assert_eq!(schedule.remaining("hub", "platform", 7.5), None);
        schedule.forget_room("basement");
        assert_eq!(schedule.due("basement", "platform"), None);
    }

    /// A schedule of `n` dormant records, in rooms that are not live.
    fn dormant_schedule(n: usize) -> WorldTimeSchedule {
        let records = (0..n).map(|i| ((format!("room_{:03}", i / 100), format!("platform_{i:05}")), i as f32));
        WorldTimeSchedule { due: Arc::new(records.collect()) }
    }

    /// The kept checksum is the fold it stands for: for the first records,
    /// after a record changes, and after the schedule goes back to the first.
    #[test]
    fn a_kept_checksum_is_the_fold_of_the_records_it_was_kept_for() {
        let first = dormant_schedule(3);
        let mut second = first.clone();
        second.forget_room("room_000");
        Arc::make_mut(&mut second.due).insert(("room_000".into(), "platform_00001".into()), 9.0);
        for (step, schedule) in [("first", &first), ("second", &second), ("first again", &first)] {
            assert_eq!(schedule.checksum(), schedule.fold(), "the kept checksum is not the fold ({step})");
        }
        assert_ne!(first.checksum(), second.checksum(), "a changed record did not move the checksum");
    }

    /// OW3: a rollback frame clones the schedule, hashes it and compares it
    /// with the live one. With 10,000 dormant records that costs little more
    /// than with none, as for the occurrence ledger (M2).
    #[test]
    fn dormant_records_add_little_to_a_snapshot_of_the_schedule() {
        fn median_snapshot(schedule: &WorldTimeSchedule) -> std::time::Duration {
            let mut times: Vec<_> = (0..101)
                .map(|_| {
                    // AMBITION_REVIEW(determinism): wall clock, in a test. It
                    // measures the cost of a snapshot, and no simulation code
                    // reads it.
                    let start = std::time::Instant::now();
                    let snapshot = std::hint::black_box(schedule.clone());
                    std::hint::black_box(snapshot.checksum());
                    std::hint::black_box(snapshot == *schedule);
                    start.elapsed()
                })
                .collect();
            times.sort();
            times[50]
        }
        let (empty, full) = (dormant_schedule(0), dormant_schedule(10_000));
        let (empty, full) = (median_snapshot(&empty), median_snapshot(&full));
        assert!(
            full < empty * 5 + std::time::Duration::from_micros(20),
            "a snapshot of 10,000 dormant records took {full:?}, against {empty:?} with none"
        );
    }
}
