//! Bevy `Asset` + `AssetLoader` for the packed SFX bank.
//!
//! Two paths produce a [`crate::runtime::setup::SfxBankResource`]:
//!
//! 1. **Sync fast path** — `setup::try_load_sfx_bank_via_catalog` runs
//!    during `presentation_world` and inserts the resource immediately
//!    when the bank can be read synchronously (the `static_sfx_bank`
//!    embed or the `AMBITION_SFX_BANK_PATH` dev override).
//!
//! 2. **Async asset path (this module)** — when the sync path returns
//!    `None`, the SFX bank loads through Bevy's `AssetServer`. That
//!    handles both desktop loose filesystem and wasm HTTP fetch
//!    uniformly via Bevy's per-platform [`bevy::asset::AssetReader`].
//!    Once decoding finishes, [`promote_loaded_sfx_bank`] copies the
//!    parsed provider into the long-lived `SfxBankResource` and
//!    refreshes the typed SFX cue handles in [`crate::audio::AudioLibrary`]
//!    so already-built handles don't stay silent forever.
//!
//! The loader registers extensions `.bank` and `.sfxbank` so callers
//! can use either name; the sandbox manifest still points at
//! `audio/sfx.bank` for historical reasons.

#![cfg(feature = "audio")]

use std::sync::Arc;

use bevy::asset::{
    io::Reader, Asset, AssetApp, AssetLoader, AssetServer, Assets, Handle, LoadContext,
};
use bevy::log::{debug, info, warn};
use bevy::prelude::{App, Commands, Plugin, Res, ResMut, Resource, Startup, Update};

use ambition_audio::web_unlock::AUDIO_LOG_TARGET;
use bevy::reflect::TypePath;
use bevy_kira_audio::prelude::AudioSource as KiraAudioSource;

use ambition_sfx::{BankProvider, SfxError};

use crate::assets::sandbox_assets::{ids, SandboxAssetCatalog};
use crate::audio::AudioLibrary;
use crate::runtime::setup::SfxBankResource;

/// Loaded SFX-bank asset. Wraps the parsed [`BankProvider`] in an
/// `Arc` so the private `SfxBankResource` (in `crate::runtime::setup`)
/// and any future direct consumers can share it without re-decoding.
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
            // `kick_off_bank_load` reads the catalog resource, which is
            // inserted by `init_sandbox_resources`. Bevy's startup is
            // single-threaded enough that catalog insertion runs before
            // our system, but use `.after()` on the catalog setup if
            // future ordering changes break this.
            .add_systems(Startup, kick_off_bank_load)
            .add_systems(Update, promote_loaded_sfx_bank);
    }
}

/// Startup: if no sync-loaded `SfxBankResource` is present, ask the
/// asset server to fetch the bank through whatever
/// [`bevy::asset::AssetReader`] the active source uses (loose FS on
/// desktop / Android, HTTP on wasm).
fn kick_off_bank_load(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    catalog: Res<SandboxAssetCatalog>,
    existing: Option<Res<SfxBankResource>>,
) {
    if existing.is_some() {
        debug!(
            target: AUDIO_LOG_TARGET,
            "ambition audio: sfx bank already loaded synchronously; skipping async load"
        );
        return;
    }
    let Some(path) = catalog.path_for(&ids::sfx_bank()) else {
        warn!(
            target: AUDIO_LOG_TARGET,
            "ambition audio: audio.sfx_bank not resolvable under {} profile; SFX will play silent stubs",
            catalog.profile().label()
        );
        return;
    };
    info!(
        target: AUDIO_LOG_TARGET,
        "ambition audio: loading sfx bank from `{path}` (async via AssetServer)"
    );
    let handle: Handle<SfxBankAsset> = asset_server.load(path);
    commands.insert_resource(PendingSfxBankHandle(handle));
}

/// Update: poll for the bank asset; once it lands, install the
/// [`SfxBankResource`] and refresh the typed SFX cue handles in the
/// [`AudioLibrary`] (which was built at startup with no bank).
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
        "ambition audio: sfx bank loaded async ({} entries) — promoting to SfxBankResource",
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
