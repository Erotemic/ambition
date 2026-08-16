//! **ALICE'S BODY, now that alice has answers.**
//!
//! ⭐ alice leaves [`super::hall_humanoids`] under that file's own rule: *"one
//! file for the rest... If one of them grows a moveset or a distinct build, it
//! earns its own file that day."* This is that day — the third and fourth time
//! that rule has fired this week, after Emmy and Oiler. The walk is the same
//! 210 px/s humanoid amble and the health the same ordinary-NPC 4: nothing about
//! standing in the Hall changed, and a retune riding a migration's commit is
//! exactly what that rule exists to prevent.
//!
//! ⚠ **the MOVESET is what is new**, and it reaches the fighter through
//! `with_moveset` — see [`crate::alice_moveset`] for the table and the
//! sender/receiver split it is built on. Measured before it was written, alice
//! was 0/16 on the smash grid: no table, no action set, and no unarmed floor
//! reaching the body either, so every press was silence.

use ambition_characters::actor::CharacterLocomotion;
use ambition_characters::brain::MoveStyleSpec;
use ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(CharacterLocomotion {
            run_speed: super::hall_humanoids::HUMANOID_RUN_SPEED,
            move_style: MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_moveset(crate::alice_moveset::alice_moveset());
    // Jon 2026-08-13: ordinary humanoid/NPC baseline. Unchanged by the kit — a
    // person with a repertoire is not a bigger body.
    definition.vitals.max_health = Some(4);
    definition
}
