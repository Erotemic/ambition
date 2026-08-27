//! THE IMMORTAL TRAINING DUMMY, and the arm above says why it is a
//! separate creature rather than a flag on the sandbag: `never_dies` is a
//! character trait, so "the same dummy, invincible in this room" is not a
//! thing the model can say. The combat-feel lab's two spawns are this.
//!
//! Dropping either changes what a lab dummy looks like under a hit, and a migration is the
//! wrong place to find that out.
//!
//! no contact damage: the row authored `body_contact_damage: false`
//! beside a `contact_strength`, which is the archetype format's way of
//! saying the numbers are inert. A character says it by not speaking.

use ambition_characters::actor::CharacterLocomotion;
use ambition_characters::brain::{BrainProfile, CharacterBrainTemplate, MoveStyleSpec};
use ambition_platformer2d::character::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .as_practice_target()
        .with_locomotion(CharacterLocomotion {
            run_speed: 155.0,
            move_style: MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_death_traits(ambition_characters::actor::CharacterDeathTraits {
            never_dies: true,
            ..Default::default()
        })
        .with_autonomous_profile(BrainProfile {
            template: CharacterBrainTemplate::StandStill,
            // Notices nobody and swings at nobody — the row's
            // `attack_range: 150.0` sat beside `melee: None`, exactly as
            // the finite sandbag's did.
            aggro_radius: 0.0,
            attack_range: 0.0,
            patrol_effort: 0.6774,
            chase_effort: 1.0,
            ..Default::default()
        });
    definition.vitals.max_health = Some(9999);
    definition
}
