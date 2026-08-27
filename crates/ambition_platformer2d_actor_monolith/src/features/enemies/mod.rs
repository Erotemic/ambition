//! Actor spawn/surface state and shared movement integration for brain-driven actors.
//!
//! Grounded, aerial, and adhesive actors all integrate through `ae::step_motion`.

use super::*;

mod integration;
pub use integration::ContactAttack;



/// Shared suffix for persistent `_dead_until_rest` flags.
pub const ENEMY_DEAD_UNTIL_REST_SUFFIX: &str = "_dead_until_rest";
