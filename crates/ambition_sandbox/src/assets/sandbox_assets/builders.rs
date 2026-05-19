//! Per-domain manifest builders.
//!
//! `build_sandbox_catalog` (in `sandbox_assets/mod.rs`) calls the
//! `extend_with_*` helpers here in turn to assemble the
//! `AssetManifest` that `SandboxAssetCatalog` wraps. Each helper owns
//! one slice of authored data (worlds, tuning RON, SFX bank, fonts,
//! characters, bosses, intro sprites, music) so adding an asset is
//! one builder edit instead of a 200-line scroll through the catalog
//! file.

use ambition_asset_manager::{
    AssetEntry, AssetId, AssetKind, AssetLocation, AssetManifest, AssetSourceProfile,
    MissingAssetPolicy, PreloadGroup,
};

use crate::content::data::AudioSpec;

use super::ids;
use super::{embedded_core, EMBEDDED_INTRO_LDTK_ASSET_PATH, EMBEDDED_SANDBOX_LDTK_ASSET_PATH};

/// LDtk world entries. The primary `world.sandbox_ldtk` is required —
/// the game cannot run without it. Secondary worlds (`world.intro_ldtk`
/// today) are optional: the merge loader skips them silently if the
/// catalog reports them disabled, matching the prior "tolerate missing
/// secondary file" behavior.
///
/// Explicit `LooseFilesystem` candidates carry the absolute
/// `CARGO_MANIFEST_DIR/assets/...` path so the desktop hot-reload
/// watcher (primary world only) can find a `LocalPath` to inotify.
pub(super) fn extend_with_world_entries(manifest: &mut AssetManifest) {
    let loose_sandbox = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(crate::ldtk_world::SANDBOX_LDTK_ASSET);
    manifest.insert(
        AssetEntry::new(
            ids::sandbox_ldtk(),
            AssetKind::LdtkProject,
            crate::ldtk_world::SANDBOX_LDTK_ASSET,
        )
        .with_missing_policy(MissingAssetPolicy::Error)
        .with_preload_group(PreloadGroup::Bootstrap)
        .with_location(
            AssetSourceProfile::LooseFilesystem,
            AssetLocation::LocalPath(loose_sandbox),
        )
        .with_location(
            AssetSourceProfile::EmbeddedBinary,
            AssetLocation::embedded(EMBEDDED_SANDBOX_LDTK_ASSET_PATH),
        ),
    );

    // intro.ldtk lives next to sandbox.ldtk and is loaded by the
    // secondary-worlds merge step. Optional today because a fresh
    // checkout without the intro file should still boot the sandbox.
    let loose_intro = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("ambition/worlds/intro.ldtk");
    manifest.insert(
        AssetEntry::new(
            ids::intro_ldtk(),
            AssetKind::LdtkProject,
            "ambition/worlds/intro.ldtk",
        )
        .with_missing_policy(MissingAssetPolicy::WarnAndPlaceholder)
        .with_preload_group(PreloadGroup::Bootstrap)
        .with_location(
            AssetSourceProfile::LooseFilesystem,
            AssetLocation::LocalPath(loose_intro),
        )
        .with_location(
            AssetSourceProfile::EmbeddedBinary,
            AssetLocation::embedded(EMBEDDED_INTRO_LDTK_ASSET_PATH),
        ),
    );
}

/// Sandbox tuning RON entry. Required — the game refuses to run
/// without it. Today the live consumer is
/// [`crate::content::data::SandboxDataSpec::load_embedded`] (always via
/// `include_str!`); the catalog entry exists so future code that asks
/// for the Bevy path under a non-static profile gets a real answer.
pub(super) fn extend_with_data_entries(manifest: &mut AssetManifest) {
    manifest.insert(
        AssetEntry::new(
            ids::sandbox_data(),
            AssetKind::RonData,
            "ambition/sandbox.ron",
        )
        .with_missing_policy(MissingAssetPolicy::Error)
        .with_preload_group(PreloadGroup::Bootstrap),
    );
}

/// Packed SFX bank entry. `WarnAndPlaceholder` matches the current
/// runtime contract: a missing bank degrades to procedural / silent
/// SFX instead of refusing to start.
///
/// `AMBITION_SFX_BANK_PATH` is honored as an explicit
/// `LooseFilesystem` `LocationCandidate` so dev workflows can point
/// the sandbox at a freshly-packed bank without re-publishing assets.
pub(super) fn extend_with_sfx_bank_entry(manifest: &mut AssetManifest) {
    let mut entry = AssetEntry::new(ids::sfx_bank(), AssetKind::AudioBank, "audio/sfx.bank")
        .with_missing_policy(MissingAssetPolicy::WarnAndPlaceholder)
        .with_preload_group(PreloadGroup::SandboxCore);
    if let Ok(env_path) = std::env::var("AMBITION_SFX_BANK_PATH") {
        entry = entry.with_location(
            AssetSourceProfile::LooseFilesystem,
            AssetLocation::LocalPath(std::path::PathBuf::from(env_path)),
        );
    }
    manifest.insert(entry);
}

