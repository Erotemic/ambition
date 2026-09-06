//! Sandbox-side aggregator for the [`ambition_asset_manager`] catalog.
//!
//! This module builds the single [`Platformer2dAssetCatalog`] resource used by visible
//! sandbox systems to resolve Bevy asset paths: sprites, parallax, character and
//! boss sheets, fonts, LDtk world/data, SFX bank, and music tracks.
//!
//! Construction starts from a caller-provided image manifest, extends it with
//! caller-provided content rows (worlds, music, sprite registries), and wraps the
//! result with the active [`AssetProfile`]. Consumers ask the catalog for a path
//! and pass it to Bevy's `AssetServer`; the catalog itself performs no IO.

use std::path::PathBuf;

use bevy::prelude::Resource;

use crate::{
    AmbitionAssetCatalog, AssetId, AssetManifest, AssetProfile, AssetResolutionError,
    AssetSourceProfile, ResolvedAsset,
};

mod builders;
mod embedded;
pub mod ids;

use builders::{
    extend_with_boss_entries, extend_with_character_entries, extend_with_data_entries,
    extend_with_font_entries, extend_with_music_entries, extend_with_sfx_bank_entry,
    extend_with_sprite_pack_entries, extend_with_world_entries,
};
pub use embedded::embedded_core;
pub use embedded::{AmbitionAssetSourcePlugin, EmbeddedWorldAsset};

/// Runtime asset-profile/config values needed by the sandbox catalog builder.
#[derive(Clone, Debug)]
pub struct Platformer2dAssetCatalogConfig {
    pub sprite_folder: String,
    pub asset_profile: AssetProfile,
}

/// One optional scaled asset tier registered beside full-resolution images.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetScaleVariant {
    pub asset_id_suffix: &'static str,
    pub sprite_subdir_suffix: &'static str,
    pub parallax_subdir: &'static str,
}

/// Filename row for a character spritesheet.
///
/// `filename` is a BASENAME under the shared sprite folder — the engine's own
/// convention, and the reason a consumer's art could not get through this seam:
/// every path was reduced to a basename and rebuilt as
/// `{sprite_folder}/{filename}`, so an authored `game://sprites/mine.png` came
/// out as `sprites/game://sprites/mine.png`.
///
/// `qualified` is the escape: a path that already names its own SOURCE
/// (`game://…`, `embedded://…`) is carried through verbatim and never rebuilt.
/// A consumer's art keeps its identity from the catalog to the manifest, which
/// is what "a game gets to own its own art" has to mean past the reader.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterSpriteCatalogRow {
    pub name: String,
    pub filename: String,
    /// `Some(path)` when the catalog named a source-qualified path. Mutually
    /// exclusive with the folder convention: when it is set, `filename` is only
    /// a display echo and the manifest uses this.
    pub qualified: Option<String>,
}

/// Does this authored path name its own asset SOURCE?
///
/// Bevy's own spelling: `source://path`. Deliberately not a general URL parse —
/// the question is only whether the author already said where this lives, and
/// anything with a scheme did.
pub fn is_source_qualified(path: &str) -> bool {
    path.split_once("://")
        .is_some_and(|(scheme, rest)| !scheme.is_empty() && !rest.is_empty())
}

/// Join an authored filename to the folder convention — UNLESS the author
/// already said where the file lives.
///
/// The one place this join is allowed to happen, because doing it inline is a
/// mistake this repo has now made three times in three different seams
/// (catalog→manifest, the sheet index, and the desktop load gate), each time by
/// treating `game://sprites/x.png` as a relative path and producing
/// `sprites/game://sprites/x.png` or looking for a file at a root that could
/// never hold it. A fourth is only a matter of which builder somebody edits
/// next, so the join has a name now and the name knows the rule.
pub fn logical_asset_path(folder: &str, filename: &str) -> String {
    if is_source_qualified(filename) {
        filename.to_owned()
    } else {
        format!("{folder}/{filename}")
    }
}

