//! Compatibility facade for the LDtk backend.
//!
//! W3 moved the backend implementation to `ambition_ldtk_map`; gameplay-core
//! keeps this path while app/content callers repoint to the owning crate.

pub use ambition_ldtk_map::*;
