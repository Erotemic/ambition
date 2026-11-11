//! Return times and checkpoint ownership for authored world occurrences.

use std::collections::BTreeMap;
use std::sync::Arc;

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::lifecycle::{
    FeatureSimEntity, SessionWorldMut, session_world_component,
};

use ambition_characters::control::{DrivingParticipant, PlayerSlot};
use ambition_time::GameplayElapsed;

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
///
/// A component of the session root (C03), beside the clock its due times are
/// on: a new session's root is born with no record, so no reset is owed at a
/// session edge.
#[derive(Component, Clone, Debug, Default)]
pub struct WorldTimeSchedule {
    due: Arc<Records>,
    /// The records a live room wrote or forgot, with what each replaced
    /// (ISL-2d): a rewind of that room puts them back
    /// ([`Self::retract_room`]). Without it a replay that takes a pickup on
    /// its first tick keeps the mispredicted run's due time, since a record
    /// keeps the first departure's. Bookkeeping for a rewind, not a fact:
    /// not compared or hashed, and kept only within the rewind horizon.
    written: Arc<Vec<ScheduleWrite>>,
}

/// One record a live room wrote or forgot, and the record it replaced.
#[derive(Clone, Debug, PartialEq)]
struct ScheduleWrite {
    at: ambition_platformer2d_shared_tangle::lifecycle::RoomWrite,
    key: (String, String),
    replaced: Option<Due>,
}

type Records = BTreeMap<(String, String), Due>;

