//! Spritesheet asset bundle + on-disk loading.
//!
//! Each character is identified by a stable `character_id` keyed in
//! `assets/data/character_catalog.ron` (loaded by
//! [`crate::actor::character_catalog`]). The catalog provides the
//! display name + on-disk path; the per-character `CharacterSheetSpec`
//! (frame/grid/anchor metadata) is resolved at startup by
//! [`sheet_for_character_id`] — a single table that maps
//! catalog ids to the hardcoded `*_SHEET` consts in `sheets.rs`.
//!
//! Missing files are not errors — callers fall back to colored
//! rectangles (the game must always run regardless of asset state).
//! All path/existence policy goes through
//! [`crate::assets::sandbox_assets::SandboxAssetCatalog`]; this module
//! no longer owns any `target_os = "android"` cfg branches or
//! `BEVY_ASSET_ROOT` probes.
//!
//! ## Phase 6 cleanup (2026-05-24)
//!
//! Before Phase 6 this module duplicated character metadata in a
//! `NPC_SPRITE_REGISTRY` table (display name + filename + sheet
//! const) and a parallel `npc_sprite_label` display-name → catalog-
//! id mapper. Both are gone now: the catalog is the single source
//! of `display_name` and on-disk path, while `sheet_for_character_id`
//! is the only place that pairs a catalog id with its sheet const.

use std::collections::HashMap;
use std::sync::LazyLock;

use bevy::prelude::*;

use ambition_asset_manager::AssetId;

use super::sheets::CharacterSheetSpec;
use crate::actor::character_catalog::EMBEDDED_CATALOG;
use crate::assets::sandbox_assets::{ids, SandboxAssetCatalog};
use crate::features::FeatureVisualKind;

#[derive(Clone)]
pub struct CharacterSpriteAsset {
    pub texture: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub spec: CharacterSheetSpec,
}

/// Holds optional spritesheet handles. A missing PNG produces a
/// `None` (or absent map entry); callers fall back to colored
/// rectangles.
#[derive(Resource, Default, Clone)]
pub struct CharacterSpriteAssets {
    /// Player-specific compact robot sheet. Preferred for the player
    /// entity; `setup.rs` falls back to `robot` when this is missing
    /// so debug builds without the regenerated sheet still render.
    pub player: Option<CharacterSpriteAsset>,
    /// Base "cute scout" robot sheet. Kept around for future robot-target
    /// callers that want the original proportions; the player itself now
    /// uses `player` above.
    pub robot: Option<CharacterSpriteAsset>,
    pub goblin: Option<CharacterSpriteAsset>,
    pub sandbag: Option<CharacterSpriteAsset>,
    /// Per-NPC sprite sheets keyed by the NPC's display name (which
    /// is `Authored.name` post-Phase-2 — the LDtk parser translates
    /// `NpcSpawn.character_id` to `display_name` via the catalog,
    /// then downstream consumers look up sprites by display name).
    /// Phase-7+ work can flip this to `character_id` keys to drop
    /// the display-name indirection.
    pub npcs: HashMap<String, CharacterSpriteAsset>,
    /// Per-prop sprite sheets keyed by the LDtk `Prop.kind` field
    /// (e.g. `intro_cart`, `lab_genesis_vat`, `gate_ring`,
    /// `gate_portal`). Story-content plugins extend this via
    /// `build_prop_sprite_asset` — the sandbox itself doesn't ship
    /// any props in its base registry.
    pub props: HashMap<String, CharacterSpriteAsset>,
}

impl CharacterSpriteAssets {
    pub fn enemy_asset(&self, kind: FeatureVisualKind) -> Option<&CharacterSpriteAsset> {
        match kind {
            FeatureVisualKind::Enemy => self.goblin.as_ref(),
            FeatureVisualKind::Sandbag => self.sandbag.as_ref().or(self.goblin.as_ref()),
            _ => None,
        }
    }

    /// Pick a character spritesheet for an NPC by its authored name.
    /// Returns `None` for NPCs that have no registered sprite —
    /// those keep the default `EntitySprite::NpcTerminal` rectangle.
    pub fn npc_asset_for_name(&self, name: &str) -> Option<&CharacterSpriteAsset> {
        self.npcs.get(name)
    }

