//! **Active session audio authority** — which provider's authored music/SFX
//! are live RIGHT NOW.
//!
//! [`crate::catalog::AudioCatalogRegistry`] is App-local *storage*: every
//! linked provider's authored fragments, immutable after registration. This
//! resource is the *selection*: the one provider whose registries currently
//! own playback. The two are deliberately separate authorities — storage
//! outlives sessions (cached assets may persist), selection does not.
//!
//! Who writes it:
//! - a session-routed host's shell bridge selects on gameplay-session
//!   activation and clears on retirement (`ambition_game_shell::session_audio`);
//! - a direct-entry host (no launcher) selects its sole provider statically at
//!   composition time.
//!
//! Consumers (the music-intent resolver, playback drivers) read the selection
//! and treat `None` as deliberate silence: no session, or a provider that
//! registered no audio. They never fall back to "whichever registry happens to
//! be resident" — that would resurrect first-install-wins authority.

use std::collections::BTreeSet;

use ambition_sfx::SfxId;
use bevy::prelude::Resource;

use crate::spec::{MusicRegistry, SfxRegistry};

/// Provider-relative playback authority for one frame of music intent.
///
/// The music director enforces this: a track id may drive the base channel only
/// when the active provider authorizes it. This is precisely what stops a track
/// that merely EXISTS in the process-wide combined asset library from being
/// playable by a session whose provider never authored it — the combined library
/// is *storage*, this is *permission*.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum MusicAuthority {
    /// No gameplay session governs music this frame (frontend/title, or a
    /// direct-entry host before selection). The director must not change base
    /// playback — the host's frontend policy owns silence/title there.
    #[default]
    Ungoverned,
    /// A session is live; only these track ids (base channel) and cue ids
    /// (adaptive layers) may play. BOTH empty is a provider that authored no
    /// music — DELIBERATE silence, not "retain whatever is playing." That
    /// distinction is the fix for the ambiguity where an empty candidate list
    /// meant "leave the previous track running."
    Governed {
        authorized: BTreeSet<String>,
        /// Adaptive cue ids this provider authored. An adaptive cue that merely
        /// exists in the process-wide `MusicCueCatalog` but is foreign to the
        /// active provider cannot start — the exact adaptive analogue of the
        /// simple-track filter.
        authorized_cues: BTreeSet<String>,
    },
}

impl MusicAuthority {
    /// A governed authority permitting exactly `authorized` simple tracks and no
    /// adaptive cues. Cues are added separately by the intent resolver
    /// ([`Self::authorize_cues`]), so the selection layer (which knows only
    /// track ids) constructs this and the content layer folds in the cue ids.
    pub fn governed(authorized: impl IntoIterator<Item = String>) -> Self {
        Self::Governed {
            authorized: authorized.into_iter().collect(),
            authorized_cues: BTreeSet::new(),
        }
    }

    /// Add authorized adaptive cue ids to a governed authority (no-op when
    /// ungoverned). Used by `compute_music_intent` to project the active
    /// provider's cue ids onto the authority the director enforces.
    pub fn authorize_cues(&mut self, cues: impl IntoIterator<Item = String>) {
        if let Self::Governed {
            authorized_cues, ..
        } = self
        {
            authorized_cues.extend(cues);
        }
    }

    /// True when `track_id` is allowed to drive the base channel this frame.
    pub fn allows(&self, track_id: &str) -> bool {
        match self {
            Self::Ungoverned => true,
            Self::Governed { authorized, .. } => authorized.contains(track_id),
        }
    }

    /// True when `cue_id` is allowed to drive an adaptive layer this frame.
    pub fn allows_cue(&self, cue_id: &str) -> bool {
        match self {
            Self::Ungoverned => true,
            Self::Governed {
                authorized_cues, ..
            } => authorized_cues.contains(cue_id),
        }
    }

    /// True when the active provider authored no music at all — no simple tracks
    /// AND no adaptive cues — so the director must stop playback rather than
    /// retain the previous session's track/cue.
    pub fn is_deliberate_silence(&self) -> bool {
        matches!(
            self,
            Self::Governed { authorized, authorized_cues }
                if authorized.is_empty() && authorized_cues.is_empty()
        )
    }

    /// True when a session governs playback (as opposed to a frontend route).
    pub fn is_governed(&self) -> bool {
        matches!(self, Self::Governed { .. })
    }
}

