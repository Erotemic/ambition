//! Sandbox-side body-mode driver.
//!
//! The module is split by concern:
//! - `mechanics` owns crouch / climb / morph-ball state transitions.
//! - `morph_ball` owns the procedural morph-ball sprite and visual sync.
//! - `tests` owns the body-mode regression suite.

mod mechanics;
mod morph_ball;

#[cfg(test)]
mod tests;

pub use mechanics::update_body_mode;
pub use morph_ball::{
    build_morph_ball_image, build_morph_ball_sprite, spawn_morph_ball_visual,
    sync_morph_ball_visual, MorphBallSprite, MorphBallVisual,
};
