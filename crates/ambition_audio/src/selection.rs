//! App-local active audio context.
//!
//! [`crate::catalog::AudioCatalogRegistry`] stores every linked provider's
//! authored definitions. [`ActiveAudioSelection`] identifies the one shell
//! activation that owns playback now. Frontend routes and gameplay sessions use
//! the same mechanism: a title screen may own title music and menu SFX, while a
//! retired gameplay activation cannot leak queued work into it.

use std::collections::{BTreeMap, BTreeSet};

use ambition_sfx::{AudioContextOwner, PresentationSourceId, SfxId};
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

/// Authored audio profile for one frontend shell route.
///
/// Explicit, not an exception to gameplay authority. A launcher, startup,
/// loading, or select route may own one title track and a narrow menu-SFX
/// allowlist. The provider supplies the source definitions; the declaration
/// picks the subset for that screen. Declarations live in
/// [`FrontendAudioRegistry`], keyed by route.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrontendAudioProfile {
    provider_id: String,
    title_track: Option<String>,
    sfx_ids: BTreeSet<SfxId>,
}

impl FrontendAudioProfile {
    pub fn new(provider_id: impl Into<String>) -> Self {
        let provider_id = provider_id.into();
        assert!(
            !provider_id.trim().is_empty(),
            "frontend audio provider cannot be empty"
        );
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

/// Every frontend audio declaration in this App, keyed by owning route, plus
/// the profile in effect.
///
/// Two kinds of entry:
///
/// * a route declaration ("this screen sounds like this"), made by the
///   screen's author; it travels with the provider into any host;
/// * the host default ("screens I own sound like this"), made once by the
///   host for its launcher, startup, and loading routes.
///
/// A route without its own declaration uses the host default. With no default
/// either, the route is silent; it never inherits another route's music.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct FrontendAudioRegistry {
    by_route: BTreeMap<String, FrontendAudioProfile>,
    host_default: Option<FrontendAudioProfile>,
    /// The resolved profile of the most recent frontend route: what frontend
    /// playback acts on.
    ///
    /// Not cleared when that route deactivates. This is a cheap precaution
    /// for a route change that spans frames (for example a frontend route
    /// behind a load barrier); no test depends on it.
    in_effect: Option<FrontendAudioProfile>,
}

impl FrontendAudioRegistry {
    /// Declare the frontend sound of one route. Later declarations of the same
    /// route replace earlier ones.
    pub fn declare_route(&mut self, route_id: impl Into<String>, profile: FrontendAudioProfile) {
        self.by_route.insert(route_id.into(), profile);
    }

    /// Declare the answer for routes that declare nothing themselves.
    pub fn set_host_default(&mut self, profile: FrontendAudioProfile) {
        self.host_default = Some(profile);
    }

    pub fn host_default(&self) -> Option<&FrontendAudioProfile> {
        self.host_default.as_ref()
    }

    pub fn declared_for(&self, route_id: &str) -> Option<&FrontendAudioProfile> {
        self.by_route.get(route_id)
    }

    /// What `route_id` sounds like: its own declaration, else the host default.
    pub fn resolve(&self, route_id: &str) -> Option<&FrontendAudioProfile> {
        self.by_route.get(route_id).or(self.host_default.as_ref())
    }

    /// Resolve `route_id` and make it the profile in effect.
    ///
    /// Returns the resolved profile, so the caller does not need a second
    /// call.
    pub fn enter_route(&mut self, route_id: &str) -> Option<&FrontendAudioProfile> {
        self.in_effect = self.resolve(route_id).cloned();
        self.in_effect.as_ref()
    }

    /// The profile frontend playback acts on. See the `in_effect` field for
    /// why it outlives the activation that selected it.
    pub fn in_effect(&self) -> Option<&FrontendAudioProfile> {
        self.in_effect.as_ref()
    }

