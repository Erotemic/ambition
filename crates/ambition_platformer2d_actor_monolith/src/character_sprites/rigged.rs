//! Attach a ready sheet's transform flipbook, for the rigged sprites.
//!
//! A sheet that publishes a part flipbook (`<target>_parts.ron`, see
//! `ambition_sprite_sheet::character::rigged`) gets its flipbook pages loaded
//! beside its sheet pages, at the tier the sheet resolved, when the
//! composition admits the flipbooks ([`RiggedSpriteAdmission`], on by default). The pages ride on
//! the sheet's realization, so the demand, quality and retirement roads that
//! own the sheet own them too: a sheet republished at another tier starts
//! without pages and is given the new tier's.
//!
//! The pages go on the sheet while they load, as the sheet's own pages do.
//! The renderer shows them only when every page is ready (see
//! `ambition_render::rendering::actors::rigged`), so a body never draws parts
//! that have no pixels yet.
//!
//! This is a system of its own, not part of the materializer, so that the
//! baked load road stays the same with the switch off.
//!
//! ⛔ With the switch off this does nothing, and no flipbook page is loaded.

use std::collections::HashSet;
use std::sync::Arc;

use bevy::prelude::*;

use ambition_persistence::settings::TextureResolutionScale;
use ambition_sprite_sheet::character::rigged::{RiggedSpriteAdmission, RiggedSpriteAsset, RiggedSpritePages};
use ambition_sprite_sheet::character::CharacterSpriteAsset;

/// The image-stage road flipbook pages are demanded on, so the residency
/// census counts part texels apart from sheet texels.
pub const RIGGED_SPRITE_ROAD: &str = "character-parts";

/// Mirror the composition's switch into the sheet table, where the decode
/// reads it ([`ambition_sprite_sheet::character::CharacterSpriteAssets::parts_admitted`]).
pub fn mirror_rigged_admission(
    admission: Option<Res<RiggedSpriteAdmission>>,
    assets: Option<ResMut<ambition_sprite_sheet::game_assets::GameAssets>>,
) {
    let admit = admission.is_some_and(|admission| admission.admit);
    if let Some(mut assets) = assets {
        if assets.characters.parts_admitted() != admit {
            assets.bypass_change_detection().characters.set_parts_admitted(admit);
        }
    }
}

/// Load the flipbook pages of every ready sheet that publishes a flipbook for
/// its resolved tier and does not carry them yet.
///
/// `unpublished` remembers the `(target, tier)` pairs that publish no
/// flipbook, so a sheet without one is looked up once, not every frame.
pub fn attach_rigged_sprite_pages(
    admission: Option<Res<RiggedSpriteAdmission>>,
    assets: Option<ResMut<ambition_sprite_sheet::game_assets::GameAssets>>,
    asset_server: Option<Res<AssetServer>>,
    mut unpublished: Local<HashSet<(String, TextureResolutionScale)>>,
) {
    if !admission.is_some_and(|admission| admission.admit) {
        return;
    }
    let (Some(mut assets), Some(asset_server)) = (assets, asset_server) else {
        return;
    };
    // Through `bypass_change_detection` first: most frames attach nothing, and
    // a `GameAssets` marked changed every frame would wake every reader of it.
    let characters = &mut assets.bypass_change_detection().characters;
    let mut attached = false;
    characters.attach_rigged_pages(|asset| {
        let key = (asset.spec.base_sheet_key().to_owned(), asset.resolved_tier);
        if unpublished.contains(&key) {
            return None;
        }
        let pages = rigged_pages_for(asset, &asset_server);
        if pages.is_none() {
            unpublished.insert(key);
        }
        attached |= pages.is_some();
        pages
    });
    if attached {
        assets.set_changed();
    }
}

/// The flipbook pages of `asset`'s sheet at its resolved tier, loaded from the
/// directory its sheet page loaded from, or `None` when that tier publishes no
/// flipbook.
///
/// # Panics
///
/// When the tier's table is not the full table's (another part count): both
/// are generated from one publish, so that is a stale tier. Also when the
/// flipbook does not state each row of the sheet as a part clip or a baked
/// clip: the sheet and the flipbook are published together, so that is a
/// stale or broken publish.
pub fn rigged_pages_for(asset: &CharacterSpriteAsset, asset_server: &AssetServer) -> Option<RiggedSpritePages> {
    let sheet_page = asset_server.get_path(asset.texture.id())?.to_string();
    let directory = sheet_page.rsplit_once('/').map_or("", |(directory, _)| directory);
    rigged_pages_in(&asset.spec, asset.resolved_tier, directory, asset_server)
}

/// The flipbook pages of `spec`'s sheet at `tier`, loaded from `directory`
/// (where that tier's sheet pages are), or `None` when that tier publishes no
/// flipbook. Panics as [`rigged_pages_for`] does.
pub fn rigged_pages_in(
    spec: &ambition_sprite_sheet::character::CharacterSheetSpec,
    tier: TextureResolutionScale,
    directory: &str,
    asset_server: &AssetServer,
) -> Option<RiggedSpritePages> {
    // ⛔ By the SHEET's key, never `spec.target()`: a generator sheet's target
    // names its generator (`robot_archivist`'s is "robot"), so a target key
    // gave one sheet another's flipbook and the hall panicked (2026-10-03).
    // A flipbook is published per sheet, under the sheet's name.
    let target = spec.base_sheet_key();
    let full = RiggedSpriteAsset::baked(target)?;
    // A measured verdict at publish: this character's parts cost more than
    // its sheet, so it is drawn baked (the flipbook stays published).
    if full.realize == ambition_sprite_sheet::character::rigged::Realize::Baked {
        return None;
    }
    full.check_rows(spec.row_names()).unwrap_or_else(|error| {
        panic!(
            "the part flipbook of `{target}` {error} (the realized sheet's {} rows: {:?})",
            spec.row_names().count(),
            spec.row_names().collect::<Vec<_>>()
        )
    });
    let flipbook = full
        .for_tier(tier)?
        .unwrap_or_else(|error| panic!("the part flipbook of `{target}` {error}"));
    let pages = flipbook
        .pages
        .iter()
        .map(|page| {
            let path = if directory.is_empty() {
                page.clone()
            } else {
                format!("{directory}/{page}")
            };
            ambition_sprite_sheet::game_assets::load_part_page(asset_server, RIGGED_SPRITE_ROAD, path)
        })
        .collect();
    Some(RiggedSpritePages {
        flipbook: Arc::new(flipbook),
        pages,
    })
}
