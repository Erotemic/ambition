//! Character sprite asset loading and catalog-to-sheet resolution.
//!
//! Character ids, display metadata, and asset paths come from the App-local
//! character catalog; sheet geometry is joined through `ambition_sprite_sheet`.
//! Missing art is allowed so simulation can run with placeholder presentation.
//! Path and source policy stays in `Platformer2dAssetCatalog`.

use bevy::prelude::*;

use ambition_asset_manager::AssetId;

use crate::assets::platformer_assets::{ids, Platformer2dAssetCatalog};
use ambition_characters::actor::character_catalog::{
    CharacterCatalog, CharacterCatalogData, CharacterPortraitRef,
};
use ambition_persistence::settings::{TextureResolutionScale, VisualQualityBudget};
use ambition_platformer2d_core as ae;
// Keep the catalog-to-sheet join qualified so call sites distinguish the
// data-level resolver from this App-local wrapper.
use ambition_sprite_sheet::character::catalog_join;
use ambition_sprite_sheet::character::sheets;
use ambition_sprite_sheet::character::{
    CharacterSheetSpec, CharacterSpriteAsset, CharacterSpriteAssets, CharacterSpritePage,
    SpriteBodyCollision, TextureResolutionScale as SpriteTextureResolutionScale,
};
use ambition_sprite_sheet::PortraitSheetRegistry;

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
/// * the registered target decides WHICH sheet. The definition is the
///   authority §4.1 is building toward, and a provider that names a sheet in the
///   call it makes should not be overruled by a fragment it may not own.
/// * the catalog row still supplies resolution-independent TUNING
///   (`collision_scale`, `frame_sample_inset`, `feet_anchor_y`) and the scaled
///   variant lookup, because that is where quality tiers are authored. Taking the
///   target from one place and the tuning from the other is deliberate, not a
///   layering accident.
/// * a disagreement is LOGGED rather than silently resolved, since it means two
///   declarations of the same character exist and one of them is stale — exactly
///   the drift the single-registration seam is meant to end.
pub fn sheet_for_declared_character(
    // Sheets a PROVIDER authored, consulted before the engine's baked cache.
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

/// Resolve a character's PORTRAIT, with the registered definition winning.
///
/// The registry is optional; compositions may use the catalog convention or provide their own portrait path.
///
/// Registered portrait targets resolve through manifests while catalog rows may
/// still derive concrete portrait paths. Keep those authoring roads separate and
/// converge them on one resolved portrait type:
///
/// * a registered target names a portrait PRODUCT (`"alice"`), resolved
///   through the manifests' own `target` field. A provider that registers a
///   character in Rust can give it a face without editing anybody's catalog.
/// * the catalog row derives concrete paths from the gameplay sheet's name,
///   which is how all 144 of today's portraits resolve and will keep resolving.
/// * a disagreement is LOGGED rather than silently resolved — same argument as
///   the sheet road: it means two declarations of one character exist and one of
///   them is stale.
///
/// `None` for the registry is the opt-out, not an error. A composition that
/// installs no `PortraitSheetRegistry` falls straight through to the catalog
/// convention, which is what "possible to ignore" has to mean in code. An
/// unresolved TARGET does the same rather than failing: the sheet road's answer
/// to a target it cannot find is a named placeholder, never a panic.
pub fn portrait_for_declared_character(
    portraits: Option<&PortraitSheetRegistry>,
    character_catalog: &CharacterCatalog,
    registered_target: Option<&str>,
    character_id: &str,
) -> Option<CharacterPortraitRef> {
    let from_catalog = character_catalog.portrait_ref(character_id);
    let Some(target) = registered_target.map(str::trim).filter(|t| !t.is_empty()) else {
        return from_catalog;
    };
    let Some(registry) = portraits else {
        // A target was authored and nothing can resolve it here. Say so once —
        // silently using the catalog would make an authored face look like a
        // typo in the sheet name.
        bevy::log::warn!(
            target: "ambition_platformer2d::character_sprites",
            "character `{character_id}` names portrait target `{target}` and this \
             composition installs no `PortraitSheetRegistry`; falling back to the \
             catalog's derived portrait.",
        );
        return from_catalog;
    };
    let Some((manifest_path, manifest)) = registry.manifest_for_target(target) else {
        bevy::log::warn!(
            target: "ambition_platformer2d::character_sprites",
            "character `{character_id}` names portrait target `{target}`, which no \
             baked manifest claims; falling back to the catalog's derived portrait.",
        );
        return from_catalog;
    };
    if let Some(catalog_ref) = from_catalog.as_ref() {
        if catalog_ref.manifest != manifest_path {
            bevy::log::warn!(
                target: "ambition_platformer2d::character_sprites",
                "character `{character_id}` names portrait target `{target}` \
                 (`{manifest_path}`) in its registered definition but derives \
                 `{}` from its catalog row; using the registered one. Two \
                 declarations of one character disagree — delete the stale one.",
                catalog_ref.manifest,
            );
        }
    }
    // the manifest's `image` is a BARE FILENAME (`"alice_portraits.png"`) while every loader
    // speaks asset-relative paths.
    let directory = manifest_path
        .rsplit_once('/')
        .map(|(head, _)| head)
        .unwrap_or("");
    let image = if directory.is_empty() {
        manifest.image.clone()
    } else {
        format!("{directory}/{}", manifest.image)
    };
    Some(CharacterPortraitRef {
        image,
        manifest: manifest_path.to_string(),
        default_clip: manifest.default_clip.clone(),
        still_clip: manifest.still_clip.clone(),
    })
}

/// Resolve a sheet from the caller's assembled App-local catalog.
///
/// The join itself is [`catalog_join::sheet_for_character_id_from_data`]; this adapts the
/// Bevy resource to the plain catalog data it takes.
pub fn sheet_for_character_id_in(
    authored: &sheets::AuthoredSheets,
    character_catalog: &CharacterCatalog,
    character_id: &str,
) -> Option<CharacterSheetSpec> {
    catalog_join::sheet_for_character_id_from_data(authored, character_catalog.data(), character_id)
}

/// The manifest target + resolution-independent tuning for a catalog `cid`,
/// when it has a catalog row that names a sheet. This is what
/// [`build_optional_via_catalog`] needs to fetch the scaled-variant record
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

/// Derive sprite-body collision from the caller's App-local catalog.
///
/// The derivation itself is
/// [`catalog_join::sprite_body_collision_for_character_id_from_data`]; this adapts the
/// Bevy resource to the plain catalog data it takes.
pub fn sprite_body_collision_for_character_id_in(
    // U1 stage B: a body's collision box is DERIVED from its sheet, so a
    // consumer-authored sheet has to reach this or a third party's character
    // renders from its own art and collides with the engine's default box.
    authored: &sheets::AuthoredSheets,
    character_catalog: &CharacterCatalog,
    character_id: &str,
    ldtk_collision: ae::Vec2,
) -> Option<SpriteBodyCollision> {
    catalog_join::sprite_body_collision_for_character_id_from_data(
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
/// character.
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

/// the tier vocabulary exists twice — `ambition_persistence` owns the one
/// a setting is written in, `ambition_sprite_sheet` owns the one a sheet lookup
/// speaks — and these two functions are the whole bridge. They are written
/// adjacent so a tier added to one enum and not the other stops compiling here
/// rather than silently resolving as something else.
fn persistence_texture_scale(
    scale: SpriteTextureResolutionScale,
) -> ambition_persistence::settings::TextureResolutionScale {
    match scale {
        SpriteTextureResolutionScale::Potato => {
            ambition_persistence::settings::TextureResolutionScale::Potato
        }
        SpriteTextureResolutionScale::Quarter => {
            ambition_persistence::settings::TextureResolutionScale::Quarter
        }
        SpriteTextureResolutionScale::Half => {
            ambition_persistence::settings::TextureResolutionScale::Half
        }
        SpriteTextureResolutionScale::Full => {
            ambition_persistence::settings::TextureResolutionScale::Full
        }
    }
}

fn sprite_texture_scale(
    scale: ambition_persistence::settings::TextureResolutionScale,
) -> SpriteTextureResolutionScale {
    match scale {
        ambition_persistence::settings::TextureResolutionScale::Potato => {
            SpriteTextureResolutionScale::Potato
        }
        ambition_persistence::settings::TextureResolutionScale::Quarter => {
            SpriteTextureResolutionScale::Quarter
        }
        ambition_persistence::settings::TextureResolutionScale::Half => {
            SpriteTextureResolutionScale::Half
        }
        ambition_persistence::settings::TextureResolutionScale::Full => {
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
/// It is now an implementation detail of the engine materializer in [`crate::character_runtime`];
/// nothing outside the engine should reach for it, because an app that forgets to is an app whose
/// characters silently render as rectangles. Which of the two halves of a decode failed — a sheet
/// DESCRIPTION or its IMAGE. They are different bugs with different fixes, and reporting both as
/// "no sheet resolved" sent one investigation into a metadata seam that was already correct.
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
        // THE RE-ENTRY CACHE, and it is this line. A character another room already
        // prepared costs nothing to stage again: no sheet lookup, no atlas build, no handle
        // request. Pinned by `re_demanding_a_resident_character_repeats_no_preparation`, which
        // counts atlas layouts and goes 1 → 2 the moment this returns early no longer.
        ambition_sprite_sheet::character::CharacterSheetState::Ready(_) => {
            return SpriteMaterialization::Ready
        }
        ambition_sprite_sheet::character::CharacterSheetState::Declared { character_id } => {
            character_id.to_string()
        }
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
/// Choose the (spec, image id, tier that image is) triple under the quality
/// budget. Upgrades to a scaled variant only when both the variant record
/// was baked *and* the variant image resolves under the active asset profile —
/// so the atlas rects (from the spec) always address the PNG that actually
/// loads. Returns the base pair otherwise (and always for props /
/// `variant: None`). Gameplay collision is untouched; it reads the base record
/// separately.
///
/// the third element is the RESOLVED tier and it is not derivable from the budget: only
/// this function knows whether the upgrade happened, and both ways of failing it — no baked
/// record, no image under the profile — land on the authored full-resolution PNG.
fn resolve_variant_pair(
    catalog: &Platformer2dAssetCatalog,
    base_id: &AssetId,
    base_spec: &CharacterSheetSpec,
    variant: Option<(&str, &sheets::SheetTuning)>,
    quality: Option<&VisualQualityBudget>,
) -> (CharacterSheetSpec, AssetId, TextureResolutionScale) {
    if let (Some((target, tuning)), Some(q)) = (variant, quality) {
        let scale = q.sprites.effective_scale();
        if scale != TextureResolutionScale::Full {
            if let Some(variant_id) =
                crate::assets::platformer_assets::scaled_asset_id(base_id, scale)
            {
                if catalog.try_path_for_load(&variant_id).is_some() {
                    if let Some(spec) = sheets::try_load_spec_for_target_scaled(
                        target,
                        tuning,
                        sprite_texture_scale(scale),
                    ) {
                        return (spec, variant_id, scale);
                    }
                }
            }
        }
    }
    // The authored PNG. Whatever was asked for, these are full-resolution
    // pixels — that is what "the base" means.
    (
        base_spec.clone(),
        base_id.clone(),
        TextureResolutionScale::Full,
    )
}

/// The tier a character sheet realizes FOR under `quality`.
///
/// The one authority for the REQUEST: the materializer stamps every realization
/// with it as [`CharacterSpriteAsset::requested_tier`] and the quality
/// transition compares against that, so the two cannot disagree.
///
/// Keying the transition on the pixels makes such a realization permanently unequal to the
/// active tier, so it would be retired and rebuilt every single frame, forever. Stamping what
/// it ANSWERS makes the transition idempotent by construction: whatever the materializer
/// produces is, by definition, this tier's answer, so the next comparison is equal.
///
/// the fact about the pixels is not discarded, it is recorded separately —
/// [`resolve_variant_pair`] returns it and it becomes
/// [`CharacterSpriteAsset::resolved_tier`], which is what residency reporting
/// reads. Two questions, two fields; this function answers only the first.
pub fn character_sprite_tier(quality: Option<&VisualQualityBudget>) -> TextureResolutionScale {
    quality
        .map(|q| q.sprites.effective_scale())
        .unwrap_or(TextureResolutionScale::Full)
}

/// The sheet tier a ROOM caps its characters at, or `None` for no cap.
///
/// Measured 2026-09-01: a Hall of Characters pedestal is drawn 132 px tall at
/// 1080p and loads 496 x 528 frames — 4x linear, 16x areal — and the whole
/// hall at Full is 434 MP against 38 MP at Quarter. Drawn size is a property
/// of the room's camera framing, so the cap is a room fact; today it is
/// derived from the authored `gallery` flag (every pedestal room), and an
/// authored per-room field can replace this derivation without touching a
/// consumer.
pub fn room_sprite_tier_cap(
    room: &ambition_platformer2d_world::rooms::RoomMetadata,
) -> Option<TextureResolutionScale> {
    room.gallery.then_some(TextureResolutionScale::Quarter)
}

/// `(floor, ceiling)` for the characters standing in `room`: the ceiling is
/// the user's setting (`character_sprite_tier`), the floor is that setting
/// lowered by the room's cap. A realization outside the range is stale; see
/// `CharacterSpriteAssets::has_stale_realizations_outside` for why a Full
/// sheet inside a Quarter room is NOT.
pub fn room_character_tier_bounds(
    quality: Option<&VisualQualityBudget>,
    room: Option<&ambition_platformer2d_world::rooms::RoomMetadata>,
) -> (TextureResolutionScale, TextureResolutionScale) {
    let ceiling = character_sprite_tier(quality);
    let floor = room
        .and_then(room_sprite_tier_cap)
        .map_or(ceiling, |cap| cap.min(ceiling));
    (floor, ceiling)
}

/// The budget to REALIZE a room's characters with: the user's budget with its
/// sprite tier lowered to the room's floor. A capped room asks for scaled
/// variants even when the setting would not, because the cap IS a request for
/// smaller pixels. No room, or no cap: the budget unchanged.
pub fn budget_for_room(
    quality: &VisualQualityBudget,
    room: Option<&ambition_platformer2d_world::rooms::RoomMetadata>,
) -> VisualQualityBudget {
    if room.and_then(room_sprite_tier_cap).is_none() {
        return quality.clone();
    }
    let (floor, _) = room_character_tier_bounds(Some(quality), room);
    let mut budget = quality.clone();
    budget.sprites.resolution_scale = floor;
    budget.sprites.prefer_scaled_variants = true;
    budget
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
    let (spec, id, resolved) = resolve_variant_pair(catalog, base_id, base_spec, variant, quality);
    let (spec, id) = (&spec, &id);
    let requested = character_sprite_tier(quality);
    let Some(path) = catalog.try_path_for_load(id) else {
        if let Some(label) = log_label {
            eprintln!(
                "[character_sprites] {label} spritesheet missing under {} profile (id {id}) — falling back to colored rectangle",
                catalog.profile().label(),
            );
        }
        return None;
    };
    Some(load_sprite_pages(
        asset_server,
        layouts,
        &path,
        spec,
        requested,
        resolved,
    ))
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
    // The tier the caller ASKED for. Threaded rather than re-derived: this
    // function is the ONE place a realization is built, so it is the one place
    // that can stamp both halves of the answer at once.
    requested: TextureResolutionScale,
    // The tier `page0_path` and `spec` actually came from — equal to `requested` when the
    // variant existed, `Full` when the caller fell back to the authored PNG.
    resolved: TextureResolutionScale,
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
        requested_tier: requested,
        resolved_tier: resolved,
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
/// Build a prop's sprite asset from the quality-tiered shared sprite pack
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
        .unwrap_or(ambition_persistence::settings::TextureResolutionScale::Full);
    let tuning = base_spec.tuning();
    let (spec, tier) =
        sheets::try_load_pack_spec_for_target(target, &tuning, sprite_texture_scale(scale))?;
    // `tier` is the pack the loader LANDED on — `full` when the requested tier was never
    // generated — and it is the physical truth about every page path below.
    let resolved = persistence_texture_scale(
        ambition_sprite_sheet::sprite_packs::scale_for_pack_tier(tier)
            .expect("catalog_for_scale only ever answers with a tier dir name"),
    );
    // Profile-gate page 0 through the sandbox catalog like every other
    // sprite; sibling pages resolve from the spec's page_images against
    // page 0's directory (the pack pages all share the tier dir).
    let id = crate::assets::platformer_assets::ids::sprite_pack_page0(tier);
    let path = catalog.try_path_for_load(&id)?;
    Some(load_sprite_pages(
        asset_server,
        layouts,
        &path,
        &spec,
        scale,
        resolved,
    ))
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

/// Decode the effect sheets the ENGINE itself draws — every entry of
/// [`ambition_sprite_sheet::fx::FX_SHEETS`], with no content, catalog or LDtk
/// prop involved.
///
/// this is the registration that did not exist. `spawn_effect` reaches
/// for FX art from `ambition_render`, but until now the only way that art got
/// loaded was for a GAME to declare it: Ambition's intro listed
/// `generic_explosions` in its LDtk-prop table, and nothing else in the
/// workspace listed anything. So Smash, Sanic and Mary-O drew the no-asset
/// particle fallback for every effect, forever. An engine that draws an asset
/// has to be able to ship it.
///
/// Base resolution only, like [`load_prop_sheet_for_target`]: an effect is a
/// short-lived overlay and no quality profile has asked to shrink one yet.
/// Sheets whose manifest was not baked are skipped and reported — the caller
/// keeps the particle fallback for anything they would have drawn.
pub fn load_fx_sheets(
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    sprite_folder: &str,
) -> ambition_sprite_sheet::game_assets::FxSheetAssets {
    let mut set = ambition_sprite_sheet::game_assets::FxSheetAssets::default();
    let mut missing: Vec<&'static str> = Vec::new();
    for sheet in ambition_sprite_sheet::fx::FX_SHEETS {
        let Some(spec) = ambition_sprite_sheet::fx::fx_sheet_spec(sheet.target) else {
            missing.push(sheet.target);
            continue;
        };
        let page0_path = format!("{sprite_folder}/{}_spritesheet.png", sheet.target);
        set.insert(
            sheet.target,
            load_sprite_pages(
                asset_server,
                layouts,
                &page0_path,
                &spec,
                TextureResolutionScale::Full,
                TextureResolutionScale::Full,
            ),
        );
    }
    if !missing.is_empty() {
        bevy::log::warn!(
            target: "ambition_platformer2d::fx_sheets",
            "{}/{} engine FX sheets decoded; no baked manifest for {:?}, so effects on \
             those sheets fall back to a particle burst",
            set.len(),
            ambition_sprite_sheet::fx::FX_SHEETS.len(),
            missing,
        );
    }
    set
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
    // Base resolution only, and the stamp says so on BOTH halves: this path
    // never consults a quality budget, so nothing was asked for beyond `Full`
    // and nothing but the authored PNG was loaded.
    Some(load_sprite_pages(
        asset_server,
        layouts,
        &page0_path,
        &spec,
        TextureResolutionScale::Full,
        TextureResolutionScale::Full,
    ))
}

#[cfg(test)]
mod sprite_body_collision_tests {
    use super::*;

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

    /// A consumer's own art survives catalog assembly.
    ///
    /// The engine's convention is a basename under the shared sprite folder, and
    /// every path went through it: `game://sprites/mine.png` had no `sprites/`
    /// prefix to strip, so the whole string became the "filename" and the
    /// manifest rebuilt it as `sprites/game://sprites/mine.png` — a path to
    /// nothing, silently placeheld into a bare box. The reader could reach the
    /// consumer's tree the whole time; nothing could ADDRESS it from a catalog
    /// .
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
        let record = sheets::record_for_sheet_key(target).unwrap();
        let metrics = record.body_metrics.as_ref().unwrap();
        let (body_w, body_h) = metrics
            .body_pixel_extent(ambition_sprite_sheet::character::CharacterAnim::Idle)
            .unwrap();
        let frame_w = record.frame_width.max(1) as f32;
        let frame_h = record.frame_height.max(1) as f32;

        // Both branches are asserted rather than one deleted, because "the legacy
        // path is untouched where no height applies" is half the contract and is
        // what keeps crawlers and any future unauthored kind working.
        let height = entry
            .standing_height
            .or_else(|| entry.body_kind.default_standing_height())
            .filter(|h| *h > 0.0);
        match height {
            Some(height) => {
                assert!(
                    (derived.collision.y - height).abs() < 1e-3,
                    "{cid} states a standing height of {height} but its visible \
                     body measures {}",
                    derived.collision.y,
                );
                // The art is scaled, never stretched: the quad keeps the sheet's
                // frame aspect.
                let frame_aspect = frame_w / frame_h;
                let render_aspect = derived.render_size.x / derived.render_size.y;
                assert!(
                    (render_aspect - frame_aspect).abs() < 1e-3,
                    "{cid}'s render quad {:?} does not keep the frame aspect \
                     {frame_aspect}",
                    derived.render_size,
                );
            }
            None => {
                let legacy =
                    sheets::sprite_render_size(&spec, bevy::math::Vec2::new(ldtk.x, ldtk.y));
                assert!((derived.render_size.x - legacy.x).abs() < 1e-3);
                assert!((derived.render_size.y - legacy.y).abs() < 1e-3);
            }
        }

        // (2) collision == (body / frame) × render.
        let expect_x = body_w / frame_w * derived.render_size.x;
        let expect_y = body_h / frame_h * derived.render_size.y;
        assert!((derived.collision.x - expect_x).abs() < 1e-3);
        assert!((derived.collision.y - expect_y).abs() < 1e-3);
        assert!(derived.collision.x > 0.0 && derived.collision.y > 0.0);
    }

    /// A registry holding one portrait product, built the way the baked table
    /// builds the real one.
    fn portrait_registry() -> PortraitSheetRegistry {
        PortraitSheetRegistry::from_baked_table(&[(
            "sprites/borrowed_face_portraits.ron",
            r#"(
                target: "borrowed_face",
                image: "borrowed_face_portraits.png",
                frame_width: 256,
                frame_height: 320,
                default_clip: "default",
                clips: {
                    "default": (duration_ms: 0, looping: false, frames: [(x: 0, y: 0, w: 256, h: 320)]),
                },
            )"#,
        )])
    }

    fn catalog_with_a_sheet() -> CharacterCatalog {
        CharacterCatalog::from_data(
            ambition_characters::actor::character_catalog::parse_catalog(
                r#"(
                    brain_presets: { "stand_still": StandStill },
                    action_set_presets: {
                        "peaceful": (move_style: Walk, melee: None, ranged: None, special: None),
                    },
                    characters: {
                        "hero": (
                            display_name: "Hero",
                            spritesheet: "sprites/hero_spritesheet.png",
                            manifest: "sprites/hero_spritesheet.ron",
                            tier: MainHall, body_kind: Standard, composition: None,
                            default_brain: "stand_still", default_action_set: "peaceful",
                        ),
                    },
                )"#,
            ),
        )
    }

    /// A character that names no portrait keeps the catalog result.
    #[test]
    fn a_character_with_no_portrait_target_gets_the_catalogs_derived_portrait() {
        let catalog = catalog_with_a_sheet();
        let resolved =
            portrait_for_declared_character(Some(&portrait_registry()), &catalog, None, "hero")
                .expect("the catalog derives a portrait from the sheet name");
        assert_eq!(resolved.image, "sprites/hero_portraits.png");
        assert_eq!(resolved.manifest, "sprites/hero_portraits.ron");
        assert_eq!(resolved.default_clip, "default");
    }

    /// A registered TARGET outranks the convention, which is the whole
    /// feature: a character registered in Rust can bring its own face without
    /// editing anybody's catalog.
    #[test]
    fn a_registered_portrait_target_outranks_the_catalog_convention() {
        let catalog = catalog_with_a_sheet();
        let resolved = portrait_for_declared_character(
            Some(&portrait_registry()),
            &catalog,
            Some("borrowed_face"),
            "hero",
        )
        .expect("the named target resolves");
        // the manifest's own `image` is a BARE filename; the resolver joins it
        // to the manifest's directory. Without that this reads
        // "borrowed_face_portraits.png" and loads nothing, silently.
        assert_eq!(resolved.image, "sprites/borrowed_face_portraits.png");
        assert_eq!(resolved.manifest, "sprites/borrowed_face_portraits.ron");
    }

    /// Both ways of having no resolver fall through, rather than failing.
    ///
    /// A composition with no `PortraitSheetRegistry` is the opt-out, and a target
    /// nothing claims is an authoring mistake — neither should cost a character
    /// its face when the catalog can still answer.
    #[test]
    fn an_unresolvable_target_falls_back_to_the_catalog_rather_than_failing() {
        let catalog = catalog_with_a_sheet();
        for (registry, target) in [
            (None, Some("borrowed_face")),
            (Some(portrait_registry()), Some("a_target_nobody_baked")),
        ] {
            let resolved =
                portrait_for_declared_character(registry.as_ref(), &catalog, target, "hero")
                    .expect("the catalog still answers");
            assert_eq!(resolved.image, "sprites/hero_portraits.png");
        }
    }

    /// A character the catalog does not carry and that names no target has no
    /// face, and says so with `None` rather than an invented path.
    #[test]
    fn a_character_nothing_knows_resolves_to_no_portrait() {
        let catalog = catalog_with_a_sheet();
        assert!(portrait_for_declared_character(
            Some(&portrait_registry()),
            &catalog,
            None,
            "nobody"
        )
        .is_none());
    }

    #[test]
    fn the_portrait_registry_can_be_asked_by_target() {
        let registry = portrait_registry();
        let (path, manifest) = registry
            .manifest_for_target("borrowed_face")
            .expect("the baked manifest names this target");
        assert_eq!(path, "sprites/borrowed_face_portraits.ron");
        assert_eq!(manifest.target, "borrowed_face");
        assert_eq!(
            registry.available_targets().collect::<Vec<_>>(),
            vec!["borrowed_face"]
        );
        assert!(registry.manifest_for_target("nobody").is_none());
    }
}

#[cfg(test)]
mod room_tier_tests {
    use super::*;
    use ambition_persistence::settings::VisualQualityProfile;
    use ambition_platformer2d_world::rooms::RoomMetadata;

    fn budget(profile: VisualQualityProfile) -> VisualQualityBudget {
        VisualQualityBudget::for_profile(profile)
    }

    fn gallery() -> RoomMetadata {
        RoomMetadata {
            gallery: true,
            ..Default::default()
        }
    }

    /// The hall's pedestals draw 132 px tall and loaded 496 px frames; a
    /// gallery caps its characters at Quarter, whatever the setting above it.
    #[test]
    fn a_gallery_lowers_the_floor_to_quarter_under_a_full_setting() {
        let ultra = budget(VisualQualityProfile::Ultra);
        assert_eq!(character_sprite_tier(Some(&ultra)), TextureResolutionScale::Full);
        let (floor, ceiling) = room_character_tier_bounds(Some(&ultra), Some(&gallery()));
        assert_eq!((floor, ceiling), (TextureResolutionScale::Quarter, TextureResolutionScale::Full));
        let realized_with = budget_for_room(&ultra, Some(&gallery()));
        assert_eq!(realized_with.sprites.effective_scale(), TextureResolutionScale::Quarter);
    }

    /// A setting already below the cap is the floor: the cap never RAISES a tier.
    #[test]
    fn a_potato_setting_stays_potato_in_a_gallery() {
        let potato = budget(VisualQualityProfile::Potato);
        let (floor, ceiling) = room_character_tier_bounds(Some(&potato), Some(&gallery()));
        assert_eq!(floor, ceiling);
        assert_eq!(
            budget_for_room(&potato, Some(&gallery())).sprites.effective_scale(),
            potato.sprites.effective_scale()
        );
    }

    /// No cap, no change: an ordinary room realizes exactly the setting, and the
    /// range collapses to the one tier the old exact rule compared against.
    #[test]
    fn an_ordinary_room_is_the_setting_alone() {
        let ultra = budget(VisualQualityProfile::Ultra);
        let plain = RoomMetadata::default();
        assert_eq!(room_sprite_tier_cap(&plain), None);
        let (floor, ceiling) = room_character_tier_bounds(Some(&ultra), Some(&plain));
        assert_eq!(floor, ceiling);
        assert_eq!(budget_for_room(&ultra, Some(&plain)), ultra);
        assert_eq!(room_character_tier_bounds(Some(&ultra), None).0, ceiling);
    }
}
