//! BOSS-REPLAY-RETRACTION (Q51, Q56): a replay that makes a boss undefeated
//! again makes it undefeated for every boss family, and the consequences of
//! the defeat go with it.
//!
//! The checkpoint is the reset baseline (2026-08-15), and a death reset is
//! temporal per object (Q124): a defeat recorded AFTER the last committed
//! checkpoint is retracted by a replay of its room, and a defeat recorded
//! before it survives, in the same room too.
//!
//! The save's boss row holds no time, so this domain keeps the delta: the
//! placements cleared since the last checkpoint ([`BossDefeatsSinceCheckpoint`]).
//! The generic boss road records an entry at its `Cleared` edge. A checkpoint
//! commit and a fresh run forget them all. A load starts with none, because
//! the file a load reads IS the baseline. On an admitted replay,
//! [`retract_boss_defeats_on_replay`] puts each entry of the replay's room back
//! to `Untouched`, despawns the boss's reward chest, clears its looted flag,
//! puts back the quest steps its defeat advanced, and announces
//! [`BossDefeatRetracted`], so the domain that owns another consequence (the
//! item domain owns the mints) retracts it.
//!
//! ⚠ A re-fight that content asks for (cut-rope's "try again") is a second
//! road with its own owner, not an exception to this rule.

use std::collections::BTreeMap;

use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::{
    Commands, Entity, Message, MessageReader, MessageWriter, Query, Res, ResMut, Resource, With,
};

/// One boss defeat recorded since the last committed checkpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BossDefeatSinceCheckpoint {
    /// The live room the boss fell in. Two instances of one room can be
    /// live, so the instance, not the definition, says which fight it was.
    pub room: Option<LiveRoomInstance>,
    /// The id of the room definition the boss fell in: the key that remains
    /// when its instance has retired and the room is live again.
    pub definition: String,
    /// The boss's simulation identity: the parent its mints name.
    pub boss: Option<SimId>,
    /// The participants whose horizons own the defeat, in seat order: those
    /// whose bodies were in the boss's live room when it fell, less each one
    /// whose restore has since gone back past it. A death keeps a defeat that
    /// another participant owns (Q151), also after its room retired.
    pub present: Vec<ambition_characters::control::PlayerSlot>,
}

/// The boss placements the save recorded `Cleared` since the last committed
/// checkpoint, by placement id.
///
/// Rollback state with a real value: the defeat edge writes it on a tick, and
/// a rewind across that tick must take it back with the save row.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct BossDefeatsSinceCheckpoint {
    defeats: BTreeMap<String, BossDefeatSinceCheckpoint>,
}

impl BossDefeatsSinceCheckpoint {
    /// Record a defeat of placement `placement`. A later defeat of the same
    /// placement replaces the earlier one.
    pub fn record(&mut self, placement: impl Into<String>, defeat: BossDefeatSinceCheckpoint) {
        self.defeats.insert(placement.into(), defeat);
    }

    /// The defeats recorded since the last checkpoint.
    pub fn defeats(&self) -> impl Iterator<Item = (&String, &BossDefeatSinceCheckpoint)> {
        self.defeats.iter()
    }

    /// Forget every defeat: a checkpoint commit makes them part of the baseline.
    pub fn forget_all(&mut self) {
        if !self.defeats.is_empty() {
            self.defeats.clear();
        }
    }

    /// Take out the defeats a replay of live room `replayed` (an instance of
    /// definition `definition`) retracts: those that fell in `replayed`, and
    /// those that fell in an earlier instance of `definition` that is no longer
    /// live. A defeat in another live instance of the same definition is that
    /// instance's, and stays.
    pub fn take_for_replay(
        &mut self,
        replayed: Option<LiveRoomInstance>,
        definition: &str,
        live: &[LiveRoomInstance],
    ) -> Vec<(String, BossDefeatSinceCheckpoint)> {
        let retracted: Vec<String> = self
            .defeats
            .iter()
            .filter(|(_, defeat)| match (defeat.room, replayed) {
                (Some(fell_in), Some(replayed)) if fell_in == replayed => true,
                (fell_in, _) => {
                    defeat.definition == definition
                        && fell_in.is_none_or(|fell_in| !live.contains(&fell_in))
                }
            })
            .map(|(placement, _)| placement.clone())
            .collect();
        retracted
            .into_iter()
            .filter_map(|placement| {
                let defeat = self.defeats.remove(&placement)?;
                Some((placement, defeat))
            })
            .collect()
    }

