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

    /// The authored name of `id` in any loaded bank.
    ///
    /// Not scoped to one provider: the caller reports that a cue is missing
    /// for its provider, so only another provider's bank can name it (for
    /// example, "Sanic has no clip for `boss.shatter`").
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
pub struct SfxBankHandles {
    handles: BTreeMap<String, Handle<SfxBankAsset>>,
    admitted: BTreeMap<String, Arc<BankProvider>>,
    rejected: BTreeMap<String, Arc<BankProvider>>,
}

impl SfxBankHandles {
    /// This provider's bank is still loading, so a cue that misses now may
    /// play soon. The miss diagnostic depends on "not yet" versus "never".
    pub fn is_loading(&self, provider_id: &str, asset_server: &AssetServer) -> bool {
        self.handles
            .get(provider_id)
            .is_some_and(|handle| asset_server.load_state(handle).is_loading())
    }
}

/// Provider bank assets that reached a terminal load failure.
///
/// A failed reload keeps the accepted bank available.
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
/// when the accepted rate is abnormal, log it with the most recent record
/// (resolved to its bank name), so a runaway emitter shows in the log.
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
    audio_catalogs: Option<Res<crate::catalog::AudioCatalogRegistry>>,
) {
    let mut requested = BTreeMap::<String, String>::new();
    for (provider, path) in catalog.iter() {
        requested.insert(provider.to_owned(), path.to_owned());
    }
    if let Some(path) = legacy_path {
        // The resident bank serves the host's own provider and every provider
        // whose audio fragment declares it.
        let declared = audio_catalogs
            .iter()
            .flat_map(|catalogs| catalogs.resident_sfx_bank_users())
            .map(str::to_owned);
        for provider_id in std::iter::once(path.provider_id.clone()).chain(declared) {
            match requested.get(&provider_id) {
                Some(existing) if existing != &path.asset_path => panic!(
                    "provider '{}' has conflicting SFX bank paths '{}' and '{}'",
                    provider_id, existing, path.asset_path
                ),
                _ => {
                    requested.insert(provider_id, path.asset_path.clone());
                }
            }
        }
    }
    let mut pending = SfxBankHandles::default();
    for (provider_id, asset_path) in requested {
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

/// Retain loaded handles and admit changed banks as one transaction.
fn promote_loaded_sfx_bank(
    handles: Option<ResMut<SfxBankHandles>>,
    asset_server: Res<AssetServer>,
    assets: Res<Assets<SfxBankAsset>>,
    mut banks: ResMut<SfxBankResource>,
    mut bank_ids: ResMut<SfxBankRegistry>,
    mut selection: ResMut<ActiveAudioSelection>,
    mut failed_banks: ResMut<FailedSfxBankLoads>,
) {
    let Some(mut handles) = handles else { return; };
    let mut ready = BTreeMap::new();
    for (provider, handle) in &handles.handles {
        if let Some(asset) = assets.get(handle) {
            if !handles.admitted.get(provider).is_some_and(|old| Arc::ptr_eq(old, &asset.provider)) {
                ready.insert(provider.clone(), asset.provider.clone());
            }
        } else if let LoadState::Failed(error) = asset_server.load_state(handle) {
            if !failed_banks.errors.contains_key(provider) {
                let message = format!("SFX bank failed to load: {error:?}");
                bevy::log::error!(target: AUDIO_LOG_TARGET, "provider '{provider}': {message}");
                failed_banks.errors.insert(provider.clone(), message);
            }
        }
    }
    if ready.is_empty() || (ready.len() == handles.rejected.len() && ready.iter().all(|(provider, bank)|
        handles.rejected.get(provider).is_some_and(|old| Arc::ptr_eq(old, bank)))) { return; }
    let fingerprints = ready.iter().map(|(provider, bank)|
        (provider.clone(), bank.content_fingerprints())).collect();
    if let Err(error) = bank_ids.replace_banks(fingerprints) {
        for provider in ready.keys() {
            let message = format!("SFX bank composition failed: {error}");
            if failed_banks.errors.get(provider) != Some(&message) {
                bevy::log::error!(target: AUDIO_LOG_TARGET, "provider '{provider}': {message}");
                failed_banks.errors.insert(provider.clone(), message);
            }
        }
        handles.rejected = ready;
        return;
    }
    handles.rejected.clear();
    for (provider, bank) in ready {
        failed_banks.errors.remove(&provider);
        banks.providers.insert(provider.clone(), bank.clone());
        handles.admitted.insert(provider.clone(), bank);
        selection.refresh_provider_sfx_ids(&provider, bank_ids.ids_for(&provider));
        info!(target: AUDIO_LOG_TARGET, "provider '{provider}' SFX bank is ready");
    }
}

/// Name a cue for a human, from whatever authority can still spell it.
///
/// An [`SfxId`] is a one-way hash, so an id alone names nothing an author can
/// search for. Three sources know almost every name: the engine's `ids` table
/// (typed cues), the active procedural registry (open ids in RON), and every
/// loaded bank's name section (open ids another provider packs). A cue none of
/// them knows is reported as anonymous.
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

/// Run condition of [`audio_play_sfx_messages`]: a cue waits, or the last
/// run's records are still to be cleared. Peeks; the reader takes the cues.
pub fn a_cue_waits_or_was_played(
    messages: Res<bevy::ecs::message::Messages<OwnedSfxMessage>>,
    playback: Res<SfxPlaybackState>,
) -> bool {
    !messages.is_empty() || !playback.played_this_pass.is_empty()
}

pub fn audio_play_sfx_messages(
    mut messages: MessageReader<OwnedSfxMessage>,
    selection: Res<ActiveAudioSelection>,
    banks: Res<SfxBankResource>,
    pending: Option<Res<SfxBankHandles>>,
    failed_banks: Res<FailedSfxBankLoads>,
    asset_server: Res<AssetServer>,
    sfx_channel: Res<AudioChannel<SfxChannel>>,
    output: Option<Res<crate::output::AudioOutputMode>>,
    mut cache: ResMut<ProviderSfxHandleCache>,
    mut audio_sources: ResMut<Assets<KiraAudioSource>>,
    mut playback: ResMut<SfxPlaybackState>,
    audio_catalogs: Option<Res<crate::catalog::AudioCatalogRegistry>>,
    mut first_play_logged: Local<bool>,
) {
    playback.played_this_pass.clear();
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
        // A borrowed bank does not replace a cue the provider authored itself.
        let own_voice = audio_catalogs
            .as_deref()
            .is_some_and(|catalogs| catalogs.borrows_resident_sfx_bank(provider_id))
            && source_registry.is_some_and(|registry| registry.spec_for_id(id).is_some());
        let (bank, bank_fingerprint) = if own_voice {
            (None, None)
        } else {
            (
                banks.provider(provider_id),
                banks.fingerprint_for(provider_id, id),
            )
        };
        let resolved = cache.handle_for(
            provider_id,
            id,
            source_registry,
            bank,
            bank_fingerprint,
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
                // Name the cue and the cause, once per (provider, cue,
                // diagnosis). The causes are different failures, including a
                // cue that is only early.
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
        let record = SfxPlaybackRecord {
            owner,
            presentation_source: source.clone(),
            provider_id: provider_id.to_owned(),
            id,
            source: resolved.source,
        };
        playback.played_this_pass.push(record.clone());
        playback.last_played = Some(record);
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
        describe_sfx_id, FailedSfxBankLoads, SfxBankHandles, SfxBankAssetAppExt,
        SfxBankAssetPlugin, SfxBankResource,
    };
    use crate::catalog::SfxBankRegistry;
    use crate::selection::ActiveAudioSelection;
    use crate::spec::{SfxRegistry, SfxSpec, SoundCueKey, WaveformSpec};

    fn bank_asset(payload: u8) -> super::SfxBankAsset {
        let id = SfxId::from_static("fixture.cue");
        let mut bytes = vec![0u8; 105];
        bytes[..8].copy_from_slice(b"AMBNDSFX");
        bytes[8..12].copy_from_slice(&1u32.to_le_bytes());
        bytes[12..16].copy_from_slice(&1u32.to_le_bytes());
        bytes[16..24].copy_from_slice(&40u64.to_le_bytes());
        bytes[24..32].copy_from_slice(&104u64.to_le_bytes());
        bytes[32..40].copy_from_slice(&105u64.to_le_bytes());
        bytes[40..48].copy_from_slice(&id.hash().to_le_bytes());
        bytes[48..56].copy_from_slice(&104u64.to_le_bytes());
        bytes[56..60].copy_from_slice(&1u32.to_le_bytes());
        bytes[61] = 1;
        bytes[64..68].copy_from_slice(&48000u32.to_le_bytes());
        bytes[104] = payload;
        super::SfxBankAsset { provider: std::sync::Arc::new(ambition_sfx::BankProvider::from_bytes(bytes).unwrap()) }
    }

    #[test]
    fn a_shared_bank_reload_updates_all_providers_and_keeps_conflicting_bytes_out() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_resource::<ActiveAudioSelection>()
            .add_plugins(SfxBankAssetPlugin);
        app.update();
        let handle = app.world_mut().resource_mut::<bevy::asset::Assets<super::SfxBankAsset>>().add(bank_asset(1));
        app.world_mut().insert_resource(super::SfxBankHandles {
            handles: [("a".to_owned(), handle.clone()), ("b".to_owned(), handle.clone())].into(),
            ..Default::default()
        });
        app.update();
        let first = app.world().resource::<SfxBankResource>().fingerprints_for("a");
        app.world_mut().resource_mut::<bevy::asset::Assets<super::SfxBankAsset>>().insert(handle.id(), bank_asset(2)).unwrap();
        app.update();
        let banks = app.world().resource::<SfxBankResource>();
        let second = banks.fingerprints_for("a");
        assert_ne!(first, second);
        assert_eq!(second, banks.fingerprints_for("b"));
        let conflict = app.world_mut().resource_mut::<bevy::asset::Assets<super::SfxBankAsset>>().add(bank_asset(3));
        app.world_mut().resource_mut::<super::SfxBankHandles>().handles.insert("c".to_owned(), conflict);
        app.update();
        assert_eq!(app.world().resource::<SfxBankResource>().fingerprints_for("a"), second);
        assert!(app.world().resource::<SfxBankResource>().provider("c").is_none());
        assert_eq!(app.world().resource::<SfxBankRegistry>().ids_for("c"), BTreeSet::new());
    }

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
                .get_resource::<SfxBankHandles>()
                .map_or(true, |pending| {
                    !pending.is_loading("broken_provider", app.world().resource::<bevy::asset::AssetServer>())
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

    /// The suppression key is (provider, cue, diagnosis). Keyed on the cue
    /// alone, a second provider's identical miss would be hidden, and an
    /// early miss would keep its diagnosis after the bank proved it absent.
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

    /// A hash names nothing. The engine's id table names typed cues; loaded
    /// banks' name sections cover open ids another provider packs.
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
