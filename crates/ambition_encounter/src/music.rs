//! Session-owned encounter music intent.
//!
//! Each source writes or releases its own claim. The focused encounter
//! takes precedence over base arena music, which takes precedence over
//! room music. A source must release its claim even when it stops
//! producing events or its actor despawns. Clearing a whole priority
//! tier would also erase claims that belong to other sources.
//!
//! Claims are scoped to the live room. Readers choose the strongest
//! claimed tier among participant rooms, with the primary seat breaking
//! ties. A roomless fixture uses the roomless tier.

use bevy::prelude::Component;

use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;

/// Encounter music claims per live room.
/// Claims are owner-checked; readers use the highest active tier.
#[derive(Component, Default, Debug, Clone)]
pub struct EncounterMusicRequest {
    rooms: std::collections::BTreeMap<Option<LiveRoomInstance>, RoomMusicTiers>,
}

/// Who claims the music of a room: a kind of source and which source of that
/// kind. A kind with one source (the boss music, Mary-O's star) is its kind
/// alone, from `&'static str`. A kind with many sources names each one by a
/// durable id, so two of them in one room are two candidates: two encounter
/// scripts (`MusicSource::instance`). The id must survive a rewind, so it is
/// a `SimId` and not an ECS `Entity`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MusicSource {
    kind: &'static str,
    /// Empty for a kind with one source.
    instance: String,
}

impl MusicSource {
    /// The source `instance` of the kind `kind`.
    pub fn instance(kind: &'static str, instance: impl Into<String>) -> Self {
        Self {
            kind,
            instance: instance.into(),
        }
    }

    /// The kind of this source.
    pub fn kind(&self) -> &'static str {
        self.kind
    }
}

impl From<&'static str> for MusicSource {
    fn from(kind: &'static str) -> Self {
        Self::instance(kind, String::new())
    }
}

/// The two tiers of one live room.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
struct RoomMusicTiers {
    /// The priority candidates (a focused fight, e.g. a boss), one for each
    /// source that claims this room. Any of them overrides `base_track`.
    /// Each source owns its own candidate, so a release takes out only that
    /// candidate and the others stay (review 2026-10-05, P6).
    claims: std::collections::BTreeMap<MusicSource, PriorityClaim>,
    /// Lower-priority encounter track (a wave / arena lockdown). Written every
    /// frame — `Some(track)` while in flight, `None` otherwise — so its
    /// per-frame `None` can never override a priority claim.
    base_track: Option<String>,
}

/// One source's claim on the priority tier of a room.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PriorityClaim {
    track: String,
    /// The simulation tick on which this source began to claim the room. A
    /// source that claims again keeps it, so a source that states its claim
    /// on each tick does not move ahead of the others.
    began: u64,
}

impl RoomMusicTiers {
    fn is_empty(&self) -> bool {
        self.claims.is_empty() && self.base_track.is_none()
    }

    /// The claim that plays: the one that began latest, because the newest
    /// fight is the one the player looks at. Of claims that began on one
    /// tick, the source that sorts first (by kind, then by instance). Both
    /// are values, not the order in which the systems ran.
    fn winning_claim(&self) -> Option<&PriorityClaim> {
        self.claims
            .iter()
            .max_by(|(owner_a, a), (owner_b, b)| a.began.cmp(&b.began).then(owner_b.cmp(owner_a)))
            .map(|(_, claim)| claim)
    }
}

impl EncounterMusicRequest {
    /// The winning desired track of `room`: the higher-priority tier beats the
    /// base tier, and either beats the room default (resolved downstream in
    /// the intent adapter).
    pub fn desired_track(&self, room: Option<LiveRoomInstance>) -> Option<&str> {
        let tiers = self.rooms.get(&room)?;
        tiers
            .winning_claim()
            .map(|claim| claim.track.as_str())
            .or(tiers.base_track.as_deref())
    }

    /// Claim the priority tier of `room` for `owner`, on simulation tick
    /// `now`. This is `owner`'s candidate in that room: a claim by another
    /// source does not replace it, and the claim that began latest plays
    /// (see `RoomMusicTiers::winning_claim`). A source that claims again
    /// keeps the tick it began on, also when it changes its track.
    pub fn claim_priority(
        &mut self,
        room: Option<LiveRoomInstance>,
        owner: impl Into<MusicSource>,
        track: impl Into<String>,
        now: u64,
    ) {
        let owner = owner.into();
        let track = track.into();
        let tiers = self.rooms.entry(room).or_default();
        match tiers.claims.get_mut(&owner) {
            Some(claim) => {
                if claim.track != track {
                    claim.track = track;
                }
            }
            None => {
                tiers.claims.insert(owner, PriorityClaim { track, began: now });
            }
        }
    }

