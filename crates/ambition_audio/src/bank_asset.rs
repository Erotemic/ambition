//! Bevy `Asset` + `AssetLoader` for the packed SFX bank.
//!
//! Hosts decide *which* bank path to load by inserting [`SfxBankAssetPath`].
//! This module owns the reusable pieces after that: decode the `.bank` /
//! `.sfxbank` asset, promote it into [`SfxBankResource`], refresh the typed SFX
//! handles in [`AudioLibrary`], and drain [`ambition_sfx::SfxMessage`] into the
//! Kira SFX channel.

use std::sync::Arc;

use ambition_sfx::{BankProvider, SfxError, SfxMessage, SfxProvider};
use bevy::asset::{
    io::Reader, Asset, AssetApp, AssetLoader, AssetServer, Assets, Handle, LoadContext,
};
use bevy::log::{debug, info, warn};
use bevy::prelude::{
    App, Commands, Local, MessageReader, Plugin, Res, ResMut, Resource, Startup, Update,
};
use bevy::reflect::TypePath;
use bevy_kira_audio::prelude::{AudioChannel, AudioControl, AudioSource as KiraAudioSource};

use crate::library::{AudioLibrary, SfxChannel, SfxMessageCue, SoundCue};
use crate::render::SfxBankHandleCache;
use crate::web_unlock::AUDIO_LOG_TARGET;

/// Host-supplied Bevy asset path for the async bank load.
///
/// The path usually comes from an app/content catalog. Keeping that resolution
/// outside `ambition_audio` lets the reusable loader avoid naming any one
/// game's asset profile type.
#[derive(Resource, Clone, Debug)]
pub struct SfxBankAssetPath(pub String);

/// Process-wide handle to the loaded SFX bank, when one was found at startup.
/// Wrapped in `Arc` so systems that need to play catalog SFX can clone cheaply
/// and look up by id without re-reading the file.
#[derive(Resource, Clone)]
pub struct SfxBankResource(pub Arc<BankProvider>);

/// Loaded SFX-bank asset. Wraps the parsed [`BankProvider`] in an `Arc` so
/// the [`SfxBankResource`] and any future direct consumers can share it
/// without re-decoding.
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
            Self::Io(e) => write!(f, "io: {e}"),
            Self::Sfx(e) => write!(f, "sfx bank: {e}"),
        }
    }
}

impl std::error::Error for SfxBankLoaderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Sfx(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for SfxBankLoaderError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<SfxError> for SfxBankLoaderError {
    fn from(e: SfxError) -> Self {
        Self::Sfx(e)
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
        _ctx: &mut LoadContext<'_>,
    ) -> Result<SfxBankAsset, SfxBankLoaderError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let provider = BankProvider::from_bytes(bytes)?;
        Ok(SfxBankAsset {
            provider: Arc::new(provider),
        })
    }

    fn extensions(&self) -> &[&str] {
        &["bank", "sfxbank"]
    }
}

/// In-flight handle for the async bank load. Removed once
/// [`promote_loaded_sfx_bank`] sees the asset land.
#[derive(Resource)]
pub struct PendingSfxBankHandle(pub Handle<SfxBankAsset>);

pub struct SfxBankAssetPlugin;

impl Plugin for SfxBankAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<SfxBankAsset>()
            .register_asset_loader(SfxBankLoader)
            .add_systems(Startup, kick_off_bank_load)
            .add_systems(Update, promote_loaded_sfx_bank);
    }
}

/// Startup: if no sync-loaded [`SfxBankResource`] is present, ask the asset
/// server to fetch the host-selected bank path through Bevy's active
/// [`bevy::asset::AssetReader`] (loose FS on desktop / Android, HTTP on wasm).
fn kick_off_bank_load(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    path: Option<Res<SfxBankAssetPath>>,
    existing: Option<Res<SfxBankResource>>,
) {
    if existing.is_some() {
        debug!(
            target: AUDIO_LOG_TARGET,
            "ambition audio: sfx bank already loaded synchronously; skipping async load"
        );
        return;
    }
    let Some(path) = path else {
        warn!(
            target: AUDIO_LOG_TARGET,
            "ambition audio: no SfxBankAssetPath resource; SFX will play silent stubs"
        );
        return;
    };
    info!(
        target: AUDIO_LOG_TARGET,
        "ambition audio: loading sfx bank from `{}` (async via AssetServer)",
        path.0
    );
    let handle: Handle<SfxBankAsset> = asset_server.load(path.0.clone());
    commands.insert_resource(PendingSfxBankHandle(handle));
}

/// Update: poll for the bank asset; once it lands, install the
/// [`SfxBankResource`] and refresh the typed SFX cue handles in the
/// [`AudioLibrary`] (which may have been built at startup with no bank).
fn promote_loaded_sfx_bank(
    mut commands: Commands,
    pending: Option<Res<PendingSfxBankHandle>>,
    assets: Res<Assets<SfxBankAsset>>,
    existing: Option<Res<SfxBankResource>>,
    library: Option<ResMut<AudioLibrary>>,
    audio_sources: Option<ResMut<Assets<KiraAudioSource>>>,
) {
    if existing.is_some() {
        return;
    }
    let Some(pending) = pending else {
        return;
    };
    let Some(asset) = assets.get(&pending.0) else {
        return;
    };
    let provider = asset.provider.clone();
    info!(
        target: AUDIO_LOG_TARGET,
        "ambition audio: sfx bank loaded async ({} entries) - promoting to SfxBankResource",
        provider.entry_count()
    );
    let mut refreshed_library = false;
    if let (Some(mut library), Some(mut audio_sources)) = (library, audio_sources) {
        library.refresh_sfx_from_bank(&mut audio_sources, provider.as_ref());
        refreshed_library = true;
    }
    info!(
        target: AUDIO_LOG_TARGET,
        "ambition audio: SfxBankResource installed (audio_library_refreshed={refreshed_library})"
    );
    commands.insert_resource(SfxBankResource(provider));
    commands.remove_resource::<PendingSfxBankHandle>();
}

pub fn audio_play_sfx_messages(
    mut messages: MessageReader<SfxMessage>,
    library: Res<AudioLibrary>,
    sfx_channel: Res<AudioChannel<SfxChannel>>,
    bank: Option<Res<SfxBankResource>>,
    mut cache: ResMut<SfxBankHandleCache>,
    mut audio_sources: ResMut<Assets<KiraAudioSource>>,
    mut first_play_logged: Local<bool>,
) {
    for message in messages.read() {
        if !*first_play_logged {
            info!(
                target: AUDIO_LOG_TARGET,
                "ambition audio: first SFX play attempt (cue={:?}, bank_loaded={})",
                message.cue(),
                bank.is_some()
            );
            *first_play_logged = true;
        }
        if let Some(cue) = message.cue() {
            sfx_channel.play(library.sfx_handle(cue));
            continue;
        }
        let SfxMessage::Play { id, .. } = *message else {
            continue;
        };
        // A string-keyed `Play` naming a procedural cue (e.g. the moveset's
        // "player.slash" swing) resolves to the guaranteed typed sound rather
        // than falling through to a possibly-absent bank sample.
        if let Some(cue) = SoundCue::from_sfx_id(id) {
            sfx_channel.play(library.sfx_handle(cue));
            continue;
        }
        let bank_provider = bank.as_deref().map(|bank| &*bank.0 as &dyn SfxProvider);
        let Some(handle) = cache.handle_for(id, bank_provider, audio_sources.as_mut()) else {
            continue;
        };
        sfx_channel.play(handle);
    }
}
