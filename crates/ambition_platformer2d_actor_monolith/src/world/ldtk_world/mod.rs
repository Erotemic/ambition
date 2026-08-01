//! Compatibility facade for the LDtk backend.
//!
//! W3 moved the backend implementation to `ambition_platformer2d_ldtk`; gameplay-core
//! keeps this path while app/content callers repoint to the owning crate.

pub use ambition_platformer2d_ldtk::*;
