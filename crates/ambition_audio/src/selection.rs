//! App-local active audio context.
//!
//! [`crate::catalog::AudioCatalogRegistry`] stores every linked provider's
//! authored definitions. [`ActiveAudioSelection`] identifies the one shell
//! activation that owns playback now. Frontend routes and gameplay sessions use
//! the same mechanism: a title screen may own title music and menu SFX, while a
//! retired gameplay activation cannot leak queued work into it.

use std::collections::BTreeSet;

use ambition_sfx::{AudioContextOwner, SfxId};
use bevy::prelude::{Message, Resource};

use crate::spec::{MusicRegistry, SfxRegistry};

/// Exact transition between shell-owned audio contexts.
///
/// Lower-level playback and gameplay crates consume this neutral fact to reset
/// activation-local request/director state without depending on the shell crate.
#[derive(Message, Clone, Copy, Debug, Eq, PartialEq)]
pub struct AudioContextChanged {
    pub previous: Option<AudioContextOwner>,
    pub current: Option<AudioContextOwner>,
}

/// Provider-relative playback authority for one frame of music intent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum MusicAuthority {
    /// No active audio context. The music director may not start gameplay music.
    #[default]
    Denied,
    /// The active context permits exactly these simple tracks and adaptive cues.
    Governed {
        authorized: BTreeSet<String>,
        authorized_cues: BTreeSet<String>,
    },
}

impl MusicAuthority {
    pub fn governed(authorized: impl IntoIterator<Item = String>) -> Self {
        Self::Governed {
            authorized: authorized.into_iter().collect(),
            authorized_cues: BTreeSet::new(),
        }
    }

    pub fn authorize_cues(&mut self, cues: impl IntoIterator<Item = String>) {
        if let Self::Governed {
            authorized_cues, ..
        } = self
        {
            authorized_cues.extend(cues);
        }
    }

    pub fn allows(&self, track_id: &str) -> bool {
        matches!(self, Self::Governed { authorized, .. } if authorized.contains(track_id))
    }

    pub fn allows_cue(&self, cue_id: &str) -> bool {
        matches!(
            self,
            Self::Governed {
                authorized_cues,
                ..
            } if authorized_cues.contains(cue_id)
        )
    }

    pub fn is_deliberate_silence(&self) -> bool {
        matches!(
            self,
            Self::Governed {
                authorized,
                authorized_cues,
            } if authorized.is_empty() && authorized_cues.is_empty()
        )
    }

    pub fn is_governed(&self) -> bool {
        matches!(self, Self::Governed { .. })
    }
}

/// Provider-relative playback authority for sound effects.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SfxAuthority {
    /// No active audio context. Gameplay and frontend SFX are both denied.
    #[default]
    Denied,
    /// The active context permits exactly these authored ids.
    Governed { authorized: BTreeSet<SfxId> },
}

impl SfxAuthority {
    pub fn governed(authorized: impl IntoIterator<Item = SfxId>) -> Self {
        Self::Governed {
            authorized: authorized.into_iter().collect(),
        }
    }

    pub fn allows(&self, id: SfxId) -> bool {
        matches!(self, Self::Governed { authorized } if authorized.contains(&id))
    }

    pub fn is_deliberate_silence(&self) -> bool {
        matches!(self, Self::Governed { authorized } if authorized.is_empty())
    }

    pub fn is_governed(&self) -> bool {
        matches!(self, Self::Governed { .. })
    }
}

/// Host-authored audio profile for frontend shell experiences.
///
/// The profile is explicit rather than an exception to gameplay authority. A
/// launcher/startup/loading route may own one title track and a narrow menu-SFX
/// allowlist. The provider supplies the actual source definitions; the host
/// chooses which subset belongs to its frontend.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct FrontendAudioProfile {
    provider_id: String,
    title_track: Option<String>,
    sfx_ids: BTreeSet<SfxId>,
}

impl FrontendAudioProfile {
    pub fn new(provider_id: impl Into<String>) -> Self {
        let provider_id = provider_id.into();
        assert!(!provider_id.trim().is_empty(), "frontend audio provider cannot be empty");
        Self {
            provider_id,
            title_track: None,
            sfx_ids: BTreeSet::new(),
        }
    }

    pub fn with_title_track(mut self, track_id: impl Into<String>) -> Self {
        self.title_track = Some(track_id.into());
        self
    }

    pub fn with_sfx(mut self, ids: impl IntoIterator<Item = SfxId>) -> Self {
        self.sfx_ids.extend(ids);
        self
    }

    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    pub fn title_track(&self) -> Option<&str> {
        self.title_track.as_deref()
    }

    pub fn sfx_ids(&self) -> &BTreeSet<SfxId> {
        &self.sfx_ids
    }
}

/// The provider-relative audio authority of the active shell context.
#[derive(Resource, Default, Debug, Clone)]
pub struct ActiveAudioSelection {
    current: Option<ActiveAudioAuthority>,
}

