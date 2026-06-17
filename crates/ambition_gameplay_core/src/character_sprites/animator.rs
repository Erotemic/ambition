//! Per-entity animation cursor component.
//!
//! [`CharacterAnimator`] tracks the current animation, frame index,
//! per-frame elapsed time, and a "non-looping clip held" flag for
//! Slash / Hit / Death. Each frame, [`CharacterAnimator::tick`]
//! advances the cursor by `dt` and returns the flat atlas index
//! the renderer should display.

use bevy::prelude::*;

use super::anim::{non_looping, CharacterAnim};
use super::sheets::CharacterSheetSpec;

/// Per-character animation cursor.
#[derive(Component)]
pub struct CharacterAnimator {
    pub spec: CharacterSheetSpec,
    pub current: CharacterAnim,
    pub frame: usize,
    pub elapsed: f32,
    /// Once a non-looping clip (Slash/Hit/Death) finishes its last frame
    /// we hold there until `set` switches to a new animation.
    pub clip_held: bool,
}

impl CharacterAnimator {
    pub fn new(spec: &CharacterSheetSpec) -> Self {
        Self {
            spec: spec.clone(),
            current: CharacterAnim::Idle,
            frame: 0,
            elapsed: 0.0,
            clip_held: false,
        }
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
