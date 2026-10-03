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
    /// When `Some`, the clip row is SLAVED to a move's normalized progress
    /// (`floor(phase * frames)`) instead of the sheet's frame clock. Set each
    /// frame by [`Self::slave_clip_to`]; cleared by any new request.
    clip_phase: Option<f32>,
    /// Drawing the current row's MIRROR row (the character seen from its other
    /// side) instead of flipping it. Set each frame by [`Self::face`].
    mirrored: bool,
    pub frame: usize,
    pub elapsed: f32,
    /// Once a non-looping clip (Slash/Hit/Death) finishes its last frame
    /// we hold there until `set` switches to a new animation.
    pub clip_held: bool,
    social_pose: SocialPose,
    /// Base render size + anchor, set at spawn.
    pub render_basis: Option<RenderBasis>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SocialPose {
    Stand,
    SittingDown,
    Sitting,
    StandingUp,
}

impl CharacterAnimator {
    pub fn new(asset: &CharacterSpriteAsset) -> Self {
        Self {
            spec: asset.spec.clone(),
            pages: asset.pages.clone(),
            current: CharacterAnim::Idle,
            // No move is playing on a body that has just been built.
            clip_slot: None,
            clip_phase: None,
            mirrored: false,
            frame: 0,
            elapsed: 0.0,
            clip_held: false,
            social_pose: SocialPose::Stand,
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
        let trim = self.spec.frame_trim_at(self.drawn_slot(), self.frame);
        // A mirror row is the whole frame mirrored, so its feet sit at the
        // mirrored anchor: the same answer a flip of the authored row gives.
        let mut anchor = basis.feet_anchor;
        if self.mirrored {
            anchor.x = -anchor.x;
        }
        Some(trimmed_render(&trim, basis.render_size, anchor))
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
        let authored = match self.clip_slot {
            Some(slot) => Some(slot),
            None => self.spec.row_for_anim(self.current),
        }?;
        Some(self.mirror_of(authored))
    }

    /// The row slot the authored drawing comes from (clip or pose), before
    /// facing.
    fn authored_slot(&self) -> usize {
        self.clip_slot
            .unwrap_or_else(|| self.spec.slot_for_anim(self.current))
    }

    /// `slot`, or its mirror row while drawing the other side.
    fn mirror_of(&self, slot: usize) -> usize {
        if self.mirrored {
            self.spec.mirror_slot(slot).unwrap_or(slot)
        } else {
            slot
        }
    }

    /// The row slot actually drawn this frame.
    fn drawn_slot(&self) -> usize {
        self.mirror_of(self.authored_slot())
    }

    /// Face the drawing. `flip` is whether the renderer would mirror the art;
    /// the answer is whether it STILL must.
    ///
    /// A sheet drawn from both sides (`SheetRow::mirror_of`) answers a flip by
    /// drawing the mirror row, so an asymmetric character keeps its asymmetry
    /// on the correct side; every other sheet is flipped as before. Call after
    /// the frame's request and before [`Self::tick`].
    pub fn face(&mut self, flip: bool) -> bool {
        self.mirrored = flip && self.spec.mirror_slot(self.authored_slot()).is_some();
        flip && !self.mirrored
    }

    /// The page image index the current frame draws from (per-frame, since a
    /// packed animation can span pages).
    pub fn current_page(&self) -> u32 {
        // A clip row can be on a different page than the semantic pose, and a
        // mirror row on a different page than its original.
        self.spec.page_of_at(self.drawn_slot(), self.frame)
    }

    pub fn request(&mut self, anim: CharacterAnim) {
        let anim = self.spec.resolve_anim(anim);
        if self.current == anim && self.clip_slot.is_none() {
            return;
        }
        // A semantic request also clears the clip. A stale clip would pin the
        // body to one authored row.
        let had_clip = self.clip_slot.take().is_some();
        self.clip_phase = None;
        if self.current == anim && !had_clip {
            return;
        }
        self.current = anim;
        self.frame = 0;
        self.elapsed = 0.0;
        self.clip_held = false;
    }

    /// Select a sheet-authored social pose for the addressed actor.
    pub fn request_actor_pose<'a>(
        &mut self,
        anim: CharacterAnim,
        clip: impl IntoIterator<Item = &'a str>,
        has_clip: bool,
        conversation_held: bool,
        barking: bool,
    ) {
        let can_sit = self.spec.maps(CharacterAnim::SitDown)
            && self.spec.maps(CharacterAnim::SitIdle)
            && self.spec.maps(CharacterAnim::StandUp);
        if anim == CharacterAnim::Death {
            self.social_pose = SocialPose::Stand;
            self.request(anim);
            return;
        }
        if can_sit {
            self.social_pose = match (conversation_held, self.social_pose) {
                (true, SocialPose::Stand | SocialPose::StandingUp) => SocialPose::SittingDown,
                (true, SocialPose::SittingDown) if self.clip_held => SocialPose::Sitting,
                (false, SocialPose::Sitting | SocialPose::SittingDown) => SocialPose::StandingUp,
                (false, SocialPose::StandingUp) if self.clip_held => SocialPose::Stand,
                (_, phase) => phase,
            };
            let pose = match self.social_pose {
                SocialPose::SittingDown => Some(CharacterAnim::SitDown),
                SocialPose::Sitting => Some(CharacterAnim::SitIdle),
                SocialPose::StandingUp => Some(CharacterAnim::StandUp),
                SocialPose::Stand => None,
            };
            if let Some(pose) = pose {
                self.request(pose);
                return;
            }
        }
        if barking && self.spec.maps(CharacterAnim::Bark) {
            self.request(CharacterAnim::Bark);
        } else if has_clip {
            self.request_clip(clip, anim);
        } else {
            self.request(anim);
        }
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

    /// Drive the current clip row by a move's normalized progress, or `None`
    /// to let it run on the sheet's own frame clock.
    ///
    /// Call after the frame's request: a request that changes the drawing
    /// clears it. A phase with no clip row showing is ignored — the body fell
    /// back to a semantic pose, which keeps its own clock.
    pub fn slave_clip_to(&mut self, phase: Option<f32>) {
        self.clip_phase = self.clip_slot.and(phase);
    }

    /// How far the current frame has run toward the next, in `0..1`: the `t`
    /// an in-between is drawn at (`rigged::RiggedSpriteAsset::tween_into`).
    /// `0` while a clip holds its last frame, and for a row with no clock. A
    /// clip slaved to a move takes the fraction of the move's progress inside
    /// the frame it selects.
    pub fn frame_phase(&self) -> f32 {
        if self.clip_held {
            return 0.0;
        }
        let row = match self.clip_slot {
            Some(slot) => self.spec.row_at(slot),
            None => self.spec.row(self.current),
        };
        if let (Some(_), Some(phase)) = (self.clip_slot, self.clip_phase) {
            return (phase.clamp(0.0, 1.0) * row.frame_count as f32).fract();
        }
        if row.duration_secs <= 0.0 {
            return 0.0;
        }
        (self.elapsed / row.duration_secs).clamp(0.0, 1.0)
    }

    /// Advance the animation. Returns the flat atlas index for the current frame.
    pub fn tick(&mut self, dt: f32) -> usize {
        self.advance(dt);
        // Whatever advanced, the index is the row actually DRAWN: the authored
        // row, or its mirror while the character shows its other side.
        self.spec.flat_index_at(self.drawn_slot(), self.frame)
    }

    fn advance(&mut self, dt: f32) {
        // An authored clip is keyed by row; all else by pose.
        if let Some(slot) = self.clip_slot {
            self.advance_slot(slot, dt);
            return;
        }
        let row = self.spec.row(self.current);
        if row.frame_count == 0 || row.duration_secs <= 0.0 || self.clip_held {
            return;
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
    }

    /// [`Self::advance`] for an authored clip, keyed by its resolved row slot.
    ///
    /// An authored clip does not loop. The move's timeline owns its length, so
    /// the drawing holds the last frame, as `non_looping` does for attack poses.
    fn advance_slot(&mut self, slot: usize, dt: f32) {
        let row = self.spec.row_at(slot);
        if let (Some(phase), true) = (self.clip_phase, row.frame_count > 0) {
            // Slaved to the move: the row spans the move's duration exactly.
            let frame = (phase.clamp(0.0, 1.0) * row.frame_count as f32) as usize;
            self.frame = frame.min(row.frame_count - 1);
            self.elapsed = 0.0;
            self.clip_held = self.frame + 1 == row.frame_count;
            return;
        }
        if row.frame_count == 0 || row.duration_secs <= 0.0 || self.clip_held {
            return;
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
    }
}
