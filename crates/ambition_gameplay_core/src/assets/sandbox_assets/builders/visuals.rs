//! Visual asset builders: UI fonts + character / boss / intro
//! spritesheets.

use ambition_asset_manager::{
    AssetEntry, AssetId, AssetKind, AssetManifest, MissingAssetPolicy, PreloadGroup,
};

use super::super::{embedded_core, ids};
use super::with_embedded_core_candidate;
use crate::assets::game_assets::insert_scaled_image_entry;
use crate::persistence::settings::TextureResolutionScale;

/// UI font entries — the bundled fonts that ship with the sandbox
/// (Inter Display + JetBrains Mono) plus the legacy `font.*.legacy`
/// fallbacks that older saves expect.
///
/// Under `static_core_assets`, the three canonical fonts also carry an
/// authored `EmbeddedBinary` candidate so WebStatic / BundledStatic
/// resolve them through the embedded source.
pub(in super::super) fn extend_with_font_entries(manifest: &mut AssetManifest) {
    manifest.insert(with_embedded_core_candidate(
        AssetEntry::new(
            ids::font_dialog_regular(),
            AssetKind::Font,
            "fonts/bundled/InterDisplay-Regular.otf",
        )
        .with_missing_policy(MissingAssetPolicy::WarnAndPlaceholder)
        .with_preload_group(PreloadGroup::Hud),
        embedded_core::FONT_DIALOG_REGULAR_URL,
    ));
    manifest.insert(
        AssetEntry::new(
            AssetId::new("font.dialog_regular.legacy"),
            AssetKind::Font,
            "fonts/local/InterDisplay-Regular.otf",
        )
        .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
        .with_preload_group(PreloadGroup::Hud),
    );
    manifest.insert(with_embedded_core_candidate(
        AssetEntry::new(
            ids::font_dialog_semibold(),
            AssetKind::Font,
            "fonts/bundled/InterDisplay-SemiBold.otf",
        )
        .with_missing_policy(MissingAssetPolicy::WarnAndPlaceholder)
        .with_preload_group(PreloadGroup::Hud),
        embedded_core::FONT_DIALOG_SEMIBOLD_URL,
    ));
    manifest.insert(
        AssetEntry::new(
            AssetId::new("font.dialog_semibold.legacy"),
            AssetKind::Font,
            "fonts/local/InterDisplay-SemiBold.otf",
        )
        .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
        .with_preload_group(PreloadGroup::Hud),
    );
    manifest.insert(with_embedded_core_candidate(
        AssetEntry::new(
            ids::font_debug_mono(),
            AssetKind::Font,
            "fonts/bundled/JetBrainsMono-Regular.ttf",
        )
        .with_missing_policy(MissingAssetPolicy::WarnAndPlaceholder)
        .with_preload_group(PreloadGroup::Hud),
        embedded_core::FONT_DEBUG_MONO_URL,
    ));
    manifest.insert(
        AssetEntry::new(
            AssetId::new("font.debug_mono.legacy"),
            AssetKind::Font,
            "fonts/local/DejaVuSansMono.ttf",
        )
        .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
        .with_preload_group(PreloadGroup::Hud),
    );
}

/// Character sprite entries — one per `character_sprites::CHARACTER_SPRITE_REGISTRY`
/// row (player / robot / goblin / sandbag + every NPC sheet). Pulls
/// the canonical filename list from `character_sprites` so adding a new
/// NPC sheet there auto-registers the catalog id.
///
/// The four primary character sheets (`player`, `robot`, `goblin`,
/// `sandbag`) carry an `EmbeddedBinary` candidate under
/// `static_core_assets` so the wasm build renders the protagonist + the
/// basic enemy set without falling back to colored rectangles.
pub(in super::super) fn extend_with_character_entries(
    manifest: &mut AssetManifest,
    sprite_folder: &str,
) {
    for (name, filename) in crate::character_sprites::all_character_sprite_filenames() {
        let id = ids::character_sprite(&name);
        let logical_path = format!("{sprite_folder}/{filename}");
        let mut entry = AssetEntry::new(id, AssetKind::Image, logical_path)
            .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
            .with_preload_group(PreloadGroup::SandboxCore);
        if let Some(embedded_url) = character_sprite_embedded_url(&name) {
            entry = with_embedded_core_candidate(entry, embedded_url);
        }
        manifest.insert(entry);
        for scale in [
            TextureResolutionScale::Half,
            TextureResolutionScale::Quarter,
        ] {
            insert_scaled_image_entry(
                manifest,
                &ids::character_sprite(&name),
                &format!("{}/{}", scale.asset_subdir(sprite_folder), filename),
                scale,
                PreloadGroup::SandboxCore,
            );
        }
    }
}

/// Return the embedded-core URL for a character sprite label, when
/// that sheet is part of the core embedded set. Pairs with the
/// `EmbeddedAssetRegistry` insertions in `register_embedded_core_assets`.
fn character_sprite_embedded_url(name: &str) -> Option<&'static str> {
    match name {
        "player" => Some(embedded_core::SPRITE_PLAYER_URL),
        "robot" => Some(embedded_core::SPRITE_ROBOT_URL),
        "goblin" => Some(embedded_core::SPRITE_GOBLIN_URL),
        "sandbag" => Some(embedded_core::SPRITE_SANDBAG_URL),
        _ => None,
    }
}

/// Boss sprite entries — gradient sentinel + mockingbird today.
pub(in super::super) fn extend_with_boss_entries(
    manifest: &mut AssetManifest,
    sprite_folder: &str,
) {
    for (name, filename) in crate::boss_encounter::sprites::all_boss_sprite_filenames() {
        let id = ids::boss_sprite(name);
        let logical_path = format!("{sprite_folder}/{filename}");
        manifest.insert(
            AssetEntry::new(id, AssetKind::Image, logical_path)
                .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
                .with_preload_group(PreloadGroup::SandboxCore),
        );
        for scale in [
            TextureResolutionScale::Half,
            TextureResolutionScale::Quarter,
        ] {
            insert_scaled_image_entry(
                manifest,
                &ids::boss_sprite(name),
                &format!("{}/{}", scale.asset_subdir(sprite_folder), filename),
                scale,
                PreloadGroup::SandboxCore,
            );
        }
    }
}

// `extend_with_intro_sprite_entries` moved to `crate::content::intro::sprites`
// (content extends the manifest through `build_sandbox_catalog_with`).
