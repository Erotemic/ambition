//! Spritesheet asset bundle + on-disk loading.
//!
//! Each character is identified by a stable `character_id` keyed in
//! `assets/data/character_catalog.ron` (loaded by
//! [`ambition_characters::actor::character_catalog`]). The catalog provides the
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

use bevy::prelude::*;

use ambition_asset_manager::AssetId;

use crate::assets::sandbox_assets::{ids, SandboxAssetCatalog};
use crate::character_roster::catalog;
use ambition_engine_core as ae;
use ambition_persistence::settings::VisualQualityBudget;
use ambition_sprite_sheet::character::{
    CharacterSheetSpec, CharacterSpriteAsset, CharacterSpritePage,
    TextureResolutionScale as SpriteTextureResolutionScale,
};
use ambition_sprite_sheet::BodyMetrics;

pub use ambition_sprite_sheet::character::CharacterSpriteAssets;

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
    if let Some(entry) = catalog().characters.get(character_id) {
        if let Some(target) = entry.manifest_target() {
            let tuning = entry
                .sprite_tuning
                .map(|spec| {
                    super::sheets::SheetTuning::from_parts(
                        spec.collision_scale,
                        spec.frame_sample_inset,
                        spec.feet_anchor_y,
                    )
                })
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

/// The manifest target + resolution-independent tuning for a catalog `cid`,
/// when it has a catalog row that names a sheet. This is what
/// [`build_optional_via_catalog`] needs to fetch the **scaled-variant** record
/// keyed `<target>.<suffix>`. `None` for ids resolved through the manifest-by-id
/// fallback (they stay at base resolution — acceptable, they render fine).
fn character_variant_tuning(cid: &str) -> Option<(&'static str, super::sheets::SheetTuning)> {
    let entry = catalog().characters.get(cid)?;
    let target = entry.manifest_target()?;
    let tuning = entry
        .sprite_tuning
        .map(|spec| {
            super::sheets::SheetTuning::from_parts(
                spec.collision_scale,
                spec.frame_sample_inset,
                spec.feet_anchor_y,
            )
        })
        .unwrap_or_default();
    Some((target, tuning))
}

/// Collision footprint derived from a character's *published sprite body
/// metrics*, plus the render-quad size that keeps the on-screen sprite
/// identical to the legacy `collision_scale` render.
///
/// `collision` is the world-space box around the **visible body** (the
/// `body_pixel_bbox` / `body_pixel_parts` the generator measured from the
/// rendered art), so an actor's hitbox matches what the player sees instead
/// of an authored LDtk rectangle.
///
/// `render_size` is exactly what `sprite_render_size(spec, ldtk_collision)`
/// produces today — the caller stores it so the renderer draws the sprite at
/// its current size even though the collision box shrank to the body. (The
/// renderer's `collision_scale` path assumes `collision == visible body`;
/// once the collision IS the body, the render must come from the stored size
/// rather than re-deriving `body * collision_scale`, which double-scales.)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteBodyCollision {
    pub collision: ae::Vec2,
    pub render_size: ae::Vec2,
}

/// Pixel-space extent of the visible body in the sheet's idle/rest frame.
/// Prefers the multi-part bounding box (disjoint-piece characters) and falls
/// back to the single `body_pixel_bbox`. `None` when neither is published or
/// the box is degenerate.
fn body_pixel_extent(metrics: &BodyMetrics) -> Option<(f32, f32)> {
    if !metrics.body_pixel_parts.is_empty() {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for part in &metrics.body_pixel_parts {
            min_x = min_x.min(part.x as f32);
            min_y = min_y.min(part.y as f32);
            max_x = max_x.max((part.x + part.w) as f32);
            max_y = max_y.max((part.y + part.h) as f32);
        }
        let (w, h) = (max_x - min_x, max_y - min_y);
        return (w > 0.0 && h > 0.0).then_some((w, h));
    }
    let bbox = metrics.body_pixel_bbox?;
    (bbox.w > 0 && bbox.h > 0).then_some((bbox.w as f32, bbox.h as f32))
}

/// Derive a character's collision box from its published sprite body metrics,
/// given the authored LDtk collision (used only to anchor the render scale).
///
/// Returns `None` when the character has no catalog row, no loadable spec, or
/// no published `body_metrics` — the caller then keeps the LDtk bounds. This
/// is the "sprite metadata supersedes the spawn box when present, else fall
/// back to LDtk" rule (matching the boss `body_metrics` pipeline, generalized
/// to ordinary catalog characters).
pub fn sprite_body_collision_for_character_id(
    character_id: &str,
    ldtk_collision: ae::Vec2,
) -> Option<SpriteBodyCollision> {
    let entry = catalog().characters.get(character_id)?;
    let target = entry.manifest_target()?;
    let spec = sheet_for_character_id(character_id)?;
    let record = super::sheets::record_for_target(target)?;
    let metrics = record.body_metrics.as_ref()?;
    let (body_w, body_h) = body_pixel_extent(metrics)?;
    let frame_w = record.frame_width.max(1) as f32;
    let frame_h = record.frame_height.max(1) as f32;
    // The size the renderer draws today: full frame scaled to the LDtk box.
    let render = super::sheets::sprite_render_size(
        &spec,
        bevy::math::Vec2::new(ldtk_collision.x, ldtk_collision.y),
    );
    // The visible body occupies (body / frame) of that render quad.
    let collision = ae::Vec2::new(body_w / frame_w * render.x, body_h / frame_h * render.y);
    Some(SpriteBodyCollision {
        collision,
        render_size: ae::Vec2::new(render.x, render.y),
    })
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
    let mut out: Vec<(String, String)> = Vec::with_capacity(catalog().characters.len());
    for (cid, entry) in catalog().characters.iter() {
        let filename = entry
            .spritesheet
            .strip_prefix("sprites/")
            .unwrap_or(entry.spritesheet.as_str())
            .to_string();
        out.push((cid.clone(), filename));
    }
    out
}

fn sprite_texture_scale(
    scale: crate::persistence::settings::TextureResolutionScale,
) -> SpriteTextureResolutionScale {
    match scale {
        crate::persistence::settings::TextureResolutionScale::Potato => {
            SpriteTextureResolutionScale::Potato
        }
        crate::persistence::settings::TextureResolutionScale::Quarter => {
            SpriteTextureResolutionScale::Quarter
        }
        crate::persistence::settings::TextureResolutionScale::Half => {
            SpriteTextureResolutionScale::Half
        }
        crate::persistence::settings::TextureResolutionScale::Full => {
            SpriteTextureResolutionScale::Full
        }
    }
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
    quality: Option<&VisualQualityBudget>,
) -> CharacterSpriteAssets {
    let mut out = CharacterSpriteAssets::default();
    let mut total = 0usize;
    let mut loaded = 0usize;
    let mut skipped_no_spec: Vec<&str> = Vec::new();
    let mut skipped_no_path: Vec<&str> = Vec::new();
    // NB: `catalog` here is the ASSET catalog param; the roster comes from
    // the installed character catalog.
    for (cid, entry) in crate::character_roster::catalog().characters.iter() {
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
        let variant_tuning = character_variant_tuning(cid);
        let variant = variant_tuning.as_ref().map(|(t, tn)| (*t, tn));
        let Some(asset) = build_optional_via_catalog(
            catalog,
            asset_server,
            layouts,
            &asset_id,
            &sheet_spec,
            variant,
            Some(cid),
            quality,
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
                out.npcs.insert(cid.clone(), asset.clone());
                out.npcs.insert(entry.display_name.clone(), asset.clone());
                out.player = Some(asset);
            }
            "robot" => {
                out.npcs.insert(cid.clone(), asset.clone());
                out.npcs.insert(entry.display_name.clone(), asset.clone());
                out.robot = Some(asset);
            }
            "goblin" => {
                out.npcs.insert(cid.clone(), asset.clone());
                out.npcs.insert(entry.display_name.clone(), asset.clone());
                out.goblin = Some(asset);
            }
            "sandbag" => {
                out.npcs.insert(cid.clone(), asset.clone());
                out.npcs.insert(entry.display_name.clone(), asset.clone());
                out.sandbag = Some(asset);
            }
            _ => {
                out.npcs.insert(cid.clone(), asset.clone());
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
/// Choose the (spec, image id) pair under the quality budget. Upgrades to a
/// scaled variant **only when both** the variant record was baked *and* the
/// variant image resolves under the active asset profile — so the atlas rects
/// (from the spec) always address the PNG that actually loads. Returns the base
/// pair otherwise (and always for props / `variant: None`). Gameplay collision
/// is untouched; it reads the base record separately.
fn resolve_variant_pair(
    catalog: &SandboxAssetCatalog,
    base_id: &AssetId,
    base_spec: &CharacterSheetSpec,
    variant: Option<(&str, &super::sheets::SheetTuning)>,
    quality: Option<&VisualQualityBudget>,
) -> (CharacterSheetSpec, AssetId) {
    if let (Some((target, tuning)), Some(q)) = (variant, quality) {
        if q.sprites.prefer_scaled_variants {
            let scale = q.sprites.resolution_scale;
            if scale != crate::persistence::settings::TextureResolutionScale::Full {
                if let Some(variant_id) =
                    crate::assets::sandbox_assets::scaled_asset_id(base_id, scale)
                {
                    if catalog.try_path_for_load(&variant_id).is_some() {
                        if let Some(spec) = super::sheets::try_load_spec_for_target_scaled(
                            target,
                            tuning,
                            sprite_texture_scale(scale),
                        ) {
                            return (spec, variant_id);
                        }
                    }
                }
            }
        }
    }
    (base_spec.clone(), base_id.clone())
}

fn build_optional_via_catalog(
    catalog: &SandboxAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    base_id: &AssetId,
    base_spec: &CharacterSheetSpec,
    variant: Option<(&str, &super::sheets::SheetTuning)>,
    log_label: Option<&str>,
    quality: Option<&VisualQualityBudget>,
) -> Option<CharacterSpriteAsset> {
    // Pick base-or-variant atomically so the spec rects match the loaded PNG.
    let (spec, id) = resolve_variant_pair(catalog, base_id, base_spec, variant, quality);
    let (spec, id) = (&spec, &id);
    let Some(path) = catalog.try_path_for_load(id) else {
        if let Some(label) = log_label {
            eprintln!(
                "[character_sprites] {label} spritesheet missing under {} profile (id {id}) — falling back to colored rectangle",
                catalog.profile().label(),
            );
        }
        return None;
    };
    Some(load_sprite_pages(asset_server, layouts, &path, spec))
}

/// Build one `(texture, layout)` per page image and assemble the sprite
/// asset. `page0_path` is the already-resolved (profile-gated) Bevy asset
/// path of page 0; sibling pages resolve their filename from the spec's
/// `page_images` list against page 0's directory. Shared by the per-target
/// sheet path and the shared-pack path — the page algebra is identical.
fn load_sprite_pages(
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    page0_path: &str,
    spec: &CharacterSheetSpec,
) -> CharacterSpriteAsset {
    let parent = page0_path
        .rsplit_once('/')
        .map(|(dir, _)| dir)
        .unwrap_or("");
    let page_count = spec.page_count().max(1);
    let pages: Vec<CharacterSpritePage> = (0..page_count)
        .map(|page| {
            // Page 0 uses the resolved path verbatim; later pages resolve
            // their filename against page 0's directory.
            let page_path = if page == 0 {
                page0_path.to_owned()
            } else {
                let file = spec
                    .page_images
                    .get(page as usize)
                    .cloned()
                    .unwrap_or_else(|| format!("page_{page}.png"));
                if parent.is_empty() {
                    file
                } else {
                    format!("{parent}/{file}")
                }
            };
            CharacterSpritePage {
                texture: asset_server.load(page_path),
                layout: layouts.add(spec.build_atlas_for_page(page)),
            }
        })
        .collect();
    let texture = pages[0].texture.clone();
    let layout = pages[0].layout.clone();
    CharacterSpriteAsset {
        texture,
        layout,
        spec: spec.clone(),
        pages,
    }
}

/// Build a single NPC sprite asset by resolving its catalog id.
/// Story-content plugins (for example `ambition_content::intro::plugin`)
/// call this once per row in their authored NPC table; the matching
/// catalog entries come from the sandbox asset catalog builders or the
/// equivalent content-owned install helper for that story pack.
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
    build_optional_via_catalog(catalog, asset_server, layouts, id, spec, None, None, None)
}

/// Build a single Prop sprite asset. Same shape as
/// [`build_npc_sprite_asset`] — kept as a separate name so story-
/// content plugins reading from `INTRO_PROP_REGISTRY` (or future
/// equivalents) clearly distinguish prop-table inserts from NPC-table
/// inserts.
/// Build a prop's sprite asset from the quality-tiered **shared sprite pack**
/// (`assets/sprite_packs/<tier>/`) instead of its per-target sheet.
///
/// The pack tier follows the active quality budget (mirroring
/// `resolve_variant_pair` semantics: budgets that don't prefer scaled
/// variants stay on `full`), and the spec + page images come from the SAME
/// tier catalog, so rects always address the pages that load. Tuning +
/// feet anchor are lifted verbatim from `base_spec`, keeping the packed
/// prop pixel-placement-identical to the per-target path. Returns `None`
/// when no pack was generated (fresh checkout), the target isn't packed,
/// or the pack pages are gated by the asset profile — the caller falls
/// back to [`build_prop_sprite_asset`].
pub fn build_prop_sprite_asset_packed(
    catalog: &SandboxAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    target: &str,
    base_spec: &CharacterSheetSpec,
    quality: Option<&VisualQualityBudget>,
) -> Option<CharacterSpriteAsset> {
    let scale = quality
        .filter(|q| q.sprites.prefer_scaled_variants)
        .map(|q| q.sprites.resolution_scale)
        .unwrap_or(crate::persistence::settings::TextureResolutionScale::Full);
    let tuning = base_spec.tuning();
    let (spec, tier) =
        super::sheets::try_load_pack_spec_for_target(target, &tuning, sprite_texture_scale(scale))?;
    // Profile-gate page 0 through the sandbox catalog like every other
    // sprite; sibling pages resolve from the spec's page_images against
    // page 0's directory (the pack pages all share the tier dir).
    let id = crate::assets::sandbox_assets::ids::sprite_pack_page0(tier);
    let path = catalog.try_path_for_load(&id)?;
    Some(load_sprite_pages(asset_server, layouts, &path, &spec))
}

pub fn build_prop_sprite_asset(
    catalog: &SandboxAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    id: &AssetId,
    spec: &CharacterSheetSpec,
) -> Option<CharacterSpriteAsset> {
    build_optional_via_catalog(catalog, asset_server, layouts, id, spec, None, None, None)
}

#[cfg(test)]
mod sprite_body_collision_tests {
    use super::*;
    use crate::character_sprites::registry::{BodyMetrics, NamedPixelRect, PixelRect};

    fn metrics_with_bbox(bbox: Option<PixelRect>, parts: Vec<NamedPixelRect>) -> BodyMetrics {
        BodyMetrics {
            body_pixel_bbox: bbox,
            body_pixel_parts: parts,
            animations: Default::default(),
            feet_pixel: None,
            feet_anchor_norm: None,
        }
    }

    #[test]
    fn body_extent_prefers_single_bbox_when_no_parts() {
        let m = metrics_with_bbox(
            Some(PixelRect {
                x: 8,
                y: 5,
                w: 106,
                h: 83,
            }),
            vec![],
        );
        assert_eq!(body_pixel_extent(&m), Some((106.0, 83.0)));
    }

    #[test]
    fn body_extent_bounds_disjoint_parts() {
        // Two parts at x∈[0,32] and x∈[96,128], y∈[40,90] → bbox 128 × 50.
        let m = metrics_with_bbox(
            // bbox present but ignored: parts win for disjoint bodies.
            Some(PixelRect {
                x: 0,
                y: 0,
                w: 1,
                h: 1,
            }),
            vec![
                NamedPixelRect {
                    name: "left".into(),
                    x: 0,
                    y: 40,
                    w: 32,
                    h: 50,
                },
                NamedPixelRect {
                    name: "right".into(),
                    x: 96,
                    y: 40,
                    w: 32,
                    h: 50,
                },
            ],
        );
        assert_eq!(body_pixel_extent(&m), Some((128.0, 50.0)));
    }

    #[test]
    fn body_extent_rejects_degenerate_box() {
        let m = metrics_with_bbox(
            Some(PixelRect {
                x: 0,
                y: 0,
                w: 0,
                h: 10,
            }),
            vec![],
        );
        assert_eq!(body_pixel_extent(&m), None);
    }

    /// Contract on the real catalog→sheet pipeline: when a character has
    /// published body metrics, (1) the render quad equals exactly what the
    /// legacy `collision_scale` path produces (sprite unchanged), and (2) the
    /// derived collision is the visible body = (body / frame) × render. Skips
    /// when no baked sheet exposes metrics (sprites are gitignored / not yet
    /// regenerated on a clean checkout).
    #[test]
    fn derived_collision_is_the_visible_body_and_preserves_the_render() {
        let ldtk = ae::Vec2::new(40.0, 60.0);
        let Some((cid, derived)) = catalog()
            .characters
            .keys()
            .find_map(|cid| sprite_body_collision_for_character_id(cid, ldtk).map(|d| (cid, d)))
        else {
            return; // no baked sheet with metrics available
        };
        let entry = catalog().characters.get(cid).unwrap();
        let target = entry.manifest_target().unwrap();
        let spec = sheet_for_character_id(cid).unwrap();
        let record = super::super::sheets::record_for_target(target).unwrap();
        let metrics = record.body_metrics.as_ref().unwrap();
        let (body_w, body_h) = body_pixel_extent(metrics).unwrap();
        let frame_w = record.frame_width.max(1) as f32;
        let frame_h = record.frame_height.max(1) as f32;

        // (1) render == legacy sprite_render_size(spec, ldtk).
        let legacy =
            super::super::sheets::sprite_render_size(&spec, bevy::math::Vec2::new(ldtk.x, ldtk.y));
        assert!((derived.render_size.x - legacy.x).abs() < 1e-3);
        assert!((derived.render_size.y - legacy.y).abs() < 1e-3);

        // (2) collision == (body / frame) × render.
        let expect_x = body_w / frame_w * derived.render_size.x;
        let expect_y = body_h / frame_h * derived.render_size.y;
        assert!((derived.collision.x - expect_x).abs() < 1e-3);
        assert!((derived.collision.y - expect_y).abs() < 1e-3);
        assert!(derived.collision.x > 0.0 && derived.collision.y > 0.0);
    }
}