    /// Release the priority tier of `room`, but only if `owner` still holds
    /// it. A source with nothing to say says nothing, rather than silencing
    /// whoever does.
    pub fn release_priority(&mut self, room: Option<LiveRoomInstance>, owner: impl Into<MusicSource>) {
        let owner = owner.into();
        self.release_priority_where(owner.kind(), |source, claimed| *source == owner && claimed == room);
    }

    /// Release each claim of a source of the kind `kind` in each room for
    /// which `release` is true. A kind that states the claims of all its
    /// sources each frame releases here the sources and rooms it has nothing
    /// to say for.
    pub fn release_priority_where(
        &mut self,
        kind: &'static str,
        mut release: impl FnMut(&MusicSource, Option<LiveRoomInstance>) -> bool,
    ) {
        for (room, tiers) in &mut self.rooms {
            tiers
                .claims
                .retain(|source, _| source.kind() != kind || !release(source, *room));
        }
        self.rooms.retain(|_, tiers| !tiers.is_empty());
    }

    /// The priority track that plays in `room`, if any, and WHO is not on
    /// offer: a caller that wants to change the tier goes through
    /// [`Self::claim_priority`] or [`Self::release_priority`] so the owner
    /// check cannot be skipped.
    pub fn priority_track(&self, room: Option<LiveRoomInstance>) -> Option<&str> {
        self.rooms.get(&room)?.winning_claim().map(|claim| claim.track.as_str())
    }

    /// The track of `owner`'s own candidate in `room`, whether or not it is
    /// the one that plays.
    pub fn claim_of(&self, room: Option<LiveRoomInstance>, owner: impl Into<MusicSource>) -> Option<&str> {
        Some(self.rooms.get(&room)?.claims.get(&owner.into())?.track.as_str())
    }

    /// Publish the BASE tier of every room: each room in `tracks` gets its
    /// track and every other room gets none. Unowned on purpose: it is
    /// rewritten every frame, and [`Self::desired_track`] ranks the priority
    /// tier above it, so a per-frame `None` here can never silence a focused
    /// fight.
    pub fn set_base_tracks(
        &mut self,
        tracks: impl IntoIterator<Item = (Option<LiveRoomInstance>, String)>,
    ) {
        for tiers in self.rooms.values_mut() {
            tiers.base_track = None;
        }
        for (room, track) in tracks {
            let tiers = self.rooms.entry(room).or_default();
            if tiers.base_track.is_none() {
                tiers.base_track = Some(track);
            }
        }
        self.rooms.retain(|_, tiers| !tiers.is_empty());
    }

    pub fn base_track(&self, room: Option<LiveRoomInstance>) -> Option<&str> {
        self.rooms.get(&room)?.base_track.as_deref()
    }

