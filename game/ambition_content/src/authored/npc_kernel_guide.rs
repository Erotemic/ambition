//! THE KERNEL GUIDE'S OWN IDENTITY — and deliberately no combat kit.
//!
//! Jon, W8 playtest, closing D56: *"Kernel Guide gets its own
//! `CharacterDefinition`. Character identity is not sprite identity. Kernel
//! Guide may temporarily borrow another presentation/sheet through the normal
//! presentation mechanism, but its gameplay/content identity remains Kernel
//! Guide. Do not invent a combat kit or capabilities merely to fill the
//! definition."*
//!
//! ⭐⭐ SO THE FILE IS SHORT ON PURPOSE, and the absences are the content. Every
//! other file in this directory arrived because a character grew a moveset; this
//! one arrives because a character grew an IDENTITY, which is a different reason
//! and is allowed to produce a smaller table. A `with_moveset` here would be
//! inventing a fighter out of a tutorial NPC to make the file look finished.
//!
//! ⛔ WHAT THE ROW ALREADY SAYS IS NOT REPEATED. `character_catalog.ron` states
//! its sheet, its manifest, its `sprite_tuning`, its tier, its
//! `default_brain: patrol_peaceful`, its `default_action_set: peaceful`, its
//! barks and its hall dialogue id — all of it still true and none of it this
//! file's to restate. What was missing was a registration at all: the guide was
//! built through the archetype road, so nothing prepared it as a character.

use ambition_characters::actor::CharacterLocomotion;
use ambition_characters::brain::MoveStyleSpec;
use ambition_platformer2d::character::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition.with_locomotion(CharacterLocomotion {
        // The hall walk it shares with every other humanoid who lives there.
        // Its `patrol_peaceful` policy ambles well under this either way; the
        // number is what the body could do if something drove it.
        run_speed: super::hall_humanoids::HUMANOID_RUN_SPEED,
        move_style: MoveStyleSpec::Walk,
        ..Default::default()
    });
    // The humanoid four, the same as Alice and the rest of the hall. ⛔ NOT a
    // balance decision waiting to be made: *"do not retain fallback health or
    // incomplete body definitions because we are waiting for balance
    // decisions"*. A tutorial NPC is a person, and a person has four.
    definition.vitals.max_health = Some(4);
    definition
}
