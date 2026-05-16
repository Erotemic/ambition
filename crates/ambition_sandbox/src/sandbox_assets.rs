//! Sandbox-side aggregator for the [`ambition_asset_manager`] catalog.
//!
//! Owns the construction of the **single** [`SandboxAssetCatalog`]
//! resource that every visible-sandbox subsystem reads from to resolve
//! Bevy asset paths. The catalog covers:
//!
//! - entity sprites + parallax layers (built from `game_assets.rs`'s
//!   `sandbox_image_manifest`)
//! - character spritesheets (player / robot / goblin / sandbag + NPC
//!   registry)
//! - boss spritesheets (gradient-sentinel + mockingbird)
//! - UI fonts (dialog regular / semibold + debug mono, with legacy
//!   per-source fallbacks)
//! - the LDtk world (`world.sandbox_ldtk`)
//! - the sandbox tuning RON (`data.sandbox`)
//! - the packed SFX bank (`audio.sfx_bank`)
//! - every music track in `sandbox_data.audio.music_tracks`
//!
//! Construction order (`build_sandbox_catalog`):
//! 1. start with the image manifest from
//!    [`crate::game_assets::sandbox_image_manifest`]
//! 2. extend with character / boss / font / music / world / data / SFX
//!    entries from the helpers in this module
//! 3. wrap into [`SandboxAssetCatalog`] with the active
//!    [`AssetProfile`].
//!
//! Everywhere else: ask the catalog for a path via
//! [`SandboxAssetCatalog::path_for`]; hand the string to Bevy's
//! `AssetServer`; let Bevy's per-platform `AssetReader` do the actual
//! IO. The catalog does **not** perform IO itself.

use std::path::PathBuf;

use bevy::prelude::Resource;

use ambition_asset_manager::{
    AmbitionAssetCatalog, AssetEntry, AssetId, AssetKind, AssetLocation, AssetManifest,
    AssetProfile, AssetResolutionError, AssetSourceProfile, MissingAssetPolicy, PreloadGroup,
    ResolvedAsset,
};

use crate::data::AudioSpec;
use crate::game_assets::{sandbox_image_manifest, GameAssetConfig};

/// Stable [`AssetId`] constructors for the fixed-vocabulary sandbox
/// assets. Bulk per-enum entries (entity sprites, parallax layers)
/// have their own builders in `game_assets.rs`; the music helper here
/// stays generic because music ids come from the RON catalog at
/// runtime.
pub mod ids {
    use ambition_asset_manager::AssetId;

    pub const SANDBOX_LDTK: &str = "world.sandbox_ldtk";
    pub const SANDBOX_DATA: &str = "data.sandbox";
    pub const SFX_BANK: &str = "audio.sfx_bank";
    pub const FONT_DIALOG_REGULAR: &str = "font.dialog_regular";
    pub const FONT_DIALOG_SEMIBOLD: &str = "font.dialog_semibold";
    pub const FONT_DEBUG_MONO: &str = "font.debug_mono";

    pub fn sandbox_ldtk() -> AssetId {
        AssetId::new(SANDBOX_LDTK)
    }
    pub fn sandbox_data() -> AssetId {
        AssetId::new(SANDBOX_DATA)
    }
    pub fn sfx_bank() -> AssetId {
        AssetId::new(SFX_BANK)
    }
    pub fn font_dialog_regular() -> AssetId {
        AssetId::new(FONT_DIALOG_REGULAR)
    }
    pub fn font_dialog_semibold() -> AssetId {
        AssetId::new(FONT_DIALOG_SEMIBOLD)
    }
    pub fn font_debug_mono() -> AssetId {
        AssetId::new(FONT_DEBUG_MONO)
    }

    /// `music.track.<id>` where `id` is the [`crate::data::MusicTrackSpec::id`]
    /// authored in `sandbox.ron`. The runtime registers one catalog entry
    /// per track and looks them up by this id.
    pub fn music_track(track_id: &str) -> AssetId {
        AssetId::new(format!("music.track.{track_id}"))
    }

    /// `sprite.character.<name>` for a character spritesheet. `name`
    /// is the sandbox-side label used by
    /// `crate::character_sprites::assets` (e.g. `player`, `robot`,
    /// `goblin`, or an NPC sprite key derived from the LDtk
    /// `NpcSpawn.name` field).
    pub fn character_sprite(name: &str) -> AssetId {
        AssetId::new(format!("sprite.character.{name}"))
    }

