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
        .with_autonomous_profile_named("medium_striker");
        // AND ITS OWN MOVES. A seated fighter whose character said nothing fell
        // back to one generic swipe, and that floor's goal was DELETION, one
        // adopter at a time. The goblin was the third character to state a table
        // and the first ENEMY to; as of 2026-08-31 every id on the Smash roster
        // states one, so the fallback has no adopters left (see `select.rs`).
        // ⭐⭐ ITS MOVES ARE CONTENT NOW, NOT CODE (fast-iteration I2, step 5).
        // The table this line compiled in is `assets/data/movesets/goblin.ron`,
        // declared in `pack.ron`, validated by the `moveset` schema and applied
        // in `crate::character_catalog::authored_intrinsics` — the one seam
        // every buildable character passes through.
        // ⛔ The Rust table still exists as the EXPORTER's source and the parity
        // oracle's subject. The host reads NEITHER, so editing it changes nothing
        // until it is re-exported.
    definition.vitals.max_health = Some(5);
    definition
}
