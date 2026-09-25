//! Character sprite-sheet vocabulary and Bevy-side animation helpers.
//!
//! The content-free layer: animation row ids, generated sheet manifests,
//! atlas geometry, and the per-entity animator component. Asset-profile policy
//! stays in the host crate. The catalog join reads a catalog it is given; it
//! does not own one.

use bevy::prelude::*;
use bevy::sprite::Anchor;

pub mod anim;
pub mod animator;
mod assets;
pub mod catalog_join;
pub mod sheets;

pub use anim::{non_looping, ActorAnimOverride, ActorBarkGesture, CharacterAnim};
pub use animator::{CharacterAnimator, RenderBasis};
pub use assets::{CharacterSheetState, CharacterSpriteAssets};
pub use catalog_join::{
    sheet_for_character_id_from_data, sprite_body_collision_for_character_id_from_data,
    SpriteBodyCollision,
};
pub use sheets::*;

/// Texture-quality tiers understood by the baked sprite variant tables.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextureResolutionScale {
    Potato,
    Quarter,
    Half,
    #[default]
    Full,
}

impl TextureResolutionScale {
    pub fn asset_id_suffix(self) -> Option<&'static str> {
        match self {
            Self::Full => None,
            Self::Half => Some("0_5x"),
            Self::Quarter => Some("0_25x"),
            Self::Potato => Some("potato"),
        }
    }
}

/// One page image of a possibly split character sheet.
#[derive(Clone)]
pub struct CharacterSpritePage {
    pub texture: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
}

/// One physical realization of a character's art.
///
/// Image handles are strong; dropping this value is the eviction mechanism.
/// `requested_tier` records which quality request this realization satisfies,
/// while `resolved_tier` records which baked pixels are actually resident. They
/// may differ when the requested variant is unavailable.
#[derive(Clone)]
pub struct CharacterSpriteAsset {
    pub texture: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub spec: CharacterSheetSpec,
    pub pages: Vec<CharacterSpritePage>,
    /// Quality request this realization satisfies. Use this as the convergence
    /// key for live quality changes, even when the loader had to use fallback
    /// pixels from a different [`Self::resolved_tier`].
    pub requested_tier: ambition_persistence::settings::TextureResolutionScale,
    /// Quality tier of the pixels actually resident in memory. Use this for
    /// residency accounting, not for deciding whether a request has converged.
    pub resolved_tier: ambition_persistence::settings::TextureResolutionScale,
}

/// Build a character presentation that is valid on the same frame it becomes drawable.
///
/// A packed or trimmed atlas stores only the opaque sub-rectangle of each
/// logical frame. [`build_character_sprite_with_render_size`] builds the
/// logical frame size, and [`CharacterAnimator`] owns the per-frame trim. Both
/// must apply before the first render. Otherwise a new trimmed frame draws for
/// one frame as if its pixels filled the logical frame, then changes size.
///
/// This is the construction point for ordinary character sprites. It seeds the
/// animator's logical render basis, applies frame zero's trim, and selects
/// frame zero's page on split sheets. Later animation ticks continue from this
/// geometry.
pub fn build_character_presentation_with_render_size(
    asset: &CharacterSpriteAsset,
    render_size: Vec2,
    mut anchor: Anchor,
) -> (Sprite, Anchor, CharacterAnimator) {
    let mut sprite = build_character_sprite_with_render_size(asset, render_size);
    let mut animator = CharacterAnimator::new(asset);
    animator.ensure_render_basis(render_size, anchor.0);

    if animator.is_paged() {
        let page = animator.current_page();
        if let Some(page) = animator.pages.get(page as usize) {
            sprite.image = page.texture.clone();
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                atlas.layout = page.layout.clone();
            }
        }
    }

    if let Some((trimmed_size, trimmed_anchor)) = animator.current_render() {
        sprite.custom_size = Some(trimmed_size);
        anchor.0 = trimmed_anchor;
    }

    (sprite, anchor, animator)
}

/// Collision-sized convenience wrapper around
/// [`build_character_presentation_with_render_size`].
pub fn build_character_presentation(
    asset: &CharacterSpriteAsset,
    collision: Vec2,
    anchor: Anchor,
) -> (Sprite, Anchor, CharacterAnimator) {
    build_character_presentation_with_render_size(
        asset,
        sprite_render_size(&asset.spec, collision),
        anchor,
    )
}
