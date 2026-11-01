//! Projectile combat execution above the reusable projectile model.
//!
//! This crate owns attack clashes, swept target contacts,
//! interception, charging, and hit delivery. The projectile model remains in `ambition_projectiles`.

mod clash;
pub use clash::clash_arbitration;
pub(crate) mod feature_contact;
pub mod intercept;

pub mod systems;
#[cfg(test)]
mod module_entity_bolt_damage_tests;

pub use systems::{charge_projectile_input, step_projectiles, ProjectileStepSet};
