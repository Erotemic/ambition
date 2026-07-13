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

/// The provider-relative audio authority of the active gameplay session.
/// `Default` = nothing selected (frontend routes, or before composition).
#[derive(Resource, Default, Debug, Clone)]
pub struct ActiveAudioSelection {
    current: Option<ActiveAudioAuthority>,
}

/// One selected provider's live audio authority.
#[derive(Debug, Clone)]
pub struct ActiveAudioAuthority {
    /// Provider id in the audio catalog registry (usually the experience id).
    pub provider_id: String,
    /// The provider's authored music, `None` when it registered none —
    /// a DELIBERATE empty set, not a fallback slot.
    pub music: Option<MusicRegistry>,
    /// The provider's authored SFX, `None` when it registered none.
    pub sfx: Option<SfxRegistry>,
}

impl ActiveAudioSelection {
    /// Select `provider_id`'s audio, replacing any previous selection.
    pub fn select(
        &mut self,
        provider_id: impl Into<String>,
        music: Option<MusicRegistry>,
        sfx: Option<SfxRegistry>,
    ) {
        self.current = Some(ActiveAudioAuthority {
            provider_id: provider_id.into(),
            music,
            sfx,
        });
    }

    /// A statically selected value for direct-entry hosts.
    pub fn selected(
        provider_id: impl Into<String>,
        music: Option<MusicRegistry>,
        sfx: Option<SfxRegistry>,
    ) -> Self {
        let mut selection = Self::default();
        selection.select(provider_id, music, sfx);
        selection
    }

    /// Retire playback authority (returning to a frontend route).
    pub fn clear(&mut self) {
        self.current = None;
    }

    pub fn current(&self) -> Option<&ActiveAudioAuthority> {
        self.current.as_ref()
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

        selection.select("sanic", Some(music("you_are_too_slow")), None);
        assert_eq!(selection.provider_id(), Some("sanic"));
        assert_eq!(
            selection.music().map(|m| m.default_track.as_str()),
            Some("you_are_too_slow")
        );

        // Switching providers REPLACES — no residue of the previous authority.
        selection.select("mary_o", None, None);
        assert_eq!(selection.provider_id(), Some("mary_o"));
        assert!(
            selection.music().is_none(),
            "a provider that authored no music is a deliberate empty set"
        );

        selection.clear();
        assert!(selection.current().is_none());
    }
}