/// The scaled-variant sibling of [`logical_asset_path`], or `None` when the
/// author owns the path.
///
/// A `sprites_0_5x/…` twin is generated into the ENGINE's tree by this repo's
/// own tooling. Inventing that layout inside somebody else's asset source is a
/// convention they never agreed to, so a source-qualified asset simply has no
/// variants — it renders at full resolution, which is correct and visible,
/// rather than resolving to a path that does not exist.
pub fn scaled_logical_asset_path(
    folder: &str,
    subdir_suffix: &str,
    filename: &str,
) -> Option<String> {
    (!is_source_qualified(filename))
        .then(|| format!("{folder}_{subdir_suffix}/{filename}"))
}

/// Filename row for a boss spritesheet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BossSpriteCatalogRow {
    pub name: String,
    pub filename: String,
}

/// Music row reduced to the asset-catalog data needed by this crate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MusicCatalogRow {
    pub id: String,
    pub asset_path: String,
}

/// World row reduced to the asset-catalog data needed by this crate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldCatalogRow {
    pub id: AssetId,
    pub asset_path: String,
    pub required: bool,
    pub loose_path: Option<PathBuf>,
    pub embedded_bevy_path: Option<&'static str>,
}

/// Caller-provided catalog rows that would otherwise force this foundational
/// crate to import gameplay/session/content modules.
#[derive(Clone, Debug, Default)]
pub struct Platformer2dAssetCatalogInputs {
    pub scale_variants: Vec<AssetScaleVariant>,
    pub character_sprites: Vec<CharacterSpriteCatalogRow>,
    pub boss_sprites: Vec<BossSpriteCatalogRow>,
    pub music_tracks: Vec<MusicCatalogRow>,
    pub worlds: Vec<WorldCatalogRow>,
}

/// Wrapped [`AmbitionAssetCatalog`] + active [`AssetProfile`].
///
/// One instance per app session, installed as a Bevy `Resource` by
/// [`crate::schedule::init_sandbox_resources`]. Every subsystem that loads
/// an asset goes through this; nothing else owns asset-source policy.
///
/// Cheap to clone (the underlying manifest is wrapped in an `Arc`-like
/// shared shape inside [`AmbitionAssetCatalog`]'s `Clone` impl).
#[derive(Resource, Clone, Debug)]
pub struct Platformer2dAssetCatalog {
    catalog: AmbitionAssetCatalog,
    profile: AssetProfile,
}

