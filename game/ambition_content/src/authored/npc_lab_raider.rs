//! Lab Raider definition for the intro raid corridor.
//!
//! It shares the `medium_striker` policy and its existing body facts with the
//! goblin. Its action set remains catalog-authored; do not duplicate it here.

use ambition_characters::actor::{CharacterLocomotion, ContactDamage};
use ambition_characters::brain::MoveStyleSpec;
use ambition_platformer2d::character::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(CharacterLocomotion {
            run_speed: 170.0,
            move_style: MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_contact_damage(ContactDamage {
            strength: 0.70,
            amount: 1,
        })
        .with_autonomous_profile_named("medium_striker");
    definition.vitals.max_health = Some(5);
    definition
}
