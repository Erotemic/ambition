//! THE GOBLIN BAND. Five sandbox placements (`annex_goblin_a/b`,
//! `pg_goblin_a/b/c`) that have been wearing the `medium_striker`
//! ARCHETYPE — a whole body, borrowed for its fighting style.
//!
//! it NAMES its policy rather than carrying one, which is the
//! Group-B/Group-C split arriving: the archetype's controller half is now
//! `autonomous_profiles: { "medium_striker": .. }` in the catalog, and any
//! number of creatures may point at it while keeping their own bodies. A
//! lab raider and a skitter are the next two.
//!
//! the key is PROVIDER-NAMESPACED on assembly, so the reference is
//! `ambition::medium_striker` rather than the local name — two games may
//! both author a "medium_striker" and neither wins.

use ambition_characters::actor::{CharacterLocomotion, ContactDamage};
use ambition_characters::brain::MoveStyleSpec;
use ambition_platformer2d::character::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(CharacterLocomotion {
            run_speed: 170.0,
            move_style: MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_contact_damage(ContactDamage {
            strength: 0.70,
            amount: 1,
        })
        // `BrainProfileRef` resolves it against this definition's own provider.
        .with_autonomous_profile_named("medium_striker")
        // AND ITS OWN MOVES. Every
        // seated fighter whose character says nothing takes
        // `smash_fighter_kit()` — one generic swipe — and that floor's
        // goal is DELETION, one adopter at a time. The goblin is the
        // third character in the game to state a table and the first
        // ENEMY to.
        .with_moveset(crate::goblin_moveset::goblin_moveset());
    definition.vitals.max_health = Some(5);
    definition
}
