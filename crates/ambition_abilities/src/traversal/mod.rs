//! Traversal abilities a held item FIRES: grapple and mark/recall, and the
//! blink rule (the blink and the dive are procedural modules that ask for a
//! transit).
//!
//! Possession, teleport, trapdoor, and flyline are not here. They share the
//! kernel's `abilities/traversal/` directory, but they are runtime-registered
//! control authority, not wielded verbs. See the crate header.

pub mod blink;
pub mod grapple;
pub mod mark_recall;
