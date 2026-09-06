//! Provider-qualified SFX bank loading and playback.
//!
//! A Bevy App may cache banks for many linked providers. Playback never reads a
//! process-global "current bank": it resolves through the active audio
//! context's provider and the request's captured owner.

use std::collections::BTreeMap;
use std::sync::Arc;

use ambition_sfx::{BankProvider, OwnedSfxMessage, SfxError, SfxId, SfxProvider};
use bevy::asset::{
    io::Reader, Asset, AssetApp, AssetLoader, AssetServer, Assets, Handle, LoadContext, LoadState,
};
use bevy::log::{debug, info};
use bevy::prelude::{
    App, Commands, Local, MessageReader, Plugin, Res, ResMut, Resource, Startup, Update,
};
use bevy::reflect::TypePath;
use bevy_kira_audio::prelude::{AudioChannel, AudioControl, AudioSource as KiraAudioSource};

use crate::catalog::SfxBankRegistry;
use crate::library::{sfx_message_target_id, SfxChannel};
use crate::render::{ProviderSfxHandleCache, SfxPlaybackRecord, SfxPlaybackState, SfxSourceMiss};
use crate::selection::ActiveAudioSelection;
use crate::spec::SfxRegistry;
use crate::web_unlock::AUDIO_LOG_TARGET;

/// Host-supplied provider-qualified asset path for one packed bank.
#[derive(Resource, Clone, Debug)]
pub struct SfxBankAssetPath {
    pub provider_id: String,
    pub asset_path: String,
}

impl SfxBankAssetPath {
    pub fn new(provider_id: impl Into<String>, asset_path: impl Into<String>) -> Self {
        let provider_id = provider_id.into();
        assert!(
            !provider_id.trim().is_empty(),
            "SFX bank provider id cannot be empty"
        );
        Self {
            provider_id,
            asset_path: asset_path.into(),
        }
    }
}

/// Provider-composable packed-bank paths. Hosts may register any number of
/// linked providers; the loader caches all of them while playback remains
/// governed by the active audio context.
#[derive(Resource, Clone, Debug, Default)]
pub struct SfxBankAssetCatalog {
    paths: BTreeMap<String, String>,
}

impl SfxBankAssetCatalog {
    pub fn register(
        &mut self,
        provider_id: impl Into<String>,
        asset_path: impl Into<String>,
    ) -> Result<(), String> {
        let provider_id = provider_id.into();
        let asset_path = asset_path.into();
        if provider_id.trim().is_empty() || asset_path.trim().is_empty() {
            return Err("SFX bank provider and asset path must not be empty".to_owned());
        }
        if let Some(existing) = self.paths.get(&provider_id) {
            if existing == &asset_path {
                return Ok(());
            }
            return Err(format!(
                "SFX bank provider '{provider_id}' registered both '{existing}' and '{asset_path}'"
            ));
        }
        self.paths.insert(provider_id, asset_path);
        Ok(())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.paths
            .iter()
            .map(|(provider, path)| (provider.as_str(), path.as_str()))
    }
}

pub trait SfxBankAssetAppExt {
    fn register_sfx_bank_asset(
        &mut self,
        provider_id: impl Into<String>,
        asset_path: impl Into<String>,
    ) -> &mut Self;
}

impl SfxBankAssetAppExt for App {
    fn register_sfx_bank_asset(
        &mut self,
        provider_id: impl Into<String>,
        asset_path: impl Into<String>,
    ) -> &mut Self {
        self.world_mut()
            .get_resource_or_insert_with(SfxBankAssetCatalog::default)
            .register(provider_id, asset_path)
            .unwrap_or_else(|error| panic!("{error}"));
        self
    }
}

/// Runtime banks indexed by provider. Cached storage does not confer authority;
/// [`ActiveAudioSelection`] chooses which provider may resolve a request.
#[derive(Resource, Clone, Default)]
pub struct SfxBankResource {
    providers: BTreeMap<String, Arc<BankProvider>>,
}

