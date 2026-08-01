//! Compatibility facade for boss sprite-sheet types.
//!
//! The canonical implementation moved down to `ambition_sprite_sheet::boss` so
//! render can animate bosses without depending on `ambition_platformer2d_actor_monolith`.

pub use ambition_sprite_sheet::boss::*;

#[cfg(test)]
use bevy::prelude::*;

#[cfg(test)]
mod tests;
