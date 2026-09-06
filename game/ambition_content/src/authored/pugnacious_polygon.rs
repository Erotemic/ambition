//! Pugnacious Polygon — brawler archetype.
//!
//! The unarmed companion to the sword reference rig. It intentionally shares the
//! same broad safe-pose vocabulary while using larger fists and close-range body
//! mechanics so future humanoid rigs have both armed and unarmed references.

use ambition_platformer2d::character::CharacterDefinition;

pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(ambition_characters::actor::CharacterLocomotion {
            run_speed: 230.0,
            move_style: ambition_characters::brain::MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_moveset(crate::pugnacious_polygon_moveset::pugnacious_polygon_moveset());
    definition.vitals.max_health = Some(6);
    definition
}
