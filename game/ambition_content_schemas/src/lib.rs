//! The schemas Ambition's own pack names beside the engine's.
//!
//! The engine's schemas are a capability's rules for any game. These are
//! Ambition's own: tuning that only its code reads (`rules`). The game
//! compiles its pack against them (`pack::pack_schemas`), and the check
//! binary of this crate gives them to the content CLI, so one list is read by
//! both.

pub mod rules;

use ambition_content_pack::SchemaRegistration;

/// The capability that owns Ambition's own schemas.
pub const AMBITION_CAPABILITY: &str = "ambition";

/// Every schema Ambition's pack names beside the engine's.
pub fn ambition_schemas() -> Vec<SchemaRegistration> {
    vec![rules::ambition_rules_schema()]
}
