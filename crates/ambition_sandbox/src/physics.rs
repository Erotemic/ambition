//! Shared world physics facade.
//!
//! Re-exports the content-free gravity runtime from
//! `ambition_platformer_primitives::gravity` under the sandbox's stable
//! `crate::physics::{…}` path.
//!
//! The gravity *mechanic* layered on top (`GravityFlipSwitch`, the room-reset
//! reset, the `GravityPlugin`, and the zone / switch visuals) stays sandbox-side
//! in `crate::mechanics::gravity` because it depends on sandbox content
//! (audio / features / app schedule / presentation); it consumes the moved core
//! types through this facade.
pub use ambition_platformer_primitives::gravity::*;
