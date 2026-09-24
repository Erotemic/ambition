//! The boss pattern's thinking: tick, control flow, validator, seeds and
//! profile.
//!
//! `ambition_characters` owns what a character is: `brain/boss_pattern/mod.rs`
//! has the pattern vocabulary, the cfg and the state (everything `Brain`'s
//! snapshot encoder reads and `BrainSnapshot` names). This crate owns how a
//! boss thinks.
//!
//! `content_schema.rs` reads `ambition_content_pack::PreparedContentPack`, so
//! this crate has an optional `content_pack` dependency behind a feature of
//! the same name (as `ambition_characters` and `ambition_combat` do): a game
//! that never validates its content must not link a compiler.

/// The `boss_seed_library` and `boss_validator_bands` authored-content schemas
/// this capability owns. Behind `content_pack`: a game that never validates its
/// content must not link a compiler.
#[cfg(feature = "content_pack")]
pub mod content_schema;
pub mod control_flow;
pub mod profile;
pub mod seeds;
pub mod tick;
pub mod validator;

pub use tick::{tick_boss_pattern, tick_boss_pattern_via_state_machine};

// Every submodule's tests need a `mod` line in its parent. A file that nothing
// declares is not a build error; its tests simply never run.
#[cfg(test)]
mod tests;