impl SfxBankResource {
    pub fn register(
        &mut self,
        provider_id: impl Into<String>,
        provider: Arc<BankProvider>,
    ) -> Result<(), String> {
        let provider_id = provider_id.into();
        if let Some(existing) = self.providers.get(&provider_id) {
            let existing_fingerprints = existing.content_fingerprints();
            let incoming_fingerprints = provider.content_fingerprints();
            if existing_fingerprints == incoming_fingerprints {
                return Ok(());
            }
            return Err(format!(
                "provider '{provider_id}' attempted to replace its loaded SFX bank with different content"
            ));
        }
        self.providers.insert(provider_id, provider);
        Ok(())
    }

    pub fn provider(&self, provider_id: &str) -> Option<&dyn SfxProvider> {
        self.providers
            .get(provider_id)
            .map(|provider| provider.as_ref() as &dyn SfxProvider)
    }

    /// The authored spelling of `id` according to ANY loaded bank.
    ///
    /// Deliberately not scoped to one provider: the caller asking is the one
    /// reporting that a cue is missing HERE, so the only banks that can name it
    /// are the ones that do have it. "Sanic has no clip for `boss.shatter`" is
    /// the sentence worth printing, and only Ambition's bank can supply the
    /// `boss.shatter` half of it.
    pub fn name_anywhere(&self, id: SfxId) -> Option<&str> {
        self.providers
            .values()
            .find_map(|provider| provider.name_for(id))
    }

    pub fn ids_for(&self, provider_id: &str) -> std::collections::BTreeSet<SfxId> {
        self.providers
            .get(provider_id)
            .map(|provider| provider.iter_ids().map(|(id, _)| id).collect())
            .unwrap_or_default()
    }

    pub fn fingerprints_for(&self, provider_id: &str) -> BTreeMap<SfxId, u64> {
        self.providers
            .get(provider_id)
            .map(|provider| provider.content_fingerprints())
            .unwrap_or_default()
    }

    pub fn fingerprint_for(&self, provider_id: &str, id: SfxId) -> Option<u64> {
        self.providers
            .get(provider_id)
            .and_then(|provider| provider.fingerprint_of(id))
    }

    /// Human-readable name for a bank id, when the bank ships a name section.
    pub fn name_for(&self, provider_id: &str, id: SfxId) -> Option<&str> {
        self.providers
            .get(provider_id)
            .and_then(|provider| provider.name_for(id))
    }
}

#[derive(Asset, TypePath)]
pub struct SfxBankAsset {
    pub provider: Arc<BankProvider>,
}

#[derive(Default, TypePath)]
pub struct SfxBankLoader;

#[derive(Debug)]
pub enum SfxBankLoaderError {
    Io(std::io::Error),
    Sfx(SfxError),
}

impl std::fmt::Display for SfxBankLoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "io: {error}"),
            Self::Sfx(error) => write!(f, "sfx bank: {error}"),
        }
    }
}

impl std::error::Error for SfxBankLoaderError {}

impl From<std::io::Error> for SfxBankLoaderError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<SfxError> for SfxBankLoaderError {
    fn from(error: SfxError) -> Self {
        Self::Sfx(error)
    }
}

impl AssetLoader for SfxBankLoader {
    type Asset = SfxBankAsset;
    type Settings = ();
    type Error = SfxBankLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _context: &mut LoadContext<'_>,
    ) -> Result<SfxBankAsset, SfxBankLoaderError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(SfxBankAsset {
            provider: Arc::new(BankProvider::from_bytes(bytes)?),
        })
    }

    fn extensions(&self) -> &[&str] {
        &["bank", "sfxbank"]
    }
}

#[derive(Resource, Default)]
pub struct PendingSfxBankHandles {
    handles: BTreeMap<String, Handle<SfxBankAsset>>,
}

impl PendingSfxBankHandles {
    /// This provider's bank is still in flight, so a cue that misses right now
    /// may well play a moment later. The difference between "not yet" and
    /// "never" is the whole value of the miss diagnostic.
    pub fn is_loading(&self, provider_id: &str, asset_server: &AssetServer) -> bool {
        self.handles
            .get(provider_id)
            .is_some_and(|handle| asset_server.load_state(handle).is_loading())
    }
}

/// Provider bank assets that reached a terminal load failure.
///
/// Kept after the pending handle is removed so later cue diagnostics can say
/// "the bank failed" instead of regressing to "no bank is registered yet".
#[derive(Resource, Default)]
pub struct FailedSfxBankLoads {
    errors: BTreeMap<String, String>,
}

