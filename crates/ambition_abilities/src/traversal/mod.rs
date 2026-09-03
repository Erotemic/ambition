//! Traversal abilities a held item FIRES: blink, dive, grapple, mark/recall.
//!
//! ⛔ Possession, teleport, trapdoor and flyline are NOT here. They share the
//! kernel's `abilities/traversal/` directory and nothing else: they are
//! runtime-registered control authority, not wielded verbs. The crate header
//! carries the measurement.

pub mod blink;
pub mod dive;
pub mod grapple;
pub mod mark_recall;
