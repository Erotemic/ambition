//! Compatibility facade for boss sprite-sheet types.
//!
//! TODO(compat-remove): migrate remaining `ambition_boss_encounter::sprites` callers to
//! `ambition_sprite_sheet::boss`, then delete this module.

pub use ambition_sprite_sheet::boss::*;

#[cfg(test)]
use bevy::prelude::*;

#[cfg(test)]
mod tests;