    /// A statically selected profile for a direct-entry App that runs no shell
    /// routing at all. There is no route to key by and no handoff to survive.
    pub fn direct(profile: FrontendAudioProfile) -> Self {
        Self {
            by_route: BTreeMap::new(),
            host_default: Some(profile.clone()),
            in_effect: Some(profile),
        }
    }
}

/// Declare frontend audio at plugin-build time.
///
/// Like [`crate::catalog::AudioCatalogAppExt`]: a provider states what its
/// screens sound like next to the audio fragment it registers, and the
/// declaration goes with the provider into any host.
pub trait FrontendAudioAppExt {
    /// "This screen sounds like this." Made by whoever authored the screen.
    fn declare_route_frontend_audio(
        &mut self,
        route_id: impl Into<String>,
        profile: FrontendAudioProfile,
    ) -> &mut Self;

    /// "The screens I own sound like this." Made by the host, once.
    fn set_host_frontend_audio(&mut self, profile: FrontendAudioProfile) -> &mut Self;
}

impl FrontendAudioAppExt for bevy::prelude::App {
    fn declare_route_frontend_audio(
        &mut self,
        route_id: impl Into<String>,
        profile: FrontendAudioProfile,
    ) -> &mut Self {
        let mut registry = self
            .world()
            .get_resource::<FrontendAudioRegistry>()
            .cloned()
            .unwrap_or_default();
        registry.declare_route(route_id, profile);
        self.insert_resource(registry);
        self
    }

    fn set_host_frontend_audio(&mut self, profile: FrontendAudioProfile) -> &mut Self {
        let mut registry = self
            .world()
            .get_resource::<FrontendAudioRegistry>()
            .cloned()
            .unwrap_or_default();
        registry.set_host_default(profile);
        self.insert_resource(registry);
        self
    }
}

/// The provider-relative audio authority of the active shell context.
#[derive(Resource, Default, Debug, Clone)]
pub struct ActiveAudioSelection {
    current: Option<ActiveAudioAuthority>,
    /// Presentation sources that two different providers both tried to claim.
    /// Recorded, not fatal. The same provider re-authorizing its own source
    /// (as async bank ids arrive) is routine and not a conflict.
    sfx_source_conflicts: Vec<SfxSourceClaimConflict>,
}

/// Two providers claiming one presentation source. See
/// [`ActiveAudioSelection::sfx_source_conflicts`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SfxSourceClaimConflict {
    pub source: PresentationSourceId,
    /// The provider that claimed it first and still owns it.
    pub holder: String,
    /// The provider that was refused. Its cues will not resolve under this
    /// source; it needs one of its own.
    pub rejected: String,
}

/// One presentation source authorized inside the active audio context.
///
/// Source identity is stable and authored; provider identity selects the
/// backing procedural registry or bank. They are separate so a prepared
/// character or stage can expose a stable package id without tying emitters
/// to storage.
#[derive(Debug, Clone)]
struct ActiveSfxSource {
    provider_id: String,
    sfx: Option<SfxRegistry>,
    authorized: BTreeSet<SfxId>,
    /// `None` means all provider ids are eligible. `Some` is a narrow frontend
    /// allowlist that remains narrow when a packed bank arrives later.
    explicit_allowlist: Option<BTreeSet<SfxId>>,
}

impl ActiveSfxSource {
    fn new(
        provider_id: String,
        sfx: Option<SfxRegistry>,
        bank_ids: BTreeSet<SfxId>,
        explicit_allowlist: Option<BTreeSet<SfxId>>,
    ) -> Self {
        let authorized = Self::authorized_ids(&sfx, bank_ids, explicit_allowlist.as_ref());
        Self {
            provider_id,
            sfx,
            authorized,
            explicit_allowlist,
        }
    }

    fn authorized_ids(
        sfx: &Option<SfxRegistry>,
        bank_ids: BTreeSet<SfxId>,
        explicit_allowlist: Option<&BTreeSet<SfxId>>,
    ) -> BTreeSet<SfxId> {
        let mut provider_sfx = sfx
            .as_ref()
            .map(SfxRegistry::authorized_cue_ids)
            .unwrap_or_default();
        provider_sfx.extend(bank_ids);
        explicit_allowlist
            .map(|allowlist| allowlist.intersection(&provider_sfx).copied().collect())
            .unwrap_or(provider_sfx)
    }

    fn refresh_bank_ids(&mut self, bank_ids: BTreeSet<SfxId>) {
        self.authorized =
            Self::authorized_ids(&self.sfx, bank_ids, self.explicit_allowlist.as_ref());
    }

