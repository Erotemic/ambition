//! Compatibility facade over extracted platformer-runtime surfaces plus monolith-owned orientation.
//!
//! TODO(compat-remove): migrate `platformer_runtime::{lifecycle,schedule,math,transit,body,collision}`
//! callers to their owning crates, then delete those re-export modules and leave no generic runtime
//! facade inside the actor monolith.


pub mod collision;
pub mod orientation;
pub mod prelude;
