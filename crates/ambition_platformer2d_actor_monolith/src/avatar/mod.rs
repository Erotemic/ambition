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
    InitialBodyPolicy, StartingCharacter, WornControlGateSet,
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

/// Install the avatar's player-input stage into `schedule`.
///
/// ⭐ THE CRATE NAMES ITS OWN SYSTEMS AND THE COMPOSITION NAMES THE PHASES. Every
/// set used here is `shared_tangle`'s published `PlayerInputSet`, which this crate
/// already depends on — which is the whole test for whether a block can be carved
/// at all: **an owner that cannot NAME its anchors cannot install itself**, and
/// excluding shared vocabulary from ATTRIBUTION is not the same as excluding it
/// from REACHABILITY.
///
/// ⛔ AND `close_death_interlude` IS DELIBERATELY NOT HERE, though a census listed
/// it beside these. It orders `.before(sandbox_reset::RoomReplayAdmission)`, a set
/// owned by `ambition_platformer2d_runtime`, and this crate does not depend on the
/// runtime — measured, not assumed. ⇒ That block is IRREDUCIBLE: only the
/// composition can name both sides, which is a composition doing its job rather
/// than a leak.
pub fn install_avatar_player_input(
    app: &mut bevy::prelude::App,
    schedule: impl bevy::ecs::schedule::ScheduleLabel + Clone,
) {
    use ambition_platformer2d_shared_tangle::schedule::PlayerInputSet;
    use bevy::prelude::IntoScheduleConfigs as _;

    // Derive the canonical persona before brain/effect consumers. Identity
    // changes refresh the full persona; live HostCode ability edits preserve
    // authored movement state.
    app.add_systems(
        schedule.clone(),
        apply_worn_character_gameplay.in_set(PlayerInputSet::Persona),
    );
    // Universal-brain seam: translate this frame's slot input into each
    // controlled body's `ActorControl` frame.
    app.add_systems(
        schedule.clone(),
        tick_controlled_brains
            .in_set(ControlledBrainTick)
            .in_set(PlayerInputSet::Brain),
    );
    // Body-mode policy (crouch / morph / climb) consumes FINISHED control and
    // its slot gestures, so it runs after both publication phases and before
    // `WorldPrepSet::Integrate` consumes the resize/mode change. ⭐ An autonomous
    // body's mode follows THIS tick's decision rather than the last one — the AI
    // frame did not exist yet when this sat in `PlayerInput`.
    //
    // ⚠ THE CHAIN IS THIS CRATE'S FACT, not the composition's: the body mode has
    // to settle before the poses that read it are synced, and neither system is
    // meaningful to a host that does not know what a body mode is.
    //
    // ⛔ THESE THREE COMMENTS CAME WITH THE CODE, and carrying them was not
    // tidiness. A carve DELETES the block it moves, and every reason written
    // beside that block goes with it unless somebody carries it — which is the
    // one edit shape where losing a specification produces no warning and no
    // failing test.
    app.add_systems(
        schedule,
        (
            crate::body_mode::update_body_mode,
            sync_player_actor_poses,
        )
            .chain()
            .in_set(PlayerInputSet::BodyMode),
    );
}
