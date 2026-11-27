//! Deterministic snapshot values and registration for rollback backends.

pub mod hash;
pub mod snapshot;

#[cfg(feature = "ecs")]
mod registration;
