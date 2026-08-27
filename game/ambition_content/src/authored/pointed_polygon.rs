//! Pointed Polygon — sword archetype.
//!
//! This is intentionally an uncomplicated humanoid body. Its main authoring value
//! is that the sprite rig supplies safe reference poses that later humanoids can
//! copy before adding bespoke anatomy or personality.

use ambition_platformer2d::character::CharacterDefinition;

pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(ambition_characters::actor::CharacterLocomotion {
            run_speed: 220.0,
            move_style: ambition_characters::brain::MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_moveset(crate::pointed_polygon_moveset::pointed_polygon_moveset());
    definition.vitals.max_health = Some(5);
    definition
}
