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
}
