//! Shared world physics facade.
//!
//! The gravity runtime (the world's redirectable gravity state, gravity zones /
//! their per-frame snapshot, the [`GravityCtx`] system param, the
//! `oscillate`/`collect`/`tick_temporary`/`resolve_active` systems, and the pure
//! orientation helpers) is content-free and now lives in the
//! `ambition_platformer_runtime::gravity` module (Stage 16 / S4). This module
//! re-exports the whole surface unchanged so every `crate::physics::{…}` path
//! across the sandbox (the ~5 portal files, items, projectiles, enemies, the
//! gravity mechanic, presentation) keeps resolving.
//!
//! The gravity *mechanic* layered on top (`GravityFlipSwitch`, the room-reset
//! reset, the `GravityPlugin`, and the zone / switch visuals) stays sandbox-side
//! in `crate::mechanics::gravity` because it depends on sandbox content
//! (audio / features / app schedule / presentation); it consumes the moved core
//! types through this facade.
pub use ambition_platformer_runtime::gravity::*;
