//! **Moving-platform presentation lived here, and no longer exists.**
//!
//! W3 moved the authored spec, runtime state and collision-world composition to
//! `ambition_platformer2d_world::platforms`. What stayed was the visual — a
//! `MovingPlatformVisual` component, a spawn called from inside the room
//! construction transaction, and a per-tick sync — because it named Bevy sprite
//! and lifecycle types.
//!
//! ⛔⛔ **that is deleted (2026-08-14), not moved field-for-field.** The visual is
//! now `ambition_render::rendering::moving_platforms`, a render family that
//! reconciles pictures from the authoritative `MovingPlatformSet` the way every
//! other room feature is drawn. The construction commit spawns no visual at all.
//!
//! ⭐ **the transaction was the thing blocking the carve, and joining the
//! existing reactive model dissolved it** rather than requiring a new
//! construction → presentation message: a family that only derives pictures
//! cannot split a transaction it does not participate in. See
//! `docs/planning/engine/kinematic-world-objects.md` K3.
//!
//! Import the world-owned types from their owner,
//! `ambition_platformer2d_world::platforms`.