    /// Merge another view of the same source into this one.
    ///
    /// Union the authorized sets and prefer a present registry over an absent
    /// one, so the result does not depend on arrival order. During an async
    /// bank load, two views describe different instants; the union is true
    /// after both.
    fn absorb(&mut self, other: ActiveSfxSource) {
        if self.sfx.is_none() {
            self.sfx = other.sfx;
        }
        self.authorized.extend(other.authorized);
    }
}

/// One frontend, gameplay, or direct-entry context's live audio authority.
#[derive(Debug, Clone)]
pub struct ActiveAudioAuthority {
    owner: AudioContextOwner,
    provider_id: String,
    primary_sfx_source: PresentationSourceId,
    music: Option<MusicRegistry>,
    authorized_music: BTreeSet<String>,
    authorized_cues: BTreeSet<String>,
    sfx_sources: BTreeMap<PresentationSourceId, ActiveSfxSource>,
    preferred_track: Option<String>,
}

impl ActiveAudioSelection {
    /// Select a gameplay session. Every track/cue/SFX authored by its primary
    /// provider is eligible; exact request ownership still decides whether
    /// queued work is current. Additional cast/stage/ruleset sources may be
    /// authorized with [`Self::authorize_sfx_source`].
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
        let primary_sfx_source = PresentationSourceId::new(provider_id.clone());
        let primary =
            ActiveSfxSource::new(provider_id.clone(), sfx, bank_ids, explicit_sfx_allowlist);
        // A new authority starts with an empty conflict list. Conflicts
        // describe the active selection; carried across a change, one bad
        // session would be reported forever.
        self.sfx_source_conflicts.clear();
        self.current = Some(ActiveAudioAuthority {
            owner,
            provider_id,
            primary_sfx_source: primary_sfx_source.clone(),
            music,
            authorized_music,
            authorized_cues: BTreeSet::new(),
            sfx_sources: BTreeMap::from([(primary_sfx_source, primary)]),
            preferred_track,
        });
    }

    pub fn clear(&mut self) {
        self.current = None;
        self.sfx_source_conflicts.clear();
    }

    pub fn clear_if_owner(&mut self, owner: AudioContextOwner) {
        if self.owner() == Some(owner) {
            self.current = None;
            self.sfx_source_conflicts.clear();
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
        self.current
            .as_ref()
            .map(|authority| authority.provider_id.as_str())
    }

    pub fn primary_sfx_source(&self) -> Option<&PresentationSourceId> {
        self.current
            .as_ref()
            .map(|authority| &authority.primary_sfx_source)
    }

    pub fn music(&self) -> Option<&MusicRegistry> {
        self.current
            .as_ref()
            .and_then(|authority| authority.music.as_ref())
    }

    /// Compatibility view of the primary provider's procedural registry.
    pub fn sfx(&self) -> Option<&SfxRegistry> {
        let source = self.primary_sfx_source()?;
        self.sfx_for_source(source)
    }

    pub fn sfx_for_source(&self, source: &PresentationSourceId) -> Option<&SfxRegistry> {
        self.current.as_ref()?.sfx_sources.get(source)?.sfx.as_ref()
    }

    /// Is this presentation source allowed to resolve cues in the current session?
    ///
    /// Different from [`Self::sfx_for_source`], which also requires a
    /// procedural registry. A source authorized with `sfx: None` (bank-only,
    /// or a catalog not yet loaded) is fully authorized but would look denied
    /// through that lookup.
    pub fn is_sfx_source_authorized(&self, source: &PresentationSourceId) -> bool {
        self.current
            .as_ref()
            .is_some_and(|current| current.sfx_sources.contains_key(source))
    }

    pub fn sfx_provider_for_source(&self, source: &PresentationSourceId) -> Option<&str> {
        self.current
            .as_ref()?
            .sfx_sources
            .get(source)
            .map(|source| source.provider_id.as_str())
    }

    pub fn preferred_track(&self) -> Option<&str> {
        self.current
            .as_ref()
            .and_then(|authority| authority.preferred_track.as_deref())
    }

    /// Add one authored presentation source to the current session authority.
    ///
    /// Does not change the session owner or primary music provider. The
    /// character/stage preparation layer supplies the source set; audio owns
    /// only the source-to-provider binding and cue allowlist.
    pub fn authorize_sfx_source(
        &mut self,
        source: impl Into<PresentationSourceId>,
        provider_id: impl Into<String>,
        sfx: Option<SfxRegistry>,
        bank_ids: BTreeSet<SfxId>,
    ) {
        // Split the borrow: the conflict arm needs both fields.
        let conflicts = &mut self.sfx_source_conflicts;
        let Some(current) = self.current.as_mut() else {
            return;
        };
        let source = source.into();
        let provider_id = provider_id.into();
        let candidate = ActiveSfxSource::new(provider_id, sfx, bank_ids, None);
        match current.sfx_sources.get_mut(&source) {
            // The same provider authorizing its own source again: routine,
            // because bank ids arrive asynchronously. Merge, do not replace.
            // The authorized set only grows within a session, so the result
            // does not depend on caller order or bank load time, and an early
            // empty-bank view cannot downgrade a richer one.
            Some(existing) if existing.provider_id == candidate.provider_id => {
                existing.absorb(candidate);
            }
            // A different provider claiming an owned source: a content
            // conflict, and no merge is correct. Record it and keep the first
            // claim, so the result does not depend on iteration order. Not a
            // panic: the worst outcome is that one provider's cues do not
            // resolve.
            Some(existing) => {
                let conflict = SfxSourceClaimConflict {
                    source: source.clone(),
                    holder: existing.provider_id.clone(),
                    rejected: candidate.provider_id.clone(),
                };
                // Recorded, not logged: this crate builds without `bevy_log`.
                // A value can be asserted by tests and reported once by a
                // full-Bevy reporter.
                if !conflicts.contains(&conflict) {
                    conflicts.push(conflict);
                }
            }
            None => {
                current.sfx_sources.insert(source, candidate);
            }
        }
    }

    /// Presentation sources two different providers both tried to claim.
    ///
    /// Empty is the only correct state for shipped content. A non-empty list
    /// means some provider's cues silently do not resolve.
    pub fn sfx_source_conflicts(&self) -> &[SfxSourceClaimConflict] {
        &self.sfx_source_conflicts
    }

    /// Refresh one provider's runtime bank identities after asynchronous load.
    /// Every active source backed by that provider changes together.
    pub fn refresh_provider_sfx_ids(&mut self, provider_id: &str, bank_ids: BTreeSet<SfxId>) {
        let Some(current) = self.current.as_mut() else {
            return;
        };
        for source in current
            .sfx_sources
            .values_mut()
            .filter(|source| source.provider_id == provider_id)
        {
            source.refresh_bank_ids(bank_ids.clone());
        }
    }

    pub fn authorize_adaptive_cues(&mut self, cues: impl IntoIterator<Item = String>) {
        if let Some(current) = self.current.as_mut() {
            current.authorized_cues.extend(cues);
        }
    }

    /// Compatibility view of the primary source authority.
    pub fn sfx_authority(&self) -> SfxAuthority {
        let Some(source) = self.primary_sfx_source() else {
            return SfxAuthority::Denied;
        };
        self.sfx_authority_for_source(source)
    }

    pub fn sfx_authority_for_source(&self, source: &PresentationSourceId) -> SfxAuthority {
        self.current
            .as_ref()
            .and_then(|authority| authority.sfx_sources.get(source))
            .map(|source| SfxAuthority::Governed {
                authorized: source.authorized.clone(),
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
                one_shot: false,
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
    fn one_session_can_authorize_two_independent_presentation_sources() {
        let mut selection = ActiveAudioSelection::default();
        selection.select_gameplay(
            3,
            "ambition",
            None,
            Some(sfx([SoundCueKey::Dash])),
            BTreeSet::new(),
        );
        selection.authorize_sfx_source(
            "sanic.cast",
            "sanic",
            Some(sfx([SoundCueKey::Dash, SoundCueKey::Jump])),
            BTreeSet::new(),
        );

        let ambition = PresentationSourceId::new("ambition");
        let sanic = PresentationSourceId::new("sanic.cast");
        assert_eq!(
            selection.sfx_provider_for_source(&ambition),
            Some("ambition")
        );
        assert_eq!(selection.sfx_provider_for_source(&sanic), Some("sanic"));
        assert!(selection
            .sfx_authority_for_source(&sanic)
            .allows(SoundCueKey::Jump.sfx_id()));
        assert!(
            !selection
                .sfx_authority_for_source(&ambition)
                .allows(SoundCueKey::Jump.sfx_id()),
            "equal cue vocabularies remain source-relative"
        );
        assert_eq!(selection.provider_id(), Some("ambition"));
    }

    /// §4.5 / §3.5: one logical cue id emitted from two sources must resolve
    /// to two providers. `ProviderSfxHandleCache` is keyed
    /// `(provider_id, SfxId)`, so the emission must keep its source. Routed
    /// only by session, Sanic's dash would play Ambition's sound.
    #[test]
    fn cue_resolves_through_its_emitting_source_not_the_active_provider() {
        let mut selection = ActiveAudioSelection::default();
        // An Ambition-owned session (a crossover match: one owner, several sources).
        selection.select_gameplay(
            9,
            "ambition",
            None,
            Some(sfx([SoundCueKey::Dash])),
            BTreeSet::new(),
        );
        selection.authorize_sfx_source(
            "sanic.cast",
            "sanic",
            Some(sfx([SoundCueKey::Dash])),
            BTreeSet::new(),
        );

        let host = PresentationSourceId::new("ambition");
        let guest = PresentationSourceId::new("sanic.cast");
        let dash = SoundCueKey::Dash.sfx_id();

        // Both sources authorize the SAME cue id...
        assert!(selection.sfx_authority_for_source(&host).allows(dash));
        assert!(selection.sfx_authority_for_source(&guest).allows(dash));
        // ...and each resolves against its own provider registry or bank:
        // same id, different sound.
        assert_eq!(selection.sfx_provider_for_source(&host), Some("ambition"));
        assert_eq!(
            selection.sfx_provider_for_source(&guest),
            Some("sanic"),
            "a guest cast member's cue must not resolve through the session's \
             primary provider merely because that provider is the active one"
        );
        // The session owner does not change. Ownership says which session may
        // reach the speakers; source says whose package supplies the cue
        // (§4.5).
        assert_eq!(selection.owner(), Some(AudioContextOwner::Gameplay(9)));
        assert_eq!(selection.provider_id(), Some("ambition"));
    }

    #[test]
    fn an_unknown_source_is_denied_even_when_its_cue_is_primary_authorized() {
        let mut selection = ActiveAudioSelection::default();
        selection.select_gameplay(
            3,
            "ambition",
            None,
            Some(sfx([SoundCueKey::Dash])),
            BTreeSet::new(),
        );
        let unknown = PresentationSourceId::new("not.staged");
        assert_eq!(
            selection.sfx_authority_for_source(&unknown),
            SfxAuthority::Denied
        );
        assert_eq!(selection.sfx_provider_for_source(&unknown), None);
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

#[cfg(test)]
mod source_claim_tests {
    use super::*;

    fn cue(name: &str) -> SfxId {
        SfxId::new(name)
    }

    /// Is this cue playable under this source right now?
    fn allows(selection: &ActiveAudioSelection, source: &str, id: &str) -> bool {
        match selection.sfx_authority_for_source(&PresentationSourceId::new(source)) {
            SfxAuthority::Denied => false,
            SfxAuthority::Governed { authorized } => authorized.contains(&cue(id)),
        }
    }

    fn gameplay() -> ActiveAudioSelection {
        let mut selection = ActiveAudioSelection::default();
        selection.select_gameplay(1, "host", None, None, Default::default());
        selection
    }

    /// Re-authorizing a source as bank ids load is routine, not a conflict.
    #[test]
    fn a_later_view_of_the_same_source_adds_cues_rather_than_crashing() {
        let mut selection = gameplay();
        selection.authorize_sfx_source("sanic", "sanic", None, BTreeSet::from([cue("sanic.dash")]));
        selection.authorize_sfx_source("sanic", "sanic", None, BTreeSet::from([cue("sanic.ring")]));

        assert!(selection.is_sfx_source_authorized(&PresentationSourceId::new("sanic")));
        for id in ["sanic.dash", "sanic.ring"] {
            assert!(
                allows(&selection, "sanic", id),
                "`{id}` was lost: re-authorizing must UNION, or whichever view \
                 arrived last silently narrows what the source may play"
            );
        }
        assert!(selection.sfx_source_conflicts().is_empty());
    }

    /// Order must not matter. The union is true after both views, so an async
    /// bank load is safe.
    #[test]
    fn the_merge_does_not_depend_on_which_view_arrived_first() {
        let ids = |selection: &ActiveAudioSelection| {
            ["a", "b"]
                .into_iter()
                .filter(|id| allows(selection, "sanic", id))
                .count()
        };

        let mut forward = gameplay();
        forward.authorize_sfx_source("sanic", "sanic", None, BTreeSet::from([cue("a")]));
        forward.authorize_sfx_source("sanic", "sanic", None, BTreeSet::from([cue("b")]));

        let mut backward = gameplay();
        backward.authorize_sfx_source("sanic", "sanic", None, BTreeSet::from([cue("b")]));
        backward.authorize_sfx_source("sanic", "sanic", None, BTreeSet::from([cue("a")]));

        assert_eq!(ids(&forward), 2);
        assert_eq!(ids(&backward), 2);
    }

    /// A conflict describes the active selection, not process history. Carried
    /// across a selection change, one bad session would mark every later clean
    /// one.
    #[test]
    fn a_new_selection_does_not_inherit_the_previous_one_s_conflicts() {
        let mut selection = gameplay();
        selection.authorize_sfx_source("shared", "sanic", None, BTreeSet::from([cue("a")]));
        selection.authorize_sfx_source("shared", "mary_o", None, BTreeSet::from([cue("b")]));
        assert_eq!(
            selection.sfx_source_conflicts().len(),
            1,
            "the fixture never produced a conflict, so this proves nothing"
        );

        // Ending the session clears it...
        selection.clear();
        assert!(
            selection.sfx_source_conflicts().is_empty(),
            "a conflict outlived the selection it described"
        );

        // ...and so does selecting a new authority on the same resource. A
        // shell host re-selects between two games without clearing first.
        let mut across_games = gameplay();
        across_games.authorize_sfx_source("shared", "sanic", None, BTreeSet::from([cue("a")]));
        across_games.authorize_sfx_source("shared", "mary_o", None, BTreeSet::from([cue("b")]));
        assert_eq!(across_games.sfx_source_conflicts().len(), 1);
        across_games.select_gameplay(2, "host", None, None, Default::default());
        assert!(
            across_games.sfx_source_conflicts().is_empty(),
            "the next game's audio authority reported the previous game's conflict"
        );
    }

    /// A real conflict is not merged. Two providers under one source identity
    /// would resolve every cue to one of them. The first claim holds, and the
    /// conflict is recorded, not fatal.
    #[test]
    fn two_providers_claiming_one_source_is_recorded_and_the_first_holds() {
        let mut selection = gameplay();
        selection.authorize_sfx_source("shared", "sanic", None, BTreeSet::from([cue("a")]));
        selection.authorize_sfx_source("shared", "mary_o", None, BTreeSet::from([cue("b")]));

        let conflicts = selection.sfx_source_conflicts();
        assert_eq!(
            conflicts.len(),
            1,
            "the conflict must be reported: {conflicts:?}"
        );
        assert_eq!(conflicts[0].holder, "sanic");
        assert_eq!(conflicts[0].rejected, "mary_o");

        assert!(allows(&selection, "shared", "a"));
        assert!(
            !allows(&selection, "shared", "b"),
            "the rejected provider's cues must NOT be merged in — that is the \
             difference between a late bank and two providers colliding"
        );
    }

    /// Reported once, not once per tick: the authorizer runs every frame, and
    /// an unbounded list would leak.
    #[test]
    fn a_repeated_conflict_is_recorded_once() {
        let mut selection = gameplay();
        for _ in 0..5 {
            selection.authorize_sfx_source("shared", "sanic", None, BTreeSet::new());
            selection.authorize_sfx_source("shared", "mary_o", None, BTreeSet::new());
        }
        assert_eq!(selection.sfx_source_conflicts().len(), 1);
    }
}
