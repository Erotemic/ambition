//! Actor spawn/surface state and shared movement integration for brain-driven actors.
//!
//! Grounded, aerial, and adhesive actors all integrate through `ae::step_motion`.

use super::*;

mod integration;
pub use integration::ContactAttack;
pub(crate) use integration::ActorMutIntegrationExt;
#[cfg(test)]
pub(crate) use integration::SeedActorIntegrationTestExt;



