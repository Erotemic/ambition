//! Home-avatar policy and integration that has not yet moved to its final owner.
//!
//! `PrimaryPlayer` identifies the exploration home body; `ControlledSubject` identifies current
//! control authority. Generic body simulation must not branch on this module or player identity.

pub mod body_integration;
pub mod bundles;
#[cfg(test)]
mod clone_probe_tests;
pub mod components;
pub mod events;
pub mod movement_components;
pub mod starting_character;
pub mod systems;
pub mod trail;

pub use body_integration::{
    advance_moving_platforms, integrate_home_body, surface_skidding, BodyReset,
    PlayerBodyFrameOutput,
};
pub use bundles::{PlayerIdentityBundle, PlayerSimulationBundle};
pub use starting_character::{
    apply_worn_character_gameplay, apply_worn_character_overlay, apply_worn_motion_model,
    gate_worn_player_control, motion_model_spec_for_character, motion_model_spec_for_character_id,
    movement_tuning_for_character, sustain_bubble_shield, sync_charge_projectile_capability,
    InitialBodyPolicy, PersonaBaseline, StartingCharacter, WornControlGateSet,
};
// Body-generic vocabulary stays under `crate::actor`; this module exports only home-avatar policy.
pub use events::PlayerHealRequested;
pub use systems::{
    apply_player_heal_requests, blank_scripted_control_frames, regen_player_mana,
    sync_player_actor_poses, tick_controlled_brains, ControlledBrainTick,
};

/// Build the primary home body's scratch state with its authored abilities.
pub fn primary_player_scratch(
    spawn: ambition_platformer2d_core::Vec2,
    abilities: ambition_platformer2d_core::AbilitySet,
) -> ambition_platformer2d_core::BodyClusterScratch {
    ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(spawn, abilities)
}