    /// Take out the defeats a checkpoint restore retracts: every defeat since
    /// the checkpoint, except those that fell in a `spared` live room, and
    /// those a `spared_participants` participant won outside `dying_room`.
    /// A death is local to its participant and room (Q151): a room another
    /// participant holds keeps its defeats, and so does a room another
    /// participant won and left. The dying participant's own room spares
    /// nothing, also when another participant shares it: the restore
    /// rebuilds it.
    ///
    /// A kept defeat keeps only the spared participants as its winners: the
    /// dying participant's horizon has gone back past it. So a later restore
    /// of another participant does not keep it for a winner whose own
    /// restore already took it back.
    pub fn take_for_restore(
        &mut self,
        spared: &[LiveRoomInstance],
        spared_participants: &[ambition_characters::control::PlayerSlot],
        dying_room: Option<LiveRoomInstance>,
    ) -> Vec<(String, BossDefeatSinceCheckpoint)> {
        let (mut kept, taken): (BTreeMap<_, _>, BTreeMap<_, _>) = std::mem::take(&mut self.defeats)
            .into_iter()
            .partition(|(_, defeat)| Self::a_restore_keeps(defeat, spared, spared_participants, dying_room));
        for defeat in kept.values_mut() {
            defeat.present.retain(|seat| spared_participants.contains(seat));
        }
        self.defeats = kept;
        taken.into_iter().collect()
    }

    /// The defeats that [`Self::take_for_restore`] with the same arguments
    /// would take, without taking them: what the restore's acceptance reads
    /// to pin the bag it promises.
    pub fn retracted_by_restore<'a>(
        &'a self,
        spared: &'a [LiveRoomInstance],
        spared_participants: &'a [ambition_characters::control::PlayerSlot],
        dying_room: Option<LiveRoomInstance>,
    ) -> impl Iterator<Item = (&'a String, &'a BossDefeatSinceCheckpoint)> + 'a {
        self.defeats
            .iter()
            .filter(move |(_, defeat)| !Self::a_restore_keeps(defeat, spared, spared_participants, dying_room))
    }

    /// The one rule of what a checkpoint restore keeps (Q151): a defeat in a
    /// spared live room, or one a spared participant won outside the dying
    /// participant's own room.
    fn a_restore_keeps(
        defeat: &BossDefeatSinceCheckpoint,
        spared: &[LiveRoomInstance],
        spared_participants: &[ambition_characters::control::PlayerSlot],
        dying_room: Option<LiveRoomInstance>,
    ) -> bool {
        defeat.room.is_some_and(|room| spared.contains(&room))
            || (defeat.room != dying_room
                && defeat.present.iter().any(|seat| spared_participants.contains(seat)))
    }

    /// Entity-free value projection: two peers that disagree about which
    /// defeats a replay would retract have diverged.
    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::{checksum_bytes, put_str, put_u64};
        let mut bytes = Vec::new();
        put_u64(&mut bytes, self.defeats.len() as u64);
        for (placement, defeat) in &self.defeats {
            put_str(&mut bytes, placement);
            put_u64(
                &mut bytes,
                defeat.room.map_or(0, |room| u64::from(room.ordinal()) + 1),
            );
            put_str(&mut bytes, &defeat.definition);
            put_str(&mut bytes, defeat.boss.as_ref().map_or("", SimId::as_str));
            put_u64(&mut bytes, defeat.present.len() as u64);
            for seat in &defeat.present {
                put_u64(&mut bytes, u64::from(seat.0));
            }
        }
        checksum_bytes(&bytes)
    }
}

/// The system set of the generic boss road's replay retraction
/// ([`retract_boss_defeats_on_replay`]). A domain that retracts another
/// consequence of a defeat orders after this set, not after the system.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BossDefeatRetraction;