/// UI font entries — the bundled fonts that ship with the sandbox
/// (Inter Display + JetBrains Mono) plus the legacy `font.*.legacy`
/// fallbacks that older saves expect.
///
/// Under `static_core_assets`, the three canonical fonts also carry an
/// authored `EmbeddedBinary` candidate so WebStatic / BundledStatic
/// resolve them through the embedded source.
pub(super) fn extend_with_font_entries(manifest: &mut AssetManifest) {
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

/// Attach an `EmbeddedBinary` `LocationCandidate` to `entry` IFF the
/// `static_core_assets` feature is enabled. Without the feature the
/// embedded source has no bytes for the URL, so adding the candidate
/// would mislead the resolver into trying to load a 404.
#[cfg(feature = "static_core_assets")]
fn with_embedded_core_candidate(entry: AssetEntry, embedded_url: &'static str) -> AssetEntry {
    entry.with_location(
        AssetSourceProfile::EmbeddedBinary,
        AssetLocation::embedded(embedded_url.to_string()),
    )
}

#[cfg(not(feature = "static_core_assets"))]
fn with_embedded_core_candidate(entry: AssetEntry, _embedded_url: &'static str) -> AssetEntry {
    entry
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
pub(super) fn extend_with_character_entries(manifest: &mut AssetManifest, sprite_folder: &str) {
    for (name, filename) in crate::presentation::character_sprites::all_character_sprite_filenames()
    {
        let id = ids::character_sprite(name);
        let logical_path = format!("{sprite_folder}/{filename}");
        let mut entry = AssetEntry::new(id, AssetKind::Image, logical_path)
            .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
            .with_preload_group(PreloadGroup::SandboxCore);
        if let Some(embedded_url) = character_sprite_embedded_url(name) {
            entry = with_embedded_core_candidate(entry, embedded_url);
        }
        manifest.insert(entry);
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
pub(super) fn extend_with_boss_entries(manifest: &mut AssetManifest, sprite_folder: &str) {
    for (name, filename) in crate::boss_encounter::sprites::all_boss_sprite_filenames() {
        let id = ids::boss_sprite(name);
        let logical_path = format!("{sprite_folder}/{filename}");
        manifest.insert(
            AssetEntry::new(id, AssetKind::Image, logical_path)
                .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
                .with_preload_group(PreloadGroup::SandboxCore),
        );
    }
}

/// Intro NPC + prop sprite entries. The intro story content owns its
/// own `INTRO_NPC_SPRITE_REGISTRY` / `INTRO_PROP_REGISTRY` constants
/// in `crate::intro::sprites`; this helper walks both at catalog-build
/// time so the intro plugin's load systems can resolve their assets
/// via `catalog.try_path_for_load(...)` like every other loader.
///
/// IDs are `sprite.character.intro_<name_snake>` for NPCs and
/// `sprite.character.intro_prop_<kind_snake>` for props. Both use
/// `SilentPlaceholder` because missing intro art falls back to colored
/// rectangles per the existing contract.
pub(super) fn extend_with_intro_sprite_entries(manifest: &mut AssetManifest, sprite_folder: &str) {
    use crate::intro::sprites::{
        intro_npc_asset_id, intro_npc_sprite_rows, intro_prop_asset_id, intro_prop_sprite_rows,
    };
    for (name, filename, _spec) in intro_npc_sprite_rows() {
        let id = intro_npc_asset_id(name);
        let logical_path = format!("{sprite_folder}/{filename}");
        manifest.insert(
            AssetEntry::new(id, AssetKind::Image, logical_path)
                .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
                .with_preload_group(PreloadGroup::SandboxCore),
        );
    }
    for (kind, filename, _spec) in intro_prop_sprite_rows() {
        let id = intro_prop_asset_id(kind);
        let logical_path = format!("{sprite_folder}/{filename}");
        manifest.insert(
            AssetEntry::new(id, AssetKind::Image, logical_path)
                .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
                .with_preload_group(PreloadGroup::SandboxCore),
        );
    }
}

/// Music track entries — one per `MusicTrackSpec` in the audio spec
/// that has an `asset_path` (pre-rendered OGG). Tracks without
/// `asset_path` are skipped at both the catalog layer and the
/// `AudioLibrary` layer (the procedural fundsp music generator was
/// retired; see `docs/archive/retired/fundsp-audio.md` for the historical note).
/// Spec authors must add a pre-rendered OGG via
/// `tools/ambition_music_renderer` or remove the track from
/// `sandbox.ron`.
pub(super) fn extend_with_music_entries(manifest: &mut AssetManifest, audio: &AudioSpec) {
    for track in &audio.music_tracks {
        let Some(asset_path) = track.asset_path.as_deref() else {
            continue;
        };
        let id = ids::music_track(&track.id);
        manifest.insert(
            AssetEntry::new(id, AssetKind::AudioClip, asset_path.to_string())
                .with_missing_policy(MissingAssetPolicy::WarnAndPlaceholder)
                .with_preload_group(PreloadGroup::SandboxCore),
        );
    }
}
