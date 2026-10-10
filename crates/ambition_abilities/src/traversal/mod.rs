//! Traversal abilities a held item FIRES: the grapple; and the blink rule
//! and the mark, which the blink, dive and mark/recall modules use through
//! their ports.
//!
//! Possession, teleport, trapdoor, and flyline are not here. They share the
//! kernel's `abilities/traversal/` directory, but they are runtime-registered
//! control authority, not wielded verbs. See the crate header.

pub mod blink;
pub mod grapple;
pub mod mark_recall;