    /// `sprite.boss.<name>` for a boss spritesheet.
    pub fn boss_sprite(name: &str) -> AssetId {
        AssetId::new(format!("sprite.boss.{name}"))
    }
}

/// Wrapped [`AmbitionAssetCatalog`] + active [`AssetProfile`].
///
/// One instance per app session, installed as a Bevy `Resource` by
/// [`crate::app::init_sandbox_resources`]. Every subsystem that loads
/// an asset goes through this; nothing else owns asset-source policy.
///
/// Cheap to clone (the underlying manifest is wrapped in an `Arc`-like
/// shared shape inside [`AmbitionAssetCatalog`]'s `Clone` impl).
#[derive(Resource, Clone, Debug)]
pub struct SandboxAssetCatalog {
    catalog: AmbitionAssetCatalog,
    profile: AssetProfile,
}

impl SandboxAssetCatalog {
    /// Construct from a fully-built [`AmbitionAssetCatalog`] + the
    /// active profile. Prefer [`build_sandbox_catalog`] from
    /// production code; this is the seam for unit tests that author
    /// a partial manifest.
    pub fn new(catalog: AmbitionAssetCatalog, profile: AssetProfile) -> Self {
        Self { catalog, profile }
    }

    /// Convenience: build a desktop-dev catalog from the embedded
    /// sandbox spec, suitable for headless / RL / test fixtures that
    /// don't have a [`crate::game_assets::GameAssetConfig`] resource
    /// in hand. Production startup builds the catalog from the live
    /// `GameAssetConfig` via [`build_sandbox_catalog`].
    pub fn for_desktop_dev_default() -> Self {
        let mut config = crate::game_assets::GameAssetConfig::default();
        config.asset_profile = AssetProfile::DesktopDevLoose;
        let spec = crate::data::SandboxDataSpec::load_embedded();
        build_sandbox_catalog(&config, &spec.audio)
    }

    pub fn catalog(&self) -> &AmbitionAssetCatalog {
        &self.catalog
    }

    pub fn profile(&self) -> AssetProfile {
        self.profile
    }

    pub fn path_for(&self, id: &AssetId) -> Option<String> {
        self.catalog.path_for(id, self.profile)
    }

    pub fn resolve(&self, id: &AssetId) -> Result<ResolvedAsset, AssetResolutionError> {
        self.catalog.resolve(id, self.profile)
    }

    /// Local filesystem path the LDtk hot-reload watcher should poll,
    /// when both the active profile and the resolved location support
    /// it. `None` everywhere else (bundled / web / no-assets).
    pub fn hot_reload_local_path(&self, id: &AssetId) -> Option<PathBuf> {
        let resolved = self.resolve(id).ok()?;
        if !resolved.supports_hot_reload() {
            return None;
        }
        resolved.location.as_local_path().map(|p| p.to_path_buf())
    }

    /// Sandbox-side gate for *optional* image loads (entity sprites,
    /// parallax layers, character spritesheets, boss sheets, fonts).
    /// Mirrors the per-profile policy that used to live ad-hoc in
    /// `game_assets.rs::should_attempt_optional_image_load`.
    ///
    /// - Desktop (DevLoose / Installed / SteamDeck): pre-check the
    ///   host filesystem so missing optional art falls back to the
    ///   colored-rectangle / `Bevy` default-font path before Bevy
    ///   logs a load failure.
    /// - Android / iOS bundle: trust the packager and let Bevy try.
    /// - Web / BundledStatic / IpfsGateway: skip the load entirely —
    ///   optional images aren't packaged with these builds today.
    ///   Future per-asset `LocationCandidate`s opt back in once
    ///   packaging lands.
    /// - NoAssets / Headless: skip (the catalog already returned None).
    pub fn should_attempt_optional_load(&self, path: &str) -> bool {
        match self.profile {
            AssetProfile::DesktopDevLoose
            | AssetProfile::DesktopInstalled
            | AssetProfile::SteamDeckInstalled => desktop_loose_file_exists(path),
            AssetProfile::AndroidBundle | AssetProfile::IosBundle => true,
            AssetProfile::WebStatic
            | AssetProfile::WebHttp
            | AssetProfile::BundledStatic
            | AssetProfile::IpfsGatewayPlaceholder => false,
            AssetProfile::NoAssets | AssetProfile::Headless => false,
        }
    }