/// Provider-relative playback authority for sound effects.
///
/// The SFX consumer enforces this exactly as the music director enforces
/// [`MusicAuthority`]: an [`ambition_sfx::SfxMessage`] may play only when the
/// active provider authorized the [`SfxId`] it resolves to. This is what stops
/// a sound that merely EXISTS in the process-wide resident bank / synth handle
/// table from being audible in a session whose provider never authored it — the
/// bank is *storage*, this is *permission*.
///
/// A provider's authorized set is *declared*: the ids of the procedural cues it
/// authors ([`SfxRegistry::authorized_cue_ids`]) plus the bank ids it
/// contributes ([`crate::catalog::SfxBankRegistry`]). The typed cue shortcuts
/// (jump, dash, hit, …) are gated by this too — a provider hears a cue only when
/// it declared it, never because the resident synth table happens to hold it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SfxAuthority {
    /// No gameplay session governs SFX this frame (frontend/title, or a host
    /// with no selection). Nothing is rejected — frontend routes emit no
    /// gameplay SFX, and a permissive default preserves standalone behavior.
    #[default]
    Ungoverned,
    /// A session is live; only these ids may play. An EMPTY set is a provider
    /// that authored no SFX — DELIBERATE silence (Mary-O), not "play whatever
    /// the resident bank still holds."
    Governed { authorized: BTreeSet<SfxId> },
}

impl SfxAuthority {
    /// A governed authority permitting exactly `authorized`.
    pub fn governed(authorized: impl IntoIterator<Item = SfxId>) -> Self {
        Self::Governed {
            authorized: authorized.into_iter().collect(),
        }
    }

    /// True when `id` may play this frame.
    pub fn allows(&self, id: SfxId) -> bool {
        match self {
            Self::Ungoverned => true,
            Self::Governed { authorized } => authorized.contains(&id),
        }
    }

    /// True when the active provider authored no SFX — every emission is
    /// dropped rather than resolving against the resident bank.
    pub fn is_deliberate_silence(&self) -> bool {
        matches!(self, Self::Governed { authorized } if authorized.is_empty())
    }

    /// True when a session governs SFX (as opposed to a frontend route).
    pub fn is_governed(&self) -> bool {
        matches!(self, Self::Governed { .. })
    }
}

/// Host policy for what plays at frontend/title routes (no gameplay session).
///
/// The engine's frontend audio system enforces silence when returning to a
/// frontend route; this resource lets a HOST override that with a deliberate
/// title theme. `None` (the default) is silence. The named track must exist in
/// the host's assembled `AudioLibrary`. Engine = mechanism, host = which track:
/// no engine crate names a specific song.
#[derive(Resource, Default, Debug, Clone)]
pub struct FrontendMusicPolicy {
    /// Track id to loop at frontend routes, or `None` for silence.
    pub title_track: Option<String>,
}

impl FrontendMusicPolicy {
    pub fn title(track_id: impl Into<String>) -> Self {
        Self {
            title_track: Some(track_id.into()),
        }
    }
}

/// The provider-relative audio authority of the active gameplay session.
/// `Default` = nothing selected (frontend routes, or before composition).
#[derive(Resource, Default, Debug, Clone)]
pub struct ActiveAudioSelection {
    current: Option<ActiveAudioAuthority>,
}

/// One selected provider's live audio authority.
#[derive(Debug, Clone)]
pub struct ActiveAudioAuthority {
    /// The gameplay-session scope token that owns this selection, or `None`
    /// for a statically-selected direct-entry host (which owns no session and
    /// is never cleared by a retirement). This is the identity that makes
    /// retirement safe: a delayed retirement for an OLDER session must not
    /// clear a NEWER session's audio (see [`ActiveAudioSelection::clear_if_owner`]).
    pub owner: Option<u64>,
    /// Provider id in the audio catalog registry (usually the experience id).
    pub provider_id: String,
    /// The provider's authored music, `None` when it registered none —
    /// a DELIBERATE empty set, not a fallback slot.
    pub music: Option<MusicRegistry>,
    /// The provider's authored SFX synth registry, `None` when it registered
    /// none. Its procedural cues contribute to the authorized id set.
    pub sfx: Option<SfxRegistry>,
    /// The bank [`SfxId`]s this provider contributes (from
    /// [`crate::catalog::SfxBankRegistry`]), on top of its procedural cues.
    /// Empty for a provider that ships no bank (it authorizes only its cues).
    pub sfx_ids: BTreeSet<SfxId>,
}

