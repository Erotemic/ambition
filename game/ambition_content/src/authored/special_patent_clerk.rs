//! **THE PATENT CLERK, read back off its own row.** Its
//! `gameplay_description` says *"a high-mastery heavyweight controller …
//! turns careful observation into unusually strong parries and
//! finishers"* — heavyweight, controller, finishers — and those three
//! words are the table. See the module doc; the design was already
//! written down and nobody had read it back.
//!
//! ⛔ MOVES ONLY, and the classification mechanic (MASS / ENERGY / MOVING
//! / AT REST, reference frames, the elevator recovery) is deliberately NOT
//! here: those are systems, not swings, and writing them as move windows
//! would be the wholesale-migration failure mode wearing a content commit.

use ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition =
        definition.with_moveset(crate::patent_clerk_moveset::patent_clerk_moveset());
    // ⭐ Jon 2026-08-13: **6**, and the reasoning that makes it safe to write is
    // his — *"these are ordinary tuning values that can be changed later if they
    // feel wrong in play … do not retain fallback health or incomplete body
    // definitions because we are waiting for balance decisions."* Six rather
    // than the humanoid four because his own description is *"a high-mastery
    // heavyweight controller"*, and heavyweight is the word the moveset above
    // was built from.
    definition.vitals.max_health = Some(6);
    definition
}
