//! Attach a ready sheet's transform flipbook, for the rigged-sprite trial.
//!
//! A sheet that publishes a part flipbook (`<target>_parts.ron`, see
//! `ambition_sprite_sheet::character::rigged`) gets its flipbook pages loaded
//! beside its sheet pages, at the tier the sheet resolved, when the
//! composition admits the trial ([`RiggedSpriteAdmission`]). The pages ride on
//! the sheet's realization, so the demand, quality and retirement roads that
//! own the sheet own them too: a sheet republished at another tier starts
//! without pages and is given the new tier's.
//!
//! ⛔ With the trial off this does nothing, and no flipbook page is loaded.

use std::collections::HashSet;
use std::sync::Arc;

use bevy::prelude::*;

use ambition_persistence::settings::TextureResolutionScale;
use ambition_sprite_sheet::character::rigged::{RiggedSpriteAdmission, RiggedSpriteAsset, RiggedSpritePages};
use ambition_sprite_sheet::character::CharacterSpriteAsset;

/// The image-stage road flipbook pages are demanded on, so the residency
/// census counts part texels apart from sheet texels.
pub const RIGGED_SPRITE_ROAD: &str = "character-parts";

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
        let key = (asset.spec.target().to_owned(), asset.resolved_tier);
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
/// are generated from one publish, so that is a stale tier.
pub fn rigged_pages_for(asset: &CharacterSpriteAsset, asset_server: &AssetServer) -> Option<RiggedSpritePages> {
    let target = asset.spec.target();
    let full = RiggedSpriteAsset::baked(target)?;
    let flipbook = full
        .for_tier(asset.resolved_tier)?
        .unwrap_or_else(|error| panic!("the part flipbook of `{target}` {error}"));
    let sheet_page = asset_server.get_path(asset.texture.id())?.to_string();
    let directory = sheet_page.rsplit_once('/').map_or("", |(directory, _)| directory);
    let pages = flipbook
        .pages
        .iter()
        .map(|page| {
            let path = if directory.is_empty() {
                page.clone()
            } else {
                format!("{directory}/{page}")
            };
            ambition_sprite_sheet::game_assets::load_sheet_image(asset_server, RIGGED_SPRITE_ROAD, path)
        })
        .collect();
    Some(RiggedSpritePages {
        flipbook: Arc::new(flipbook),
        pages,
    })
}
