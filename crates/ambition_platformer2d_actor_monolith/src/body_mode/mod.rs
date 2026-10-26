//! Body-mode transitions from accepted control and contact facts.
//!
//! The driver applies crouch, climb, ball, and standing geometry to one body.
//! Presentation reads the resulting body state.

mod mechanics;

pub use mechanics::update_body_mode;
