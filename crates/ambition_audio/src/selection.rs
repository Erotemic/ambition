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
/// The profile is explicit rather than an exception to gameplay authority. A
/// launcher/startup/loading/select route may own one title track and a narrow
/// menu-SFX allowlist. The provider supplies the actual source definitions; the
/// declaration chooses which subset belongs to that screen.
///
/// Declarations now live in [`FrontendAudioRegistry`], keyed by route.
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

/// Every frontend audio DECLARATION in this App, keyed by the route that owns
/// it — plus the one profile currently in effect.
///
/// Two kinds of entry, because there are two honest claims:
///
/// * a route declaration — "this screen sounds like this", made by whoever
///   authored the screen, and it travels with the provider into any host;
/// * the host default — "screens I own sound like this", made once by the
///   host for its own launcher/startup/loading routes.
///
/// A route with no declaration of its own falls back to the host default. A host
/// with no default either leaves that route deliberately silent rather than
/// inheriting somebody else's music.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct FrontendAudioRegistry {
    by_route: BTreeMap<String, FrontendAudioProfile>,
    host_default: Option<FrontendAudioProfile>,
    /// The resolved profile of the most recent frontend route — what frontend
    /// playback is acting on.
    ///
    /// Not cleared when that route deactivates. The reason is narrower than
    /// it first looks, and the difference is worth stating because the obvious
    /// justification is wrong.
    ///
    /// So this is a cheap precaution, NOT a demonstrated fix, and it is written down that way
    /// rather than as a claim a test backs. If a route change ever does span frames — a frontend
    /// route behind a real load barrier is the candidate — this is already correct and no test had
    /// to predict it.
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
    /// Returns the resolved profile so the caller does not have to ask twice —
    /// the two-step form is how an authority grows a follow-up call nobody
    /// remembers to make.
    pub fn enter_route(&mut self, route_id: &str) -> Option<&FrontendAudioProfile> {
        self.in_effect = self.resolve(route_id).cloned();
        self.in_effect.as_ref()
    }

    /// The profile frontend playback is acting on. See [`Self::in_effect`]'s
    /// field docs for why this outlives the activation that selected it.
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
/// Mirrors [`crate::catalog::AudioCatalogAppExt`]: a provider states what its
/// own screens sound like beside the audio fragment it already registers, and
/// composing that provider into any host carries the declaration with it.
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
    /// Presentation sources two different providers both tried to claim.
    ///
    /// Recorded rather than fatal. It also could not tell that case apart from the ROUTINE one: the
    /// same provider re-authorizing its own source as asynchronously-loaded bank ids arrive.
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
/// backing procedural registry/bank. They are separate so a future prepared
/// character or stage may expose a stable package id without coupling every
/// emitter to storage details.
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

    /// Fold another view of the SAME source into this one.
    ///
    /// Union on the authorized set and prefer a present registry over an absent
    /// one, so the result is independent of which view arrived first. Two callers
    /// looking at one source at different moments of an async bank load are both
    /// telling the truth about a different instant; the union is the only answer
    /// that is true at every instant after both.
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
        // A new authority starts with a clean conflict list. The conflicts
        // describe cues that fail to resolve UNDER THE ACTIVE SELECTION — that
        // is what the accessor's own documentation promises — so carrying them
        // across a selection change turns a live diagnostic into historical
        // residue, and every later clean session reports the first bad one
        // forever.
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
    /// Distinct from [`Self::sfx_for_source`], which answers "and does it have a
    /// PROCEDURAL registry" — a source authorized with `sfx: None` (bank-only, or a
    /// provider whose catalog has not loaded) is fully authorized and would read as
    /// unauthorized through that lookup. Conflating the two makes a legitimately
    /// registry-free provider look denied.
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
    /// This does not change the session owner or primary music provider. The
    /// character/stage preparation layer supplies the exact source set; audio
    /// owns only the stable source-to-provider binding and cue allowlist.
    pub fn authorize_sfx_source(
        &mut self,
        source: impl Into<PresentationSourceId>,
        provider_id: impl Into<String>,
        sfx: Option<SfxRegistry>,
        bank_ids: BTreeSet<SfxId>,
    ) {
        // Split the borrow up front: the conflict list and the authority are
        // independent fields, and the conflict arm needs both.
        let conflicts = &mut self.sfx_source_conflicts;
        let Some(current) = self.current.as_mut() else {
            return;
        };
        let source = source.into();
        let provider_id = provider_id.into();
        let candidate = ActiveSfxSource::new(provider_id, sfx, bank_ids, None);
        match current.sfx_sources.get_mut(&source) {
            // The SAME provider authorizing its own source again. Legitimate and routine: bank ids
            // arrive asynchronously, so two callers on two ticks hold two honest views of the same
            // source.
            //
            // Merged, not replaced, and that is the load-bearing choice: the
            // authorized set only GROWS within a session, so the outcome does not
            // depend on which caller ran first or on when a bank finished loading.
            // Replacement would let an early empty-bank view silently downgrade a
            // richer one.
            Some(existing) if existing.provider_id == candidate.provider_id => {
                existing.absorb(candidate);
            }
            // A DIFFERENT provider claiming a source another already owns. That is
            // a content conflict, not a timing artifact, and no merge is correct:
            // two providers' cue tables under one identity means every cue
            // resolves to whichever won.
            //
            // Loud and deterministic rather than fatal. A panic here kills a
            // running game over a misconfiguration whose worst honest outcome is
            // one provider's cues not resolving — and the same judgement the
            // binding boundary makes (a placeholder beats a session that refuses
            // to boot). FIRST wins, so the result does not depend on iteration
            // order.
            Some(existing) => {
                let conflict = SfxSourceClaimConflict {
                    source: source.clone(),
                    holder: existing.provider_id.clone(),
                    rejected: candidate.provider_id.clone(),
                };
                // RECORDED, not logged. This crate builds without `bevy_log`
                // (default features off), and a value beats a log line anyway: a
                // test can assert on it, and a tick-scoped reporter in a
                // full-Bevy crate can surface each conflict ONCE instead of
                // every frame.
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

    /// §4.5, and §3.5's single point of loss.
    ///
    /// `ProviderSfxHandleCache` was already keyed `(provider_id, SfxId)`, so the resolution table
    /// was source-qualified the whole time and only the emission had lost the emitter.
    ///
    /// So the sharp case is ONE logical cue id emitted from two sources. Under the
    /// old routing both resolved to the session's primary provider and Sanic's dash
    /// played Ambition's sound. Here they must resolve to different providers.
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
        // ...and each resolves against its OWN provider registry/bank, which is the
        // whole point: same id, different sound.
        assert_eq!(selection.sfx_provider_for_source(&host), Some("ambition"));
        assert_eq!(
            selection.sfx_provider_for_source(&guest),
            Some("sanic"),
            "a guest cast member's cue must not resolve through the session's \
             primary provider merely because that provider is the active one"
        );
        // The session owner is unchanged by any of this: ownership says which live
        // session may reach the speakers, source says whose package supplies the
        // cue, and overloading one into the other is what §4.5 forbids.
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

    /// The routine case that used to be a panic.
    ///
    /// Bank ids load asynchronously, so two callers on two ticks hold two honest views of one
    /// source.
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

    /// Order must not matter. A union is the only merge that is true at every
    /// instant after both views, which is what makes an async bank load safe.
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

    /// A conflict describes the ACTIVE selection, not the history of the process.
    ///
    /// `sfx_source_conflicts` documents itself as "a provider's cues currently
    /// fail to resolve". Carrying the list across a selection change makes that
    /// sentence false: one bad session poisons every later clean one, and the
    /// diagnostic quietly becomes residue nobody can act on.
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

        // ...and so does selecting a NEW authority on the same resource, which
        // is the path a shell host takes between two games — it re-selects
        // rather than clearing first.
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

    /// The genuine conflict, which is NOT a merge.
    ///
    /// Two providers under one source identity means every cue resolves to
    /// whichever won. Recorded and deterministic — first claim holds — rather
    /// than fatal: a panic kills a running game over a misconfiguration whose
    /// worst honest outcome is one provider's cues not resolving.
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

    /// Reported once, not once per tick. The production authorizer runs every
    /// frame, and a conflict list that grows without bound is a leak and a log
    /// nobody can read.
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
