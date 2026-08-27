//! Traversal abilities: blink, dive, grapple, possession, mark/recall, teleport.

pub mod blink;
pub mod dive;
pub mod grapple;
pub mod mark_recall;
pub mod possession;
/// The authored teleport a MOVE fires, with the ledge assist a recovery needs.
/// Beside [`blink`] because they share `blink_target`, the one teleport rule.
pub mod teleport;
pub mod trapdoor;
