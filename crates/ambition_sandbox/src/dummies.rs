//! Bevy sandbox compatibility wrapper for engine-owned dummy logic.
//!
//! Dummies used to live in this crate. They now live in `ambition_engine`
//! because health, stun, knockback, death, and respawn are simulation rules.
//! Keeping this tiny re-export preserves the existing sandbox module imports.

pub use ambition_engine::{spawn_dummies, Dummy, DummyKind};
