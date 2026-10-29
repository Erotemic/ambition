//! Build asset catalog inputs from the selected content registries.

use ambition_asset_manager::platformer_assets as core;
use ambition_asset_manager::{AssetManifest, AssetProfile};
use ambition_characters::actor::character_catalog::CharacterCatalog;

use ambition_asset_manager::content_library::ContentLibraries;
use ambition_sprite_sheet::game_assets::GameAssetConfig;

use core::{
    AssetScaleVariant, BossSpriteCatalogRow, CharacterSpriteCatalogRow, EmbeddedWorldAsset,
    MusicCatalogRow, Platformer2dAssetCatalog, Platformer2dAssetCatalogConfig, Platformer2dAssetCatalogInputs,
    WorldCatalogRow,
};

/// Build the asset source from the selected profile and world manifest.
pub fn asset_source_for_profile(
    profile: AssetProfile,
    manifest: &ambition_platformer2d_world::world_manifest::WorldManifest,
) -> core::AmbitionAssetSourcePlugin {
    let embedded_worlds = manifest.worlds.iter().filter_map(|source| {
        Some(EmbeddedWorldAsset {
            bevy_path: source.embedded_bevy_path?,
            text: source.embedded_text?,
        })
    }).collect();
    core::AmbitionAssetSourcePlugin::with_embedded_worlds(profile, embedded_worlds)
}

/// Convenience: build a desktop-dev catalog from the embedded sandbox spec,
/// suitable for headless / RL / test fixtures. The caller supplies the authored
/// music registry explicitly (from its App-local `AudioCatalogRegistry` or a
/// content fixture) — this seam consults no process-global audio state.
pub fn desktop_dev_default_catalog(
    libraries: &ContentLibraries,
    character_catalog: &CharacterCatalog,
    boss_catalog: &ambition_boss_encounter::BossCatalog,
    music: &ambition_audio::spec::MusicRegistry,
    worlds: &ambition_platformer2d_world::world_manifest::WorldManifest,
) -> Platformer2dAssetCatalog {
    let config = GameAssetConfig {
        asset_profile: AssetProfile::DesktopDevLoose,
        ..Default::default()
    };
    build_platformer2d_asset_catalog(libraries, &config, character_catalog, boss_catalog, music, worlds)
}

/// Build the shared sprite/parallax/audio/world catalog.
///
/// A composition that owns procedural rooms and ships no `.ldtk` file passes
/// a world-less [`WorldManifest`](ambition_platformer2d_world::world_manifest::WorldManifest)
/// (`&WorldManifest::default()`): it then contributes no world rows, and every
/// ordinary image, character, boss, data, SFX, font, sprite-pack, and music
/// entry still lands. That replaces the old `_without_worlds` twin — with the
/// manifest an explicit argument, "no worlds" is just a value, not a second
/// function.
pub fn build_platformer2d_asset_catalog(
    libraries: &ContentLibraries,
    config: &GameAssetConfig,
    character_catalog: &CharacterCatalog,
    boss_catalog: &ambition_boss_encounter::BossCatalog,
    music: &ambition_audio::spec::MusicRegistry,
    worlds: &ambition_platformer2d_world::world_manifest::WorldManifest,
) -> Platformer2dAssetCatalog {
    build_sandbox_catalog_with(
        libraries,
        config,
        character_catalog,
        boss_catalog,
        music,
        worlds,
        |_| {},
    )
}

pub fn build_sandbox_catalog_with(
    libraries: &ContentLibraries,
    config: &GameAssetConfig,
    character_catalog: &CharacterCatalog,
    boss_catalog: &ambition_boss_encounter::BossCatalog,
    music: &ambition_audio::spec::MusicRegistry,
    worlds: &ambition_platformer2d_world::world_manifest::WorldManifest,
    extend: impl FnOnce(&mut AssetManifest),
) -> Platformer2dAssetCatalog {
    let core_config = Platformer2dAssetCatalogConfig {
        sprite_folder: config.sprite_folder.clone(),
        asset_profile: config.asset_profile,
    };
    let image_manifest = libraries.asset_manifest().unwrap_or_else(|error| panic!("{error}"));
    let inputs = sandbox_catalog_inputs(character_catalog, boss_catalog, music, worlds);
    core::build_sandbox_catalog_with(&core_config, image_manifest, &inputs, |manifest| {
        extend(manifest);
        libraries.bind_catalog(manifest).unwrap_or_else(|error| panic!("{error}"));
    }).with_libraries(libraries).unwrap_or_else(|error| panic!("{error}"))
}