/// One frontend, gameplay, or direct-entry context's live audio authority.
#[derive(Debug, Clone)]
pub struct ActiveAudioAuthority {
    owner: AudioContextOwner,
    provider_id: String,
    music: Option<MusicRegistry>,
    sfx: Option<SfxRegistry>,
    authorized_music: BTreeSet<String>,
    authorized_cues: BTreeSet<String>,
    authorized_sfx: BTreeSet<SfxId>,
    /// `None` means all provider bank ids are part of this context (gameplay /
    /// direct entry). `Some` is the frontend's explicit narrow allowlist.
    explicit_sfx_allowlist: Option<BTreeSet<SfxId>>,
    preferred_track: Option<String>,
}

impl ActiveAudioSelection {
    /// Select a gameplay session. Every track/cue/SFX authored by its provider
    /// is eligible; exact request ownership still decides whether queued work is
    /// current.
    pub fn select_gameplay(
        &mut self,
        owner: u64,
        provider_id: impl Into<String>,
        music: Option<MusicRegistry>,
        sfx: Option<SfxRegistry>,
        bank_ids: BTreeSet<SfxId>,
    ) {
        self.select_provider(
            AudioContextOwner::Gameplay(owner),
            provider_id.into(),
            music,
            sfx,
            bank_ids,
            None,
            None,
            None,
        );
    }

    /// Select one frontend shell activation. The actual source definitions come
    /// from `music` / `sfx`; the profile restricts playback to its title track
    /// and menu cue allowlist.
    pub fn select_frontend(
        &mut self,
        activation_id: u64,
        profile: &FrontendAudioProfile,
        music: Option<MusicRegistry>,
        sfx: Option<SfxRegistry>,
        bank_ids: BTreeSet<SfxId>,
    ) {
        let explicit_music = profile.title_track.iter().cloned().collect();
        let explicit_sfx = profile.sfx_ids.clone();
        self.select_provider(
            AudioContextOwner::Frontend(activation_id),
            profile.provider_id.clone(),
            music,
            sfx,
            bank_ids,
            Some(explicit_music),
            Some(explicit_sfx),
            profile.title_track.clone(),
        );
    }

    /// A statically selected value for direct-entry hosts.
    pub fn selected_direct(
        provider_id: impl Into<String>,
        music: Option<MusicRegistry>,
        sfx: Option<SfxRegistry>,
        bank_ids: BTreeSet<SfxId>,
    ) -> Self {
        let mut selection = Self::default();
        selection.select_provider(
            AudioContextOwner::Direct,
            provider_id.into(),
            music,
            sfx,
            bank_ids,
            None,
            None,
            None,
        );
        selection
    }

