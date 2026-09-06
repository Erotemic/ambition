//! BOB'S BODY, now that bob has answers.
//!
//! bob leaves [`super::hall_humanoids`] under that file's own rule: *"one file for the rest...
//! If one of them grows a moveset or a distinct build, it earns its own file that day."* This
//! is that day — the third and fourth time that rule has fired this week, after Emmy and Oiler.

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
        .with_moveset(crate::bob_moveset::bob_moveset());
    // person with a repertoire is not a bigger body.
    definition.vitals.max_health = Some(4);
    definition
}
