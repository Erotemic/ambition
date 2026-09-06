//! THE PATENT CLERK, read back off its own row. Its
//! `gameplay_description` says *"a high-mastery heavyweight controller …
//! turns careful observation into unusually strong parries and
//! finishers"* — heavyweight, controller, finishers — and those three
//! words are the table. See the module doc; the design was already
//! written down and nobody had read it back.

use ambition_platformer2d::character::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        // AC5: the last fact standing between him and his own body. "A
        // high-mastery HEAVYWEIGHT controller" is his own description, so he
        // walks heavy. His `patrol_peaceful` policy ambles at its own absolute
        // 28 px/s either way; this is what the body could do when driven.
        .with_locomotion(ambition_characters::actor::CharacterLocomotion {
            run_speed: 195.0,
            move_style: ambition_characters::brain::MoveStyleSpec::WalkHeavy,
            ..Default::default()
        })
        .with_moveset(crate::patent_clerk_moveset::patent_clerk_moveset());
    // his — *"these are ordinary tuning values that can be changed later if they
    // feel wrong in play … do not retain fallback health or incomplete body
    // definitions because we are waiting for balance decisions."* Six rather
    // than the humanoid four because his own description is *"a high-mastery
    // heavyweight controller"*, and heavyweight is the word the moveset above
    // was built from.
    definition.vitals.max_health = Some(6);
    definition
}
