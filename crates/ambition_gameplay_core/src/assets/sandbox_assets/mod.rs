//! Gameplay adapter for the reusable sandbox asset catalog.
//!
//! `ambition_asset_manager::sandbox_assets` owns catalog/resource/profile
//! behavior. This module only gathers Ambition-game rows from gameplay/content
//! registries and delegates to that crate, preserving the historical
//! `ambition_gameplay_core::assets::sandbox_assets` import path while the
//! surrounding asset presentation code is still being carved.

use ambition_asset_manager::sandbox_assets as core;
use ambition_asset_manager::{AssetManifest, AssetProfile};

use crate::assets::game_assets::{sandbox_image_manifest, GameAssetConfig};

pub use core::embedded_core;
pub use core::ids;
pub use core::{
    AssetScaleVariant, BossSpriteCatalogRow, CharacterSpriteCatalogRow, EmbeddedWorldAsset,
    MusicCatalogRow, SandboxAssetCatalog, SandboxAssetConfig, SandboxCatalogInputs,
    WorldCatalogRow,
};

/// Bevy plugin that registers embedded asset bytes under the paths emitted by
/// the shared sandbox catalog.
pub struct AmbitionAssetSourcePlugin(core::AmbitionAssetSourcePlugin);

impl AmbitionAssetSourcePlugin {
    pub fn for_profile(profile: AssetProfile) -> Self {
        let embedded_worlds = crate::ldtk_world::world_manifest()
            .worlds
            .iter()
            .filter_map(|source| {
                Some(EmbeddedWorldAsset {
                    bevy_path: source.embedded_bevy_path?,
                    text: source.embedded_text?,
                })
            })
            .collect();
        Self(core::AmbitionAssetSourcePlugin::with_embedded_worlds(
            profile,
            embedded_worlds,
        ))
    }
}

impl bevy::prelude::Plugin for AmbitionAssetSourcePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        self.0.build(app);
    }
}

/// Convenience: build a desktop-dev catalog from the embedded sandbox spec,
/// suitable for headless / RL / test fixtures.
pub fn desktop_dev_default_catalog() -> SandboxAssetCatalog {
    let config = GameAssetConfig {
        asset_profile: AssetProfile::DesktopDevLoose,
        ..Default::default()
    };
    let music = crate::session::data::authored_music_registry();
    build_sandbox_catalog(&config, music)
}

pub fn build_sandbox_catalog(
    config: &GameAssetConfig,
    music: &crate::session::data::MusicRegistry,
) -> SandboxAssetCatalog {
    build_sandbox_catalog_with(config, music, |_| {})
}

pub fn scaled_asset_id(
    id: &ambition_asset_manager::AssetId,
    scale: crate::persistence::settings::TextureResolutionScale,
) -> Option<ambition_asset_manager::AssetId> {
    core::scaled_asset_id(id, scale.asset_id_suffix())
}

pub fn build_sandbox_catalog_with(
    config: &GameAssetConfig,
    music: &crate::session::data::MusicRegistry,
    extend: impl FnOnce(&mut AssetManifest),
) -> SandboxAssetCatalog {
    let core_config = SandboxAssetConfig {
        sprite_folder: config.sprite_folder.clone(),
        asset_profile: config.asset_profile,
    };
    let image_manifest = sandbox_image_manifest(&config.sprite_folder);
    let inputs = sandbox_catalog_inputs(music);
    core::build_sandbox_catalog_with(&core_config, image_manifest, &inputs, extend)
}

pub fn sandbox_catalog_inputs(music: &crate::session::data::MusicRegistry) -> SandboxCatalogInputs {
    SandboxCatalogInputs {
        scale_variants: texture_scale_variants(),
        character_sprites: crate::character_sprites::all_character_sprite_filenames()
            .into_iter()
            .map(|(name, filename)| CharacterSpriteCatalogRow { name, filename })
            .collect(),
        boss_sprites: crate::boss_encounter::sprites::all_boss_sprite_filenames()
            .into_iter()
            .map(|(name, filename)| BossSpriteCatalogRow {
                name: name.to_string(),
                filename: filename.to_string(),
            })
            .collect(),
        music_tracks: music
            .tracks
            .iter()
            .map(|track| MusicCatalogRow {
                id: track.id.clone(),
                asset_path: track.resolved_asset_path(),
            })
            .collect(),
        worlds: crate::ldtk_world::world_manifest()
            .worlds
            .iter()
            .map(|source| WorldCatalogRow {
                id: source.id.clone(),
                asset_path: source.asset_path.clone(),
                required: source.required,
                loose_path: source.loose_path.clone(),
                embedded_bevy_path: source.embedded_bevy_path,
            })
            .collect(),
    }
}

fn texture_scale_variants() -> Vec<AssetScaleVariant> {
    crate::persistence::settings::TextureResolutionScale::MANIFEST_VARIANTS
        .iter()
        .filter_map(|scale| {
            Some(AssetScaleVariant {
                asset_id_suffix: scale.asset_id_suffix()?,
                sprite_subdir_suffix: match scale {
                    crate::persistence::settings::TextureResolutionScale::Half => "0_5x",
                    crate::persistence::settings::TextureResolutionScale::Quarter => "0_25x",
                    crate::persistence::settings::TextureResolutionScale::Potato => "potato",
                    crate::persistence::settings::TextureResolutionScale::Full => return None,
                },
                parallax_subdir: scale.parallax_subdir(),
            })
        })
        .collect()
}
