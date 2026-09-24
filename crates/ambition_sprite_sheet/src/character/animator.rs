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

#[derive(Clone, Copy, Debug)]
pub struct RenderBasis {
    pub render_size: Vec2,
    pub feet_anchor: Vec2,
}

/// Per-character animation cursor.
#[derive(Component)]
pub struct CharacterAnimator {
    pub spec: CharacterSheetSpec,
    /// Per-page texture and layout handles, cloned from the source asset. The
    /// renderer swaps the `Sprite` image and layout when the animation is on
    /// another page of a split sheet. Length is 1 for a single-PNG sheet.
    pub pages: Vec<CharacterSpritePage>,
    pub current: CharacterAnim,
    /// The authored clip row slot, when a requested clip exists on this sheet.
    ///
    /// `current` has only the semantic body states. Fighter sheets also have
    /// rows with no variant (`smash_forward`, `air_dodge`, `tumble`). When this
    /// is `Some`, the row decides the drawing and `current` does not.
    /// `None` means draw the semantic pose.
    clip_slot: Option<usize>,
    pub frame: usize,
    pub elapsed: f32,
    /// Once a non-looping clip (Slash/Hit/Death) finishes its last frame
    /// we hold there until `set` switches to a new animation.
    pub clip_held: bool,
    /// Base render size + anchor, set at spawn.
    pub render_basis: Option<RenderBasis>,
}

impl CharacterAnimator {
    pub fn new(asset: &CharacterSpriteAsset) -> Self {
        Self {
            spec: asset.spec.clone(),
            pages: asset.pages.clone(),
            current: CharacterAnim::Idle,
            // No move is playing on a body that has just been built.
            clip_slot: None,
            frame: 0,
            elapsed: 0.0,
            clip_held: false,
            render_basis: None,
        }
    }

    /// Initialize the full-logical trim basis once.
    ///
    /// Ordinary construction sets this before the sprite is drawable, so frame
    /// zero has correct trimmed geometry. The animation chokepoint keeps a
    /// fallback that captures the basis for specialized or legacy paths. After
    /// it is set, the basis does not change: all trimmed frames project from it.
    pub fn ensure_render_basis(&mut self, render_size: Vec2, feet_anchor: Vec2) {
        if self.render_basis.is_none() {
            self.render_basis = Some(RenderBasis {
                render_size,
                feet_anchor,
            });
        }
    }

    /// The trimmed render size and anchor for the current frame.
    /// Same rule as [`Self::tick`]: if a clip is playing, the slot decides.
    pub fn current_render(&self) -> Option<(Vec2, Vec2)> {
        if !self.spec.is_trimmed() {
            return None;
        }
        let basis = self.render_basis.as_ref()?;
        let trim = match self.clip_slot {
            Some(slot) => self.spec.frame_trim_at(slot, self.frame),
            None => self.spec.frame_trim(self.current, self.frame),
        };
        Some(trimmed_render(&trim, basis.render_size, basis.feet_anchor))
    }

    /// True when the sheet has more than one page image, so the renderer must
    /// select the page each frame.
    pub fn is_paged(&self) -> bool {
        self.pages.len() > 1
    }

    /// The sheet ROW this frame is being drawn from, as an index into
    /// `record.rows`.
    ///
    /// This is the row, not the pose. A sheet can draw `current` from a row with
    /// a different name, and a playing clip uses a row `current` does not name.
    /// Code that reproduces the screen (the moveset inspector) needs this row.
    ///
    /// `None` when the sheet has no row for the current pose. The caller must
    /// then draw nothing, not guess a row.
    pub fn drawn_row(&self) -> Option<usize> {
        match self.clip_slot {
            Some(slot) => Some(slot),
            None => self.spec.row_for_anim(self.current),
        }
    }

    /// The page image index the current frame draws from (per-frame, since a
    /// packed animation can span pages).
    pub fn current_page(&self) -> u32 {
        match self.clip_slot {
            // A clip row can be on a different page than the semantic pose.
            Some(slot) => self.spec.page_of_at(slot, self.frame),
            None => self.spec.page_of(self.current, self.frame),
        }
    }

    pub fn request(&mut self, anim: CharacterAnim) {
        let anim = self.spec.resolve_anim(anim);
        if self.current == anim && self.clip_slot.is_none() {
            return;
        }
        // A semantic request also clears the clip. A stale clip would pin the
        // body to one authored row.
        let had_clip = self.clip_slot.take().is_some();
        if self.current == anim && !had_clip {
            return;
        }
        self.current = anim;
        self.frame = 0;
        self.elapsed = 0.0;
        self.clip_held = false;
    }

    /// Play an authored CLIP if this sheet has one of `chain`; otherwise the
    /// semantic pose.
    ///
    /// Order: the exact row, then the author's fallbacks, then the structural
    /// pose ladder of [`Self::request`]. An unresolvable chain goes to the
    /// semantic ladder, not to row zero, so a missing attack row does not draw
    /// idle.
    pub fn request_clip<'a>(
        &mut self,
        chain: impl IntoIterator<Item = &'a str>,
        fallback: CharacterAnim,
    ) {
        let Some(slot) = self.spec.clip_slot(chain) else {
            self.request(fallback);
            return;
        };
        if self.clip_slot == Some(slot) {
            return;
        }
        self.clip_slot = Some(slot);
        self.frame = 0;
        self.elapsed = 0.0;
        self.clip_held = false;
    }

    /// Advance the animation. Returns the flat atlas index for the current frame.
    pub fn tick(&mut self, dt: f32) -> usize {
        // An authored clip is keyed by row; all else by pose.
        if let Some(slot) = self.clip_slot {
            return self.tick_slot(slot, dt);
        }
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

    /// [`Self::tick`] for an authored clip, keyed by its resolved row slot.
    ///
    /// An authored clip does not loop. The move's timeline owns its length, so
    /// the drawing holds the last frame, as `non_looping` does for attack poses.
    fn tick_slot(&mut self, slot: usize, dt: f32) -> usize {
        let row = self.spec.row_at(slot);
        if row.frame_count == 0 || row.duration_secs <= 0.0 || self.clip_held {
            return self.spec.flat_index_at(slot, self.frame);
        }
        self.elapsed += dt;
        while self.elapsed >= row.duration_secs {
            self.elapsed -= row.duration_secs;
            if self.frame + 1 >= row.frame_count {
                self.frame = row.frame_count - 1;
                self.clip_held = true;
                break;
            }
            self.frame += 1;
        }
        self.spec.flat_index_at(slot, self.frame)
    }
}
