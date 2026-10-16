//! The Void Star: the enemy that no stomp can hurt.
//!
//! A small hole in the world in the shape of a star, which goes along the
//! ground on its points. It is all data: its row is the walker the AI Slop is,
//! and no rule in this crate makes its top safe, so a stomp on it is body
//! contact and a hit. A spark, a quasar or a shell kills it.
//!
//! The name is how `void*` is said.

/// The brain key a level placement carries beside `character_id`.
pub const VOID_STAR_BRAIN_KEY: &str = "mary_o_void_star";
/// The catalog id, and the sheet target.
pub const VOID_STAR_CHARACTER_ID: &str = "void_star";
