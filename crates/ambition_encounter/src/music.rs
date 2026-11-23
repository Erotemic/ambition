//! Music claims owned by their source and live room.

use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
use bevy::prelude::Component;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Component, Default, Debug, Clone)]
pub struct EncounterMusicRequest {
    rooms: BTreeMap<Option<LiveRoomInstance>, BTreeMap<MusicSource, SourceClaim>>,
}

/// A stable source identity. The instance is a simulation identity, never an entity handle.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MusicSource {
    kind: &'static str,
    instance: String,
}

impl MusicSource {
    pub fn instance(kind: &'static str, instance: impl Into<String>) -> Self {
        Self {
            kind,
            instance: instance.into(),
        }
    }

    pub fn kind(&self) -> &'static str {
        self.kind
    }
    pub fn instance_id(&self) -> &str {
        &self.instance
    }
}

impl From<&'static str> for MusicSource {
    fn from(kind: &'static str) -> Self {
        Self::instance(kind, String::new())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MusicClaimTier {
    Base = 1,
    Focused = 2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceClaim {
    track: Option<String>,
    tier: MusicClaimTier,
    began: u64,
    attempt: u64,
}

/// One live occurrence published by a domain owner.
#[derive(Debug, Clone)]
pub struct MusicOccurrence {
    pub room: Option<LiveRoomInstance>,
    pub source_id: String,
    pub attempt: u64,
    pub track: Option<String>,
}

/// The complete candidate before presentation selects a musical program.
#[derive(Debug, Clone, Copy)]
pub struct MusicRequestCandidate<'a> {
    pub room: Option<LiveRoomInstance>,
    pub source: &'a MusicSource,
    pub track: Option<&'a str>,
    pub tier: MusicClaimTier,
    pub began: u64,
    pub attempt: u64,
}

impl EncounterMusicRequest {
    fn claim(
        &mut self,
        room: Option<LiveRoomInstance>,
        source: MusicSource,
        track: Option<String>,
        tier: MusicClaimTier,
        attempt: Option<u64>,
        now: u64,
    ) {
        let claims = self.rooms.entry(room).or_default();
        if let Some(claim) = claims.get_mut(&source) {
            if attempt.is_some_and(|attempt| attempt != claim.attempt) {
                claim.attempt = attempt.expect("a new attempt is present");
                claim.began = now;
            }
            claim.track = track;
            claim.tier = tier;
        } else {
            claims.insert(
                source,
                SourceClaim {
                    track,
                    tier,
                    began: now,
                    attempt: attempt.unwrap_or(now),
                },
            );
        }
    }

    pub fn claim_priority(
        &mut self,
        room: Option<LiveRoomInstance>,
        owner: impl Into<MusicSource>,
        track: impl Into<String>,
        now: u64,
    ) {
        self.claim(
            room,
            owner.into(),
            Some(track.into()),
            MusicClaimTier::Focused,
            None,
            now,
        );
    }

    pub fn claim_base(
        &mut self,
        room: Option<LiveRoomInstance>,
        owner: impl Into<MusicSource>,
        track: impl Into<String>,
        now: u64,
    ) {
        self.claim(
            room,
            owner.into(),
            Some(track.into()),
            MusicClaimTier::Base,
            None,
            now,
        );
    }

    /// Replace this domain's occurrence claims. Other domains keep their claims.
    pub fn publish_occurrences(
        &mut self,
        kind: &'static str,
        tier: MusicClaimTier,
        now: u64,
        occurrences: impl IntoIterator<Item = MusicOccurrence>,
    ) {
        let mut active = BTreeSet::new();
        for occurrence in occurrences {
            let source = MusicSource::instance(kind, occurrence.source_id);
            active.insert((occurrence.room, source.clone()));
            self.claim(
                occurrence.room,
                source,
                occurrence.track,
                tier,
                Some(occurrence.attempt),
                now,
            );
        }
        self.release_priority_where(kind, |source, room| {
            !active.contains(&(room, source.clone()))
        });
    }

    pub fn candidates(&self) -> impl Iterator<Item = MusicRequestCandidate<'_>> {
        self.rooms.iter().flat_map(|(room, claims)| {
            claims
                .iter()
                .map(move |(source, claim)| MusicRequestCandidate {
                    room: *room,
                    source,
                    track: claim.track.as_deref(),
                    tier: claim.tier,
                    began: claim.began,
                    attempt: claim.attempt,
                })
        })
    }

    fn winning_track(
        &self,
        room: Option<LiveRoomInstance>,
        tier: Option<MusicClaimTier>,
    ) -> Option<&str> {
        self.candidates()
            .filter(|candidate| {
                candidate.room == room
                    && tier.is_none_or(|tier| candidate.tier == tier)
                    && candidate.track.is_some()
            })
            .max_by(|a, b| {
                (a.tier, a.began)
                    .cmp(&(b.tier, b.began))
                    .then_with(|| b.source.cmp(a.source))
            })
            .and_then(|candidate| candidate.track)
    }

    /// A track projection for simulation consumers. Program selection reads all candidates.
    pub fn desired_track(&self, room: Option<LiveRoomInstance>) -> Option<&str> {
        self.winning_track(room, None)
    }

    pub fn priority_track(&self, room: Option<LiveRoomInstance>) -> Option<&str> {
        self.winning_track(room, Some(MusicClaimTier::Focused))
    }

    pub fn base_track(&self, room: Option<LiveRoomInstance>) -> Option<&str> {
        self.winning_track(room, Some(MusicClaimTier::Base))
    }

    pub fn claim_of(
        &self,
        room: Option<LiveRoomInstance>,
        owner: impl Into<MusicSource>,
    ) -> Option<&str> {
        self.rooms.get(&room)?.get(&owner.into())?.track.as_deref()
    }

    pub fn release_priority(
        &mut self,
        room: Option<LiveRoomInstance>,
        owner: impl Into<MusicSource>,
    ) {
        let owner = owner.into();
        self.release_priority_where(owner.kind(), |source, claimed| {
            *source == owner && claimed == room
        });
    }

    pub fn release_priority_where(
        &mut self,
        kind: &'static str,
        mut release: impl FnMut(&MusicSource, Option<LiveRoomInstance>) -> bool,
    ) {
        for (room, claims) in &mut self.rooms {
            claims.retain(|source, _| source.kind() != kind || !release(source, *room));
        }
        self.rooms.retain(|_, claims| !claims.is_empty());
    }

    pub fn holds_claims_of_kind(&self, kind: &str) -> bool {
        self.candidates()
            .any(|candidate| candidate.source.kind() == kind)
    }

    pub fn priority_of(&self, room: Option<LiveRoomInstance>) -> u8 {
        self.candidates()
            .filter(|candidate| candidate.room == room)
            .map(|candidate| candidate.tier as u8)
            .max()
            .unwrap_or(0)
    }

    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::{
            checksum_bytes, put_opt_str, put_str, put_u32, put_u64, put_u8,
        };
        let mut bytes = Vec::new();
        put_u64(&mut bytes, self.rooms.len() as u64);
        for (room, claims) in &self.rooms {
            match room {
                Some(room) => {
                    put_u8(&mut bytes, 1);
                    put_u32(&mut bytes, room.ordinal());
                }
                None => put_u8(&mut bytes, 0),
            }
            put_u64(&mut bytes, claims.len() as u64);
            for (source, claim) in claims {
                put_str(&mut bytes, source.kind());
                put_str(&mut bytes, source.instance_id());
                put_u8(&mut bytes, claim.tier as u8);
                put_u64(&mut bytes, claim.attempt);
                put_u64(&mut bytes, claim.began);
                put_opt_str(&mut bytes, claim.track.as_deref());
            }
        }
        checksum_bytes(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The peer checksum sees each part of a claim, and not the order of the
    /// claims. The control: the same claims made in the other order.
    #[test]
    fn occurrence_publication_keeps_each_source_and_its_attempt() {
        let room = Some(LiveRoomInstance::ACTIVATION);
        let rows = |reverse: bool, attempt: u64| {
            let mut rows = vec![
                MusicOccurrence {
                    room,
                    source_id: "a".into(),
                    attempt,
                    track: Some("first".into()),
                },
                MusicOccurrence {
                    room,
                    source_id: "b".into(),
                    attempt: 1,
                    track: Some("second".into()),
                },
            ];
            if reverse {
                rows.reverse();
            }
            rows
        };
        let publish = |reverse| {
            let mut request = EncounterMusicRequest::default();
            request.publish_occurrences("boss", MusicClaimTier::Focused, 5, rows(reverse, 1));
            request
        };
        let mut request = publish(false);
        assert_eq!(request.candidates().count(), 2);
        assert_eq!(request.desired_track(room), Some("first"));
        assert_eq!(request.checksum(), publish(true).checksum());
        request.publish_occurrences("boss", MusicClaimTier::Focused, 9, rows(false, 1));
        assert!(request.candidates().all(|candidate| candidate.began == 5));
        let previous = request.checksum();
        request.publish_occurrences("boss", MusicClaimTier::Focused, 10, rows(false, 2));
        let restarted = request
            .candidates()
            .find(|candidate| candidate.source.instance_id() == "a")
            .unwrap();
        assert_eq!((restarted.attempt, restarted.began), (2, 10));
        assert_ne!(request.checksum(), previous);
        request.claim_priority(room, "scene", "scene_track", 11);
        request.publish_occurrences("boss", MusicClaimTier::Focused, 12, []);
        assert_eq!(request.candidates().count(), 1);
        assert_eq!(request.desired_track(room), Some("scene_track"));
    }

    #[test]
    fn an_adaptive_only_source_has_identity_and_priority_without_a_track() {
        let room = Some(LiveRoomInstance::ACTIVATION);
        let mut request = EncounterMusicRequest::default();
        request.publish_occurrences(
            "boss",
            MusicClaimTier::Focused,
            5,
            [MusicOccurrence {
                room,
                source_id: "placement:wolf".into(),
                attempt: 3,
                track: None,
            }],
        );
        let source = request.candidates().next().unwrap();
        assert_eq!(
            (source.source.instance_id(), source.attempt, source.track),
            ("placement:wolf", 3, None)
        );
        assert_eq!(request.priority_of(room), 2);
        assert_eq!(request.desired_track(room), None);
    }

    #[test]
    fn the_checksum_sees_each_part_of_a_claim_and_not_its_order() {
        let room = Some(LiveRoomInstance::ACTIVATION);
        let state = |edit: &dyn Fn(&mut EncounterMusicRequest)| {
            let mut music = EncounterMusicRequest::default();
            music.claim_priority(room, "boss", "boss_theme", 3);
            music.claim_priority(room, MusicSource::instance("script", "a"), "a_theme", 4);
            music.claim_base(room, "arena", "arena", 0);
            edit(&mut music);
            music.checksum()
        };
        let base = state(&|_| {});
        let reordered = {
            let mut music = EncounterMusicRequest::default();
            music.claim_base(room, "arena", "arena", 0);
            music.claim_priority(room, MusicSource::instance("script", "a"), "a_theme", 4);
            music.claim_priority(room, "boss", "boss_theme", 3);
            music.checksum()
        };
        let differs = [
            state(&|music| music.claim_priority(room, "boss", "other_theme", 9)),
            state(&|music| music.release_priority(room, "boss")),
            state(&|music| {
                music.claim_priority(room, MusicSource::instance("script", "b"), "a_theme", 4)
            }),
            state(&|music| music.claim_base(room, "arena", "other", 0)),
            state(&|music| {
                music.release_priority(room, "boss");
                music.claim_priority(room, "boss", "boss_theme", 5);
            }),
            state(&|music| {
                music.claim_priority(room.map(|room| room.next()), "boss", "boss_theme", 3)
            }),
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
        music.claim_base(first, "arena", "wave_theme", 0);
        assert_eq!(
            (music.desired_track(first), music.desired_track(second)),
            (Some("wave_theme"), Some("boss_theme"))
        );
        music.release_priority(first, "boss");
        music.release_priority(second, "someone_else");
        assert_eq!(
            music.desired_track(second),
            Some("boss_theme"),
            "the claim stays"
        );
        music.release_priority(second, "boss");
        music.release_priority(first, "arena");
        assert_eq!(
            (music.desired_track(first), music.desired_track(second)),
            (None, None)
        );
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
        assert_eq!(
            music.desired_track(room),
            Some("intro_theme"),
            "control: the later claim plays"
        );
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
