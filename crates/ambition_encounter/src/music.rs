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
/// it plays for (the primary seat's, Q150). So Bob's boss in `switch_lab` does
/// not take the music from Alice in the hub. The `None` room is the world of a
/// composition with no live room (a fixture): its writers and its reader name
/// no room, so they meet there. Only rooms with a claim are stored.
#[derive(Component, Default, Debug, Clone)]
pub struct EncounterMusicRequest {
    rooms: std::collections::BTreeMap<Option<LiveRoomInstance>, RoomMusicTiers>,
}

/// The two tiers of one live room.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
struct RoomMusicTiers {
    /// Higher-priority encounter track (a focused fight — e.g. a boss).
    /// Overrides `base_track` while set.
    priority_track: Option<String>,
    /// Lower-priority encounter track (a wave / arena lockdown). Written every
    /// frame — `Some(track)` while in flight, `None` otherwise — so its
    /// per-frame `None` can never override `priority_track`.
    base_track: Option<String>,
    /// Who claimed [`Self::priority_track`], so a source can release only its
    /// own claim without cancelling another writer's higher-priority request.
    priority_owner: Option<&'static str>,
}

impl RoomMusicTiers {
    fn is_empty(&self) -> bool {
        self.priority_track.is_none() && self.base_track.is_none()
    }
}

impl EncounterMusicRequest {
    /// The winning desired track of `room`: the higher-priority tier beats the
    /// base tier, and either beats the room default (resolved downstream in
    /// the intent adapter).
    pub fn desired_track(&self, room: Option<LiveRoomInstance>) -> Option<&str> {
        let tiers = self.rooms.get(&room)?;
        tiers.priority_track.as_deref().or(tiers.base_track.as_deref())
    }

    /// Claim the priority tier of `room` for `owner`. A later claim wins
    /// outright — two focused fights at once in one room is not a state worth
    /// arbitrating, and the most recent one is the one the player is looking
    /// at.
    pub fn claim_priority(
        &mut self,
        room: Option<LiveRoomInstance>,
        owner: &'static str,
        track: impl Into<String>,
    ) {
        let track = track.into();
        let tiers = self.rooms.entry(room).or_default();
        if tiers.priority_track.as_deref() != Some(track.as_str())
            || tiers.priority_owner != Some(owner)
        {
            tiers.priority_track = Some(track);
            tiers.priority_owner = Some(owner);
        }
    }

    /// Release the priority tier of `room`, but only if `owner` still holds
    /// it. A source with nothing to say says nothing, rather than silencing
    /// whoever does.
    pub fn release_priority(&mut self, room: Option<LiveRoomInstance>, owner: &'static str) {
        self.release_priority_where(owner, |claimed| claimed == room);
    }

    /// Release `owner`'s claim in every room for which `release` is true. A
    /// source that states its claims for every room each frame releases
    /// the rooms it has nothing to say for here.
    pub fn release_priority_where(
        &mut self,
        owner: &'static str,
        mut release: impl FnMut(Option<LiveRoomInstance>) -> bool,
    ) {
        for (room, tiers) in &mut self.rooms {
            if tiers.priority_owner == Some(owner) && release(*room) {
                tiers.priority_track = None;
                tiers.priority_owner = None;
            }
        }
        self.rooms.retain(|_, tiers| !tiers.is_empty());
    }

    /// The claimed priority track of `room`, if any, and WHO is not on offer:
    /// a caller that wants to change the tier goes through
    /// [`Self::claim_priority`] or [`Self::release_priority`] so the owner
    /// check cannot be skipped.
    pub fn priority_track(&self, room: Option<LiveRoomInstance>) -> Option<&str> {
        self.rooms.get(&room)?.priority_track.as_deref()
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
        music.claim_priority(second, "boss", "boss_theme");
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
}
