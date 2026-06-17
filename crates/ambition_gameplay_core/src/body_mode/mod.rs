//! Sandbox-side body-mode driver: facade re-exporting [`update_body_mode`].
//!
//! [`mechanics`] owns the whole driver — the crouch / climb / morph-ball
//! / stand-up state transitions read from input + contact state and ask
//! the engine to flip the player's `BodyMode`. (Morph-ball sprite visuals
//! live elsewhere, not in this module.)

mod mechanics;

pub use mechanics::update_body_mode;