/// When one gone occurrence comes back, and whose horizons own that.
#[derive(Clone, Debug, PartialEq)]
struct Due {
    at: f32,
    /// The participants whose bodies were in the occurrence's live room when
    /// the record was made, in seat order. A rewind of one of them takes it
    /// out; the record goes back when none is left.
    owners: Vec<PlayerSlot>,
}

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
            .map(|due| due.at)
    }

    /// Whose horizons own the record of `feature` in room `room`.
    pub fn owners(&self, room: &str, feature: &str) -> Option<&[PlayerSlot]> {
        self.due
            .get(&(room.to_string(), feature.to_string()))
            .map(|due| due.owners.as_slice())
    }

    /// How long the occurrence `feature` of room `room` stays gone at time
    /// `now`: `Some` while its return is not yet due, `None` when it is whole
    /// or due.
    pub fn remaining(&self, room: &str, feature: &str, now: f32) -> Option<f32> {
        self.due(room, feature)
            .map(|due| due - now)
            .filter(|remaining| *remaining > 0.0)
    }

    /// The records and their due times, in key order.
    pub fn records(&self) -> impl Iterator<Item = (&(String, String), f32)> {
        self.due.iter().map(|(key, due)| (key, due.at))
    }

    /// The occurrence `feature` of room `room` comes back at `due`, owned by
    /// `owners`. Keeps an existing record: the first departure decides the
    /// time and the owners.
    pub fn record(&mut self, room: &str, feature: &str, due: f32, owners: &[PlayerSlot]) {
        self.record_from(None, room, feature, due, owners);
    }

    /// [`Self::record`], written from a live room on a tick: a rewind of that
    /// room takes the record back.
    pub fn record_from(
        &mut self,
        written: Option<ambition_platformer2d_shared_tangle::lifecycle::RoomWrite>,
        room: &str,
        feature: &str,
        due: f32,
        owners: &[PlayerSlot],
    ) {
        let key = (room.to_string(), feature.to_string());
        if !self.due.contains_key(&key) {
            Arc::make_mut(&mut self.due).insert(
                key.clone(),
                Due {
                    at: due,
                    owners: owners.to_vec(),
                },
            );
            self.log(written, key, None);
        }
    }

    fn log(
        &mut self,
        written: Option<ambition_platformer2d_shared_tangle::lifecycle::RoomWrite>,
        key: (String, String),
        replaced: Option<Due>,
    ) {
        let Some(at) = written else {
            return;
        };
        let log = Arc::make_mut(&mut self.written);
        log.retain(|write| !write.at.beyond_the_horizon(at.tick));
        log.push(ScheduleWrite { at, key, replaced });
    }

    /// `room` rewinds to `saved_at`: put back the records it wrote or forgot
    /// after that tick, newest first.
    pub fn retract_room(&mut self, room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance, saved_at: u64) {
        if !self.written.iter().any(|write| write.at.retracted_by(room, saved_at)) {
            return;
        }
        let (taken, kept): (Vec<_>, Vec<_>) =
            self.written.iter().cloned().partition(|write| write.at.retracted_by(room, saved_at));
        self.written = Arc::new(kept);
        let records = Arc::make_mut(&mut self.due);
        for write in taken.into_iter().rev() {
            match write.replaced {
                Some(due) => records.insert(write.key, due),
                None => records.remove(&write.key),
            };
        }
    }

    /// A checkpoint restore keeps only the `kept` participants' horizons: take
    /// every other participant out of each record's owners, and forget a
    /// record with no owner left.
    pub fn keep_only_owners(&mut self, kept: &[PlayerSlot]) {
        let changes = self.due.values().any(|due| {
            due.owners.is_empty() || due.owners.iter().any(|owner| !kept.contains(owner))
        });
        if !changes {
            return;
        }
        let records = Arc::make_mut(&mut self.due);
        for due in records.values_mut() {
            due.owners.retain(|owner| kept.contains(owner));
        }
        records.retain(|_, due| !due.owners.is_empty());
    }

    /// Forget every record of a room in `live`, and every record with no owner.
    pub fn forget_live_and_unowned(&mut self, live: &[String]) {
        if self
            .due
            .iter()
            .any(|((room, _), due)| live.contains(room) || due.owners.is_empty())
        {
            Arc::make_mut(&mut self.due)
                .retain(|(room, _), due| !live.contains(room) && !due.owners.is_empty());
        }
    }

    /// Whether the occurrence `feature` of room `room` has a record. A mirror
    /// asks this of each occurrence on each tick, through a read: an empty
    /// schedule (the common case) makes no key, and a tick with nothing to
    /// forget does not mark the schedule changed.
    pub fn holds(&self, room: &str, feature: &str) -> bool {
        !self.due.is_empty()
            && self
                .due
                .contains_key(&(room.to_string(), feature.to_string()))
    }

    /// Forget the record of the occurrence `feature` of room `room`.
    pub fn forget(&mut self, room: &str, feature: &str) {
        self.forget_from(None, room, feature);
    }

    /// [`Self::forget`], from a live room on a tick: a rewind of that room
    /// puts the record back.
    pub fn forget_from(
        &mut self,
        written: Option<ambition_platformer2d_shared_tangle::lifecycle::RoomWrite>,
        room: &str,
        feature: &str,
    ) {
        let key = (room.to_string(), feature.to_string());
        if self.due.contains_key(&key) {
            let replaced = Arc::make_mut(&mut self.due).remove(&key);
            self.log(written, key, replaced);
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
        if !self.written.is_empty() {
            self.written = Arc::default();
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
        let mut kept = KEPT
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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
            put_u64(&mut bytes, u64::from(due.at.to_bits()));
            put_u64(&mut bytes, due.owners.len() as u64);
            for owner in &due.owners {
                put_u64(&mut bytes, u64::from(owner.0));
            }
        }
        checksum_bytes(&bytes)
    }
}

/// The time each gone occurrence stays gone as of now, by (room definition
/// id, authored id): the records whose return is not yet due. A room commit
/// reads it (`PersistedFates::with_scheduled_returns`).
pub fn remaining_scheduled_returns(world: &World) -> BTreeMap<(String, String), f32> {
    session_world_component::<WorldTimeSchedule>(world)
        .map_or_else(BTreeMap::new, |schedule| remaining_in(schedule, world))
}

