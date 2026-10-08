//! The single encounter→audio music-intent stream.
//!
//! One session-owned `EncounterMusicRequest` component carries the desired track from every
//! encounter source with an EXPLICIT priority, so a per-frame encounter tick —
//! which writes its source every frame, including `None` when nothing of its
//! kind is in flight — can never clobber a concurrent higher-priority
//! encounter's music.
//!
//! The two slots express priority directly rather than naming encounter kinds:
//! a focused encounter writes `priority_track`, while a lower-priority arena
//! writes `base_track`.
//!
//! # ⛔⛔ THE RULE EVERY SOURCE MUST OBEY, and three shipped bugs that broke it
//!
//! **A source's CLEARING path must be reachable on a frame when the source has
//! nothing to say.** Not merely present — REACHABLE. All three failures below
//! shipped with a clearing path sitting right beside the writing one, and in each
//! the clear could not run at the moment it was needed. Because `desired_track`
//! ranks this stream ABOVE room music, a stale value does not linger quietly: it
//! plays the wrong track in every room the player enters.
//!
//! 1. **A one-shot claimed and nothing released.** `CUT_ROPE_MUSIC_OWNER` was
//!    claimed inside a room-REPLAY handler — and a death is a replay. Its only
//!    release was the `None` arm of that same one-shot, so leaving the room kept
//!    the boss's intro. Fixed with a system that has no run condition.
//! 2. **An EFFECT fires once; a DESPAWN fires nothing.** `SCRIPT_MUSIC_OWNER`'s
//!    release was reachable only while a live `EncounterScript` emitted
//!    `SetMusic(None)`. An encounter that ended without that beat, or simply
//!    despawned, took its claim with it.
//! 3. **A guard the write did not need sat above it.** `base_track`'s write —
//!    documented right here as happening every frame including `None` — sat below
//!    `if player_body_q.is_empty() { return; }`. A death and a room transition are
//!    both frames with no player body, and both are when an encounter stops being
//!    in flight, so the track latched.
//!
//! ⭐ **THE CHECK, before adding a source:** name the frame on which your source
//! goes quiet, and prove the clearing write runs on it. `ambition_boss_encounter`'s
//! `BOSS_MUSIC_OWNER` states the shape to copy — *"this system has no run
//! condition, so it reaches the 'no boss is fighting' arm on every frame of every
//! game."*
//!
//! ⚠ **And release only your OWN claim.** [`Self::release_priority`] is
//! owner-checked for this reason: clearing the tier outright silences whoever
//! legitimately holds it, which that crate's comments record having shipped once.
//!
//! ⛤ **AND THE FIELDS ARE PRIVATE SINCE 2026-09-18, so the owner check is the
//! compiler's rule rather than a habit.** NINE production files write this
//! component and it is the largest multi-writer population in the session-world
//! census. MEASURED with comments stripped before the change: six of the nine
//! touched `claim_priority`/`release_priority` and NOTHING else, one wrote only
//! the base tier, one wrote only `last_applied` (the presentation adapter's
//! mirror, which nothing read; deleted 2026-09-24), and one is the session reset
//! clearing it at a boundary — a perfect separation held entirely by convention
//! over `pub` fields. ⚠ THIS SAID EIGHT UNTIL 2026-09-18 and the number was a
//! receipt from a blind instrument: the reset writes through
//! `session_world_component_mut`, a spelling the census did not read at the time,
//! so the ninth file was counted by neither. ⇒ The discipline was already
//! universal; what was missing was anything to keep it that way. The priority
//! tier can now only be reached through the owner check.

use bevy::prelude::Component;

use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;

/// Music request from the encounter layer to the audio backend. Each source
/// writes its OWN priority tier; the music-intent adapter reads
/// [`Self::desired_track`] (priority beats base) and writes nothing back.
///
/// ⭐ THE TIERS ARE KEPT PER LIVE ROOM (customer 2). A fight claims the tier of
/// the room it is fought in, and the music intent reads the tier of the room
/// it plays for: of the participants' rooms, the one with the highest
/// [`Self::priority_of`], the primary seat's on a tie (Q150, Q72). So Bob's
/// boss in `switch_lab` outranks Alice's ambient music in the hub, and an
/// encounter in her room does not lose to his ambient music. The `None` room
/// is the world of a
/// composition with no live room (a fixture): its writers and its reader name
/// no room, so they meet there. Only rooms with a claim are stored.
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
