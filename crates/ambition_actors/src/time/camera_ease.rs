//! Compatibility facade for camera easing/shake presentation state.
//!
//! Canonical definitions live in `ambition_platformer_primitives::camera_ease`
//! so render/host/runtime code can share the resource types without depending on
//! the actor-domain crate.

pub use ambition_platformer_primitives::camera_ease::*;
