//! The Officer — easter-egg brawler humanoid.
//!
//! A state trooper who wandered into a fighting game: out of uniform from the
//! neck down, entirely in character from the neck up. He is the Pugnacious
//! Polygon's archetype wearing a different person — unarmed, close-range, and
//! on the same skeleton and clip vocabulary.
//!
//! Nothing may depend on him being selectable. He is meant to be found.

use ambition_platformer2d::character::CharacterDefinition;

pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(ambition_characters::actor::CharacterLocomotion {
            // The brawler archetype's own number, for the reason his moveset is
            // the brawler's: he is that archetype.
            run_speed: 230.0,
            move_style: ambition_characters::brain::MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_moveset(crate::officer_moveset::officer_moveset());
    definition.vitals.max_health = Some(6);
    definition
}
