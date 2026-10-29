//! Build asset roles from game-supplied content rows.

use crate::{AssetEntry, AssetId, AssetKind, AssetManifest, MissingAssetPolicy, PreloadGroup};

use super::{scaled_asset_id, AssetScaleVariant};

mod audio;
mod visuals;
mod world;

pub(super) use audio::extend_with_music_entries;
pub(super) use visuals::{extend_with_boss_entries, extend_with_character_entries};
pub(super) use world::extend_with_world_entries;

pub(super) fn insert_scaled_image_entry(
    manifest: &mut AssetManifest,
    base_id: &AssetId,
    logical_path: &str,
    scale: &AssetScaleVariant,
    preload_group: PreloadGroup,
) {
    let Some(id) = scaled_asset_id(base_id, Some(scale.asset_id_suffix)) else {
        return;
    };
    manifest.insert(
        AssetEntry::new(id, AssetKind::Image, logical_path)
            .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
            .with_preload_group(preload_group),
    );
}
