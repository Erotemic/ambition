//! Projectile combat execution above the reusable projectile model.
//!
//! This crate owns allegiance, swept target contacts, interception, charging,
//! and hit delivery. The projectile model remains in `ambition_projectiles`.

mod allegiance;
pub(crate) mod feature_contact;
pub mod intercept;
pub use allegiance::{stamp_new_projectile_allegiance, ProjectileAllegiance};

pub mod systems;
#[cfg(test)]
mod module_entity_bolt_damage_tests;

pub use systems::{charge_projectile_input, step_projectiles, ProjectileStepSet};


mod snapshot_impls;
mod rollback_registration;
pub use rollback_registration::register_rollback_state;