impl Platformer2dAssetCatalog {
    /// Construct from a fully-built [`AmbitionAssetCatalog`] + the
    /// active profile. Prefer [`build_platformer2d_asset_catalog`] from
    /// production code; this is the seam for unit tests that author
    /// a partial manifest.
    pub fn new(catalog: AmbitionAssetCatalog, profile: AssetProfile) -> Self {
        Self { catalog, profile }
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

    /// Resolve `id` and apply the per-profile load gate in one call.
    ///
    /// Returns `Some(path)` when the loader should hand the path to
    /// Bevy's `AssetServer::load`; `None` when the loader should fall
    /// back (colored rectangle, silent SFX, Bevy default font, etc.).
    ///
    /// This is the only function loaders need to call — it combines:
    /// - `path_for(id)` (resolver),
    /// - the per-profile "is this asset actually available?" gate
    ///   ([`Self::should_attempt_resolved_load`]).
    ///
    /// Consumers that need the local on-disk path (the SFX bank byte
    /// loader, the LDtk hot-reload watcher) go through
    /// [`Self::resolve_local_file_path`] / [`Self::hot_reload_local_path`].
    pub fn try_path_for_load(&self, id: &AssetId) -> Option<String> {
        let resolved = self.resolve(id).ok()?;
        let path = resolved.bevy_asset_path()?;
        if self.should_attempt_resolved_load(&resolved, &path) {
            Some(path)
        } else {
            None
        }
    }

    /// Resolve a scaled visual variant first, then silently fall back to the
    /// canonical full-resolution asset id. This is the runtime quality-profile
    /// seam for optional images and spritesheets.
    pub fn try_quality_path_for_load(
        &self,
        id: &AssetId,
        scale_asset_id_suffix: Option<&str>,
        prefer_scaled_variant: bool,
    ) -> Option<String> {
        if prefer_scaled_variant {
            if let Some(variant_id) = scaled_asset_id(id, scale_asset_id_suffix) {
                if let Some(path) = self.try_path_for_load(&variant_id) {
                    return Some(path);
                }
            }
        }
        self.try_path_for_load(id)
    }

    /// Per-profile load gate keyed on a fully-resolved entry.
    ///
    /// - Desktop (DevLoose / Installed / SteamDeck): pre-check the host
    ///   filesystem via the candidate-roots walker
    ///   ([`desktop_candidate_roots`]) so missing optional art falls back
    ///   to colored rectangles / Bevy's default font before Bevy logs a
    ///   load failure. Required assets always attempt the load so the
    ///   `MissingAssetPolicy::Error` path can surface a useful error.
    /// - Android / iOS bundle: trust the packager; let Bevy's platform
    ///   `AssetReader` try the load.
    /// - Web / BundledStatic: attempt the load when the entry has an
    ///   authored embedded candidate (the bytes are packaged via
    ///   `embedded_asset!`); skip otherwise to preserve colored-rectangle
    ///   fallback.
    /// - WebHttp: attempt only when the entry has an authored
    ///   `HttpRemote` candidate. Optional images today have none, so they
    ///   fall back to placeholders.
    /// - IpfsGatewayPlaceholder: attempt when an authored `IpfsGateway`
    ///   candidate is present.
    /// - NoAssets / Headless: never attempt (catalog already returned
    ///   None for `path_for`; this is exhaustive-match insurance).
    pub fn should_attempt_resolved_load(&self, resolved: &ResolvedAsset, path: &str) -> bool {
        match self.profile {
            AssetProfile::DesktopDevLoose
            | AssetProfile::DesktopInstalled
            | AssetProfile::SteamDeckInstalled => {
                resolved.missing_policy.is_required()
                    // A SOURCE-QUALIFIED path belongs to a custom `AssetSource`, and this
                    // pre-check cannot see inside one: it walks the desktop asset roots looking
                    // for `<root>/<rel>`, and `game://sprites/x.png` is not a relative file
                    // path — no candidate can ever exist.
                    //
                    // The source owns its own existence check, exactly as the
                    // Android/iOS arms trust the packager. Attempt the load and
                    // let the reader answer.
                    || is_source_qualified(path)
                    || self.resolve_local_file_path(path).is_some()
            }
            AssetProfile::AndroidBundle | AssetProfile::IosBundle => true,
            AssetProfile::WebStatic | AssetProfile::BundledStatic => {
                resolved.authored_candidate
                    && matches!(
                        resolved.source_used,
                        Some(AssetSourceProfile::EmbeddedBinary)
                    )
            }
            // WebServedAssets attempts every resolution that produces
            // a Bevy-pathable URL: either an authored `Embedded`
            // candidate (delivered from `EmbeddedAssetRegistry`) or
            // the synthesized `BevyPath` from `logical_path` (which
            // Bevy's wasm HTTP reader fetches from `/assets/<path>`).
            // Missing files surface as Bevy load-failure logs + the
            // renderer's existing placeholder fallbacks; we cannot
            // pre-check the host filesystem from the browser, so the
            // "trust Bevy to fetch" stance matches Android/iOS.
            AssetProfile::WebServedAssets => {
                matches!(
                    resolved.source_used,
                    Some(AssetSourceProfile::EmbeddedBinary)
                        | Some(AssetSourceProfile::InstalledFilesystem)
                )
            }
            AssetProfile::WebHttp => {
                resolved.authored_candidate
                    && matches!(resolved.source_used, Some(AssetSourceProfile::HttpRemote))
            }
            AssetProfile::IpfsGatewayPlaceholder => {
                resolved.authored_candidate
                    && matches!(resolved.source_used, Some(AssetSourceProfile::IpfsGateway))
            }
            AssetProfile::NoAssets | AssetProfile::Headless => false,
        }
    }

    /// Same gate, but for required assets. Required entries with
    /// no host-filesystem precheck always attempt the load; the
    /// resolver's `Disabled` path is what consults
    /// [`MissingAssetPolicy::Error`].
    pub fn should_attempt_required_load(&self, _path: &str) -> bool {
        !matches!(
            self.profile,
            AssetProfile::NoAssets | AssetProfile::Headless
        )
    }

    /// Locate the absolute on-disk path for a Bevy-relative asset path
    /// under the current profile, when one is available. Returns
    /// `None` for non-desktop profiles or when the file simply isn't
    /// there. Walks the same candidate roots Bevy's file `AssetReader`
    /// consults at runtime, in this order:
    ///
    /// 1. `$BEVY_ASSET_ROOT/assets/<rel>`
    /// 2. `$BEVY_ASSET_ROOT/<rel>`
    /// 3. `$CWD/assets/<rel>`
    /// 4. `$CWD/<rel>`
    /// 5. `$CARGO_MANIFEST_DIR/../ambition_platformer2d_actor_monolith/assets/<rel>`
    ///    (current dev fallback while runtime assets still live there)
    ///
    /// This is the only host-filesystem probe in the sandbox. The
    /// LDtk hot-reload watcher and the SFX bank byte loader both call
    /// through here — there is no duplicate candidate walk anywhere
    /// else in `crates/ambition_platformer2d_actor_monolith/src/`.
    pub fn resolve_local_file_path(&self, rel: &str) -> Option<std::path::PathBuf> {
        if !matches!(
            self.profile,
            AssetProfile::DesktopDevLoose
                | AssetProfile::DesktopInstalled
                | AssetProfile::SteamDeckInstalled
        ) {
            return None;
        }
        desktop_candidate_roots(rel)
            .into_iter()
            .find(|p| p.exists())
    }
}

pub fn scaled_asset_id(id: &AssetId, scale_asset_id_suffix: Option<&str>) -> Option<AssetId> {
    scale_asset_id_suffix.map(|suffix| AssetId::new(format!("{}.{}", id.as_str(), suffix)))
}

/// Build the ordered candidate roots for `rel_path` on desktop / Steam
/// Deck profiles. The only candidate-roots walker in the sandbox;
/// [`Platformer2dAssetCatalog::resolve_local_file_path`] (and through it
/// `try_path_for_load`) are the sole callers.
fn desktop_candidate_roots(rel_path: &str) -> Vec<std::path::PathBuf> {
    let rel = std::path::Path::new(rel_path);
    let mut candidates = Vec::with_capacity(5);
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
            .join("../ambition_platformer2d_actor_monolith/assets")
            .join(rel),
    );
    candidates
}

