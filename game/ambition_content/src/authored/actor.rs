//! The Actor — easter-egg sword humanoid, carrying no sword.
//!
//! A performer who commits to the role completely: long lines, weight on the
//! back foot, every gesture held a beat too long because a gesture that is not
//! held did not read from the back row. She is the Pointed Polygon's archetype
//! wearing a different person.
//!
//! ⭐ AND THE REACH IS REAL BUT TEMPORARY. Her forward smash conjures a blade of
//! stage light for exactly the frames the role calls for it, authored as the
//! swing's own axis extended past her hand — so the archetype's frame data
//! retargets onto her for the same reason it retargets onto the Author's pen.
//! Outside those frames her hands are empty and short.
//!
//! ⚠ HER SPECIALS ARE STAGE MACHINERY WITH NO RULES YET. The trap door moves
//! her and the flyline lifts her; neither publishes a hit volume, because a
//! hole in the boards and a wire hurt nobody. What they COST is gameplay and it
//! is not written, so she borrows the archetype's specials until it is.
//!
//! Nothing may depend on her being selectable. She is meant to be found.

use ambition_characters::actor::definition::CharacterDefinition;

pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(ambition_characters::actor::CharacterLocomotion {
            // ⭐ SLOWER THAN THE ARCHETYPE, and this is the one number that is
            // hers. Her authored clips are the sword archetype's held longer —
            // 60ms against its own timings, with the lunge parked for two
            // frames — and a body that moved at its speed would arrive before
            // the pose it is committing to.
            run_speed: 204.0,
            move_style: ambition_characters::brain::MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_moveset(crate::actor_moveset::actor_moveset());
    // Medium, and one point over the Author's: the reach she conjures is worth
    // less than his pen because it is only there while she commits to it, and
    // she eats the recovery either way.
    definition.vitals.max_health = Some(6);
    definition
}
