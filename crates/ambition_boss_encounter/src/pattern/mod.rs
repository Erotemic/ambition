//! THE BOSS PATTERN'S THINKING, which is this domain's own business.
//!
//! ⭐⭐ D168: a floor crate owns what a character IS; the layer above owns how it
//! THINKS. `ambition_characters` kept `brain/boss_pattern/mod.rs` — the pattern
//! vocabulary, the cfg and the state, everything `Brain`'s snapshot encoder reads
//! and everything `BrainSnapshot` names — and the tick, the control flow, the
//! validator, the seeds and the profile came here.
//!
//! ⭐ THIS CRATE WAS ALREADY THE OWNER IN EVERYTHING BUT ADDRESS: `behavior.rs`
//! re-exported `boss_pattern::profile` and `anim.rs` reads `boss_pattern_state()`.
//! The row that priced this said the boss carve should have taken it.
//!
//! ⛔ `content_schema.rs` CAME TOO, and it is the one real cost of the move —
//! priced on D168 before it was taken. It reads
//! `ambition_content_pack::PreparedContentPack`, so this crate gains an OPTIONAL
//! `content_pack` dependency behind a feature of the same name. That is the shape
//! `ambition_characters` and `ambition_combat` already use, not a new pattern:
//! a game that never validates its content must not link a compiler.
//!
//! ⚠ IT HAD NO CHOICE. `content_schema` names `profile`, `seeds` and `validator`,
//! all three of which moved here — leaving it behind would have been a floor
//! crate reaching upward.

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
