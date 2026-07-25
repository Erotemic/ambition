//! The single encounter→audio music-intent stream.
//!
//! One session-owned `EncounterMusicRequest` component carries the desired track from every
//! encounter source with an EXPLICIT priority, so a per-frame encounter tick —
//! which writes its source every frame, including `None` when nothing of its
//! kind is in flight — can never clobber a concurrent higher-priority
//! encounter's music.
//!
//! This replaces the old split into two process resources (one for waves, one for the
//! boss fight). The split existed only to keep the per-frame `None` of the
//! lower-priority source from stomping the higher one; naming the two by their
//! priority tier on one session component expresses that ordering directly (unified
//! encounter orchestration §6: "one encounter music-intent stream with explicit
//! priority/source"). Neither field names a specific encounter kind — a boss
//! fight is just a `priority_track` writer, a wave arena a `base_track` writer.

use bevy::prelude::Component;

/// Music request from the encounter layer to the audio backend. Each source
/// writes its OWN priority tier; the music-intent adapter reads
/// [`Self::desired_track`] (priority beats base) and mirrors the winner into
/// [`Self::last_applied`].
#[derive(Component, Default, Debug, Clone)]
pub struct EncounterMusicRequest {
    /// Higher-priority encounter track (a focused fight — e.g. a boss).
    /// Overrides `base_track` while set.
    pub priority_track: Option<String>,
    /// Lower-priority encounter track (a wave / arena lockdown). Written every
    /// frame — `Some(track)` while in flight, `None` otherwise — so its
    /// per-frame `None` can never override `priority_track`.
    pub base_track: Option<String>,
    /// Who claimed [`Self::priority_track`], so a source that writes its tier
    /// every frame can release its OWN claim without cancelling somebody else's.
    ///
    /// The tier had one slot and no owner, and every writer cleared it on the
    /// frame it had nothing to say. `update_boss_encounters` runs in
    /// `Progression`, later than the demos' `GameplayEffects`, and cleared it
    /// whenever no boss was mid-fight — which is always, in a demo with no
    /// bosses. Mary-O's death music was therefore set and wiped on every frame
    /// of every death beat and never played a note. Its test passed because it
    /// ran on a bare `App` where the boss system does not exist.
    pub priority_owner: Option<&'static str>,
    /// The track id last applied by the music-intent adapter, so it can detect
    /// transitions (None ↔ Some(other) ↔ Some(other2)) and for tests.
    pub last_applied: Option<String>,
}

impl EncounterMusicRequest {
    /// The winning desired track: the higher-priority tier beats the base tier,
    /// and either beats the room default (resolved downstream in the intent
    /// adapter).
    pub fn desired_track(&self) -> Option<&str> {
        self.priority_track
            .as_deref()
            .or(self.base_track.as_deref())
    }

    /// Claim the priority tier for `owner`. A later claim wins outright — two
    /// focused fights at once is not a state worth arbitrating, and the most
    /// recent one is the one the player is looking at.
    pub fn claim_priority(&mut self, owner: &'static str, track: impl Into<String>) {
        let track = track.into();
        if self.priority_track.as_deref() != Some(track.as_str())
            || self.priority_owner != Some(owner)
        {
            self.priority_track = Some(track);
            self.priority_owner = Some(owner);
        }
    }

    /// Release the priority tier, but only if `owner` still holds it. A source
    /// with nothing to say says nothing, rather than silencing whoever does.
    pub fn release_priority(&mut self, owner: &'static str) {
        if self.priority_owner == Some(owner) {
            self.priority_track = None;
            self.priority_owner = None;
        }
    }
}
