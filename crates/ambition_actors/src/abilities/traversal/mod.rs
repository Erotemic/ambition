//! Traversal abilities: blink, dive, grapple, possession, mark/recall.
//!
//! Each submodule is a self-contained player ability/weapon mechanic tied
//! to a `crate::items::Item`. Moved here from the crate root in Stage 17
//! (`crate::abilities` layer) — pure relocation, no behavior change.

pub mod blink;
pub mod dive;
pub mod grapple;
pub mod mark_recall;
pub mod possession;
