//! Developer-facing tuning and inspection tools.
//!
//! This module is intentionally sandbox-side: it is allowed to depend on Bevy
//! reflection and inspector UI crates, while `ambition_engine_core` stays focused on
//! reusable Bevy-native movement/collision logic. The reflected resources here mirror
//! engine data so live tuning can happen without forcing Bevy dependencies into
//! the reusable crate.

mod developer_tools;
mod editable;
mod profiles;

pub use developer_tools::*;
pub use editable::*;
pub use profiles::*;

#[cfg(test)]
mod tests;