impl ActiveAudioSelection {
    /// Select `provider_id`'s audio for the session identified by `owner`,
    /// replacing any previous selection. `owner` is the gameplay-session scope
    /// token; pass `None` only for a direct-entry host with no session.
    pub fn select(
        &mut self,
        owner: Option<u64>,
        provider_id: impl Into<String>,
        music: Option<MusicRegistry>,
        sfx: Option<SfxRegistry>,
        sfx_ids: BTreeSet<SfxId>,
    ) {
        self.current = Some(ActiveAudioAuthority {
            owner,
            provider_id: provider_id.into(),
            music,
            sfx,
            sfx_ids,
        });
    }

    /// A statically selected value for direct-entry hosts (no session owner).
    pub fn selected(
        provider_id: impl Into<String>,
        music: Option<MusicRegistry>,
        sfx: Option<SfxRegistry>,
        sfx_ids: BTreeSet<SfxId>,
    ) -> Self {
        let mut selection = Self::default();
        selection.select(None, provider_id, music, sfx, sfx_ids);
        selection
    }

    /// Retire playback authority unconditionally (returning to a frontend route
    /// without an identity to match, e.g. a host-level reset).
    pub fn clear(&mut self) {
        self.current = None;
    }

    /// Retire playback authority ONLY if `owner` is the session that currently
    /// holds it. A retirement carrying an older session's token is a no-op, so a
    /// delayed retirement for session A cannot silence session B.
    pub fn clear_if_owner(&mut self, owner: u64) {
        if self.current.as_ref().and_then(|a| a.owner) == Some(owner) {
            self.current = None;
        }
    }

    pub fn current(&self) -> Option<&ActiveAudioAuthority> {
        self.current.as_ref()
    }

    /// Replace the current selection's contributed bank ids in place, without
    /// rebuilding the selection. Used when the resident SFX bank finishes
    /// loading *after* a statically-selected direct-entry host already chose its
    /// provider: the cues were authorized at selection time, the bank ids become
    /// known a frame or two later. A no-op when nothing is selected.
    pub fn set_current_sfx_ids(&mut self, sfx_ids: BTreeSet<SfxId>) {
        if let Some(current) = self.current.as_mut() {
            current.sfx_ids = sfx_ids;
        }
    }

    /// The session scope token that owns the current selection, if any.
    pub fn owner(&self) -> Option<u64> {
        self.current.as_ref().and_then(|a| a.owner)
    }

    pub fn provider_id(&self) -> Option<&str> {
        self.current.as_ref().map(|a| a.provider_id.as_str())
    }

    /// The active music registry, when a session is live AND its provider
    /// authored music.
    pub fn music(&self) -> Option<&MusicRegistry> {
        self.current.as_ref().and_then(|a| a.music.as_ref())
    }

    /// The active SFX registry, when a session is live AND its provider
    /// authored SFX.
    pub fn sfx(&self) -> Option<&SfxRegistry> {
        self.current.as_ref().and_then(|a| a.sfx.as_ref())
    }

    /// The provider-relative SFX authority implied by the current selection.
    ///
    /// - no selection → [`SfxAuthority::Ungoverned`] (frontend routes);
    /// - a provider with authored SFX (cues and/or bank ids) → `Governed` with
    ///   exactly that provider's authorized id set;
    /// - a provider that authored no SFX → `Governed` with an EMPTY set, i.e.
    ///   deliberate silence.
    ///
    /// The SFX consumer consults this so a sound present in the process-wide
    /// resident bank / synth table but foreign to the active provider can never
    /// play — the exact SFX analogue of [`Self::music_authority`].
    pub fn sfx_authority(&self) -> SfxAuthority {
        match &self.current {
            None => SfxAuthority::Ungoverned,
            Some(authority) => {
                let mut authorized = authority
                    .sfx
                    .as_ref()
                    .map(SfxRegistry::authorized_cue_ids)
                    .unwrap_or_default();
                authorized.extend(authority.sfx_ids.iter().copied());
                SfxAuthority::Governed { authorized }
            }
        }
    }