    /// Pick a prop spritesheet by its `Prop.kind` registry key.
    /// Returns `None` for kinds that have no registered sheet — the
    /// prop renderer falls back to a colored placeholder rectangle.
    pub fn prop_asset_for_kind(&self, kind: &str) -> Option<&CharacterSpriteAsset> {
        self.props.get(kind)
    }
}

/// Look up the [`CharacterSheetSpec`] for a catalog `character_id` —
/// fully DATA-driven (Stage 20 / B3):
///
/// 1. The catalog row names the sheet-manifest record (its own
///    `manifest` filename root, or an explicit `sprite_target` when a
///    character renders with another character's sheet) and carries
///    the gameplay tuning (`sprite_tuning`: collision_scale /
///    frame_sample_inset / feet-anchor override).
/// 2. Ids without a catalog row fall back to the manifest-by-id load
///    with default tuning (`super::sheets::try_load_spec_for_character_id`).
///
/// The old hardcoded `*_SHEET` statics + named match are gone — adding
/// a character's bespoke tuning is a `character_catalog.ron` edit.
///
/// Returns `None` only when no manifest exists for the id — usually
/// because the renderer hasn't been run for that target; the actor
/// then renders the colored-rectangle placeholder.
pub fn sheet_for_character_id(character_id: &str) -> Option<CharacterSheetSpec> {
    if let Some(entry) = EMBEDDED_CATALOG.characters.get(character_id) {
        if let Some(target) = entry.manifest_target() {
            let tuning = entry
                .sprite_tuning
                .map(super::sheets::SheetTuning::from_spec)
                .unwrap_or_default();
            if let Some(spec) = super::sheets::try_load_spec_for_target(target, &tuning) {
                return Some(spec);
            }
        }
    }
    let spec = super::sheets::try_load_spec_for_character_id(character_id);
    if spec.is_none() {
        bevy::log::debug!(
            target: "ambition::character_sprites",
            "character_sprites: no sheet manifest for catalog id '{character_id}' — \
             actor will render the colored-rectangle placeholder",
        );
    }
    spec
}

/// Return every `(character_id, on-disk filename)` pair the catalog
/// declares, for asset-manifest registration. Used by the sandbox-
/// assets aggregator (`builders/visuals.rs::extend_with_character_entries`)
/// so adding a row to the catalog auto-registers the catalog id.
///
/// Filename is the basename of the catalog entry's `spritesheet`
/// field (stripped of the `sprites/` prefix the catalog stores them
/// under).
pub fn all_character_sprite_filenames() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::with_capacity(EMBEDDED_CATALOG.characters.len());
    for (cid, entry) in EMBEDDED_CATALOG.characters.iter() {
        let filename = entry
            .spritesheet
            .strip_prefix("sprites/")
            .unwrap_or(entry.spritesheet.as_str())
            .to_string();
        out.push((cid.clone(), filename));
    }
    out
}

