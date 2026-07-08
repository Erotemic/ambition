//! Compatibility facade for presentation camera markers.
//!
//! Canonical definitions live in `ambition_platformer_primitives::camera_layers`
//! so host/render/app code can share camera markers without depending on the
//! actor-domain crate.

pub use ambition_platformer_primitives::camera_layers::*;