    /// The provider-relative music authority implied by the current selection.
    ///
    /// - no selection → [`MusicAuthority::Ungoverned`] (frontend routes);
    /// - a provider with authored music → `Governed` with exactly that
    ///   provider's track ids;
    /// - a provider that authored no music → `Governed` with an EMPTY set,
    ///   i.e. deliberate silence.
    ///
    /// The director consults this so a track present in the process-wide
    /// combined library but foreign to the active provider can never play.
    pub fn music_authority(&self) -> MusicAuthority {
        match &self.current {
            None => MusicAuthority::Ungoverned,
            Some(authority) => MusicAuthority::Governed {
                authorized: authority
                    .music
                    .as_ref()
                    .map(|music| music.tracks.iter().map(|track| track.id.clone()).collect())
                    .unwrap_or_default(),
                // Adaptive cue ids are folded in by the intent resolver from the
                // provider's `AdaptiveCueRegistry` entry — the selection layer
                // only knows track ids.
                authorized_cues: BTreeSet::new(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{MusicTrack, SfxSpec, SoundCueKey, WaveformSpec};

    fn cue(cue: SoundCueKey) -> SfxSpec {
        SfxSpec {
            cue,
            waveform: WaveformSpec::Sine,
            frequency: 440.0,
            frequency_end: 440.0,
            duration: 0.1,
            volume: 0.5,
            attack: 0.0,
            release: 0.0,
            noise: 0.0,
        }
    }

    fn sfx(cues: impl IntoIterator<Item = SoundCueKey>) -> SfxRegistry {
        SfxRegistry {
            sample_rate: 44_100,
            sfx: cues.into_iter().map(cue).collect(),
        }
    }

    fn music(id: &str) -> MusicRegistry {
        MusicRegistry {
            default_track: id.to_string(),
            tracks: vec![MusicTrack {
                id: id.to_string(),
                display_name: id.to_string(),
                asset_path: None,
            }],
        }
    }

    #[test]
    fn selection_replaces_and_clears_cleanly() {
        let mut selection = ActiveAudioSelection::default();
        assert!(selection.current().is_none());
        assert!(selection.music().is_none());

        selection.select(
            Some(1),
            "sanic",
            Some(music("you_are_too_slow")),
            None,
            BTreeSet::new(),
        );
        assert_eq!(selection.provider_id(), Some("sanic"));
        assert_eq!(selection.owner(), Some(1));
        assert_eq!(
            selection.music().map(|m| m.default_track.as_str()),
            Some("you_are_too_slow")
        );

        // Switching providers REPLACES — no residue of the previous authority.
        selection.select(Some(2), "mary_o", None, None, BTreeSet::new());
        assert_eq!(selection.provider_id(), Some("mary_o"));
        assert!(
            selection.music().is_none(),
            "a provider that authored no music is a deliberate empty set"
        );

        selection.clear();
        assert!(selection.current().is_none());
    }

    /// Poison: a delayed retirement for an OLDER session must not clear a NEWER
    /// session's audio authority. Activate A, activate B, then deliver A's stale
    /// retirement — B must keep ownership.
    #[test]
    fn stale_retirement_does_not_clear_a_newer_selection() {
        let mut selection = ActiveAudioSelection::default();
        selection.select(
            Some(1),
            "sanic",
            Some(music("you_are_too_slow")),
            None,
            BTreeSet::new(),
        );
        // Session B (scope 2) takes over.
        selection.select(
            Some(2),
            "ambition",
            Some(music("ambition_theme")),
            None,
            BTreeSet::new(),
        );
        assert_eq!(selection.owner(), Some(2));

        // A stale retirement for scope 1 arrives late.
        selection.clear_if_owner(1);
        assert_eq!(
            selection.owner(),
            Some(2),
            "session B must still own audio after A's delayed retirement"
        );
        assert_eq!(selection.provider_id(), Some("ambition"));

        // B's own retirement DOES clear it.
        selection.clear_if_owner(2);
        assert!(selection.current().is_none());
    }

    #[test]
    fn no_selection_is_ungoverned_authority() {
        let selection = ActiveAudioSelection::default();
        assert_eq!(selection.music_authority(), MusicAuthority::Ungoverned);
        // Ungoverned permits nothing to be *rejected* — the director must not
        // change playback at a frontend route (the frontend policy owns it).
        assert!(selection.music_authority().allows("anything"));
        assert!(!selection.music_authority().is_deliberate_silence());
    }

    #[test]
    fn a_music_provider_only_authorizes_its_own_tracks() {
        // The heart of Issue 1: a Sanic session must not be able to play an
        // Ambition track that merely EXISTS in the combined library.
        let mut selection = ActiveAudioSelection::default();
        selection.select(
            Some(7),
            "sanic",
            Some(music("you_are_too_slow")),
            None,
            BTreeSet::new(),
        );
        let authority = selection.music_authority();
        assert!(authority.allows("you_are_too_slow"));
        assert!(
            !authority.allows("ambition_boss_theme"),
            "a foreign provider's track is not authorized by this session"
        );
        assert!(!authority.is_deliberate_silence());
        assert!(authority.is_governed());
    }

    #[test]
    fn a_provider_with_no_music_is_deliberate_silence() {
        // Mary-O authors no music: a live session, but nothing may play.
        let mut selection = ActiveAudioSelection::default();
        selection.select(Some(3), "mary_o", None, None, BTreeSet::new());
        let authority = selection.music_authority();
        assert!(authority.is_governed());
        assert!(
            authority.is_deliberate_silence(),
            "an empty authorized set is a stop request, not 'retain current'"
        );
        assert!(!authority.allows("you_are_too_slow"));
    }

    #[test]
    fn music_authority_governs_adaptive_cues_separately_from_tracks() {
        let mut authority = MusicAuthority::governed(vec!["a_possible_morning".to_string()]);
        // A track is authorized; no cue is until the resolver folds them in.
        assert!(authority.allows("a_possible_morning"));
        assert!(!authority.allows_cue("first_goblin_tune_v2"));
        assert!(!authority.is_deliberate_silence(), "it has a track");
        authority.authorize_cues(vec!["first_goblin_tune_v2".to_string()]);
        assert!(authority.allows_cue("first_goblin_tune_v2"));
        assert!(
            !authority.allows_cue("some_boss_cue"),
            "a cue the provider did not author stays unauthorized"
        );
    }

    #[test]
    fn neither_tracks_nor_cues_is_deliberate_silence() {
        let mut authority = MusicAuthority::governed(Vec::<String>::new());
        assert!(authority.is_deliberate_silence());
        authority.authorize_cues(vec!["cue".to_string()]);
        assert!(
            !authority.is_deliberate_silence(),
            "authorizing an adaptive cue lifts deliberate silence"
        );
        assert!(authority.allows_cue("cue"));
        // Ungoverned never restricts cues.
        assert!(MusicAuthority::Ungoverned.allows_cue("anything"));
    }

    #[test]
    fn no_selection_is_ungoverned_sfx_authority() {
        let selection = ActiveAudioSelection::default();
        assert_eq!(selection.sfx_authority(), SfxAuthority::Ungoverned);
        // Ungoverned rejects nothing — standalone/frontend SFX is unrestricted.
        assert!(selection.sfx_authority().allows(SoundCueKey::Jump.sfx_id()));
        assert!(!selection.sfx_authority().is_deliberate_silence());
    }

    #[test]
    fn an_sfx_provider_only_authorizes_its_own_cues_and_bank_ids() {
        // Sanic authors Dash + Jump cues and contributes one bank id; it must
        // not be able to play an Ambition-only id merely resident in the bank.
        let ambition_only = SfxId::from_static("boss.mirror.shatter");
        let sanic_bank = SfxId::from_static("sanic.ring.collect");
        let mut selection = ActiveAudioSelection::default();
        selection.select(
            Some(7),
            "sanic",
            None,
            Some(sfx([SoundCueKey::Dash, SoundCueKey::Jump])),
            BTreeSet::from([sanic_bank]),
        );
        let authority = selection.sfx_authority();
        assert!(authority.allows(SoundCueKey::Dash.sfx_id()));
        assert!(authority.allows(SoundCueKey::Jump.sfx_id()));
        assert!(authority.allows(sanic_bank));
        assert!(
            !authority.allows(ambition_only),
            "a foreign provider's bank id is not authorized by this session"
        );
        assert!(
            !authority.allows(SoundCueKey::Slash.sfx_id()),
            "an undeclared cue is not authorized, even though the resident synth holds it"
        );
        assert!(authority.is_governed());
        assert!(!authority.is_deliberate_silence());
    }

    #[test]
    fn a_provider_with_no_sfx_is_deliberate_silence() {
        // Mary-O authors no SFX: a live session, but nothing may play.
        let mut selection = ActiveAudioSelection::default();
        selection.select(Some(3), "mary_o", None, None, BTreeSet::new());
        let authority = selection.sfx_authority();
        assert!(authority.is_governed());
        assert!(
            authority.is_deliberate_silence(),
            "no authored SFX is a stop request, not 'resolve against the resident bank'"
        );
        assert!(!authority.allows(SoundCueKey::Jump.sfx_id()));
    }
}
