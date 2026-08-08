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
pub use assets::{CharacterSheetState, CharacterSpriteAssets};
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

/// **One physical realization of a character's art.**
///
/// The handles are STRONG: while this value is alive the images it names cannot
/// be freed, and dropping it is the whole eviction mechanism — Bevy reclaims an
/// `Image` when its last strong handle goes, so there is no evictor anywhere in
/// this codebase and there must not be one.
#[derive(Clone)]
pub struct CharacterSpriteAsset {
    pub texture: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub spec: CharacterSheetSpec,
    pub pages: Vec<CharacterSpritePage>,
    /// **The quality tier this realization ANSWERS.**
    ///
    /// A realization is not just "the art for character X", it is "the art for
    /// character X at tier T" — the spec's frame rects and the pages' pixels
    /// come from the same tier and only address each other. Recording it here,
    /// on the thing itself, is what lets a live quality change be a comparison
    /// rather than a guess: a resident realization whose tier is not the active
    /// one is stale, and a presentation bound from it is stale too.
    ///
    /// ⚠ **the TIER, not the profile and not a monotonic counter.** `Low` and
    /// `Medium` both realize sheets at `Half`, so a profile id (or a generation
    /// bumped on every Apply) would evict and re-decode the whole cast to arrive
    /// at byte-identical pixels. The tier is also directly comparable against
    /// [`SpriteTextureBudget::effective_scale`](ambition_persistence::settings::SpriteTextureBudget::effective_scale),
    /// which means "is everything resident at the active tier?" needs no second
    /// authority holding a current generation number.
    ///
    /// ⛔ **ANSWERS, not "was loaded from".** Not every sheet has every variant
    /// baked, so a `Half` budget legitimately loads a full-res PNG for some
    /// characters; stamping that `Full` would leave it permanently unequal to
    /// the active tier and the transition would rebuild it every frame forever.
    /// Whatever the materializer produces for a tier IS that tier's answer.
    pub tier: ambition_persistence::settings::TextureResolutionScale,
}
