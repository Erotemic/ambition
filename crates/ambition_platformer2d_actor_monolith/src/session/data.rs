//! Data manifests for the sandbox.
//!
//! The goal of this module is to keep tuning/audio iteration data in RON while
//! still letting the current code synthesize assets at startup. `bevy_common_assets` registers
//! `Platformer2dGameplayDefaults` as a real Bevy asset type; `load_embedded` gives us a
//! synchronous bootstrap path until the sandbox grows a loading state.
//!
//! Bevy resolves `ambition/platformer_defaults.ron` relative to the sandbox crate asset root
//! (`crates/ambition_platformer2d_actor_monolith/assets`) when this package is run through
//! Cargo, so the embedded copy intentionally lives there too. Audio lives in its own App-local
//! registries (a provider registers an `ambition_audio::catalog::AudioCatalogFragment`; hosts
//! read them from the `AudioCatalogRegistry` resource) — SFX and music are separate concerns
//! from gameplay tuning and from each other.

use ambition_platformer2d_core as ae;
use bevy::asset::{Asset, AssetServer};
use bevy::prelude::{Commands, Handle, Res, Resource};
use bevy::reflect::TypePath;
use serde::Deserialize;
#[cfg(test)]
use std::collections::HashSet;

pub const PLATFORMER_DEFAULTS_ASSET: &str = "ambition/platformer_defaults.ron";

#[derive(Clone, Debug, Deserialize, Asset, TypePath, Resource)]
pub struct Platformer2dGameplayDefaults {
    pub abilities: ae::AbilitySet,
    pub tuning: ae::MovementTuning,
}

impl Platformer2dGameplayDefaults {
    pub fn load_embedded() -> Self {
        ron::from_str(include_str!("../../assets/ambition/platformer_defaults.ron"))
            .expect("embedded assets/ambition/platformer_defaults.ron should parse")
    }
}

// Authored audio is App-local now (R3.2: the engine ships no tracks and no
// cues). A provider registers an `ambition_audio::catalog::AudioCatalogFragment`
// from `ambition_content`'s `music_registry.ron` / `sfx_registry.ron`; hosts
// read the assembled registries from the `AudioCatalogRegistry` resource (or
// pass a selected `MusicRegistry` explicitly to the catalog builders). The
// former process-global install/override seam is retired — the only remaining
// access is the `#[cfg(test)]` fixtures below, which let core audio tests
// exercise the game's real data cross-crate without core embedding it.

/// Test-only handle to the game's authored music registry. Not a runtime seam.
#[cfg(test)]
pub(crate) fn fixture_music_registry() -> &'static MusicRegistry {
    &TEST_FIXTURE_MUSIC_REGISTRY
}

/// Test-only handle to the game's authored SFX registry. Not a runtime seam.
#[cfg(test)]
pub(crate) fn fixture_sfx_registry() -> &'static SfxRegistry {
    &TEST_FIXTURE_SFX_REGISTRY
}

/// Test fixture = the game's REAL authored registries, read cross-crate
/// from `ambition_content` as an explicit cross-crate test fixture so
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
pub struct Platformer2dGameplayDefaultsHandle(pub Handle<Platformer2dGameplayDefaults>);

/// Bevy startup system: register a `Handle<Platformer2dGameplayDefaults>` so the
/// asset server keeps the underlying `.ron` alive (and emits hot
/// reload events under `bevy_dev_hot_reload`).
///
/// Resolves the path through the active
/// [`crate::assets::platformer_assets::Platformer2dAssetCatalog`] when one is
/// installed. The catalog entry
/// [`crate::assets::platformer_assets::ids::sandbox_data`] is required, so the
/// catalog never returns `Disabled` outside of `NoAssets`/`Headless`.
/// Falls back to the raw asset-path constant when no catalog resource
/// is present (visible-only init order / tests).
pub fn load_data_asset_handle(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    catalog: Option<Res<crate::assets::platformer_assets::Platformer2dAssetCatalog>>,
) {
    let path = catalog
        .as_ref()
        .and_then(|c| c.path_for(&crate::assets::platformer_assets::ids::sandbox_data()))
        .unwrap_or_else(|| PLATFORMER_DEFAULTS_ASSET.to_string());
    commands.insert_resource(Platformer2dGameplayDefaultsHandle(asset_server.load(path)));
}


#[cfg(test)]
mod tests;
