//! Unified body kinematics facade.
//!
//! [`BodyKinematics`] (the single position / velocity / AABB-size / facing
//! component shared by the player, enemies/NPCs, and bosses) now lives in the
//! content-free `ambition_platformer_primitives::body` module (Stage 16 / S2),
//! which itself re-exports the foundation definition from
//! `ambition_engine_core`. This facade re-exports it so every
//! `crate::platformer_runtime::body::BodyKinematics` reference across the
//! sandbox keeps resolving unchanged. See the runtime module for the
//! query-conflict discipline.
pub use ambition_platformer_primitives::body::BodyKinematics;
