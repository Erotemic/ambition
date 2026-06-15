//! Sandbox-side body-mode driver.
//!
//! The module is split by concern:
//! - `mechanics` owns crouch / climb / morph-ball state transitions.
//! - `morph_ball` owns the procedural morph-ball sprite and visual sync.

mod mechanics;

pub use mechanics::update_body_mode;