/// The AssetServer FILE-SOURCE root a windowed app must set as
/// `AssetPlugin.file_path` on a loose desktop dev checkout: the absolute
/// `crates/ambition_platformer2d_actor_monolith/assets` directory, where the generated sprite sheets,
/// music, dialogue, and menu icons live.
///
/// Bevy's default file root is the cwd-relative `"assets"`, which in this
/// workspace has no `sprites/` tree — so an app that does not override it renders
/// every character as a bare box while the load silently no-ops (the profile gate
/// resolves the file through [`desktop_candidate_roots`], but the default reader
/// cannot). This is the ONE value that fixes that, and it is shared by the hosted
/// app AND every standalone demo app precisely so the two cannot diverge — a demo
/// that draws nothing standalone was exactly that divergence.
///
/// Resolution mirrors the candidate walker: an explicit `BEVY_ASSET_ROOT` wins
/// (return the relative `"assets"` so the override keeps full control); else the
/// dev-checkout absolute path when it exists; else the exe-relative `"assets"`
/// default for shipped builds.
pub fn actors_desktop_asset_root() -> String {
    if std::env::var_os("BEVY_ASSET_ROOT").is_some() {
        return "assets".to_string();
    }
    let dev_assets =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ambition_platformer2d_actor_monolith/assets");
    match dev_assets.canonicalize() {
        Ok(path) if path.is_dir() => path.to_string_lossy().into_owned(),
        _ => "assets".to_string(),
    }
}

