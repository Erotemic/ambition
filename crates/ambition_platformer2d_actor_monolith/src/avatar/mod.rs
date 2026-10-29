//! Home-body capabilities and policy.
//!
//! `PrimaryPlayer` identifies the exploration home body. `DrivingParticipant`
//! identifies a driven body. `ControlledSubject` is the primary observation projection.

mod kernel_death;
pub mod bundles;
pub mod character_execution;
pub mod trail;

pub use kernel_death::publish_kernel_reset_death;
pub use bundles::{PlayerIdentityBundle, PlayerSimulationBundle};
pub use character_execution::{
    apply_worn_character_gameplay, apply_worn_character_overlay, apply_worn_motion_model,
    motion_model_spec_for_character,
    movement_tuning_for_character, sustain_bubble_shield, sync_charge_projectile_capability,
    HomeBodyAbilities,
};

/// Build the scratch state of a home body at `at`, with its abilities. Each
/// seat's home body starts from this (`session::setup::spawn_home_body`).
pub fn home_body_scratch(
    at: ambition_platformer2d_core::Vec2,
    abilities: ambition_platformer2d_core::AbilitySet,
) -> ambition_platformer2d_core::BodyClusterScratch {
    ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(at, abilities)
}

#[cfg(test)]
mod tests;
