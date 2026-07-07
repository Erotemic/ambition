//! Per-entity animation cursor component.
//!
//! [`CharacterAnimator`] tracks the current animation, frame index,
//! per-frame elapsed time, and a "non-looping clip held" flag for
//! Slash / Hit / Death. Each frame, [`CharacterAnimator::tick`]
//! advances the cursor by `dt` and returns the flat atlas index
//! the renderer should display.

use bevy::prelude::*;

use super::anim::{non_looping, CharacterAnim};
use super::sheets::{trimmed_render, CharacterSheetSpec};
use super::{CharacterSpriteAsset, CharacterSpritePage};

/// The untrimmed render size + feet anchor a character's sprite was built with.
/// Cached so a trimmed (alpha-packed) sheet can recompute the per-frame
/// `custom_size` + anchor that keeps the logical frame fixed (see
/// [`super::sheets::trimmed_render`]). Set at spawn from the same values used to
/// build the `Sprite` and `Anchor`.
#[derive(Clone, Copy, Debug)]
pub struct RenderBasis {
    pub render_size: Vec2,
    pub feet_anchor: Vec2,
}

/// Per-character animation cursor.
#[derive(Component)]
pub struct CharacterAnimator {
    pub spec: CharacterSheetSpec,
    /// Per-page texture + layout handles, cloned from the source asset so the
    /// renderer can swap the `Sprite`'s image + atlas layout when the playing
    /// animation lives on a different page of a split sheet. Length 1 for the
    /// common single-PNG sheet (the renderer then never swaps).
    pub pages: Vec<CharacterSpritePage>,
    pub current: CharacterAnim,
    pub frame: usize,
    pub elapsed: f32,
    /// Once a non-looping clip (Slash/Hit/Death) finishes its last frame
    /// we hold there until `set` switches to a new animation.
    pub clip_held: bool,
    /// Base render size + anchor, set at spawn. `None` until provided; required
    /// for trimmed sheets (the renderer falls back to the fixed spawn-time
    /// size/anchor when absent or when the sheet is untrimmed).
    pub render_basis: Option<RenderBasis>,
}

impl CharacterAnimator {
    pub fn new(asset: &CharacterSpriteAsset) -> Self {
        Self {
            spec: asset.spec.clone(),
            pages: asset.pages.clone(),
            current: CharacterAnim::Idle,
            frame: 0,
            elapsed: 0.0,
            clip_held: false,
            render_basis: None,
        }
    }

    /// Initialize the trim basis from the spawn-built sprite's size + anchor the
    /// first time the renderer applies a frame — and only then (no-op once set).
    ///
    /// The basis a trimmed sheet needs to recompute per-frame size/anchor IS the
    /// sprite's own full-logical `custom_size` + feet anchor; every spawn site
    /// built it that way (the actor path even reconstructed this arg from
    /// `sprite.custom_size`). So instead of threading it through every
    /// `CharacterAnimator::new` call site — where a forgotten call silently
    /// misaligns a trimmed sheet — the single `apply_character_frame` chokepoint
    /// captures it from the sprite. A sprite + anchor + animator is now
    /// sufficient; no spawn site can desync the basis because none provides it.
    pub fn ensure_render_basis(&mut self, render_size: Vec2, feet_anchor: Vec2) {
        if self.render_basis.is_none() {
            self.render_basis = Some(RenderBasis {
                render_size,
                feet_anchor,
            });
        }
    }

    /// Per-frame `(custom_size, anchor)` for the CURRENT frame, or `None` when
    /// the sheet is untrimmed (or no basis is set) — callers then keep the
    /// fixed spawn-time size/anchor, so untrimmed sheets are unaffected.
    pub fn current_render(&self) -> Option<(Vec2, Vec2)> {
        if !self.spec.is_trimmed() {
            return None;
        }
        let basis = self.render_basis.as_ref()?;
        let trim = self.spec.frame_trim(self.current, self.frame);
        Some(trimmed_render(&trim, basis.render_size, basis.feet_anchor))
    }

    /// True when the sheet is split across more than one page image, so the
    /// renderer must select the active animation's page each frame. Single-page
    /// sheets (the common case) skip the swap entirely.
    pub fn is_paged(&self) -> bool {
        self.pages.len() > 1
    }

    /// The page image index the current frame draws from (per-frame, since a
    /// packed animation can span pages).
    pub fn current_page(&self) -> u32 {
        self.spec.page_of(self.current, self.frame)
    }

    pub fn request(&mut self, anim: CharacterAnim) {
        let anim = self.spec.resolve_anim(anim);
        if self.current == anim {
            return;
        }
        self.current = anim;
        self.frame = 0;
        self.elapsed = 0.0;
        self.clip_held = false;
    }

    /// Advance the animation. Returns the flat atlas index for the current frame.
    pub fn tick(&mut self, dt: f32) -> usize {
        let row = self.spec.row(self.current);
        if row.frame_count == 0 || row.duration_secs <= 0.0 {
            return self.spec.flat_index(self.current, self.frame);
        }
        if self.clip_held {
            return self.spec.flat_index(self.current, self.frame);
        }
        self.elapsed += dt;
        while self.elapsed >= row.duration_secs {
            self.elapsed -= row.duration_secs;
            if self.frame + 1 >= row.frame_count {
                if non_looping(self.current) {
                    self.frame = row.frame_count - 1;
                    self.clip_held = true;
                    break;
                } else {
                    self.frame = 0;
                }
            } else {
                self.frame += 1;
            }
        }
        self.spec.flat_index(self.current, self.frame)
    }
}