#[cfg(test)]
mod asset_root_tests {
    use super::actors_desktop_asset_root;

    /// The resolved root must be the directory that actually holds the generated
    /// sprite sheets — the entire reason a windowed app sets it as the AssetServer
    /// `file_path`, and the fix for "a demo renders every character as a bare box
    /// standalone". Asserting the `sprites/` tree (not a specific file) keeps this
    /// robust to sprite renames.
    #[test]
    fn resolved_root_contains_the_generated_sprite_tree() {
        // An explicit override or a shipped build (no dev tree) both fall back to
        // the relative `"assets"`; the disk assertion only applies to the dev
        // checkout this test runs in.
        if std::env::var_os("BEVY_ASSET_ROOT").is_some() {
            return;
        }
        let root = actors_desktop_asset_root();
        if root == "assets" {
            return;
        }
        let sprites = std::path::Path::new(&root).join("sprites");
        assert!(
            sprites.is_dir(),
            "resolved actors asset root {root} must contain a sprites/ tree so \
             AssetServer::load(\"sprites/…png\") resolves"
        );
    }
}

/// Build the full sandbox catalog: every visible-sandbox asset id +
/// the active profile. Called once during `init_sandbox_resources`.
///
/// `inputs.music_tracks` carries the already-loaded music registry rows so
/// music-track ids land in the catalog at startup; the catalog doesn't depend
/// on disk-resident files for bootstrap.
pub fn build_platformer2d_asset_catalog(
    config: &Platformer2dAssetCatalogConfig,
    image_manifest: AssetManifest,
    inputs: &Platformer2dAssetCatalogInputs,
) -> Platformer2dAssetCatalog {
    build_sandbox_catalog_with(config, image_manifest, inputs, |_| {})
}

/// [`build_platformer2d_asset_catalog`] with a content-extension hook: the app
/// assembly passes the content layer's extra manifest entries (e.g.
/// the intro sprite rows) so this machinery module names no content.
pub fn build_sandbox_catalog_with(
    config: &Platformer2dAssetCatalogConfig,
    image_manifest: AssetManifest,
    inputs: &Platformer2dAssetCatalogInputs,
    extend: impl FnOnce(&mut AssetManifest),
) -> Platformer2dAssetCatalog {
    let mut manifest = image_manifest;
    extend_with_world_entries(&mut manifest, &inputs.worlds);
    extend_with_data_entries(&mut manifest);
    extend_with_sfx_bank_entry(&mut manifest);
    extend_with_font_entries(&mut manifest);
    extend_with_sprite_pack_entries(&mut manifest);
    extend_with_character_entries(
        &mut manifest,
        &config.sprite_folder,
        &inputs.character_sprites,
        &inputs.scale_variants,
    );
    extend_with_boss_entries(
        &mut manifest,
        &config.sprite_folder,
        &inputs.boss_sprites,
        &inputs.scale_variants,
    );
    extend_with_music_entries(&mut manifest, &inputs.music_tracks);
    extend(&mut manifest);
    Platformer2dAssetCatalog::new(AmbitionAssetCatalog::new(manifest), config.asset_profile)
}

#[cfg(test)]
mod authored_path_tests {
    use super::*;