/// A replay retracted the defeat of boss placement `placement`. The domain
/// that owns a consequence of the defeat retracts it.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct BossDefeatRetracted {
    pub placement: String,
    /// The boss's simulation identity: the parent its mints name.
    pub boss: Option<SimId>,
}

/// A committed checkpoint makes every defeat since the last one part of the
/// baseline.
pub fn forget_boss_defeats_at_checkpoint(
    mut commits: MessageReader<ambition_platformer2d_shared_tangle::lifecycle::CheckpointCommitted>,
    mut since: ResMut<BossDefeatsSinceCheckpoint>,
) {
    // Drained unconditionally, like every other reader of this channel.
    if commits.read().count() > 0 {
        since.forget_all();
    }
}

/// A fresh run has no defeat to retract: the save it starts from has none.
/// (fresh-run reducer, in `CheckpointDomainApply`)
pub fn forget_boss_defeats_on_a_fresh_run(
    fresh: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::FreshRunRestore>>,
    since: Option<ResMut<BossDefeatsSinceCheckpoint>>,
) {
    if let (Some(_), Some(mut since)) = (fresh, since) {
        since.forget_all();
    }
}

/// Take back, in `save` and `quests`, what the defeat of `placement` wrote:
/// its cleared row goes back to `Untouched`, its looted flag is cleared, and
/// the quest steps it advanced go back, in the registry and in the save rows
/// that mirror them.
///
/// ⭐ ONE FUNCTION FOR TWO ROADS: the retraction below applies it to the live
/// save, and the prospect of a checkpoint restore applies it to a copy, to
/// build the restored room from the save the retraction will leave
/// (`session::checkpoint::prospective_commit_fates`). Two copies of these
/// edits could build one room and then leave another.
pub fn retract_defeat_records(
    save: &mut ambition_persistence::save_data::AmbitionGameSaveData,
    quests: &mut ambition_persistence::quest::QuestRegistry,
    placement: &str,
) {
    if crate::placement_is_cleared(save, placement) {
        save.set_boss(placement, ambition_persistence::save_data::PersistedEncounterState::Untouched);
    }
    // The chest was not looted either: a defeat after the replay drops it
    // closed. The item domain takes back what it gave.
    let looted = ambition_encounter::encounter_reward_looted_flag(placement);
    if save.flag(&looted) {
        save.set_flag(looted, false);
    }
    for quest in quests.retract_caused_by(placement) {
        if let Some(state) = quests.get(&quest) {
            save.set_quest(&quest, state.progression, state.step);
        }
    }
}

/// The placements whose defeats a checkpoint restore `replay` will retract,
/// read from a copy: `since` is not changed.
pub fn defeats_a_restore_retracts(
    since: &BossDefeatsSinceCheckpoint,
    replay: &ambition_combat::events::RoomReplayAdmitted,
) -> Vec<String> {
    if !replay.to_checkpoint {
        return Vec::new();
    }
    since
        .clone()
        .take_for_restore(
            &replay.spared,
            &replay.spared_participants,
            replay.subject.as_ref().and_then(|subject| subject.room),
        )
        .into_iter()
        .map(|(placement, _)| placement)
        .collect()
}