    /// Entity-free value projection, for the peer checksum: each room with a
    /// claim, its priority candidates (source, track, the tick each began)
    /// and its base track. All are in key order, so two peers with one state
    /// give one value whatever order their systems claimed in.
    ///
    /// Every claimer writes this component during play, so a presence probe
    /// (one count, whatever the claims are) could not see two peers that
    /// disagree about which music a room asks for.
    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::{checksum_bytes, put_opt_str, put_str, put_u32, put_u64, put_u8};
        let mut bytes = Vec::new();
        put_u64(&mut bytes, self.rooms.len() as u64);
        for (room, tiers) in &self.rooms {
            match room {
                Some(room) => {
                    put_u8(&mut bytes, 1);
                    put_u32(&mut bytes, room.ordinal());
                }
                None => put_u8(&mut bytes, 0),
            }
            put_u64(&mut bytes, tiers.claims.len() as u64);
            for (source, claim) in &tiers.claims {
                put_str(&mut bytes, source.kind);
                put_str(&mut bytes, &source.instance);
                put_str(&mut bytes, &claim.track);
                put_u64(&mut bytes, claim.began);
            }
            put_opt_str(&mut bytes, tiers.base_track.as_deref());
        }
        checksum_bytes(&bytes)
    }

    /// The authored priority of what `room` asks to play (Q72): 2 for a
    /// focused fight's claim (a boss), 1 for an encounter's base track, 0 for
    /// no claim, where the room's own ambient music plays.
    pub fn priority_of(&self, room: Option<LiveRoomInstance>) -> u8 {
        match self.rooms.get(&room) {
            Some(tiers) if !tiers.claims.is_empty() => 2,
            Some(tiers) if tiers.base_track.is_some() => 1,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The peer checksum sees each part of a claim, and not the order of the
    /// claims. The control: the same claims made in the other order.
    #[test]
    fn the_checksum_sees_each_part_of_a_claim_and_not_its_order() {
        let room = Some(LiveRoomInstance::ACTIVATION);
        let state = |edit: &dyn Fn(&mut EncounterMusicRequest)| {
            let mut music = EncounterMusicRequest::default();
            music.claim_priority(room, "boss", "boss_theme", 3);
            music.claim_priority(room, MusicSource::instance("script", "a"), "a_theme", 4);
            music.set_base_tracks([(room, "arena".to_string())]);
            edit(&mut music);
            music.checksum()
        };
        let base = state(&|_| {});
        let reordered = {
            let mut music = EncounterMusicRequest::default();
            music.set_base_tracks([(room, "arena".to_string())]);
            music.claim_priority(room, MusicSource::instance("script", "a"), "a_theme", 4);
            music.claim_priority(room, "boss", "boss_theme", 3);
            music.checksum()
        };
        let differs = [
            state(&|music| music.claim_priority(room, "boss", "other_theme", 9)),
            state(&|music| music.release_priority(room, "boss")),
            state(&|music| music.claim_priority(room, MusicSource::instance("script", "b"), "a_theme", 4)),
            state(&|music| music.set_base_tracks([(room, "other".to_string())])),
            state(&|music| {
                music.release_priority(room, "boss");
                music.claim_priority(room, "boss", "boss_theme", 5);
            }),
            state(&|music| music.claim_priority(room.map(|room| room.next()), "boss", "boss_theme", 3)),
        ]
        .map(|checksum| checksum != base);
        assert_eq!(
            (reordered == base, differs),
            (true, [true; 6]),
            "(the reordered claims give the same checksum, each edit gives another: \
             track, release, another instance, base track, began tick, another room)"
        );
    }

    /// A claim is kept in its own room. Two rooms; the boss owner claims the
    /// second. The first room has no fight track, and a release by another
    /// owner, or in the other room, leaves the claim. Poison: key every claim
    /// to one room and the first room plays the boss.
    #[test]
    fn a_claim_is_heard_only_in_the_room_it_was_made_in() {
        let first = Some(LiveRoomInstance::ACTIVATION.next());
        let second = first.map(LiveRoomInstance::next);
        let mut music = EncounterMusicRequest::default();
        music.claim_priority(second, "boss", "boss_theme", 0);
        music.set_base_tracks([(first, "wave_theme".to_string())]);
        assert_eq!(
            (music.desired_track(first), music.desired_track(second)),
            (Some("wave_theme"), Some("boss_theme"))
        );
        music.release_priority(first, "boss");
        music.release_priority(second, "someone_else");
        assert_eq!(music.desired_track(second), Some("boss_theme"), "the claim stays");
        music.release_priority(second, "boss");
        music.set_base_tracks([]);
        assert_eq!((music.desired_track(first), music.desired_track(second)), (None, None));
        assert!(music.rooms.is_empty(), "a room with no claim is not stored");
    }

    /// Two sources claim one room, and the later one releases. The earlier
    /// one's track plays again with no new claim (review 2026-10-05, P6).
    /// Measured before: the room played nothing, because the later claim
    /// had replaced the earlier one.
    #[test]
    fn a_release_leaves_the_claim_of_another_source_in_the_room() {
        let room = Some(LiveRoomInstance::ACTIVATION.next());
        let mut music = EncounterMusicRequest::default();
        music.claim_priority(room, "boss", "boss_theme", 10);
        music.claim_priority(room, "intro", "intro_theme", 20);
        assert_eq!(music.desired_track(room), Some("intro_theme"), "control: the later claim plays");
        music.release_priority(room, "intro");
        assert_eq!(music.desired_track(room), Some("boss_theme"));
    }

    /// Two sources that claim one room on each tick: the claim that began
    /// later plays, in whichever order the two systems run. Measured before:
    /// the source that wrote last played, so the order of the systems chose.
    /// The source that began later sorts after the other by name, so a claim
    /// that moved its tick on each claim would tie, and the name would choose
    /// the other one.
    #[test]
    fn the_claim_that_plays_does_not_depend_on_the_order_of_the_claims() {
        let room = Some(LiveRoomInstance::ACTIVATION.next());
        let played = |earlier_first: bool| {
            let mut music = EncounterMusicRequest::default();
            music.claim_priority(room, "death", "death_theme", 5);
            for now in 6..9 {
                if earlier_first {
                    music.claim_priority(room, "death", "death_theme", now);
                    music.claim_priority(room, "star", "star_theme", now);
                } else {
                    music.claim_priority(room, "star", "star_theme", now);
                    music.claim_priority(room, "death", "death_theme", now);
                }
            }
            music.desired_track(room).map(str::to_string)
        };
        assert_eq!(played(true), Some("star_theme".to_string()));
        assert_eq!(played(false), Some("star_theme".to_string()));
        // Two claims that began on one tick: the source name decides.
        let mut music = EncounterMusicRequest::default();
        music.claim_priority(room, "zeta", "z_theme", 3);
        music.claim_priority(room, "alpha", "a_theme", 3);
        assert_eq!(music.desired_track(room), Some("a_theme"));
    }
}