/// Probe the sandbox `assets/<sprite_folder>/` directory for spritesheets.
///
/// Iterates the embedded character catalog and, for each entry, looks
/// up its [`CharacterSheetSpec`] via [`sheet_for_character_id`]. Asset
/// availability gates through
/// [`SandboxAssetCatalog::should_attempt_optional_load`]; missing
/// files produce no map entry (callers fall back to colored
/// rectangles).
pub fn load_character_sprites_in(
    catalog: &SandboxAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
) -> CharacterSpriteAssets {
    let mut out = CharacterSpriteAssets::default();
    let mut total = 0usize;
    let mut loaded = 0usize;
    let mut skipped_no_spec: Vec<&str> = Vec::new();
    let mut skipped_no_path: Vec<&str> = Vec::new();
    for (cid, entry) in EMBEDDED_CATALOG.characters.iter() {
        total += 1;
        let Some(sheet_spec) = sheet_for_character_id(cid) else {
            // Neither a hardcoded const nor a manifest in
            // `assets/sprites/` exists for this id — skip silently.
            // The character falls back to the colored-rectangle
            // visual until its sprite is published.
            skipped_no_spec.push(cid.as_str());
            continue;
        };
        let asset_id = ids::character_sprite(cid);
        let Some(asset) = build_optional_via_catalog(
            catalog,
            asset_server,
            layouts,
            &asset_id,
            &sheet_spec,
            Some(cid),
        ) else {
            skipped_no_path.push(cid.as_str());
            continue;
        };
        loaded += 1;
        match cid.as_str() {
            "player" => {
                // Store under the typed field for the runtime's
                // fast-path consumers (`runtime/setup.rs`,
                // `enemy_asset`). ALSO key the npcs HashMap by the
                // display name so a hall pedestal with
                // character_id="player" — whose Authored.name is the
                // display "Player" — resolves through
                // `npc_asset_for_name`. This double-keying applies
                // to every base character that ships its own typed
                // slot.
                out.npcs.insert(entry.display_name.clone(), asset.clone());
                out.player = Some(asset);
            }
            "robot" => {
                out.npcs.insert(entry.display_name.clone(), asset.clone());
                out.robot = Some(asset);
            }
            "goblin" => {
                out.npcs.insert(entry.display_name.clone(), asset.clone());
                out.goblin = Some(asset);
            }
            "sandbag" => {
                out.npcs.insert(entry.display_name.clone(), asset.clone());
                out.sandbag = Some(asset);
            }
            _ => {
                out.npcs.insert(entry.display_name.clone(), asset);
            }
        }
    }
    // Single-line startup census so a developer running the game
    // can confirm at a glance whether the catalog→sprite chain is
    // working. Bumped up to INFO so it appears under the default
    // log filter without needing `RUST_LOG=debug`.
    bevy::log::info!(
        target: "ambition::character_sprites",
        "character_sprites: {loaded}/{total} catalog entries loaded; \
         {} no spec wired (placeholder), {} no asset path (placeholder)",
        skipped_no_spec.len(),
        skipped_no_path.len(),
    );
    if !skipped_no_spec.is_empty() {
        bevy::log::debug!(
            target: "ambition::character_sprites",
            "character_sprites: no_spec ids: {skipped_no_spec:?}",
        );
    }
    if !skipped_no_path.is_empty() {
        bevy::log::debug!(
            target: "ambition::character_sprites",
            "character_sprites: no_path ids: {skipped_no_path:?}",
        );
    }
    out
}

/// Resolve the catalog id, gate on profile policy via
/// `try_path_for_load`, and call `asset_server.load(...)` if the gate
/// passes. Logs a single line to `stderr` when a labeled sprite is
/// missing (matches the prior loader's noise level).
fn build_optional_via_catalog(
    catalog: &SandboxAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    id: &AssetId,
    spec: &CharacterSheetSpec,
    log_label: Option<&str>,
) -> Option<CharacterSpriteAsset> {
    let Some(path) = catalog.try_path_for_load(id) else {
        if let Some(label) = log_label {
            eprintln!(
                "[character_sprites] {label} spritesheet missing under {} profile (id {id}) — falling back to colored rectangle",
                catalog.profile().label(),
            );
        }
        return None;
    };
    let layout = layouts.add(spec.build_atlas());
    Some(CharacterSpriteAsset {
        texture: asset_server.load(path),
        layout,
        spec: spec.clone(),
    })
}

/// Build a single NPC sprite asset by resolving its catalog id.
/// Story-content plugins (e.g. `crate::ambition_content::intro::plugin`) call this once
/// per row in their authored NPC table; the matching catalog entries
/// are registered by `crate::assets::sandbox_assets::extend_with_intro_sprite_entries`
/// (or the equivalent helper for new plugins).
///
/// Returns `None` when the catalog reports the asset disabled / not
/// loadable under the active profile — callers fall back to colored
/// rectangles.
pub fn build_npc_sprite_asset(
    catalog: &SandboxAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    id: &AssetId,
    spec: &CharacterSheetSpec,
) -> Option<CharacterSpriteAsset> {
    build_optional_via_catalog(catalog, asset_server, layouts, id, spec, None)
}

/// Build a single Prop sprite asset. Same shape as
/// [`build_npc_sprite_asset`] — kept as a separate name so story-
/// content plugins reading from `INTRO_PROP_REGISTRY` (or future
/// equivalents) clearly distinguish prop-table inserts from NPC-table
/// inserts.
pub fn build_prop_sprite_asset(
    catalog: &SandboxAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    id: &AssetId,
    spec: &CharacterSheetSpec,
) -> Option<CharacterSpriteAsset> {
    build_optional_via_catalog(catalog, asset_server, layouts, id, spec, None)
}
