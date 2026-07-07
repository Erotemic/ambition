//! Compatibility facade for room debug labels.
//!
//! W3 moved the authored label vocabulary to `ambition_world`; gameplay-core
//! keeps this path until LDtk and debug-overlay call sites are repointed.

pub use ambition_world::debug_label::*;
