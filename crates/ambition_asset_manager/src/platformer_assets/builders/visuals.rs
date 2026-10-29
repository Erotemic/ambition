//! Visual asset builders: UI fonts + character / boss / intro
//! spritesheets.

use crate::{AssetEntry, AssetKind, AssetManifest, MissingAssetPolicy, PreloadGroup};

use super::super::ids;
use super::super::{AssetScaleVariant, BossSpriteCatalogRow, CharacterSpriteCatalogRow};
use super::insert_scaled_image_entry;

/// Add the character image roles supplied by the composition.
pub(in super::super) fn extend_with_character_entries(
    manifest: &mut AssetManifest,
    sprite_folder: &str,
    rows: &[CharacterSpriteCatalogRow],
    scale_variants: &[AssetScaleVariant],
) {
    for CharacterSpriteCatalogRow {
        name,
        filename,
        qualified,
    } in rows
    {
        let id = ids::character_sprite(&name);
        // `qualified` carries the authored path when the catalog named its own
        // source. `logical_asset_path` alone decides whether to join.
        let authored = qualified.as_deref().unwrap_or(filename.as_str());
        let logical_path = super::super::logical_asset_path(sprite_folder, authored);
        let entry = AssetEntry::new(id, AssetKind::Image, logical_path)
            .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
            .with_preload_group(PreloadGroup::SandboxCore);
        manifest.insert(entry);
        for scale in scale_variants {
            let Some(scaled) = super::super::scaled_logical_asset_path(
                sprite_folder,
                scale.sprite_subdir_suffix,
                authored,
            ) else {
                continue;
            };
            insert_scaled_image_entry(
                manifest,
                &ids::character_sprite(&name),
                &scaled,
                scale,
                PreloadGroup::SandboxCore,
            );
        }
    }
}

/// Add the boss image roles supplied by the composition.
pub(in super::super) fn extend_with_boss_entries(
    manifest: &mut AssetManifest,
    sprite_folder: &str,
    rows: &[BossSpriteCatalogRow],
    scale_variants: &[AssetScaleVariant],
) {
    for BossSpriteCatalogRow { name, filename } in rows {
        let id = ids::boss_sprite(name);
        // Through the helper, not inline: a consumer-authored boss names its own
        // art the same way a consumer-authored character does.
        let logical_path = super::super::logical_asset_path(sprite_folder, filename);
        manifest.insert(
            AssetEntry::new(id, AssetKind::Image, logical_path)
                .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
                .with_preload_group(PreloadGroup::SandboxCore),
        );
        for scale in scale_variants {
            let Some(scaled) = super::super::scaled_logical_asset_path(
                sprite_folder,
                scale.sprite_subdir_suffix,
                filename,
            ) else {
                continue;
            };
            insert_scaled_image_entry(
                manifest,
                &ids::boss_sprite(name),
                &scaled,
                scale,
                PreloadGroup::SandboxCore,
            );
        }
    }
}
