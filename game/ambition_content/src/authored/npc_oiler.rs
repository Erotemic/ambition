//! OILER'S BODY, now that he has answers.
//!
//! he leaves [`super::hall_humanoids`] under that file's own rule: *"one file for four... If
//! one of them grows a moveset or a distinct build, it earns its own file that day."* This is
//! that day.
//!
//! the MOVESET is what is new, and it reaches the fighter through
//! `with_moveset` — see [`crate::oiler_moveset`] for the table and why it is
//! shaped the way it is. His `default_action_set` also stops being `peaceful` in
//! the same change; the two halves answer different questions (*may this body
//! attack* versus *what the attack is*) and a fighter needs both.

use ambition_characters::actor::CharacterLocomotion;
use ambition_characters::brain::MoveStyleSpec;
use ambition_platformer2d::character::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(CharacterLocomotion {
            run_speed: super::hall_humanoids::HUMANOID_RUN_SPEED,
            move_style: MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_moveset(crate::oiler_moveset::oiler_moveset());
    // mechanic with a wrench is not a bigger body.
    definition.vitals.max_health = Some(4);
    definition
}
