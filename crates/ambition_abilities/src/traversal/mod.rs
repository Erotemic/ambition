//! Traversal abilities a held item FIRES: blink, dive, grapple, mark/recall.
//!
//! Possession, teleport, trapdoor, and flyline are not here. They share the
//! kernel's `abilities/traversal/` directory, but they are runtime-registered
//! control authority, not wielded verbs. See the crate header.

pub mod blink;
pub mod dive;
pub mod grapple;
pub mod mark_recall;
