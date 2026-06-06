//! Proto-runtime module for reusable platformer systems.
//!
//! This is intentionally still inside `ambition_sandbox` while the plugin
//! refactor is carving stable same-crate boundaries. Code in this subtree
//! should be written as if it were already a future `platformer_runtime` crate:
//! no imports from Ambition content, presentation, app assembly, or devtool
//! modules.

pub mod collision;
pub mod lifecycle;
pub mod orientation;
pub mod prelude;
pub mod schedule;
pub mod transit;