pub struct SfxBankAssetPlugin;

impl Plugin for SfxBankAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<SfxBankAsset>()
            .register_asset_loader(SfxBankLoader)
            .init_resource::<SfxBankResource>()
            .init_resource::<SfxBankAssetCatalog>()
            .init_resource::<SfxBankRegistry>()
            .init_resource::<ProviderSfxHandleCache>()
            .init_resource::<SfxPlaybackState>()
            .init_resource::<FailedSfxBankLoads>()
            .add_systems(Startup, kick_off_bank_load)
            .add_systems(Update, promote_loaded_sfx_bank)
            .add_systems(Update, warn_on_sfx_playback_spam);
    }
}

/// A sustained accepted-playback rate above this is treated as a runaway
/// emitter, not gameplay. Dense combat peaks around ~10 cues/second.
const SFX_SPAM_WARN_PER_SECOND: f32 = 25.0;

/// Once-per-second watchdog over [`SfxPlaybackState::accepted_playbacks`]:
/// when the accepted rate is abnormal, log it with the most recent record —
/// resolved to its bank name — so a runaway emitter names itself in the log
/// instead of being an audible mystery ("walking into the boss room fires
/// insane SFX", desktop-lifecycle-3, no trace in any log).
fn warn_on_sfx_playback_spam(
    state: Res<SfxPlaybackState>,
    banks: Res<SfxBankResource>,
    time: Res<bevy::prelude::Time>,
    mut window: Local<Option<(f32, u64)>>,
) {
    let Some((elapsed, prev_accepted)) = window.as_mut() else {
        *window = Some((0.0, state.accepted_playbacks));
        return;
    };
    *elapsed += time.delta_secs();
    if *elapsed < 1.0 {
        return;
    }
    let rate = state.accepted_playbacks.saturating_sub(*prev_accepted) as f32 / *elapsed;
    *window = Some((0.0, state.accepted_playbacks));
    if rate < SFX_SPAM_WARN_PER_SECOND {
        return;
    }
    let last = state.last_played.as_ref();
    let name = last
        .and_then(|record| banks.name_for(&record.provider_id, record.id))
        .unwrap_or("<unnamed>");
    bevy::log::warn!(
        target: AUDIO_LOG_TARGET,
        "sfx playback spam: {rate:.0} accepted/s (sane peak ~10/s); most recent: '{name}' {} owner={:?} source={:?}",
        last.map(|r| r.id.to_string()).unwrap_or_default(),
        last.map(|r| r.owner),
        last.map(|r| &r.source),
    );
}

fn kick_off_bank_load(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    catalog: Res<SfxBankAssetCatalog>,
    legacy_path: Option<Res<SfxBankAssetPath>>,
    banks: Res<SfxBankResource>,
) {
    let mut requested = BTreeMap::<String, String>::new();
    for (provider, path) in catalog.iter() {
        requested.insert(provider.to_owned(), path.to_owned());
    }
    if let Some(path) = legacy_path {
        match requested.get(&path.provider_id) {
            Some(existing) if existing != &path.asset_path => panic!(
                "provider '{}' has conflicting SFX bank paths '{}' and '{}'",
                path.provider_id, existing, path.asset_path
            ),
            _ => {
                requested.insert(path.provider_id.clone(), path.asset_path.clone());
            }
        }
    }
    let mut pending = PendingSfxBankHandles::default();
    for (provider_id, asset_path) in requested {
        if banks.provider(&provider_id).is_some() {
            continue;
        }
        info!(
            target: AUDIO_LOG_TARGET,
            "ambition audio: loading provider '{}' SFX bank from '{}'",
            provider_id,
            asset_path,
        );
        pending
            .handles
            .insert(provider_id, asset_server.load(asset_path));
    }
    if pending.handles.is_empty() {
        debug!(target: AUDIO_LOG_TARGET, "ambition audio: no provider SFX banks requested");
    } else {
        commands.insert_resource(pending);
    }
}

