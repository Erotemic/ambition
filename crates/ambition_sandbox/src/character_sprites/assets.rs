//! Spritesheet asset bundle + on-disk loading.
//!
//! Each character target has its own PNG; missing files are not
//! errors — callers fall back to colored rectangles (the game must
//! always run regardless of asset state). Android assets live inside
//! the APK; desktop reads through `CARGO_MANIFEST_DIR`.

use bevy::prelude::*;

use super::sheets::{CharacterSheetSpec, GOBLIN_SHEET, ROBOT_SHEET, SANDBAG_SHEET};
use crate::features::FeatureVisualKind;

#[derive(Clone)]
pub struct CharacterSpriteAsset {
    pub texture: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub spec: CharacterSheetSpec,
}

/// Holds optional spritesheet handles. `None` = file missing → fallback.
#[derive(Resource, Default, Clone)]
pub struct CharacterSpriteAssets {
    pub robot: Option<CharacterSpriteAsset>,
    pub goblin: Option<CharacterSpriteAsset>,
    pub sandbag: Option<CharacterSpriteAsset>,
    // The boss uses the entity-sprite path (`EntitySprite::BossCore`) rather
    // than the character-spritesheet path: its generator emits non-standard
    // animation rows (rest/floor_slam/side_sweep/spike_halo/dash_echo/hit/
    // death) that don't fit `CharacterAnim`'s 8-variant grid. When/if the
    // boss gets a CharacterAnim-compatible sheet, add a `boss` field here.
}

impl CharacterSpriteAssets {
    pub fn enemy_asset(&self, kind: FeatureVisualKind) -> Option<&CharacterSpriteAsset> {
        match kind {
            FeatureVisualKind::Enemy => self.goblin.as_ref(),
            FeatureVisualKind::Sandbag => self.sandbag.as_ref().or(self.goblin.as_ref()),
            _ => None,
        }
    }
}

const ROBOT_FILENAME: &str = "robot_spritesheet.png";
const GOBLIN_FILENAME: &str = "goblin_spritesheet.png";
const SANDBAG_FILENAME: &str = "sandbag_spritesheet.png";

/// Probe the sandbox `assets/<sprite_folder>/` directory for spritesheets.
/// Missing files are not an error — callers fall back to colored rectangles.
pub fn load_character_sprites_in(
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    sprite_folder: &str,
) -> CharacterSpriteAssets {
    let robot_rel = format!("{sprite_folder}/{ROBOT_FILENAME}");
    let goblin_rel = format!("{sprite_folder}/{GOBLIN_FILENAME}");
    let sandbag_rel = format!("{sprite_folder}/{SANDBAG_FILENAME}");

    let robot = build_optional(asset_server, layouts, &robot_rel, ROBOT_SHEET);
    let goblin = build_optional(asset_server, layouts, &goblin_rel, GOBLIN_SHEET);
    let sandbag = build_optional(asset_server, layouts, &sandbag_rel, SANDBAG_SHEET);

    for (name, rel, present) in [
        ("robot", &robot_rel, robot.is_some()),
        ("goblin", &goblin_rel, goblin.is_some()),
        ("sandbag", &sandbag_rel, sandbag.is_some()),
    ] {
        if !present {
            eprintln!(
                "[character_sprites] {name} spritesheet not found at assets/{rel} — falling back to colored rectangle"
            );
        }
    }

    CharacterSpriteAssets {
        robot,
        goblin,
        sandbag,
    }
}

fn build_optional(
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    rel_path: &str,
    spec: CharacterSheetSpec,
) -> Option<CharacterSpriteAsset> {
    if !asset_exists(rel_path) {
        return None;
    }
    let layout = layouts.add(spec.build_atlas());
    Some(CharacterSpriteAsset {
        texture: asset_server.load(rel_path.to_string()),
        layout,
        spec,
    })
}

fn asset_exists(rel_path: &str) -> bool {
    // Android assets live inside the APK, not under the host-side
    // CARGO_MANIFEST_DIR. Let Bevy's Android asset reader try the load.
    #[cfg(target_os = "android")]
    {
        let _ = rel_path;
        true
    }

    // Bevy's FileAssetReader resolves assets relative to CARGO_MANIFEST_DIR
    // when running through cargo. Mirror that here so the existence check
    // matches the asset server's lookup path.
    #[cfg(not(target_os = "android"))]
    {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        std::path::Path::new(manifest_dir)
            .join("assets")
            .join(rel_path)
            .exists()
    }
}
