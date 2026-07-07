//! Compatibility facade for authored placement records.
//!
//! W3 moved the owner to `ambition_world::placements`; gameplay-core keeps this
//! module only while the remaining sim call sites are repointed.

pub use ambition_world::placements::*;