/// Promote a late bank transactionally and refresh the live context only when
/// it belongs to the same provider. Missing handles are never cached, so the
/// active session can resolve the bank immediately after this system runs.
fn promote_loaded_sfx_bank(
    mut commands: Commands,
    pending: Option<ResMut<PendingSfxBankHandles>>,
    asset_server: Res<AssetServer>,
    assets: Res<Assets<SfxBankAsset>>,
    mut banks: ResMut<SfxBankResource>,
    mut bank_ids: ResMut<SfxBankRegistry>,
    mut selection: ResMut<ActiveAudioSelection>,
    mut failed_banks: ResMut<FailedSfxBankLoads>,
) {
    let Some(mut pending) = pending else {
        return;
    };
    let failed: Vec<(String, String)> = pending
        .handles
        .iter()
        .filter_map(|(provider, handle)| match asset_server.load_state(handle) {
            LoadState::Failed(error) => Some((provider.clone(), format!("{error:?}"))),
            _ => None,
        })
        .collect();
    for (provider_id, error) in failed {
        let handle = pending
            .handles
            .remove(&provider_id)
            .expect("failed provider handle remains pending");
        let path = asset_server
            .get_path(handle.id())
            .map(|path| path.to_string())
            .unwrap_or_else(|| "<no path>".to_owned());
        bevy::log::error!(
            target: AUDIO_LOG_TARGET,
            "ambition audio: provider '{provider_id}' SFX bank '{path}' failed to load: {error}",
        );
        failed_banks
            .errors
            .insert(provider_id, format!("{path}: {error}"));
    }
    let ready: Vec<String> = pending
        .handles
        .iter()
        .filter_map(|(provider, handle)| assets.get(handle).map(|_| provider.clone()))
        .collect();
    for provider_id in ready {
        let handle = pending
            .handles
            .remove(&provider_id)
            .expect("ready provider handle remains pending");
        let asset = assets
            .get(&handle)
            .expect("ready provider bank asset remains available");
        let provider = asset.provider.clone();
        failed_banks.errors.remove(&provider_id);
        let fingerprints = provider.content_fingerprints();
        bank_ids
            .register(provider_id.clone(), fingerprints)
            .unwrap_or_else(|error| panic!("provider SFX bank composition failed: {error}"));
        banks
            .register(provider_id.clone(), provider)
            .unwrap_or_else(|error| panic!("provider SFX bank promotion failed: {error}"));
        selection.refresh_provider_sfx_ids(&provider_id, bank_ids.ids_for(&provider_id));
        info!(
            target: AUDIO_LOG_TARGET,
            "ambition audio: provider '{}' SFX bank is ready",
            provider_id,
        );
    }
    if pending.handles.is_empty() {
        commands.remove_resource::<PendingSfxBankHandles>();
    }
}

/// Name a cue for a human, from whatever authority can still spell it.
///
/// An [`SfxId`] is a one-way hash, so a diagnostic holding only the id prints
/// `SfxId(0x…)` — which names nothing an author can grep for. Three authorities
/// between them know almost every spelling in the game: the engine's `ids` table
/// (every typed cue), the active procedural registry (open ids authored in RON),
/// and the name section of every loaded bank (open ids some OTHER provider
/// packs). A cue none of them knows is genuinely anonymous, and the message says
/// so rather than pretending the hash is an answer.
fn describe_sfx_id(id: SfxId, procedural: Option<&SfxRegistry>, banks: &SfxBankResource) -> String {
    let name = ambition_sfx::ids::name_of(id)
        .or_else(|| banks.name_anywhere(id))
        .map(str::to_owned)
        .or_else(|| {
            procedural?
                .sfx
                .iter()
                .find(|spec| spec.sfx_id().ok() == Some(id))
                .and_then(|spec| spec.id.clone())
        });
    match name {
        Some(name) => format!("`{name}` ({id})"),
        None => format!("{id} (no loaded bank or registry knows its authored name)"),
    }
}