    /// Same gate, but for **required** assets. Required entries with
    /// no host-filesystem precheck always attempt the load; the
    /// resolver's `Disabled` path is what consults
    /// [`MissingAssetPolicy::Error`].
    pub fn should_attempt_required_load(&self, _path: &str) -> bool {
        !matches!(self.profile, AssetProfile::NoAssets | AssetProfile::Headless)
    }
}

/// Walk the same desktop candidate roots Bevy's file `AssetReader`
/// would, return true if any of them holds `rel_path`. This is the
/// only place in the sandbox that probes the host filesystem for
/// asset existence; every other module asks the catalog.
///
/// Lives here (not in `game_assets.rs`) because every loader gate
/// (sprites, fonts, parallax) routes through
/// [`SandboxAssetCatalog::should_attempt_optional_load`].
///
/// `[ambition_asset_manager_transition]` Marker for the in-flight
/// catalog migration. When the catalog grows native Bevy `AssetSource`
/// registration helpers, this probe collapses into the catalog itself
/// and the function can be deleted.
pub(crate) fn desktop_loose_file_exists(rel_path: &str) -> bool {
    let rel = std::path::Path::new(rel_path);
    let mut candidates = Vec::new();
    if let Some(root) = std::env::var_os("BEVY_ASSET_ROOT") {
        let root = std::path::PathBuf::from(root);
        candidates.push(root.join("assets").join(rel));
        candidates.push(root.join(rel));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("assets").join(rel));
        candidates.push(cwd.join(rel));
    }
    candidates.push(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join(rel),
    );
    candidates.into_iter().any(|path| path.exists())
}

/// Build the full sandbox catalog: every visible-sandbox asset id +
/// the active profile. Called once during `init_sandbox_resources`.
///
/// `audio` is borrowed from the already-loaded [`crate::data::SandboxDataSpec`]
/// so music-track ids land in the catalog at startup; the spec itself
/// is loaded via `include_str!`, so the catalog doesn't depend on
/// disk-resident files for bootstrap.
pub fn build_sandbox_catalog(
    config: &GameAssetConfig,
    audio: &AudioSpec,
) -> SandboxAssetCatalog {
    let mut manifest = sandbox_image_manifest(&config.sprite_folder);
    extend_with_world_entries(&mut manifest);
    extend_with_data_entries(&mut manifest);
    extend_with_sfx_bank_entry(&mut manifest);
    extend_with_font_entries(&mut manifest);
    extend_with_character_entries(&mut manifest, &config.sprite_folder);
    extend_with_boss_entries(&mut manifest, &config.sprite_folder);
    extend_with_music_entries(&mut manifest, audio);
    SandboxAssetCatalog::new(AmbitionAssetCatalog::new(manifest), config.asset_profile)
}

/// LDtk bootstrap entry — required asset that the game cannot run
/// without. Explicit `LooseFilesystem` candidate carries the absolute
/// `CARGO_MANIFEST_DIR/assets/...` path so the desktop hot-reload
/// watcher can find a `LocalPath` to inotify.
fn extend_with_world_entries(manifest: &mut AssetManifest) {
    let loose_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
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
            AssetLocation::LocalPath(loose_path),
        ),
    );
}

