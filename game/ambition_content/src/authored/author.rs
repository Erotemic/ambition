//! The Author — easter-egg sword humanoid, armed with a pen.
//!
//! The person writing the game, standing in it. He is the Pointed Polygon's
//! archetype wearing a different person: same skeleton, same clip vocabulary,
//! same reach — because the pen he carries occupies the arming sword's exact
//! axis and length, which is what let the whole swing library retarget onto him
//! untouched.
//!
//! Nothing may depend on him being selectable. He is meant to be found.

use ambition_platformer2d::character::CharacterDefinition;

pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(ambition_characters::actor::CharacterLocomotion {
            // The sword archetype's own number. He is that archetype; a
            // different walk speed here would be a balance decision nobody made.
            run_speed: 220.0,
            move_style: ambition_characters::brain::MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_moveset(crate::author_moveset::author_moveset());
    definition.vitals.max_health = Some(5);
    definition
}
