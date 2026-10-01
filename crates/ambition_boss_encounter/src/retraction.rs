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
//! to `Untouched`, despawns the boss's unopened reward chest, and announces
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

/// On an admitted replay, retract every boss defeat of the replay's live room
/// recorded since the last checkpoint, for every boss family.
///
/// The replay's room is its subject's live room, or the sole live room when
/// it names no subject. Each retracted placement's save row goes back to
/// `Untouched`, so the rebuild builds the boss alive and every gate that reads
/// `boss.cleared` closes. Its unopened reward chest goes. A chest that was
/// opened stays, with what it granted (a known issue, in the queue row).
pub fn retract_boss_defeats_on_replay(
    mut commands: Commands,
    // The admitted replay, not the request: a request the lifecycle refuses
    // must retract nothing.
    mut replays: MessageReader<ambition_combat::events::RoomReplayAdmitted>,
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    mut since: ResMut<BossDefeatsSinceCheckpoint>,
    mut save: ResMut<ambition_persistence::save::AmbitionGameSave>,
    chests: Query<
        (
            Entity,
            &ambition_combat::BossRewardChest,
            Option<&ambition_combat::Opened>,
        ),
        With<ambition_combat::ChestFeature>,
    >,
    mut retracted: MessageWriter<BossDefeatRetracted>,
) {
    for replay in replays.read() {
        let replayed = replay.subject.as_ref().and_then(|subject| subject.room);
        let Some(definition) = rooms.definition_named(replayed) else {
            continue;
        };
        let definition_id = rooms.rooms().spec(definition).id.clone();
        let live: Vec<LiveRoomInstance> = rooms.live_rooms().map(|(room, _)| room).collect();
        let replayed = replayed.or_else(|| (live.len() == 1).then(|| live[0]));
        for (placement, defeat) in since.take_for_replay(replayed, &definition_id, &live) {
            if crate::placement_is_cleared(save.data(), &placement) {
                save.data_mut().set_boss(
                    &placement,
                    ambition_persistence::save_data::PersistedEncounterState::Untouched,
                );
            }
            for (chest, reward, opened) in &chests {
                if reward.encounter_id == placement && opened.is_none() {
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
}