    #[allow(clippy::too_many_arguments)]
    fn select_provider(
        &mut self,
        owner: AudioContextOwner,
        provider_id: String,
        music: Option<MusicRegistry>,
        sfx: Option<SfxRegistry>,
        bank_ids: BTreeSet<SfxId>,
        explicit_music_allowlist: Option<BTreeSet<String>>,
        explicit_sfx_allowlist: Option<BTreeSet<SfxId>>,
        preferred_track: Option<String>,
    ) {
        let provider_music = music
            .as_ref()
            .map(|registry| {
                registry
                    .tracks
                    .iter()
                    .map(|track| track.id.clone())
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        let authorized_music = explicit_music_allowlist
            .as_ref()
            .map(|allowlist| {
                allowlist
                    .intersection(&provider_music)
                    .cloned()
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or(provider_music);
        let mut provider_sfx = sfx
            .as_ref()
            .map(SfxRegistry::authorized_cue_ids)
            .unwrap_or_default();
        provider_sfx.extend(bank_ids);
        let authorized_sfx = explicit_sfx_allowlist
            .as_ref()
            .map(|allowlist| {
                allowlist
                    .intersection(&provider_sfx)
                    .copied()
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or(provider_sfx);
        self.current = Some(ActiveAudioAuthority {
            owner,
            provider_id,
            music,
            sfx,
            authorized_music,
            authorized_cues: BTreeSet::new(),
            authorized_sfx,
            explicit_sfx_allowlist,
            preferred_track,
        });
    }

    pub fn clear(&mut self) {
        self.current = None;
    }

    pub fn clear_if_owner(&mut self, owner: AudioContextOwner) {
        if self.owner() == Some(owner) {
            self.current = None;
        }
    }

    pub fn current(&self) -> Option<&ActiveAudioAuthority> {
        self.current.as_ref()
    }

    pub fn owner(&self) -> Option<AudioContextOwner> {
        self.current.as_ref().map(|authority| authority.owner)
    }

    pub fn accepts_request_owner(&self, owner: Option<AudioContextOwner>) -> bool {
        self.owner() == owner && owner.is_some()
    }

    pub fn provider_id(&self) -> Option<&str> {
        self.current.as_ref().map(|authority| authority.provider_id.as_str())
    }

    pub fn music(&self) -> Option<&MusicRegistry> {
        self.current.as_ref().and_then(|authority| authority.music.as_ref())
    }

    pub fn sfx(&self) -> Option<&SfxRegistry> {
        self.current.as_ref().and_then(|authority| authority.sfx.as_ref())
    }

    pub fn preferred_track(&self) -> Option<&str> {
        self.current
            .as_ref()
            .and_then(|authority| authority.preferred_track.as_deref())
    }

    /// Refresh one provider's runtime bank identities after asynchronous load.
    /// The live context changes only when it belongs to that provider.
    pub fn refresh_provider_sfx_ids(&mut self, provider_id: &str, bank_ids: BTreeSet<SfxId>) {
        let Some(current) = self.current.as_mut() else {
            return;
        };
        if current.provider_id != provider_id {
            return;
        }
        let mut provider_sfx = current
            .sfx
            .as_ref()
            .map(SfxRegistry::authorized_cue_ids)
            .unwrap_or_default();
        provider_sfx.extend(bank_ids);
        current.authorized_sfx = current
            .explicit_sfx_allowlist
            .as_ref()
            .map(|allowlist| allowlist.intersection(&provider_sfx).copied().collect())
            .unwrap_or(provider_sfx);
    }

    pub fn authorize_adaptive_cues(&mut self, cues: impl IntoIterator<Item = String>) {
        if let Some(current) = self.current.as_mut() {
            current.authorized_cues.extend(cues);
        }
    }

    pub fn sfx_authority(&self) -> SfxAuthority {
        self.current
            .as_ref()
            .map(|authority| SfxAuthority::Governed {
                authorized: authority.authorized_sfx.clone(),
            })
            .unwrap_or(SfxAuthority::Denied)
    }

    pub fn music_authority(&self) -> MusicAuthority {
        self.current
            .as_ref()
            .map(|authority| MusicAuthority::Governed {
                authorized: authority.authorized_music.clone(),
                authorized_cues: authority.authorized_cues.clone(),
            })
            .unwrap_or(MusicAuthority::Denied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{MusicTrack, SfxSpec, SoundCueKey, WaveformSpec};

    fn cue(cue: SoundCueKey) -> SfxSpec {
        SfxSpec {
            cue: Some(cue),
            id: None,
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
    fn no_context_denies_audio() {
        let selection = ActiveAudioSelection::default();
        assert_eq!(selection.music_authority(), MusicAuthority::Denied);
        assert_eq!(selection.sfx_authority(), SfxAuthority::Denied);
        assert!(!selection.sfx_authority().allows(SoundCueKey::Jump.sfx_id()));
    }

    #[test]
    fn frontend_is_a_first_class_narrow_audio_context() {
        let profile = FrontendAudioProfile::new("ambition")
            .with_title_track("title")
            .with_sfx([SoundCueKey::Jump.sfx_id()]);
        let mut selection = ActiveAudioSelection::default();
        selection.select_frontend(
            11,
            &profile,
            Some(music("title")),
            Some(sfx([SoundCueKey::Jump, SoundCueKey::Dash])),
            BTreeSet::new(),
        );
        assert_eq!(selection.owner(), Some(AudioContextOwner::Frontend(11)));
        assert!(selection.music_authority().allows("title"));
        assert!(selection.sfx_authority().allows(SoundCueKey::Jump.sfx_id()));
        assert!(!selection.sfx_authority().allows(SoundCueKey::Dash.sfx_id()));
    }

    #[test]
    fn stale_same_provider_owner_is_rejected() {
        let mut selection = ActiveAudioSelection::default();
        selection.select_gameplay(
            2,
            "sanic",
            None,
            Some(sfx([SoundCueKey::Dash])),
            BTreeSet::new(),
        );
        assert!(!selection.accepts_request_owner(Some(AudioContextOwner::Gameplay(1))));
        assert!(selection.accepts_request_owner(Some(AudioContextOwner::Gameplay(2))));
    }

    #[test]
    fn stale_retirement_does_not_clear_a_newer_context() {
        let mut selection = ActiveAudioSelection::default();
        selection.select_gameplay(1, "sanic", Some(music("fast")), None, BTreeSet::new());
        selection.select_gameplay(2, "sanic", Some(music("fast")), None, BTreeSet::new());
        selection.clear_if_owner(AudioContextOwner::Gameplay(1));
        assert_eq!(selection.owner(), Some(AudioContextOwner::Gameplay(2)));
        selection.clear_if_owner(AudioContextOwner::Gameplay(2));
        assert!(selection.current().is_none());
    }

    #[test]
    fn late_bank_refresh_updates_only_the_owning_provider() {
        let late = SfxId::from_static("late.bank.id");
        let mut selection = ActiveAudioSelection::default();
        selection.select_gameplay(3, "ambition", None, None, BTreeSet::new());
        selection.refresh_provider_sfx_ids("sanic", BTreeSet::from([late]));
        assert!(!selection.sfx_authority().allows(late));
        selection.refresh_provider_sfx_ids("ambition", BTreeSet::from([late]));
        assert!(selection.sfx_authority().allows(late));
    }

    #[test]
    fn silent_gameplay_provider_is_explicit() {
        let mut selection = ActiveAudioSelection::default();
        selection.select_gameplay(3, "mary_o", None, None, BTreeSet::new());
        assert!(selection.music_authority().is_deliberate_silence());
        assert!(selection.sfx_authority().is_deliberate_silence());
    }
}
