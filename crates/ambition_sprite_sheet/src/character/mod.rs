//! Character sprite-sheet vocabulary and Bevy-side animation helpers.
//!
//! This is the content-free layer of the former gameplay-core
//! `character_sprites` module: animation row ids, generated sheet manifests,
//! atlas geometry, and the per-entity animator component. The game-specific
//! catalog join and asset-profile policy stay in the host crate.

use bevy::prelude::*;

pub mod anim;
pub mod animator;
mod assets;
pub mod sheets;

pub use anim::{non_looping, CharacterAnim};
pub use animator::{CharacterAnimator, RenderBasis};
pub use assets::CharacterSpriteAssets;
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

#[derive(Clone)]
pub struct CharacterSpriteAsset {
    pub texture: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub spec: CharacterSheetSpec,
    pub pages: Vec<CharacterSpritePage>,
}