pub fn sandbox_catalog_inputs(
    character_catalog: &CharacterCatalog,
    boss_catalog: &ambition_boss_encounter::BossCatalog,
    music: &ambition_audio::spec::MusicRegistry,
    worlds: &ambition_platformer2d_world::world_manifest::WorldManifest,
) -> Platformer2dAssetCatalogInputs {
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
    boss_catalog: &ambition_boss_encounter::BossCatalog,
    music: &ambition_audio::spec::MusicRegistry,
) -> Platformer2dAssetCatalogInputs {
    Platformer2dAssetCatalogInputs {
        scale_variants: texture_scale_variants(),
        character_sprites: all_character_sprite_filenames_in(
            character_catalog,
        )
        .into_iter()
        .map(|(name, filename, qualified)| CharacterSpriteCatalogRow {
            name,
            filename,
            qualified,
        })
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
    ambition_persistence::settings::TextureResolutionScale::MANIFEST_VARIANTS
        .iter()
        .filter_map(|scale| {
            Some(AssetScaleVariant {
                asset_id_suffix: scale.asset_id_suffix()?,
                sprite_subdir_suffix: match scale {
                    ambition_persistence::settings::TextureResolutionScale::Half => "0_5x",
                    ambition_persistence::settings::TextureResolutionScale::Quarter => "0_25x",
                    ambition_persistence::settings::TextureResolutionScale::Potato => "potato",
                    ambition_persistence::settings::TextureResolutionScale::Full => return None,
                },
                parallax_subdir: scale.parallax_subdir(),
            })
        })
        .collect()
}

/// Return every `(character_id, on-disk filename, source-qualified path)` the
/// catalog declares, for asset-manifest registration. Used by the sandbox-
/// assets aggregator (`builders/visuals.rs::extend_with_character_entries`)
/// so adding a row to the catalog auto-registers the catalog id.
///
/// Filename is the basename of the catalog entry's `spritesheet`
/// field (stripped of the `sprites/` prefix the catalog stores them
/// under).
///
/// A path that names its own SOURCE — `game://sprites/mine.png`, the spelling
/// the consumer asset overlay exists for — is returned WHOLE in the third slot
/// instead. It has no basename under the engine's sprite folder, and reducing
/// it to one produced `sprites/game://sprites/mine.png`: a path to nothing,
/// which the silent-placeholder policy then rendered as a bare box. That is why
/// "a consumer owns its own art" stopped at the asset reader and never reached a
/// character.
fn all_character_sprite_filenames_from_data(
    catalog: &ambition_characters::actor::character_catalog::CharacterCatalogData,
) -> Vec<(String, String, Option<String>)> {
    let mut out: Vec<(String, String, Option<String>)> =
        Vec::with_capacity(catalog.characters.len());
    for (cid, entry) in &catalog.characters {
        let sheet = entry.spritesheet();
        let sheet = sheet.as_str();
        if ambition_asset_manager::platformer_assets::is_source_qualified(sheet) {
            out.push((cid.clone(), sheet.to_string(), Some(sheet.to_string())));
            continue;
        }
        let filename = sheet.strip_prefix("sprites/").unwrap_or(sheet).to_string();
        out.push((cid.clone(), filename, None));
    }
    out
}

/// Project the caller's App-local catalog into asset-manifest rows.
pub fn all_character_sprite_filenames_in(
    character_catalog: &CharacterCatalog,
) -> Vec<(String, String, Option<String>)> {
    all_character_sprite_filenames_from_data(character_catalog.data())
}

#[cfg(test)]
mod character_sprite_row_tests {
    use super::*;

    const CATALOG_A: &str = r#"(
        brain_presets: { "idle": StandStill },
        action_set_presets: { "peaceful": (move_style: Walk) },
        characters: {
            "alpha": (
                display_name: "Alpha",
                manifest: "sprites/alpha.ron", tier: MainHall, body_kind: Standard,
                composition: None, default_brain: "idle",
                default_action_set: "peaceful", tags: [],
            ),
        },
    )"#;

    const CATALOG_B: &str = r#"(
        brain_presets: { "idle": StandStill },
        action_set_presets: { "peaceful": (move_style: Walk) },
        characters: {
            "beta": (
                display_name: "Beta",
                manifest: "sprites/beta.ron", tier: MainHall, body_kind: Standard,
                composition: None, default_brain: "idle",
                default_action_set: "peaceful", tags: [],
            ),
        },
    )"#;

    fn catalog(ron: &str) -> CharacterCatalog {
        CharacterCatalog::from_data(
            ambition_characters::actor::character_catalog::parse_catalog(ron),
        )
    }

    #[test]
    fn sprite_manifest_projection_obeys_the_explicit_catalog() {
        let first = catalog(CATALOG_A);
        let second = catalog(CATALOG_B);

        assert_eq!(
            all_character_sprite_filenames_in(&first),
            vec![("alpha".to_string(), "alpha.png".to_string(), None)]
        );
        assert_eq!(
            all_character_sprite_filenames_in(&second),
            vec![("beta".to_string(), "beta.png".to_string(), None)]
        );
    }

    /// A consumer's own art survives catalog assembly.
    ///
    /// The engine's convention is a basename under the shared sprite folder, and
    /// every path went through it: `game://sprites/mine.png` had no `sprites/`
    /// prefix to strip, so the whole string became the "filename" and the
    /// manifest rebuilt it as `sprites/game://sprites/mine.png` — a path to
    /// nothing, silently placeheld into a bare box. The reader could reach the
    /// consumer's tree the whole time; nothing could ADDRESS it from a catalog
    /// .
    #[test]
    fn a_source_qualified_spritesheet_keeps_its_source() {
        const CONSUMER: &str = r#"(
            brain_presets: { "idle": StandStill },
            action_set_presets: {
                "peaceful": (move_style: Walk, melee: None, ranged: None, special: None),
            },
            characters: {
                "outlander": (
                    display_name: "Outlander",
                    manifest: "game://sprites/outlander.ron", tier: MainHall,
                    body_kind: Standard, composition: None, default_brain: "idle",
                    default_action_set: "peaceful", tags: [],
                ),
            },
        )"#;
        assert_eq!(
            all_character_sprite_filenames_in(&catalog(CONSUMER)),
            vec![(
                "outlander".to_string(),
                "game://sprites/outlander.png".to_string(),
                Some("game://sprites/outlander.png".to_string()),
            )],
            "the source-qualified path must arrive whole, not reduced to a \
             basename the engine will re-root under its own tree"
        );
    }
}
