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
//! compiler's rule rather than a habit.** EIGHT production files write this
//! component and it is the largest multi-writer population in the session-world
//! census. MEASURED with comments stripped before the change: six of the eight
//! touched `claim_priority`/`release_priority` and NOTHING else, one wrote only
//! the base tier, and one wrote only `last_applied` — a perfect separation held
//! entirely by convention over `pub` fields. ⇒ The discipline was already
//! universal; what was missing was anything to keep it that way. The priority
//! tier can now only be reached through the owner check.

use bevy::prelude::Component;

/// Music request from the encounter layer to the audio backend. Each source
/// writes its OWN priority tier; the music-intent adapter reads
/// [`Self::desired_track`] (priority beats base) and mirrors the winner into
/// [`Self::last_applied`].
#[derive(Component, Default, Debug, Clone)]
pub struct EncounterMusicRequest {
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
    /// The track id last applied by the music-intent adapter, so it can detect
    /// transitions (None ↔ Some(other) ↔ Some(other2)) and for tests.
    last_applied: Option<String>,
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

    /// The claimed priority track, if any, and WHO is not on offer: a caller
    /// that wants to change the tier goes through [`Self::claim_priority`] or
    /// [`Self::release_priority`] so the owner check cannot be skipped.
    pub fn priority_track(&self) -> Option<&str> {
        self.priority_track.as_deref()
    }

    /// Publish the BASE tier. Unowned on purpose: it is rewritten every frame,
    /// including `None`, and [`Self::desired_track`] ranks the priority tier
    /// above it, so a per-frame `None` here can never silence a focused fight.
    pub fn set_base_track(&mut self, track: Option<String>) {
        self.base_track = track;
    }

    pub fn base_track(&self) -> Option<&str> {
        self.base_track.as_deref()
    }

    /// The music-intent adapter's mirror of the winner it actually applied.
    /// Diagnostics and transition detection read it; nothing else writes it.
    pub fn mark_applied(&mut self, track: Option<String>) {
        self.last_applied = track;
    }

    pub fn last_applied(&self) -> Option<&str> {
        self.last_applied.as_deref()
    }
}
