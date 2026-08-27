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
    /// Per-page texture + layout handles, cloned from the source asset so the
    /// renderer can swap the `Sprite`'s image + atlas layout when the playing
    /// animation lives on a different page of a split sheet. Length 1 for the
    /// common single-PNG sheet (the renderer then never swaps).
    pub pages: Vec<CharacterSpritePage>,
    pub current: CharacterAnim,
    /// An authored CLIP the sheet actually has, when one was requested.
    ///
    /// sprite redirect P0. `current` is a [`CharacterAnim`] — 56 semantic body
    /// states — and the new fighter sheets carry rows it has no variant for
    /// (`smash_forward`, `air_dodge`, `tumble`). A move names its clip; when this
    /// sheet has it, the drawing is keyed by ROW and `current` stops deciding.
    ///
    /// `None` is the ordinary case and means *draw the semantic pose* — every
    /// character without an authored move playing, and every sheet that has none
    /// of a move's chain.
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

    /// Both lookups clamp their row/frame, so nothing failed; the art just sat in the wrong place.
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

    /// True when the sheet is split across more than one page image, so the
    /// renderer must select the active animation's page each frame. Single-page
    /// sheets (the common case) skip the swap entirely.
    pub fn is_paged(&self) -> bool {
        self.pages.len() > 1
    }

    /// The sheet ROW this frame is being drawn from, as an index into
    /// `record.rows`.
    ///
    /// ⭐ THE ROW, NOT THE POSE. `current` is one of 56 semantic body states and
    /// a sheet may draw it from a row whose name is nothing like it — or, while a
    /// clip is playing, from a row `current` does not name at all. Anything that
    /// wants to REPRODUCE what is on screen (the moveset inspector blitting the
    /// same sub-rect) needs the row that was chosen, which is the same question
    /// `current_page` already answers for pages.
    ///
    /// `None` when the sheet has no row for the current pose, which is the case a
    /// caller must draw nothing for rather than guess a row number.
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
            // Same rule as `current_render`: a packed clip row can live on a
            // different page than the semantic pose, and swapping to the pose's
            // page draws the clip's flat index out of a texture that does not
            // contain it.
            Some(slot) => self.spec.page_of_at(slot, self.frame),
            None => self.spec.page_of(self.current, self.frame),
        }
    }

    pub fn request(&mut self, anim: CharacterAnim) {
        let anim = self.spec.resolve_anim(anim);
        if self.current == anim && self.clip_slot.is_none() {
            return;
        }
        // leaving a stale clip here would pin the body to one authored row
        // forever: a semantic request is also a statement that no clip is playing.
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
    /// the whole of P0's preference rule in one call: the exact row, then the author's
    /// fallbacks, then [`Self::request`]'s structural pose ladder.
    ///
    /// no `unwrap_or(0)`. An unresolvable chain must fall to the SEMANTIC
    /// ladder, never to row zero — drawing idle for a missing attack row looks
    /// like a character that does not swing.
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
        // an authored clip is keyed by ROW; everything else by pose.
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
    /// an authored clip never loops. A move's timeline owns how long it
    /// runs; the drawing holds its last frame rather than restarting, which is
    /// what `non_looping` says about every attack pose in the semantic
    /// vocabulary too.
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
