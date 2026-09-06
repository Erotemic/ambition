//! AI Slop definition shared by Hall and placed-enemy contexts.
//!
//! The catalog default controller describes the peaceful Hall use; the profile
//! here describes the placed enemy and may differ by context.

use ambition_characters::actor::{CharacterLocomotion, ContactDamage};
use ambition_characters::brain::{BrainProfile, CharacterBrainTemplate, MoveStyleSpec};
use ambition_platformer2d::character::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(CharacterLocomotion {
            run_speed: 42.0,
            move_style: MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_contact_damage(ContactDamage {
            strength: 0.5,
            amount: 1,
        })
        .with_autonomous_profile(BrainProfile {
            // Walks forward, reverses at walls, notices nobody. Its only
            // offense is the body it walks into you with.
            template: CharacterBrainTemplate::Wanderer,
            aggro_radius: 0.0,
            attack_range: 0.0,
            ..Default::default()
        });
    definition.vitals.max_health = Some(1);
    definition
}