    /// The rule, stated once. A path that names its own source belongs to
    /// whoever authored it; everything else follows the folder convention.
    #[test]
    fn a_source_qualified_path_is_never_rebuilt_under_a_folder() {
        assert_eq!(
            logical_asset_path("sprites", "game://sprites/mine.png"),
            "game://sprites/mine.png",
            "the folder convention was applied to a path that already named its \
             own source — this produced `sprites/game://sprites/mine.png`, a path \
             to nothing, in three separate seams before it had a name"
        );
        assert_eq!(
            logical_asset_path("sprites", "engine_art.png"),
            "sprites/engine_art.png",
            "an ordinary filename still follows the convention, or every engine \
             asset breaks"
        );
    }

    /// A consumer's asset gets NO invented scale siblings: `sprites_0_5x/…` is
    /// this repo's generated layout, not a convention somebody else's asset
    /// source agreed to.
    #[test]
    fn a_source_qualified_path_has_no_scaled_siblings() {
        assert_eq!(
            scaled_logical_asset_path("sprites", "0_5x", "game://sprites/mine.png"),
            None
        );
        assert_eq!(
            scaled_logical_asset_path("sprites", "0_5x", "engine_art.png").as_deref(),
            Some("sprites_0_5x/engine_art.png")
        );
    }

    /// The guard against a fourth layer. Every FAMILY that joins a folder to
    /// an authored filename is exercised here with a source-qualified path, and
    /// the manifest must carry it verbatim.
    ///
    /// A family added later that re-implements the join fails this test rather than waiting for
    /// somebody to render it.
    #[test]
    fn every_manifest_family_carries_a_consumers_own_path_verbatim() {
        const OWN: &str = "game://sprites/consumer_owned.png";
        let scale_variants = [AssetScaleVariant {
            asset_id_suffix: "0_5x",
            sprite_subdir_suffix: "0_5x",
            parallax_subdir: "parallax_0_5x",
        }];

        let mut manifest = AssetManifest::default();
        builders::extend_with_character_entries(
            &mut manifest,
            "sprites",
            &[CharacterSpriteCatalogRow {
                name: "consumer_hero".into(),
                filename: OWN.into(),
                qualified: Some(OWN.into()),
            }],
            &scale_variants,
        );
        builders::extend_with_boss_entries(
            &mut manifest,
            "sprites",
            &[BossSpriteCatalogRow {
                name: "consumer_boss".into(),
                filename: OWN.into(),
            }],
            &scale_variants,
        );

        // Audio joins no folder — a music row carries `asset_path` verbatim — so
        // it is covered by INSPECTION rather than by the helper. Asserting it
        // anyway is the difference between "we believe it passes through" and a
        // test that fails the day somebody adds a folder convention here, which
        // is precisely how the three sprite layers were introduced.
        builders::extend_with_music_entries(
            &mut manifest,
            &[MusicCatalogRow {
                id: "consumer_theme".into(),
                asset_path: OWN.into(),
            }],
        );

        for (family, id) in [
            ("character", ids::character_sprite("consumer_hero")),
            ("boss", ids::boss_sprite("consumer_boss")),
            ("music", ids::music_track("consumer_theme")),
        ] {
            let entry = manifest
                .get(&id)
                .unwrap_or_else(|| panic!("{family} entry is in the manifest"));
            assert_eq!(
                entry.logical_path, OWN,
                "the {family} manifest row mangled a consumer's own path — it \
                 reads `{}`, and the asset it names does not exist",
                entry.logical_path
            );
            // ...and no invented sibling under this repo's generated layout.
            // Audio has no scale variants at all, so this is vacuous there and
            // load-bearing for the two image families.
            assert!(
                manifest
                    .get(&scaled_asset_id(&id, Some("0_5x")).expect("a variant id"))
                    .is_none(),
                "a scaled sibling was invented inside somebody else's asset source"
            );
        }
    }
}
