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
//! [`crate::assets::platformer_assets::Platformer2dAssetCatalog`]; this module
//! no longer owns any `target_os = "android"` cfg branches or
//! `BEVY_ASSET_ROOT` probes.
//!
//! ## Phase 6 cleanup (2026-05-24)
//!
//! Before Phase 6 this module duplicated character metadata in a
//! `NPC_SPRITE_REGISTRY` table (display name + filename + sheet
//! const) and a parallel `npc_sprite_label` display-name → catalog-
//! id mapper. Both are gone now: the catalog is the single source
//! of `display_name` and on-disk path, while
//! `sheet_for_character_id_in` is the only place that joins a catalog id
//! to its sheet metadata through an explicit App-local catalog.

use bevy::prelude::*;

use ambition_asset_manager::AssetId;

use crate::assets::platformer_assets::{ids, Platformer2dAssetCatalog};
use ambition_characters::actor::character_catalog::{CharacterCatalog, CharacterCatalogData};
use ambition_platformer2d_core as ae;
use ambition_persistence::settings::VisualQualityBudget;
use ambition_sprite_sheet::character::sheets;
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
///    with default tuning (`sheets::try_load_spec_for_character_id`).
///
/// The old hardcoded `*_SHEET` statics + named match are gone — adding
/// a character's bespoke tuning is a `character_catalog.ron` edit.
///
/// Returns `None` only when no manifest exists for the id — usually
/// because the renderer hasn't been run for that target; the actor
/// then renders the colored-rectangle placeholder.
fn sheet_for_character_id_from_data(
    authored: &sheets::AuthoredSheets,
    catalog: &CharacterCatalogData,
    character_id: &str,
) -> Option<CharacterSheetSpec> {
    if let Some(entry) = catalog.characters.get(character_id) {
        if let Some(target) = entry.manifest_target() {
            let tuning = entry
                .sprite_tuning
                .map(|spec| {
                    sheets::SheetTuning::from_parts(
                        spec.collision_scale,
                        spec.frame_sample_inset,
                        spec.feet_anchor_y,
                    )
                })
                .unwrap_or_default();
            if let Some(spec) = sheets::try_load_spec_for_target_authored(authored, target, &tuning)
            {
                return Some(spec);
            }
        }
    }
    let spec = sheets::try_load_spec_for_character_id(character_id);
    if spec.is_none() {
        bevy::log::debug!(
            target: "ambition_platformer2d::character_sprites",
            "character_sprites: no sheet manifest for catalog id '{character_id}' — \
             actor will render the colored-rectangle placeholder",
        );
    }
    spec
}

/// Resolve a declared character's sheet, with the REGISTERED definition winning.
///
/// `register_character` accepts a `sheet` manifest target, and until now nothing
/// in production read it: the materializer resolved sheets exclusively from
/// `CharacterCatalog`, so a character registered only through the new seam got
/// `UnknownCharacter` from the art pipeline, and a character registered through
/// BOTH could name one sheet in its definition and a different one in its catalog
/// row with nothing noticing.
///
/// Precedence and why it is this way round:
///
/// * the **registered target** decides WHICH sheet. The definition is the
///   authority §4.1 is building toward, and a provider that names a sheet in the
///   call it makes should not be overruled by a fragment it may not own.
/// * the **catalog row** still supplies resolution-independent TUNING
///   (`collision_scale`, `frame_sample_inset`, `feet_anchor_y`) and the scaled
///   variant lookup, because that is where quality tiers are authored. Taking the
///   target from one place and the tuning from the other is deliberate, not a
///   layering accident.
/// * a disagreement is LOGGED rather than silently resolved, since it means two
///   declarations of the same character exist and one of them is stale — exactly
///   the drift the single-registration seam is meant to end.
pub fn sheet_for_declared_character(
    // Sheets a PROVIDER authored (queue U1), consulted before the engine's
    // baked cache. Threaded rather than reached for globally: two Apps in one
    // process must not share one game's art declarations, which is the bug the
    // baked index would have if it were writable.
    authored: &sheets::AuthoredSheets,
    character_catalog: &CharacterCatalog,
    registered_target: Option<&str>,
    character_id: &str,
) -> Option<CharacterSheetSpec> {
    let catalog_target = character_catalog
        .get(character_id)
        .and_then(|entry| entry.manifest_target());
    match (registered_target, catalog_target) {
        (Some(registered), Some(from_catalog)) if registered != from_catalog => {
            bevy::log::warn!(
                target: "ambition_platformer2d::character_sprites",
                "character `{character_id}` names sheet `{registered}` in its registered \
                 definition but `{from_catalog}` in its catalog row; using the registered \
                 one. Two declarations of one character disagree — delete the stale one.",
            );
        }
        _ => {}
    }
    let Some(target) = registered_target.or(catalog_target) else {
        // Neither names a target: fall back to the manifest-by-id lookup, which is
        // how most catalog characters have always resolved.
        return sheet_for_character_id_in(authored, character_catalog, character_id);
    };
    let tuning = character_variant_tuning(character_catalog, character_id)
        .map(|(_, tuning)| tuning)
        .unwrap_or_default();
    sheets::try_load_spec_for_target_authored(authored, target, &tuning)
        .or_else(|| sheets::try_load_spec_for_character_id(character_id))
}

