//! Wave encounters cleared in a room, and their retraction when the room
//! rewinds (SIMULATION-ISLANDS ISL-2d,
//! `docs/planning/engine/island-rollback-and-compact-peer-input.md`).
//!
//! A clear writes two shared stores the room's snapshot does not hold: the
//! save's switches linked to the encounter (greened) and the quest steps
//! waiting on the clear. The encounter's own save row is a mirror of its
//! lifecycle, which the room's snapshot rewinds, so it heals by itself.

use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, RoomWrite, ROOM_REWIND_HORIZON_TICKS};
use bevy::prelude::*;

/// One clear: the room and tick, the encounter, and each linked switch with
/// the value it had before.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncounterClear {
    pub written: RoomWrite,
    pub encounter: String,
    pub switches: Vec<(String, bool)>,
}

impl EncounterClear {
    /// The quest cause of the clear: retracting it puts back the quest
    /// steps the clear moved.
    pub fn quest_cause(&self) -> String {
        format!("encounter {} cleared in room {} at tick {}", self.encounter, self.written.room.ordinal(), self.written.tick)
    }
}

/// The wave encounters cleared in rooms within the rewind horizon, in the
/// order they were cleared. The switches and the quests are shared by rooms
/// (two live rooms of one room each run an occurrence of the encounter), so a
/// rewind of one room takes back only its own clears, and not where a later
/// clear of the same encounter stands.
///
/// Rollback state with a real value: a clear changes it on a tick.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct EncounterClearsInRooms {
    clears: Vec<EncounterClear>,
}

impl EncounterClearsInRooms {
    /// Add `clear`, and drop the clears no rewind can reach any longer;
    /// those are returned, so their quest causes can be forgotten.
    pub fn record(&mut self, clear: EncounterClear) -> Vec<EncounterClear> {
        let now = clear.written.tick;
        let (old, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut self.clears)
            .into_iter()
            .partition(|kept| kept.written.tick.saturating_add(ROOM_REWIND_HORIZON_TICKS) < now);
        self.clears = kept;
        self.clears.push(clear);
        old
    }

    /// `room` rewinds to `saved_at`: take out the clears it made after that
    /// tick, newest first, each with whether a later clear of the same
    /// encounter still stands (then its switches and quest steps stay).
    pub fn retract_room(&mut self, room: LiveRoomInstance, saved_at: u64) -> Vec<(EncounterClear, bool)> {
        let mut taken = Vec::new();
        for at in (0..self.clears.len()).rev() {
            if !self.clears[at].written.retracted_by(room, saved_at) {
                continue;
            }
            let clear = self.clears.remove(at);
            let superseded = self.clears[at..].iter().any(|later| later.encounter == clear.encounter);
            taken.push((clear, superseded));
        }
        taken
    }

    /// Entity-free value projection.
    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::{checksum_bytes, put_bool, put_str, put_u64};
        let mut bytes = Vec::new();
        put_u64(&mut bytes, self.clears.len() as u64);
        for clear in &self.clears {
            put_u64(&mut bytes, u64::from(clear.written.room.ordinal()));
            put_u64(&mut bytes, clear.written.tick);
            put_str(&mut bytes, &clear.encounter);
            put_u64(&mut bytes, clear.switches.len() as u64);
            for (switch, was) in &clear.switches {
                put_str(&mut bytes, switch);
                put_bool(&mut bytes, *was);
            }
        }
        checksum_bytes(&bytes)
    }
}

/// `room` rewinds to `saved_at`: put back the switches its clears greened
/// after that tick, and the quest steps they moved.
pub fn retract_room_encounter_clears(world: &mut World, room: LiveRoomInstance, saved_at: u64) {
    let Some(taken) = world
        .get_resource_mut::<EncounterClearsInRooms>()
        .map(|mut clears| clears.retract_room(room, saved_at))
        .filter(|taken| !taken.is_empty())
    else {
        return;
    };
    world.resource_scope(|world, mut save: Mut<ambition_persistence::save::AmbitionGameSave>| {
        let mut quests = world.resource_mut::<ambition_persistence::quest::QuestRegistry>();
        for (clear, superseded) in taken {
            let cause = clear.quest_cause();
            if superseded {
                quests.forget_cause(&cause);
                continue;
            }
            for (switch, was) in clear.switches.into_iter().rev() {
                save.data_mut().set_switch(switch, was);
            }
            for quest in quests.retract_caused_by(&cause) {
                if let Some(state) = quests.get(&quest) {
                    save.data_mut().set_quest(&quest, state.progression, state.step);
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clear(room: u32, tick: u64, encounter: &str) -> EncounterClear {
        EncounterClear {
            written: RoomWrite { room: LiveRoomInstance::from_ordinal(room), tick },
            encounter: encounter.into(),
            switches: vec![(format!("{encounter}_switch"), false)],
        }
    }

    /// A rewind takes back its room's clears after the saved tick, newest
    /// first, and says which a later clear of the same encounter supersedes;
    /// a clear before the save or from another room stays.
    #[test]
    fn a_rewind_takes_back_its_rooms_later_clears() {
        let mut clears = EncounterClearsInRooms::default();
        clears.record(clear(1, 5, "a"));
        clears.record(clear(1, 12, "a"));
        clears.record(clear(1, 13, "b"));
        clears.record(clear(2, 14, "b"));
        let taken = clears.retract_room(LiveRoomInstance::from_ordinal(1), 10);
        let taken: Vec<_> = taken.iter().map(|(clear, superseded)| (clear.encounter.as_str(), clear.written.tick, *superseded)).collect();
        assert_eq!(taken, vec![("b", 13, true), ("a", 12, false)]);
        let kept: Vec<_> = clears.clears.iter().map(|clear| clear.written.tick).collect();
        assert_eq!(kept, vec![5, 14]);
    }

    /// A clear older than the horizon leaves the log when the next is recorded.
    #[test]
    fn a_clear_beyond_the_horizon_leaves_the_log() {
        let mut clears = EncounterClearsInRooms::default();
        clears.record(clear(1, 5, "a"));
        let old = clears.record(clear(1, 5 + ROOM_REWIND_HORIZON_TICKS + 1, "b"));
        assert_eq!(old.len(), 1);
        assert_eq!(clears.clears.len(), 1);
    }
}