/// On an admitted replay, retract every boss defeat of the replay's live room
/// recorded since the last checkpoint, for every boss family. A checkpoint
/// restore retracts every defeat since the checkpoint (Q124, Q51), except in
/// the live rooms it spares: those another participant holds, because a death
/// is local to its participant and room (Q151). It also keeps a defeat that
/// another participant won in a room that is no longer live
/// ([`BossDefeatSinceCheckpoint::present`]), except in the dying
/// participant's own room, which the restore rebuilds. A New Game spares
/// nothing.
///
/// The replay's room is its subject's live room, or the sole live room when
/// it names no subject. Each retracted placement's save row goes back to
/// `Untouched`, so the rebuild builds the boss alive and every gate that reads
/// `boss.cleared` closes. Its reward chest goes, opened or not, its looted
/// flag is cleared, and the quest steps its defeat advanced go back.
pub fn retract_boss_defeats_on_replay(
    mut commands: Commands,
    // The admitted replay, not the request: a request the lifecycle refuses
    // must retract nothing.
    mut replays: ambition_combat::events::AdmittedReplays,
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    mut since: ResMut<BossDefeatsSinceCheckpoint>,
    mut save: ResMut<ambition_persistence::save::AmbitionGameSave>,
    mut quests: ResMut<ambition_persistence::quest::QuestRegistry>,
    chests: Query<(Entity, &ambition_combat::BossRewardChest), With<ambition_combat::ChestFeature>>,
    mut retracted: MessageWriter<BossDefeatRetracted>,
) {
    for replay in replays.read() {
        let taken = if replay.to_checkpoint {
            since.take_for_restore(
                &replay.spared,
                &replay.spared_participants,
                replay.subject.as_ref().and_then(|subject| subject.room),
            )
        } else {
            let replayed = replay.subject.as_ref().and_then(|subject| subject.room);
            let Some(definition) = rooms.definition_named(replayed) else {
                continue;
            };
            let definition_id = rooms.rooms().spec(definition).id.clone();
            let live: Vec<LiveRoomInstance> = rooms.live_rooms().map(|(room, _)| room).collect();
            let replayed = replayed.or_else(|| (live.len() == 1).then(|| live[0]));
            since.take_for_replay(replayed, &definition_id, &live)
        };
        for (placement, defeat) in taken {
            retract_defeat_records(save.data_mut(), &mut quests, &placement);
            for (chest, reward) in &chests {
                if reward.encounter_id == placement {
                    commands.entity(chest).despawn();
                }
            }
            retracted.write(BossDefeatRetracted {
                placement,
                boss: defeat.boss,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defeat(room: u32, definition: &str) -> BossDefeatSinceCheckpoint {
        BossDefeatSinceCheckpoint {
            room: Some(LiveRoomInstance::from_ordinal(room)),
            definition: definition.into(),
            boss: Some(SimId::placement(&format!("boss_in_{room}"))),
            present: Vec::new(),
        }
    }

    /// A replay of a live room takes the defeats that fell in it and the
    /// defeats of an earlier visit to the same room, and leaves a defeat that
    /// another live instance of the same room holds, and a defeat in another
    /// room.
    #[test]
    fn a_replay_takes_the_defeats_of_its_own_room_and_its_earlier_visits() {
        let mut since = BossDefeatsSinceCheckpoint::default();
        since.record("here", defeat(4, "arena"));
        since.record("earlier_visit", defeat(1, "arena"));
        since.record("other_instance", defeat(5, "arena"));
        since.record("other_room", defeat(6, "hall"));
        let live = [4, 5, 6].map(LiveRoomInstance::from_ordinal);
        let taken: Vec<String> = since
            .take_for_replay(Some(LiveRoomInstance::from_ordinal(4)), "arena", &live)
            .into_iter()
            .map(|(placement, _)| placement)
            .collect();
        assert_eq!(taken, vec!["earlier_visit".to_string(), "here".to_string()]);
        let kept: Vec<&String> = since.defeats().map(|(placement, _)| placement).collect();
        assert_eq!(kept, vec!["other_instance", "other_room"]);
    }

    /// A restore takes every defeat but those in a spared room, and a defeat
    /// with no live room (a fixture world) is never spared. With nothing
    /// spared (a New Game) it takes them all.
    #[test]
    fn a_restore_takes_every_defeat_but_those_in_a_spared_room() {
        let mut since = BossDefeatsSinceCheckpoint::default();
        since.record("alices", defeat(4, "arena"));
        since.record("bobs", defeat(5, "hall"));
        since.record("no_room", BossDefeatSinceCheckpoint {
            room: None,
            ..defeat(0, "arena")
        });
        let mut whole = since.clone();
        let taken: Vec<String> = since
            .take_for_restore(&[LiveRoomInstance::from_ordinal(5)], &[], None)
            .into_iter()
            .map(|(placement, _)| placement)
            .collect();
        assert_eq!(taken, vec!["alices".to_string(), "no_room".to_string()]);
        let kept: Vec<&String> = since.defeats().map(|(placement, _)| placement).collect();
        assert_eq!(kept, vec!["bobs"]);
        // Control: nothing spared takes Bob's defeat as well.
        assert_eq!(whole.take_for_restore(&[], &[], None).len(), 3);
        assert_eq!(whole.defeats().count(), 0);
    }

    /// Q151 for a room that is no longer live: a defeat Bob won in a room he
    /// has left stays when Alice dies. The controls: a defeat Alice won alone
    /// goes back, and a defeat Bob shared in Alice's own room goes back too,
    /// because her restore rebuilds that room.
    #[test]
    fn a_restore_keeps_a_defeat_another_participant_won_in_a_room_left() {
        use ambition_characters::control::PlayerSlot;
        let (alice, bob) = (PlayerSlot(0), PlayerSlot(1));
        let won_by = |room: u32, seats: &[PlayerSlot]| BossDefeatSinceCheckpoint {
            present: seats.to_vec(),
            ..defeat(room, "arena")
        };
        let mut since = BossDefeatsSinceCheckpoint::default();
        since.record("bobs_left", won_by(2, &[bob]));
        since.record("alices_left", won_by(3, &[alice]));
        since.record("shared_in_alices", won_by(4, &[alice, bob]));
        let taken: Vec<String> = since
            .take_for_restore(&[], &[bob], Some(LiveRoomInstance::from_ordinal(4)))
            .into_iter()
            .map(|(placement, _)| placement)
            .collect();
        let kept: Vec<&String> = since.defeats().map(|(placement, _)| placement).collect();
        assert_eq!(
            (taken, kept),
            (
                vec!["alices_left".to_string(), "shared_in_alices".to_string()],
                vec![&"bobs_left".to_string()],
            ),
            "(taken, kept) by Alice's death in live room #4, sparing Bob"
        );
    }

    /// Q151 across successive rewinds: a restore takes the dying participant
    /// out of each kept defeat's winners. A defeat Alice and Bob won together
    /// in a room both left stays when Bob's restore spares Alice, and then
    /// goes back when Alice's restore spares Bob: Bob's horizon has already
    /// gone back past it. Today only the primary participant's death
    /// restores, so this is the arithmetic a second participant's restore
    /// needs. The control is a defeat Bob won alone, which Alice's restore
    /// keeps.
    #[test]
    fn a_restore_takes_the_dying_participant_out_of_a_kept_defeats_winners() {
        use ambition_characters::control::PlayerSlot;
        let (alice, bob) = (PlayerSlot(0), PlayerSlot(1));
        let won_by = |room: u32, seats: &[PlayerSlot]| BossDefeatSinceCheckpoint {
            present: seats.to_vec(),
            ..defeat(room, "arena")
        };
        let mut since = BossDefeatsSinceCheckpoint::default();
        since.record("shared_left", won_by(2, &[alice, bob]));
        since.record("bobs_left", won_by(3, &[bob]));
        // Bob dies in live room #6, and Alice is spared.
        let first: Vec<String> = since
            .take_for_restore(&[], &[alice], Some(LiveRoomInstance::from_ordinal(6)))
            .into_iter()
            .map(|(placement, _)| placement)
            .collect();
        assert_eq!(first, vec!["bobs_left".to_string()], "Bob's restore took back his own defeat");
        // Alice dies in live room #7, and Bob is spared.
        let second: Vec<String> = since
            .take_for_restore(&[], &[bob], Some(LiveRoomInstance::from_ordinal(7)))
            .into_iter()
            .map(|(placement, _)| placement)
            .collect();
        assert_eq!(
            second,
            vec!["shared_left".to_string()],
            "Alice's restore kept a shared defeat for Bob, whose own restore took it back"
        );
        // Control: a defeat Bob won alone after both restores stays for him.
        since.record("bobs_later", won_by(3, &[bob]));
        assert!(
            since.take_for_restore(&[], &[bob], Some(LiveRoomInstance::from_ordinal(7))).is_empty(),
            "control: Alice's restore took back a defeat Bob won alone"
        );
    }
}