/// Resolve a sheet from the caller's assembled App-local catalog.
pub fn sheet_for_character_id_in(
    authored: &sheets::AuthoredSheets,
    character_catalog: &CharacterCatalog,
    character_id: &str,
) -> Option<CharacterSheetSpec> {
    sheet_for_character_id_from_data(authored, character_catalog.data(), character_id)
}

/// The manifest target + resolution-independent tuning for a catalog `cid`,
/// when it has a catalog row that names a sheet. This is what
/// [`build_optional_via_catalog`] needs to fetch the **scaled-variant** record
/// keyed `<target>.<suffix>`. `None` for ids resolved through the manifest-by-id
/// fallback (they stay at base resolution — acceptable, they render fine).
fn character_variant_tuning<'a>(
    character_catalog: &'a CharacterCatalog,
    cid: &str,
) -> Option<(&'a str, sheets::SheetTuning)> {
    let entry = character_catalog.get(cid)?;
    let target = entry.manifest_target()?;
    let tuning = entry
        .sprite_tuning
        .map(|spec| {
            sheets::SheetTuning::from_parts(
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
fn sprite_body_collision_for_character_id_from_data(
    authored: &sheets::AuthoredSheets,
    catalog: &CharacterCatalogData,
    character_id: &str,
    ldtk_collision: ae::Vec2,
) -> Option<SpriteBodyCollision> {
    let entry = catalog.characters.get(character_id)?;
    let target = entry.manifest_target()?;
    let spec = sheet_for_character_id_from_data(authored, catalog, character_id)?;
    let record = sheets::record_for_target(target)?;
    let metrics = record.body_metrics.as_ref()?;
    let (body_w, body_h) = body_pixel_extent(metrics)?;
    let frame_w = record.frame_width.max(1) as f32;
    let frame_h = record.frame_height.max(1) as f32;
    // The size the renderer draws today: full frame scaled to the LDtk box.
    let render = sheets::sprite_render_size(
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

/// Derive sprite-body collision from the caller's App-local catalog.
pub fn sprite_body_collision_for_character_id_in(
    // U1 stage B: a body's collision box is DERIVED from its sheet, so a
    // consumer-authored sheet has to reach this or a third party's character
    // renders from its own art and collides with the engine's default box.
    authored: &sheets::AuthoredSheets,
    character_catalog: &CharacterCatalog,
    character_id: &str,
    ldtk_collision: ae::Vec2,
) -> Option<SpriteBodyCollision> {
    sprite_body_collision_for_character_id_from_data(
        authored,
        character_catalog.data(),
        character_id,
        ldtk_collision,
    )
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
/// character (GPT 5.6, 2026-07-28).
fn all_character_sprite_filenames_from_data(
    catalog: &CharacterCatalogData,
) -> Vec<(String, String, Option<String>)> {
    let mut out: Vec<(String, String, Option<String>)> =
        Vec::with_capacity(catalog.characters.len());
    for (cid, entry) in &catalog.characters {
        let sheet = entry.spritesheet.as_str();
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
/// Iterates the caller's App-local character catalog and, for each entry,
/// looks up its [`CharacterSheetSpec`] via [`sheet_for_character_id_in`]. Asset
/// availability gates through
/// [`Platformer2dAssetCatalog::should_attempt_optional_load`]; missing
/// files produce no map entry (callers fall back to colored
/// rectangles).

/// Decode one DECLARED character's sheet and publish it under every token that
/// resolves to it.
///
/// Returns `true` when `token` resolves to a ready sheet afterwards — already
/// decoded, or decoded here. `false` = unknown token, or the asset catalog gated
/// / failed the load (the caller keeps its placeholder rectangle).
///
/// This used to be the only way a non-privileged character ever got art, and it
/// was called from application crates. It is now an implementation detail of the
/// engine materializer in [`crate::character_runtime`]; nothing outside the
/// engine should reach for it, because an app that forgets to is an app whose
/// characters silently render as rectangles.
/// Which of the two halves of a decode failed — a sheet DESCRIPTION or its
/// IMAGE. They are different bugs with different fixes, and reporting both as
/// "no sheet resolved" sent one investigation into a metadata seam that was
/// already correct (queue T2, 2026-07-28).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpriteMaterialization {
    Ready,
    /// No sheet description resolved for this character's target.
    NoSheet,
    /// The sheet resolved; the image did not.
    NoImage,
}

impl SpriteMaterialization {
    pub fn is_ready(self) -> bool {
        matches!(self, Self::Ready)
    }
}

pub fn materialize_declared_character_sprite(
    sprites: &mut CharacterSpriteAssets,
    authored: &sheets::AuthoredSheets,
    character_catalog: &CharacterCatalog,
    asset_catalog: &Platformer2dAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    quality: Option<&VisualQualityBudget>,
    // The sheet manifest target the character's REGISTERED definition names, when
    // it has one. See [`sheet_for_declared_character`] for why this outranks the
    // catalog row.
    registered_target: Option<&str>,
    token: &str,
) -> SpriteMaterialization {
    let cid = match sprites.sheet_state(token) {
        ambition_sprite_sheet::character::CharacterSheetState::Ready(_) => {
            return SpriteMaterialization::Ready
        }
        ambition_sprite_sheet::character::CharacterSheetState::Declared {
            character_id,
        } => character_id.to_string(),
        ambition_sprite_sheet::character::CharacterSheetState::Unknown => {
            return SpriteMaterialization::NoSheet
        }
    };
    let Some(sheet_spec) =
        sheet_for_declared_character(authored, character_catalog, registered_target, &cid)
    else {
        return SpriteMaterialization::NoSheet;
    };
    let asset_id = ids::character_sprite(&cid);
    let variant_tuning = character_variant_tuning(character_catalog, &cid);
    let variant = variant_tuning.as_ref().map(|(t, tn)| (*t, tn));
    let Some(asset) = build_optional_via_catalog(
        asset_catalog,
        asset_server,
        layouts,
        &asset_id,
        &sheet_spec,
        variant,
        Some(&cid),
        quality,
    ) else {
        return SpriteMaterialization::NoImage;
    };
    sprites.publish(&cid, asset);
    SpriteMaterialization::Ready
}

/// Declare every catalog character's sheet WITHOUT decoding any of it.
///
/// Nothing is privileged. There used to be four ids (`player`, `robot`,
/// `goblin`, `sandbag`) that decoded here at startup because they were the
/// "typed hot-path slots"; every other character deferred to room staging. That
/// made eagerness a property of the ENGINE's opinion about four names, so a
/// provider whose protagonist is an ordinary catalog id — Mary-O, Sanic — had its
/// hero fall through to the placeholder rectangle, and the workaround was a
/// hand-written materialization step in each application crate.
///
/// Eagerness is now whatever the DEMAND says: the engine materializer in
/// [`crate::character_runtime`] decodes the ids a session actually stages, behind
/// the reveal barrier. Startup does no sheet decoding at all, which is strictly
/// less than the four it used to do.
///
/// `asset_server`/`layouts`/`quality` are no longer needed to declare, but stay
/// in the signature: this is still where a caller proves it HAS an asset pipeline,
/// and dropping them would silently make the art-free path look identical.
pub fn load_character_sprites_in(
    authored: &sheets::AuthoredSheets,
    character_catalog: &CharacterCatalog,
    _asset_catalog: &Platformer2dAssetCatalog,
    _asset_server: &AssetServer,
    _layouts: &mut Assets<TextureAtlasLayout>,
    _quality: Option<&VisualQualityBudget>,
) -> CharacterSpriteAssets {
    let mut out = CharacterSpriteAssets::default();
    let mut total = 0usize;
    let mut declared = 0usize;
    let mut skipped_no_spec: Vec<&str> = Vec::new();
    for (cid, entry) in character_catalog.iter() {
        total += 1;
        if sheet_for_character_id_in(authored, character_catalog, cid).is_none() {
            // Neither a hardcoded const nor a manifest in `assets/sprites/`
            // exists for this id — nothing to declare. The character draws the
            // marked placeholder until its sprite is published.
            skipped_no_spec.push(cid.as_str());
            continue;
        }
        declared += 1;
        out.declare(cid, &entry.display_name);
    }
    bevy::log::info!(
        target: "ambition_platformer2d::character_sprites",
        "character_sprites: {declared}/{total} catalog entries declared, 0 decoded at startup \
         (the engine materializer decodes what a session demands); \
         {} no spec wired (placeholder)",
        skipped_no_spec.len(),
    );
    if !skipped_no_spec.is_empty() {
        bevy::log::debug!(
            target: "ambition_platformer2d::character_sprites",
            "character_sprites: no_spec ids: {skipped_no_spec:?}",
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
    catalog: &Platformer2dAssetCatalog,
    base_id: &AssetId,
    base_spec: &CharacterSheetSpec,
    variant: Option<(&str, &sheets::SheetTuning)>,
    quality: Option<&VisualQualityBudget>,
) -> (CharacterSheetSpec, AssetId) {
    if let (Some((target, tuning)), Some(q)) = (variant, quality) {
        if q.sprites.prefer_scaled_variants {
            let scale = q.sprites.resolution_scale;
            if scale != crate::persistence::settings::TextureResolutionScale::Full {
                if let Some(variant_id) =
                    crate::assets::platformer_assets::scaled_asset_id(base_id, scale)
                {
                    if catalog.try_path_for_load(&variant_id).is_some() {
                        if let Some(spec) = sheets::try_load_spec_for_target_scaled(
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
    catalog: &Platformer2dAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    base_id: &AssetId,
    base_spec: &CharacterSheetSpec,
    variant: Option<(&str, &sheets::SheetTuning)>,
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
    // `pages` stays indexed BY PAGE NUMBER — the animator addresses it as
    // `pages[frame_page]` — so the vector keeps its full length. What changes
    // is which entries carry a real handle: only the pages this target's
    // frames actually reference. For a dedicated sheet that is every page and
    // nothing changes; for a target inside a shared pack it is a handful
    // instead of the whole pack.
    let used_pages = spec.used_pages();
    let pages: Vec<CharacterSpritePage> = (0..page_count)
        .map(|page| {
            if !used_pages.contains(&page) {
                // Never sampled: no frame rect names this page. A default
                // handle costs no decode and no VRAM, and reaching it would
                // mean the frame→page mapping disagrees with `used_pages`.
                return CharacterSpritePage {
                    texture: Handle::default(),
                    layout: Handle::default(),
                };
            }
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
    // The representative texture/layout must name a page that actually
    // LOADS: readiness guards test `images.get(&asset.texture)`, and page 0
    // can be absent from a packed target's set entirely, which would leave
    // those guards waiting on a handle nothing is loading. Identical to the
    // old `pages[0]` whenever page 0 is used, which is every dedicated sheet.
    let representative = used_pages.iter().copied().next().unwrap_or(0) as usize;
    let texture = pages[representative].texture.clone();
    let layout = pages[representative].layout.clone();
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
    catalog: &Platformer2dAssetCatalog,
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
    catalog: &Platformer2dAssetCatalog,
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
        sheets::try_load_pack_spec_for_target(target, &tuning, sprite_texture_scale(scale))?;
    // Profile-gate page 0 through the sandbox catalog like every other
    // sprite; sibling pages resolve from the spec's page_images against
    // page 0's directory (the pack pages all share the tier dir).
    let id = crate::assets::platformer_assets::ids::sprite_pack_page0(tier);
    let path = catalog.try_path_for_load(&id)?;
    Some(load_sprite_pages(asset_server, layouts, &path, &spec))
}

pub fn build_prop_sprite_asset(
    catalog: &Platformer2dAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    id: &AssetId,
    spec: &CharacterSheetSpec,
) -> Option<CharacterSpriteAsset> {
    build_optional_via_catalog(catalog, asset_server, layouts, id, spec, None, None, None)
}

/// Load a prop sprite sheet straight from its generated manifest TARGET, without
/// a `Platformer2dAssetCatalog` — for a demo that registers one animated prop (a
/// collectible ring) and doesn't carry that prop in its lean asset catalog. The
/// spec comes from the build-embedded manifest index (`try_load_spec_for_target`)
/// and the page-0 image resolves to `<sprite_folder>/<target>_spritesheet.png`,
/// the same logical path the catalog would hand back at base resolution. Returns
/// `None` when no manifest was embedded for `target` (the caller keeps the static
/// fallback). Base resolution only — a demo prop needs no quality-tier gating.
pub fn load_prop_sheet_for_target(
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    sprite_folder: &str,
    target: &str,
    tuning: &sheets::SheetTuning,
) -> Option<CharacterSpriteAsset> {
    let spec = sheets::try_load_spec_for_target(target, tuning)?;
    let page0_path = format!("{sprite_folder}/{target}_spritesheet.png");
    Some(load_sprite_pages(asset_server, layouts, &page0_path, &spec))
}

#[cfg(test)]
mod sprite_body_collision_tests {
    use super::*;
    use ambition_sprite_sheet::{BodyMetrics, NamedPixelRect, PixelRect};

    const CATALOG_A: &str = r#"(
        brain_presets: { "idle": StandStill },
        action_set_presets: { "peaceful": (move_style: Walk) },
        characters: {
            "alpha": (
                display_name: "Alpha", spritesheet: "sprites/alpha.png",
                manifest: "alpha.ron", tier: MainHall, body_kind: Standard,
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
                display_name: "Beta", spritesheet: "sprites/beta.png",
                manifest: "beta.ron", tier: MainHall, body_kind: Standard,
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

    /// **A consumer's own art survives catalog assembly.**
    ///
    /// The engine's convention is a basename under the shared sprite folder, and
    /// every path went through it: `game://sprites/mine.png` had no `sprites/`
    /// prefix to strip, so the whole string became the "filename" and the
    /// manifest rebuilt it as `sprites/game://sprites/mine.png` — a path to
    /// nothing, silently placeheld into a bare box. The reader could reach the
    /// consumer's tree the whole time; nothing could ADDRESS it from a catalog
    /// (GPT 5.6, 2026-07-28).
    #[test]
    fn a_source_qualified_spritesheet_keeps_its_source() {
        const CONSUMER: &str = r#"(
            brain_presets: { "idle": StandStill },
            action_set_presets: {
                "peaceful": (move_style: Walk, melee: None, ranged: None, special: None),
            },
            characters: {
                "outlander": (
                    display_name: "Outlander", spritesheet: "game://sprites/outlander.png",
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
        let catalog = crate::character_roster::catalog();
        let Some((cid, derived)) = catalog.iter().find_map(|(cid, _)| {
            sprite_body_collision_for_character_id_in(&Default::default(), &catalog, cid, ldtk)
                .map(|derived| (cid, derived))
        }) else {
            return; // no baked sheet with metrics available
        };
        let entry = catalog.get(cid).unwrap();
        let target = entry.manifest_target().unwrap();
        let spec = sheet_for_character_id_in(&Default::default(), &catalog, cid).unwrap();
        let record = sheets::record_for_target(target).unwrap();
        let metrics = record.body_metrics.as_ref().unwrap();
        let (body_w, body_h) = body_pixel_extent(metrics).unwrap();
        let frame_w = record.frame_width.max(1) as f32;
        let frame_h = record.frame_height.max(1) as f32;

        // (1) render == legacy sprite_render_size(spec, ldtk).
        let legacy = sheets::sprite_render_size(&spec, bevy::math::Vec2::new(ldtk.x, ldtk.y));
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
