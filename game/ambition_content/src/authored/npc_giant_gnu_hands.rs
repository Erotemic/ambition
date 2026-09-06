//! Giant hand character reused for both left and right bodies.
//!
//! Collision size is derived by the spawn plan from the giant's extent rather
//! than duplicated in the character definition.

use ambition_characters::actor::CharacterLocomotion;
use ambition_characters::brain::{BrainProfile, CharacterBrainTemplate, MoveStyleSpec};
use ambition_platformer2d::character::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(CharacterLocomotion {
            // The limb router steers it every tick; the StandStill brain
            // below is inert and this speed is never asked for.
            run_speed: 0.0,
            move_style: MoveStyleSpec::WalkHeavy,
            ..Default::default()
        })
        .with_autonomous_profile(BrainProfile {
            template: CharacterBrainTemplate::StandStill,
            aggro_radius: 0.0,
            attack_range: 0.0,
            // A limb never seeks anybody: the rider's routed strikes
            // spawn the damaging hitboxes, and the hand is their vehicle.
            // `StandStill` + zero aggro is the whole of that as policy.
            ..Default::default()
        });
    definition.vitals.max_health = Some(42);
    // Lighter than the giant body, heavy enough to feel solid.
    definition.vitals.mass = Some(2.0);
    definition
}
