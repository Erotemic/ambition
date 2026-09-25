//! The giant's fist, reused for both left and right bodies.
//!
//! GNU-ton conducts them (`bosses::gnu_ton::conductor`): he poses them every
//! tick, so the brain and locomotion below are inert. The body is the fist
//! sheet's authored body at the giant's own scale.

use ambition_characters::actor::CharacterLocomotion;
use ambition_characters::brain::{BrainProfile, CharacterBrainTemplate, MoveStyleSpec};
use ambition_platformer2d::character::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_sheet("giant_gnu_fist")
        .with_sprite_authored_body(super::npc_giant_gnu::GNU_WORLD_PER_PIXEL)
        .with_locomotion(CharacterLocomotion {
            // The conductor poses it every tick; the StandStill brain below
            // is inert and this speed is never asked for.
            run_speed: 0.0,
            move_style: MoveStyleSpec::WalkHeavy,
            ..Default::default()
        })
        .with_autonomous_profile(BrainProfile {
            template: CharacterBrainTemplate::StandStill,
            aggro_radius: 0.0,
            attack_range: 0.0,
            // A fist never seeks anybody: the conductor spawns its hit
            // volumes, and the fist is their vehicle. `StandStill` + zero
            // aggro is the whole of that as policy.
            ..Default::default()
        });
    definition.vitals.max_health = Some(42);
    // Lighter than the giant body, heavy enough to feel solid.
    definition.vitals.mass = Some(2.0);
    definition
}
