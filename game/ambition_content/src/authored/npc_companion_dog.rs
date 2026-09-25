//! A friendly dog with a grounded body and a small movement kit.

use ambition_characters::actor::CharacterLocomotion;
use ambition_characters::brain::MoveStyleSpec;
use ambition_platformer2d::character::CharacterDefinition;
use ambition_platformer2d_core::AbilitySet;

pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(CharacterLocomotion {
            run_speed: 120.0,
            move_style: MoveStyleSpec::Walk,
            baseline_free_flight: Some(false),
            ..Default::default()
        })
        .with_abilities(AbilitySet {
            move_horizontal: true,
            jump: true,
            ..AbilitySet::NONE
        });
    definition.vitals.max_health = Some(4);
    definition
}