pub fn audio_play_sfx_messages(
    mut messages: MessageReader<OwnedSfxMessage>,
    selection: Res<ActiveAudioSelection>,
    banks: Res<SfxBankResource>,
    pending: Option<Res<PendingSfxBankHandles>>,
    failed_banks: Res<FailedSfxBankLoads>,
    asset_server: Res<AssetServer>,
    sfx_channel: Res<AudioChannel<SfxChannel>>,
    output: Option<Res<crate::output::AudioOutputMode>>,
    mut cache: ResMut<ProviderSfxHandleCache>,
    mut audio_sources: ResMut<Assets<KiraAudioSource>>,
    mut playback: ResMut<SfxPlaybackState>,
    mut first_play_logged: Local<bool>,
) {
    for owned in messages.read() {
        let request = owned.request;
        if !*first_play_logged {
            info!(
                target: AUDIO_LOG_TARGET,
                "ambition audio: first owned SFX play attempt (owner={:?})",
                owned.owner,
            );
            *first_play_logged = true;
        }
        if !selection.accepts_request_owner(owned.owner) {
            playback.rejected_wrong_owner = playback.rejected_wrong_owner.saturating_add(1);
            continue;
        }
        let Some(owner) = owned.owner else {
            playback.rejected_wrong_owner = playback.rejected_wrong_owner.saturating_add(1);
            continue;
        };
        let source = &owned.source;
        let Some(provider_id) = selection.sfx_provider_for_source(source) else {
            playback.rejected_unauthorized = playback.rejected_unauthorized.saturating_add(1);
            continue;
        };
        let id = sfx_message_target_id(request);
        if !selection.sfx_authority_for_source(source).allows(id) {
            playback.rejected_unauthorized = playback.rejected_unauthorized.saturating_add(1);
            continue;
        }
        let source_registry = selection.sfx_for_source(source);
        let resolved = cache.handle_for(
            provider_id,
            id,
            source_registry,
            banks.provider(provider_id),
            banks.fingerprint_for(provider_id, id),
            audio_sources.as_mut(),
        );
        let resolved = match resolved {
            Err(SfxSourceMiss::NoProviderBank) if failed_banks.errors.contains_key(provider_id) => {
                Err(SfxSourceMiss::BankLoadFailed)
            }
            other => other,
        };
        let resolved = match resolved {
            Ok(resolved) => resolved,
            Err(miss) => {
                // Name it, and say WHY, once per (provider, cue, diagnosis). The
                // counter said a cue went silent and never which one; the first
                // version of this warning named the cue but asserted one cause
                // for four different failures, including a cue that was merely
                // early. A wrong diagnosis costs more than no diagnosis.
                let first_word = playback.note_missing_source(provider_id, id, miss);
                if first_word {
                    let cue = describe_sfx_id(id, source_registry, &banks);
                    let outlook = if miss == SfxSourceMiss::NoProviderBank
                        && pending
                            .as_deref()
                            .is_some_and(|pending| pending.is_loading(provider_id, &asset_server))
                    {
                        "its bank is still loading, so this may resolve itself"
                    } else {
                        "it will stay silent until the content changes"
                    };
                    bevy::log::warn!(
                        target: AUDIO_LOG_TARGET,
                        "ambition audio: provider '{provider_id}' cannot play cue {cue} — \
                         {miss}; {outlook}",
                    );
                }
                continue;
            }
        };
        if crate::output::emits_to_device(output.as_deref()) {
            sfx_channel.play(resolved.handle);
        }
        playback.accepted_playbacks = playback.accepted_playbacks.saturating_add(1);
        playback.last_played = Some(SfxPlaybackRecord {
            owner,
            presentation_source: source.clone(),
            provider_id: provider_id.to_owned(),
            id,
            source: resolved.source,
        });
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::time::{Duration, Instant};

    use ambition_sfx::{AudioContextOwner, OwnedSfxMessage, SfxId, SfxMessage};
    use bevy::asset::AssetPlugin;
    use bevy::math::Vec2;
    use bevy::prelude::{App, MinimalPlugins};

    use super::{
        describe_sfx_id, FailedSfxBankLoads, PendingSfxBankHandles, SfxBankAssetAppExt,
        SfxBankAssetPlugin, SfxBankResource,
    };
    use crate::catalog::SfxBankRegistry;
    use crate::selection::ActiveAudioSelection;
    use crate::spec::{SfxRegistry, SfxSpec, SoundCueKey, WaveformSpec};

    fn sfx_registry(cue: SoundCueKey) -> SfxRegistry {
        SfxRegistry {
            sample_rate: 44_100,
            sfx: vec![SfxSpec {
                cue: Some(cue),
                id: None,
                waveform: WaveformSpec::Square,
                frequency: 330.0,
                frequency_end: 660.0,
                duration: 0.1,
                volume: 0.5,
                attack: 0.0,
                release: 0.02,
                noise: 0.0,
            }],
        }
    }

    #[test]
    fn failed_bank_asset_leaves_pending_and_records_a_terminal_failure() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_resource::<ActiveAudioSelection>()
            .register_sfx_bank_asset(
                "broken_provider",
                "__ambition_test_missing__/broken_provider.sfxbank",
            )
            .add_plugins(SfxBankAssetPlugin);

        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline
            && !app
                .world()
                .resource::<FailedSfxBankLoads>()
                .errors
                .contains_key("broken_provider")
        {
            app.update();
            std::thread::sleep(Duration::from_millis(1));
        }

        assert!(
            app.world()
                .resource::<FailedSfxBankLoads>()
                .errors
                .contains_key("broken_provider"),
            "a failed bank must become terminal diagnostic state"
        );
        assert!(
            app.world()
                .get_resource::<PendingSfxBankHandles>()
                .map_or(true, |pending| {
                    !pending.handles.contains_key("broken_provider")
                }),
            "a failed handle must not remain classified as loading forever"
        );
    }

    #[test]
    fn same_provider_relaunch_rejects_the_old_owner() {
        let mut selection = ActiveAudioSelection::default();
        selection.select_gameplay(
            2,
            "sanic",
            None,
            Some(sfx_registry(SoundCueKey::Dash)),
            BTreeSet::new(),
        );
        let stale = OwnedSfxMessage {
            owner: Some(AudioContextOwner::Gameplay(1)),
            source: "sanic".into(),
            request: SfxMessage::Dash { pos: Vec2::ZERO },
        };
        assert!(!selection.accepts_request_owner(stale.owner));
    }

    /// The suppression key is the fact, and the fact has three parts: which
    /// provider, which cue, and what went wrong. Keyed on the cue alone (as it
    /// first shipped), the second provider's identical silence was invisible,
    /// and a cue that missed because its bank had not loaded yet stayed
    /// diagnosed that way after the bank arrived and proved it truly absent.
    #[test]
    fn a_miss_speaks_once_per_provider_and_again_when_the_diagnosis_changes() {
        use crate::render::{SfxPlaybackState, SfxSourceMiss};
        let cue = SfxId::from_static("boss.shatter");
        let mut playback = SfxPlaybackState::default();

        assert!(playback.note_missing_source("ambition", cue, SfxSourceMiss::NotInBank));
        assert!(!playback.note_missing_source("ambition", cue, SfxSourceMiss::NotInBank));
        assert!(
            playback.note_missing_source("sanic", cue, SfxSourceMiss::NotInBank),
            "another provider's bank is another fact"
        );
        assert!(
            playback.note_missing_source("ambition", cue, SfxSourceMiss::DecodeFailed),
            "the reason changed, so the earlier line was wrong and must be corrected"
        );
        assert_eq!(playback.missing_source, 4, "every miss still counts");
    }

    /// A hash names nothing. The engine's id table is the authority for typed
    /// cues; the loaded banks' name sections cover open provider-local ids that
    /// some OTHER provider packs — which is exactly the case being reported.
    #[test]
    fn a_missing_cue_is_named_not_hashed() {
        let banks = SfxBankResource::default();
        let described = describe_sfx_id(ambition_sfx::ids::PLAYER_LAND, None, &banks);
        assert!(described.starts_with("`player.land`"), "{described}");

        let anonymous = describe_sfx_id(SfxId::from_static("nobody.packs.this"), None, &banks);
        assert!(
            anonymous.contains("no loaded bank or registry knows"),
            "an unnamed cue says so rather than passing a hash off as an answer: {anonymous}"
        );
    }

    #[test]
    fn bank_registry_accepts_benign_shared_content() {
        let id = SfxId::from_static("shared");
        let mut registry = SfxBankRegistry::default();
        registry.register("a", BTreeMap::from([(id, 7)])).unwrap();
        registry.register("b", BTreeMap::from([(id, 7)])).unwrap();
        assert_eq!(registry.ids_for("a"), BTreeSet::from([id]));
        assert_eq!(registry.ids_for("b"), BTreeSet::from([id]));
    }
}
