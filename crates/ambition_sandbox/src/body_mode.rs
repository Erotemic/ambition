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
pub use morph_ball::{build_morph_ball_sprite, spawn_morph_ball_visual, sync_morph_ball_visual};

// Re-exported for `body_mode/tests.rs`. That module is currently
// quarantined behind `#![cfg(any())]` pending the cluster-component
// test port (see `body_mode/tests.rs`'s top-of-file note). Drop the
// `#[allow(unused_imports)]` once the tests are reactivated.
#[cfg(test)]
#[allow(unused_imports)]
pub(super) use morph_ball::build_morph_ball_image;
