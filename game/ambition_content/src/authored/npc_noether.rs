//! **EMMY NO-ETHER'S BODY, now that she has answers.**
//!
//! ⭐ she leaves [`super::hall_humanoids`] under that file's own rule: *"one file
//! for four... If one of them grows a moveset or a distinct build, it earns its
//! own file that day."* This is that day, and it is the second time that rule has
//! fired this week — Oiler went first.
//!
//! The walk stays the shared humanoid amble and the health stays the ordinary-NPC
//! baseline: nothing about standing in the Hall changed, and a retune riding a
//! migration's commit is exactly what that rule exists to prevent.
//!
//! ⚠ **the MOVESET is what is new**, and it reaches the fighter through
//! `with_moveset` — see [`crate::noether_moveset`] for the table and why it is
//! shaped the way it is. Her `default_action_set` also stops being `peaceful` in
//! the same change; the two halves answer different questions (*may this body
//! attack* versus *what the attack is*) and a fighter needs both.

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
        .with_moveset(crate::noether_moveset::noether_moveset());
    // Unchanged by the kit — a mathematician with a theorem is not a bigger body.
    definition.vitals.max_health = Some(4);
    definition
}
