//! Compatibility facade over extracted platformer-runtime surfaces plus monolith-owned orientation.
//!
//! TODO(compat-remove): migrate `platformer_runtime::{lifecycle,schedule,math,transit,body,collision}`
//! callers to their owning crates, then delete those re-export modules and leave no generic runtime
//! facade inside the actor monolith.

pub use ambition_platformer2d_shared_tangle::{gravity, lifecycle, math, schedule, transit};

pub mod collision;
pub mod orientation;
pub mod prelude;