/// [`remaining_scheduled_returns`] of `schedule`, which may be a copy: the
/// schedule a commit will find after a change that has not happened yet.
pub fn remaining_in(
    schedule: &WorldTimeSchedule,
    world: &World,
) -> BTreeMap<(String, String), f32> {
    let Some(now) = session_world_component::<GameplayElapsed>(world) else {
        return BTreeMap::new();
    };
    schedule
        .due
        .iter()
        .map(|(key, due)| (key.clone(), due.at - now.0))
        .filter(|(_, remaining)| *remaining > 0.0)
        .collect()
}

/// The participants whose bodies are in the live room of `entity`, in seat
/// order: the owners of a record made for it now.
pub fn owners_beside(
    entity: Entity,
    live: &ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    participants: &Query<(Entity, &DrivingParticipant)>,
) -> Vec<PlayerSlot> {
    let Some(room) = live.of(entity) else {
        return Vec::new();
    };
    let mut owners: Vec<PlayerSlot> = participants
        .iter()
        .filter(|(body, _)| live.of(*body) == Some(room))
        .map(|(_, driver)| driver.0)
        .collect();
    owners.sort();
    owners.dedup();
    owners
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A record answers how long a breakable stays broken, and nothing once
    /// it is due.
    #[test]
    fn a_record_says_how_long_a_breakable_stays_broken() {
        let mut schedule = WorldTimeSchedule::default();
        Arc::make_mut(&mut schedule.due).insert(
            ("basement".into(), "platform".into()),
            Due {
                at: 10.0,
                owners: vec![PlayerSlot(1)],
            },
        );
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
        let records = (0..n).map(|i| {
            (
                (format!("room_{:03}", i / 100), format!("platform_{i:05}")),
                Due {
                    at: i as f32,
                    owners: vec![PlayerSlot(0)],
                },
            )
        });
        WorldTimeSchedule {
            due: Arc::new(records.collect()),
            written: Arc::default(),
        }
    }

    /// The kept checksum is the fold it stands for: for the first records,
    /// after a record changes, and after the schedule goes back to the first.
    #[test]
    fn a_kept_checksum_is_the_fold_of_the_records_it_was_kept_for() {
        let first = dormant_schedule(3);
        let mut second = first.clone();
        second.forget_room("room_000");
        Arc::make_mut(&mut second.due).insert(
            ("room_000".into(), "platform_00001".into()),
            Due {
                at: 9.0,
                owners: vec![PlayerSlot(0)],
            },
        );
        for (step, schedule) in [
            ("first", &first),
            ("second", &second),
            ("first again", &first),
        ] {
            assert_eq!(
                schedule.checksum(),
                schedule.fold(),
                "the kept checksum is not the fold ({step})"
            );
        }
        assert_ne!(
            first.checksum(),
            second.checksum(),
            "a changed record did not move the checksum"
        );
    }

    /// ISL-2d: a room's rewind puts back what its writes after the save
    /// replaced, newest first: a record it wrote goes, a record it forgot
    /// comes back. Another room's write stays, and a write beyond the rewind
    /// horizon is no longer kept.
    #[test]
    fn a_rewind_puts_back_the_records_its_room_wrote_and_forgot() {
        use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, RoomWrite, ROOM_REWIND_HORIZON_TICKS};
        let (here, there) = (LiveRoomInstance::from_ordinal(1), LiveRoomInstance::from_ordinal(2));
        let at = |room, tick| Some(RoomWrite { room, tick });
        let owners = [PlayerSlot(0)];
        let mut schedule = WorldTimeSchedule::default();
        schedule.record_from(at(here, 2), "x", "old", 9.0, &owners);
        schedule.record_from(at(here, 6), "x", "taken", 5.0, &owners);
        schedule.forget_from(at(here, 7), "x", "old");
        schedule.record_from(at(there, 8), "y", "elsewhere", 6.0, &owners);
        schedule.retract_room(here, 4);
        let due = |feature| schedule.due(if feature == "elsewhere" { "y" } else { "x" }, feature);
        assert_eq!(
            (due("old"), due("taken"), due("elsewhere")),
            (Some(9.0), None, Some(6.0)),
            "(forgotten in the window, written in the window, another room's)"
        );

        let mut far = WorldTimeSchedule::default();
        far.record_from(at(here, 1), "x", "early", 3.0, &owners);
        far.record_from(at(here, 2 + ROOM_REWIND_HORIZON_TICKS), "x", "late", 4.0, &owners);
        far.retract_room(here, 0);
        assert_eq!(far.due("x", "early"), Some(3.0), "a write beyond the horizon was still taken back");
        assert_eq!(far.due("x", "late"), None, "a write within the horizon stayed");
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

/// The time until a gone occurrence comes back: a broken breakable respawns,
/// a collected pickup regrows.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct RespawnTimer(pub f32);

mod lifecycle;
pub use lifecycle::{
    disown_scheduled_returns_on_restore, forget_scheduled_returns_on_replay,
    forget_scheduled_returns_on_restore,
};

#[cfg(test)]
mod session_tests {
    use super::*;

    /// ⭐ C03: TWO SESSION ROOTS HOLD TWO CLOCKS AND TWO SCHEDULES. Session A is
    /// live, its clock runs and it has a scheduled return. A candidate root B
    /// beside it holds neither and changes neither. After the swap the live
    /// clock is B's, counted from zero, and B has no record of A's. Before C03
    /// both were App resources, and a reset at the session edge was what kept
    /// B from reading A's.
    #[test]
    fn two_session_roots_hold_two_clocks_and_two_schedules() {
        use ambition_platformer2d_shared_tangle::lifecycle::{
            CandidateSessionRoot, SessionRoot, SessionScopeId, require_on_session_root,
            session_world_component_mut,
        };
        let dt = 1.0 / 60.0;
        let mut app = App::new();
        app.insert_resource(ambition_time::WorldTime::new(dt, dt));
        require_on_session_root::<GameplayElapsed>(&mut app);
        require_on_session_root::<WorldTimeSchedule>(&mut app);
        app.add_plugins(ambition_platformer2d_shared_tangle::lifecycle::SessionElapsedPlugin);
        let clock = |app: &App| {
            session_world_component::<GameplayElapsed>(app.world()).map(|clock| clock.0)
        };
        let records = |app: &App| {
            session_world_component::<WorldTimeSchedule>(app.world())
                .map(|schedule| schedule.records().count())
        };

        let a = app.world_mut().spawn(SessionRoot(SessionScopeId(1))).id();
        for _ in 0..3 {
            app.update();
        }
        session_world_component_mut::<WorldTimeSchedule>(app.world_mut())
            .expect("A carries a schedule")
            .record("hall", "crate", 5.0, &[PlayerSlot::PRIMARY]);
        assert_eq!(
            (clock(&app), records(&app)),
            (Some(dt + dt + dt), Some(1)),
            "precondition: A ran and scheduled"
        );

        let b = app
            .world_mut()
            .spawn(CandidateSessionRoot(SessionScopeId(2)))
            .id();
        app.update();
        assert!(
            app.world().get::<GameplayElapsed>(b).is_none()
                && app.world().get::<WorldTimeSchedule>(b).is_none(),
            "a candidate root is not a session root, so it carries no clock yet"
        );
        assert_eq!(
            (clock(&app), records(&app)),
            (Some(dt + dt + dt + dt), Some(1)),
            "preparing a candidate changed A"
        );

        app.world_mut().despawn(a);
        app.world_mut()
            .entity_mut(b)
            .remove::<CandidateSessionRoot>();
        app.world_mut()
            .entity_mut(b)
            .insert(SessionRoot(SessionScopeId(2)));
        app.update();
        assert_eq!(
            (clock(&app), records(&app)),
            (Some(dt), Some(0)),
            "B was born with A's clock or schedule: the live session counts from {:?} with {:?} record(s)",
            clock(&app),
            records(&app)
        );
    }
}

mod rollback_registration;
pub use rollback_registration::register_rollback_state;
