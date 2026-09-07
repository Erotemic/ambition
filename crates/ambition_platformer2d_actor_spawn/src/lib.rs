//! Actor spawn/construction capability extracted from the platformer actor monolith.
//!
//! This crate owns the construction primitives that turn prepared character and
//! authored spawn facts into complete simulated bodies. It deliberately does not
//! depend on `ambition_platformer2d_actor_monolith`; callers compose it from above.

pub mod actor_bundles;
pub mod actor_spawn;
pub mod character_body;

pub use actor_spawn::*;
pub use character_body::{grant_prepared_character_body, KitOwnership, ProjectedCharacterKit};