/// Sandbox tuning RON entry. Required — the game refuses to run
/// without it. Today the live consumer is
/// [`crate::data::SandboxDataSpec::load_embedded`] (always via
/// `include_str!`); the catalog entry exists so future code that asks
/// for the Bevy path under a non-static profile gets a real answer.
fn extend_with_data_entries(manifest: &mut AssetManifest) {
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
fn extend_with_sfx_bank_entry(manifest: &mut AssetManifest) {
    manifest.insert(
        AssetEntry::new(ids::sfx_bank(), AssetKind::AudioBank, "audio/sfx.bank")
            .with_missing_policy(MissingAssetPolicy::WarnAndPlaceholder)
            .with_preload_group(PreloadGroup::SandboxCore),
    );
}

/// UI font entries. Both the canonical `bundled/` paths and the legacy
/// `local/` fallback paths get their own ids so the font loader can
/// resolve each one and pick the first that exists.
fn extend_with_font_entries(manifest: &mut AssetManifest) {
    manifest.insert(
        AssetEntry::new(
            ids::font_dialog_regular(),
            AssetKind::Font,
            "fonts/bundled/InterDisplay-Regular.otf",
        )
        .with_missing_policy(MissingAssetPolicy::WarnAndPlaceholder)
        .with_preload_group(PreloadGroup::Hud),
    );
    manifest.insert(
        AssetEntry::new(
            AssetId::new("font.dialog_regular.legacy"),
            AssetKind::Font,
            "fonts/local/InterDisplay-Regular.otf",
        )
        .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
        .with_preload_group(PreloadGroup::Hud),
    );
    manifest.insert(
        AssetEntry::new(
            ids::font_dialog_semibold(),
            AssetKind::Font,
            "fonts/bundled/InterDisplay-SemiBold.otf",
        )
        .with_missing_policy(MissingAssetPolicy::WarnAndPlaceholder)
        .with_preload_group(PreloadGroup::Hud),
    );
    manifest.insert(
        AssetEntry::new(
            AssetId::new("font.dialog_semibold.legacy"),
            AssetKind::Font,
            "fonts/local/InterDisplay-SemiBold.otf",
        )
        .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
        .with_preload_group(PreloadGroup::Hud),
    );
    manifest.insert(
        AssetEntry::new(
            ids::font_debug_mono(),
            AssetKind::Font,
            "fonts/bundled/JetBrainsMono-Regular.ttf",
        )
        .with_missing_policy(MissingAssetPolicy::WarnAndPlaceholder)
        .with_preload_group(PreloadGroup::Hud),
    );
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
fn extend_with_character_entries(manifest: &mut AssetManifest, sprite_folder: &str) {
    for (name, filename) in crate::character_sprites::all_character_sprite_filenames() {
        let id = ids::character_sprite(name);
        let logical_path = format!("{sprite_folder}/{filename}");
        manifest.insert(
            AssetEntry::new(id, AssetKind::Image, logical_path)
                .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
                .with_preload_group(PreloadGroup::SandboxCore),
        );
    }
}

/// Boss sprite entries — gradient sentinel + mockingbird today.
fn extend_with_boss_entries(manifest: &mut AssetManifest, sprite_folder: &str) {
    for (name, filename) in crate::boss_sprites::all_boss_sprite_filenames() {
        let id = ids::boss_sprite(name);
        let logical_path = format!("{sprite_folder}/{filename}");
        manifest.insert(
            AssetEntry::new(id, AssetKind::Image, logical_path)
                .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
                .with_preload_group(PreloadGroup::SandboxCore),
        );
    }
}

/// Music track entries — one per `MusicTrackSpec` in the audio spec
/// that actually has an `asset_path` (pre-rendered OGG). Tracks
/// without `asset_path` are procedural-only and don't get an entry —
/// they're synthesized at runtime via `render_lofi_theme` and never
/// hit Bevy's `AssetServer`.
fn extend_with_music_entries(manifest: &mut AssetManifest, audio: &AudioSpec) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::SandboxDataSpec;
    use std::collections::HashSet;

    fn fixture_catalog() -> SandboxAssetCatalog {
        let config = GameAssetConfig::default();
        let spec = SandboxDataSpec::load_embedded();
        build_sandbox_catalog(&config, &spec.audio)
    }

    #[test]
    fn every_well_known_id_resolves_to_an_entry() {
        let catalog = fixture_catalog();
        let inner = catalog.catalog();
        for id_str in [
            ids::SANDBOX_LDTK,
            ids::SANDBOX_DATA,
            ids::SFX_BANK,
            ids::FONT_DIALOG_REGULAR,
            ids::FONT_DIALOG_SEMIBOLD,
            ids::FONT_DEBUG_MONO,
        ] {
            assert!(
                inner.manifest().get(&AssetId::new(id_str)).is_some(),
                "manifest missing well-known id `{id_str}`",
            );
        }
    }

    #[test]
    fn sandbox_ldtk_is_required_and_bootstrap() {
        let catalog = fixture_catalog();
        let entry = catalog
            .catalog()
            .manifest()
            .get(&ids::sandbox_ldtk())
            .unwrap();
        assert_eq!(entry.kind, AssetKind::LdtkProject);
        assert_eq!(entry.missing_policy, MissingAssetPolicy::Error);
        assert_eq!(entry.preload_group, Some(PreloadGroup::Bootstrap));
    }

    #[test]
    fn sandbox_data_is_required_and_bootstrap() {
        let catalog = fixture_catalog();
        let entry = catalog
            .catalog()
            .manifest()
            .get(&ids::sandbox_data())
            .unwrap();
        assert_eq!(entry.kind, AssetKind::RonData);
        assert_eq!(entry.missing_policy, MissingAssetPolicy::Error);
        assert_eq!(entry.preload_group, Some(PreloadGroup::Bootstrap));
    }

    #[test]
    fn sfx_bank_resolves_under_desktop_dev_loose() {
        let mut config = GameAssetConfig::default();
        config.asset_profile = AssetProfile::DesktopDevLoose;
        let spec = SandboxDataSpec::load_embedded();
        let catalog = build_sandbox_catalog(&config, &spec.audio);
        let path = catalog.path_for(&ids::sfx_bank()).expect("sfx_bank path");
        assert_eq!(path, "audio/sfx.bank");
    }

    #[test]
    fn ldtk_resolves_to_local_path_under_desktop_dev_loose() {
        let mut config = GameAssetConfig::default();
        config.asset_profile = AssetProfile::DesktopDevLoose;
        let spec = SandboxDataSpec::load_embedded();
        let catalog = build_sandbox_catalog(&config, &spec.audio);
        let resolved = catalog.resolve(&ids::sandbox_ldtk()).unwrap();
        // Explicit LooseFilesystem candidate -> LocalPath that the
        // hot-reload watcher can poll.
        assert!(resolved.location.as_local_path().is_some());
        assert!(resolved.supports_hot_reload());
        assert!(catalog.hot_reload_local_path(&ids::sandbox_ldtk()).is_some());
    }

    #[test]
    fn ldtk_falls_back_to_embedded_under_web_static() {
        let mut config = GameAssetConfig::default();
        config.asset_profile = AssetProfile::WebStatic;
        let spec = SandboxDataSpec::load_embedded();
        let catalog = build_sandbox_catalog(&config, &spec.audio);
        let path = catalog.path_for(&ids::sandbox_ldtk()).unwrap();
        assert_eq!(path, format!("embedded://{}", crate::ldtk_world::SANDBOX_LDTK_ASSET));
        // Web static does NOT support hot reload.
        assert!(!catalog
            .resolve(&ids::sandbox_ldtk())
            .unwrap()
            .supports_hot_reload());
    }

    #[test]
    fn bundled_static_does_not_support_hot_reload() {
        let mut config = GameAssetConfig::default();
        config.asset_profile = AssetProfile::BundledStatic;
        let spec = SandboxDataSpec::load_embedded();
        let catalog = build_sandbox_catalog(&config, &spec.audio);
        assert!(catalog.hot_reload_local_path(&ids::sandbox_ldtk()).is_none());
    }

    #[test]
    fn no_assets_disables_optional_image_and_font_entries() {
        let mut config = GameAssetConfig::default();
        config.asset_profile = AssetProfile::NoAssets;
        let spec = SandboxDataSpec::load_embedded();
        let catalog = build_sandbox_catalog(&config, &spec.audio);
        assert!(catalog.path_for(&ids::font_dialog_regular()).is_none());
        assert!(catalog.path_for(&ids::sfx_bank()).is_none());
    }

    #[test]
    fn music_track_ids_match_audio_spec() {
        let catalog = fixture_catalog();
        let spec = SandboxDataSpec::load_embedded();
        for track in &spec.audio.music_tracks {
            let id = ids::music_track(&track.id);
            if track.asset_path.is_some() {
                let entry = catalog
                    .catalog()
                    .manifest()
                    .get(&id)
                    .unwrap_or_else(|| panic!("missing music catalog entry for {id}"));
                assert_eq!(entry.kind, AssetKind::AudioClip);
            }
        }
    }

    #[test]
    fn all_catalog_ids_are_unique() {
        let catalog = fixture_catalog();
        let mut seen = HashSet::new();
        for (id, _) in catalog.catalog().manifest().iter() {
            assert!(seen.insert(id.clone()), "duplicate id: {id}");
        }
    }

    #[test]
    fn should_attempt_required_load_only_disabled_for_no_assets_profiles() {
        for (profile, expected) in [
            (AssetProfile::DesktopDevLoose, true),
            (AssetProfile::DesktopInstalled, true),
            (AssetProfile::AndroidBundle, true),
            (AssetProfile::WebStatic, true),
            (AssetProfile::BundledStatic, true),
            (AssetProfile::IpfsGatewayPlaceholder, true),
            (AssetProfile::NoAssets, false),
            (AssetProfile::Headless, false),
        ] {
            let mut config = GameAssetConfig::default();
            config.asset_profile = profile;
            let spec = SandboxDataSpec::load_embedded();
            let catalog = build_sandbox_catalog(&config, &spec.audio);
            assert_eq!(
                catalog.should_attempt_required_load("foo.png"),
                expected,
                "{}",
                profile.label(),
            );
        }
    }
}
