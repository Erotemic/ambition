//! Data manifests for the sandbox.
//!
//! The goal of this module is to keep tuning/audio iteration data in RON while
//! still letting the current code synthesize assets at startup. `bevy_common_assets` registers
//! `SandboxDataSpec` as a real Bevy asset type; `load_embedded` gives us a
//! synchronous bootstrap path until the sandbox grows a loading state.
//!
//! Bevy resolves `ambition/sandbox.ron` relative to the sandbox crate asset
//! root (`crates/ambition_actors/assets`) when this package is run through
//! Cargo, so the embedded copy intentionally lives there too. World/room
//! authoring has moved to LDtk; this RON asset intentionally owns only
//! non-spatial sandbox tuning. Audio lives in its own sibling registries
//! ([`authored_sfx_registry`] / [`authored_music_registry`]) —
//! SFX and music are separate concerns from gameplay tuning and from each
//! other.

use ambition_engine_core as ae;
use bevy::asset::{Asset, AssetServer};
use bevy::prelude::{Commands, Handle, Res, Resource};
use bevy::reflect::TypePath;
use serde::Deserialize;
#[cfg(test)]
use std::collections::HashSet;

pub const SANDBOX_DATA_ASSET: &str = "ambition/sandbox.ron";

#[derive(Clone, Debug, Deserialize, Asset, TypePath, Resource)]
pub struct SandboxDataSpec {
    pub abilities: ae::AbilitySet,
    pub tuning: ae::MovementTuning,
}

impl SandboxDataSpec {
    pub fn load_embedded() -> Self {
        ron::from_str(include_str!("../../assets/ambition/sandbox.ron"))
            .expect("embedded assets/ambition/sandbox.ron should parse")
    }
}

/// Game-installed audio registries (R3.2: the engine ships no tracks and
/// no cues — `ambition_content` owns `music_registry.ron` /
/// `sfx_registry.ron` and installs the parsed values at startup).
///
/// §5 classification: **content registry** — install-once seam, immutable
/// after install, read from pure catalog-building code
/// (`assets::sandbox_assets::desktop_dev_default_catalog`) with no `World` in
/// hand. First install wins.
static MUSIC_REGISTRY_OVERRIDE: std::sync::OnceLock<MusicRegistry> = std::sync::OnceLock::new();
static SFX_REGISTRY_OVERRIDE: std::sync::OnceLock<SfxRegistry> = std::sync::OnceLock::new();

/// Install the game's authored music registry (content calls this at
/// startup, before any catalog build). First install wins.
pub fn install_music_registry(registry: MusicRegistry) {
    let _ = MUSIC_REGISTRY_OVERRIDE.set(registry);
}

/// Install the game's authored SFX synthesis registry. First install wins.
pub fn install_sfx_registry(registry: SfxRegistry) {
    let _ = SFX_REGISTRY_OVERRIDE.set(registry);
}

/// The installed music registry; without an install the engine has NO
/// tracks (empty registry) — core tests read the game's real registry via
/// the cross-crate fixture instead.
pub fn authored_music_registry() -> &'static MusicRegistry {
    MUSIC_REGISTRY_OVERRIDE.get().unwrap_or_else(|| {
        #[cfg(test)]
        {
            &TEST_FIXTURE_MUSIC_REGISTRY
        }
        #[cfg(not(test))]
        {
            static EMPTY: std::sync::OnceLock<MusicRegistry> = std::sync::OnceLock::new();
            EMPTY.get_or_init(|| MusicRegistry {
                default_track: String::new(),
                tracks: Vec::new(),
            })
        }
    })
}

/// The installed SFX registry; empty without an install (test builds fall
/// back to the game's real registry fixture).
pub fn authored_sfx_registry() -> &'static SfxRegistry {
    SFX_REGISTRY_OVERRIDE.get().unwrap_or_else(|| {
        #[cfg(test)]
        {
            &TEST_FIXTURE_SFX_REGISTRY
        }
        #[cfg(not(test))]
        {
            static EMPTY: std::sync::OnceLock<SfxRegistry> = std::sync::OnceLock::new();
            EMPTY.get_or_init(|| SfxRegistry {
                sample_rate: 44_100,
                sfx: Vec::new(),
            })
        }
    })
}

/// Test fixture = the game's REAL authored registries, read cross-crate
/// from `ambition_content` (the `install_enemy_roster` fixture pattern) so
/// core's catalog/audio tests exercise real data without core embedding it.
#[cfg(test)]
static TEST_FIXTURE_MUSIC_REGISTRY: std::sync::LazyLock<MusicRegistry> =
    std::sync::LazyLock::new(|| {
        ron::from_str(include_str!(
            "../../../../game/ambition_content/assets/audio/music_registry.ron"
        ))
        .expect("ambition_content music_registry.ron should parse")
    });

#[cfg(test)]
static TEST_FIXTURE_SFX_REGISTRY: std::sync::LazyLock<SfxRegistry> =
    std::sync::LazyLock::new(|| {
        ron::from_str(include_str!(
            "../../../../game/ambition_content/assets/audio/sfx_registry.ron"
        ))
        .expect("ambition_content sfx_registry.ron should parse")
    });

#[derive(Resource, Clone, Debug)]
pub struct SandboxDataAsset(pub Handle<SandboxDataSpec>);

/// Bevy startup system: register a `Handle<SandboxDataSpec>` so the
/// asset server keeps the underlying `.ron` alive (and emits hot
/// reload events under `bevy_dev_hot_reload`).
///
/// Resolves the path through the active
/// [`crate::assets::sandbox_assets::SandboxAssetCatalog`] when one is
/// installed. The catalog entry
/// [`crate::assets::sandbox_assets::ids::sandbox_data`] is required, so the
/// catalog never returns `Disabled` outside of `NoAssets`/`Headless`.
/// Falls back to the raw asset-path constant when no catalog resource
/// is present (visible-only init order / tests).
pub fn load_data_asset_handle(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    catalog: Option<Res<crate::assets::sandbox_assets::SandboxAssetCatalog>>,
) {
    let path = catalog
        .as_ref()
        .and_then(|c| c.path_for(&crate::assets::sandbox_assets::ids::sandbox_data()))
        .unwrap_or_else(|| SANDBOX_DATA_ASSET.to_string());
    commands.insert_resource(SandboxDataAsset(asset_server.load(path)));
}

// Spatial/world authoring moved to LDtk. This module intentionally contains
// only non-spatial sandbox tuning; the audio data schema lives DOWN in the
// `ambition_audio` crate. Re-exported so every `crate::session::data::*`
// audio path resolves unchanged.
pub use ambition_audio::spec::{
    MusicRegistry, MusicTrack, SfxRegistry, SfxSpec, SoundCueKey, WaveformSpec,
};

#[cfg(test)]
mod tests;
