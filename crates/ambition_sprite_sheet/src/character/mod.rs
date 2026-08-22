//! Character sprite-sheet vocabulary and Bevy-side animation helpers.
//!
//! This is the content-free layer of the former gameplay-core
//! `character_sprites` module: animation row ids, generated sheet manifests,
//! atlas geometry, and the per-entity animator component. Asset-profile policy
//! stays in the host crate.
//!
//! The line the join must not cross is OWNING a catalog, and it does not: it reads one it is given.

use bevy::prelude::*;

pub mod anim;
pub mod animator;
mod assets;
pub mod catalog_join;
pub mod sheets;

pub use anim::{non_looping, ActorAnimOverride, CharacterAnim};
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

/// **One physical realization of a character's art.**
///
/// The handles are STRONG: while this value is alive the images it names cannot
/// be freed, and dropping it is the whole eviction mechanism — Bevy reclaims an
/// `Image` when its last strong handle goes, so there is no evictor anywhere in
/// this codebase and there must not be one.
///
/// ## TWO TIERS, and asking for "the tier" is always a question about one of
/// ## them
///
/// A realization sits between a REQUEST ("draw everything at `Half`") and a
/// FILESYSTEM ("this sheet has no half variant baked"), and the two do not
/// always agree. One field could only serve one of the two questions, and it
/// served the wrong one for the reader that cared most:
///
/// * *"has this request been satisfied?"* — [`Self::requested_tier`]. The
///   convergence key. Answering it with the physical tier makes a fallback
///   permanently unequal to the active tier, so the transition retires it,
///   remakes it identically, and does that again next frame, forever.
/// * *"what is physically in memory?"* — [`Self::resolved_tier`]. The residency
///   key. Answering it with the request reports `Half` while the phone holds
///   full-resolution pixels, which is exactly the number an Android memory
///   budget is decided from.
///
/// **they are not derivable from each other**, in either direction: nothing
/// but the loader knows which variants were baked, and nothing but the settings
/// know what was asked for. Both are recorded here, by the one function that
/// builds a realization, because that is the only place both are in hand.
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
    /// rather than a guess: a resident realization whose requested tier is not
    /// the active one is stale, and the return edge remakes it.
    ///
    /// **the TIER, not the profile and not a monotonic counter.** `Low` and
    /// `Medium` both realize sheets at `Half`, so a profile id (or a generation
    /// bumped on every Apply) would evict and re-decode the whole cast to arrive
    /// at byte-identical pixels. The tier is also directly comparable against
    /// [`SpriteTextureBudget::effective_scale`](ambition_persistence::settings::SpriteTextureBudget::effective_scale),
    /// which means "is everything resident at the active tier?" needs no second
    /// authority holding a current generation number.
    ///
    /// **ANSWERS, not "was loaded from" — that is [`Self::resolved_tier`].**
    /// Not every sheet has every variant baked, so a `Half` budget legitimately
    /// loads a full-res PNG for some characters; keying the transition on the
    /// bytes would leave such a realization permanently unequal to the active
    /// tier and rebuild it every frame forever. Whatever the materializer
    /// produces for a tier IS that tier's answer, which makes the transition
    /// idempotent by construction.
    pub requested_tier: ambition_persistence::settings::TextureResolutionScale,
    /// **The quality tier the bytes in memory actually came from.**
    ///
    /// Equal to [`Self::requested_tier`] whenever the requested variant existed;
    /// coarser when it did not — a `Half` request against a sheet with no baked
    /// half variant resolves `Full`, because the authored full-resolution PNG is
    /// what got decoded.
    ///
    /// **this is the residency truth, and it is the only one worth reporting
    /// to a memory budget.** [`CharacterSpriteAssets::resident_tiers`] is built
    /// from it, and a presentation binder compares against it, because both are
    /// asking about pixels: *what is in memory* and *which generation of the art
    /// is this body showing*. Neither is asking whether a setting was honoured.
    ///
    /// **never key the return edge on this.** See [`Self::requested_tier`] —
    /// a fallback realization is stale against the active tier forever, and
    /// retiring it rebuilds byte-identical pixels at 60Hz.
    pub resolved_tier: ambition_persistence::settings::TextureResolutionScale,
}
