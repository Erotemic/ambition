//! Gameplay adapter for the reusable sandbox asset catalog.
//!
//! `ambition_asset_manager::sandbox_assets` owns catalog/resource/profile
//! behavior. This module only gathers Ambition-game rows from gameplay/content
//! registries and delegates to that crate, preserving the historical
//! `ambition_actors::assets::sandbox_assets` import path while the
//! surrounding asset presentation code is still being carved.

use ambition_asset_manager::sandbox_assets as core;
use ambition_asset_manager::{AssetManifest, AssetProfile};
use ambition_characters::actor::character_catalog::CharacterCatalog;

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
    /// Built BEFORE the `App` exists (it is a plugin value passed to
    /// `add_plugins`), so the manifest arrives as a parameter — a `Res`
    /// cannot reach here.
    pub fn for_profile(profile: AssetProfile, manifest: &crate::ldtk_world::WorldManifest) -> Self {
        let embedded_worlds = manifest
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
/// suitable for headless / RL / test fixtures. The caller supplies the authored
/// music registry explicitly (from its App-local `AudioCatalogRegistry` or a
/// content fixture) — this seam consults no process-global audio state.
pub fn desktop_dev_default_catalog(
    character_catalog: &CharacterCatalog,
    boss_catalog: &crate::boss_encounter::BossCatalog,
    music: &crate::session::data::MusicRegistry,
    worlds: &crate::ldtk_world::WorldManifest,
) -> SandboxAssetCatalog {
    let config = GameAssetConfig {
        asset_profile: AssetProfile::DesktopDevLoose,
        ..Default::default()
    };
    build_sandbox_catalog(&config, character_catalog, boss_catalog, music, worlds)
}

/// Build the shared sprite/parallax/audio/world catalog.
///
/// A composition that owns procedural rooms and ships no `.ldtk` file passes
/// a world-less [`WorldManifest`](crate::ldtk_world::WorldManifest)
/// (`&WorldManifest::default()`): it then contributes no world rows, and every
/// ordinary image, character, boss, data, SFX, font, sprite-pack, and music
/// entry still lands. That replaces the old `_without_worlds` twin — with the
/// manifest an explicit argument, "no worlds" is just a value, not a second
/// function.
pub fn build_sandbox_catalog(
    config: &GameAssetConfig,
    character_catalog: &CharacterCatalog,
    boss_catalog: &crate::boss_encounter::BossCatalog,
    music: &crate::session::data::MusicRegistry,
    worlds: &crate::ldtk_world::WorldManifest,
) -> SandboxAssetCatalog {
    build_sandbox_catalog_with(
        config,
        character_catalog,
        boss_catalog,
        music,
        worlds,
        |_| {},
    )
}

pub fn scaled_asset_id(
    id: &ambition_asset_manager::AssetId,
    scale: crate::persistence::settings::TextureResolutionScale,
) -> Option<ambition_asset_manager::AssetId> {
    core::scaled_asset_id(id, scale.asset_id_suffix())
}

pub fn build_sandbox_catalog_with(
    config: &GameAssetConfig,
    character_catalog: &CharacterCatalog,
    boss_catalog: &crate::boss_encounter::BossCatalog,
    music: &crate::session::data::MusicRegistry,
    worlds: &crate::ldtk_world::WorldManifest,
    extend: impl FnOnce(&mut AssetManifest),
) -> SandboxAssetCatalog {
    let core_config = SandboxAssetConfig {
        sprite_folder: config.sprite_folder.clone(),
        asset_profile: config.asset_profile,
    };
    let image_manifest = sandbox_image_manifest(&config.sprite_folder);
    let inputs = sandbox_catalog_inputs(character_catalog, boss_catalog, music, worlds);
    core::build_sandbox_catalog_with(&core_config, image_manifest, &inputs, extend)
}

pub fn sandbox_catalog_inputs(
    character_catalog: &CharacterCatalog,
    boss_catalog: &crate::boss_encounter::BossCatalog,
    music: &crate::session::data::MusicRegistry,
    worlds: &crate::ldtk_world::WorldManifest,
) -> SandboxCatalogInputs {
    let mut inputs = sandbox_catalog_inputs_without_worlds(character_catalog, boss_catalog, music);
    inputs.worlds = worlds
        .worlds
        .iter()
        .map(|source| WorldCatalogRow {
            id: source.id.clone(),
            asset_path: source.asset_path.clone(),
            required: source.required,
            loose_path: source.loose_path.clone(),
            embedded_bevy_path: source.embedded_bevy_path,
        })
        .collect();
    inputs
}

fn sandbox_catalog_inputs_without_worlds(
    character_catalog: &CharacterCatalog,
    boss_catalog: &crate::boss_encounter::BossCatalog,
    music: &crate::session::data::MusicRegistry,
) -> SandboxCatalogInputs {
    SandboxCatalogInputs {
        scale_variants: texture_scale_variants(),
        character_sprites: crate::character_sprites::all_character_sprite_filenames_in(
            character_catalog,
        )
        .into_iter()
        .map(|(name, filename)| CharacterSpriteCatalogRow { name, filename })
        .collect(),
        boss_sprites: boss_catalog
            .sprite_filenames()
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
        worlds: Vec::new(),
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
