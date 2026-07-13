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

use bevy::prelude::Resource;

use crate::spec::{MusicRegistry, SfxRegistry};

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
    /// The provider's authored SFX, `None` when it registered none.
    pub sfx: Option<SfxRegistry>,
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
    ) {
        self.current = Some(ActiveAudioAuthority {
            owner,
            provider_id: provider_id.into(),
            music,
            sfx,
        });
    }

    /// A statically selected value for direct-entry hosts (no session owner).
    pub fn selected(
        provider_id: impl Into<String>,
        music: Option<MusicRegistry>,
        sfx: Option<SfxRegistry>,
    ) -> Self {
        let mut selection = Self::default();
        selection.select(None, provider_id, music, sfx);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::MusicTrack;

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

        selection.select(Some(1), "sanic", Some(music("you_are_too_slow")), None);
        assert_eq!(selection.provider_id(), Some("sanic"));
        assert_eq!(selection.owner(), Some(1));
        assert_eq!(
            selection.music().map(|m| m.default_track.as_str()),
            Some("you_are_too_slow")
        );

        // Switching providers REPLACES — no residue of the previous authority.
        selection.select(Some(2), "mary_o", None, None);
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
        selection.select(Some(1), "sanic", Some(music("you_are_too_slow")), None);
        // Session B (scope 2) takes over.
        selection.select(Some(2), "ambition", Some(music("ambition_theme")), None);
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
}
