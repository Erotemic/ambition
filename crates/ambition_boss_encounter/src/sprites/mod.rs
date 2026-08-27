//! Compatibility facade for boss sprite-sheet types.
//!
//! TODO(compat-remove): migrate remaining `crate::sprites` callers to
//! `ambition_sprite_sheet::boss`, then delete this module.

pub use ambition_sprite_sheet::boss::*;

#[cfg(test)]
use bevy::prelude::*;

#[cfg(test)]
mod tests;
