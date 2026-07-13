//! Provider-qualified SFX bank loading and playback.
//!
//! A Bevy App may cache banks for many linked providers. Playback never reads a
//! process-global "current bank": it resolves through the active audio
//! context's provider and the request's captured owner.

use std::collections::BTreeMap;
use std::sync::Arc;

use ambition_sfx::{BankProvider, OwnedSfxMessage, SfxError, SfxId, SfxProvider};
use bevy::asset::{
    io::Reader, Asset, AssetApp, AssetLoader, AssetServer, Assets, Handle, LoadContext,
};
use bevy::log::{debug, info};
use bevy::prelude::{
    App, Commands, Local, MessageReader, Plugin, Res, ResMut, Resource, Startup, Update,
};
use bevy::reflect::TypePath;
use bevy_kira_audio::prelude::{AudioChannel, AudioControl, AudioSource as KiraAudioSource};

use crate::catalog::SfxBankRegistry;
use crate::library::{sfx_message_target_id, SfxChannel};
use crate::render::{ProviderSfxHandleCache, SfxPlaybackRecord, SfxPlaybackState};
use crate::selection::ActiveAudioSelection;
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
        assert!(!provider_id.trim().is_empty(), "SFX bank provider id cannot be empty");
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
            .and_then(|provider| provider.content_fingerprints().get(&id).copied())
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
            .add_systems(Startup, kick_off_bank_load)
            .add_systems(Update, promote_loaded_sfx_bank);
    }
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
    assets: Res<Assets<SfxBankAsset>>,
    mut banks: ResMut<SfxBankResource>,
    mut bank_ids: ResMut<SfxBankRegistry>,
    mut selection: ResMut<ActiveAudioSelection>,
) {
    let Some(mut pending) = pending else {
        return;
    };
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

pub fn audio_play_sfx_messages(
    mut messages: MessageReader<OwnedSfxMessage>,
    selection: Res<ActiveAudioSelection>,
    banks: Res<SfxBankResource>,
    sfx_channel: Res<AudioChannel<SfxChannel>>,
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
        let Some(provider_id) = selection.provider_id() else {
            playback.rejected_wrong_owner = playback.rejected_wrong_owner.saturating_add(1);
            continue;
        };
        let id = sfx_message_target_id(request);
        if !selection.sfx_authority().allows(id) {
            playback.rejected_unauthorized = playback.rejected_unauthorized.saturating_add(1);
            continue;
        }
        let resolved = cache.handle_for(
            provider_id,
            id,
            selection.sfx(),
            banks.provider(provider_id),
            banks.fingerprint_for(provider_id, id),
            audio_sources.as_mut(),
        );
        let Some(resolved) = resolved else {
            playback.missing_source = playback.missing_source.saturating_add(1);
            continue;
        };
        sfx_channel.play(resolved.handle);
        playback.accepted_playbacks = playback.accepted_playbacks.saturating_add(1);
        playback.last_played = Some(SfxPlaybackRecord {
            owner,
            provider_id: provider_id.to_owned(),
            id,
            source: resolved.source,
        });
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use ambition_sfx::{AudioContextOwner, OwnedSfxMessage, SfxId, SfxMessage};
    use bevy::math::Vec2;

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
            request: SfxMessage::Dash { pos: Vec2::ZERO },
        };
        assert!(!selection.accepts_request_owner(stale.owner));
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
