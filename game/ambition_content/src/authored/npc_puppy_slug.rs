//! **Ambient wildlife: a wall-and-ceiling crawler that hurts on touch.**
//!
//! The row this replaces carried a `default_size` of 48x22 and it is
//! deliberately NOT here: a named catalog character sizes its body to its
//! authored SPRITE, which is the same resolution a peaceful NPC of this
//! character already gets — one silhouette per creature, whichever road
//! spawns it.

use ambition_characters::actor::{CharacterLocomotion, ContactDamage};
use ambition_characters::brain::{BrainProfile, CharacterBrainTemplate, MoveStyleSpec};
use ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(CharacterLocomotion {
            run_speed: 80.0,
            move_style: MoveStyleSpec::Slither,
            // Crawlid-style: hugs the surface normal and probes ledges
            // so it never walks off a platform.
            surface_walker: true,
            // Knocked off its surface when hit — falls with gravity for
            // a moment, then re-attaches on landing.
            cling_breaks_on_hit: true,
            baseline_free_flight: Some(false),
        })
        .with_contact_damage(ContactDamage {
            strength: 0.55,
            amount: 1,
        })
        // ⭐⭐ **IT CRAWLS. THAT IS THE WHOLE VERB LIST.**
        //
        // Jon, 2026-08-12: *"If the slug does not have a double jump ability it
        // should not be able to double jump. The point of a slug is that it
        // shows that it is spawned happily even though it basically has no
        // moves."*
        //
        // ⛔ **it authored NO mask, and a body that authors none receives the
        // road's.** Both roads hand out a humanoid: exploration's
        // `ActorBody::locomotion_abilities()` grants jump + variable jump +
        // double jump, and a Smash seat INTERSECTS the stage's fighter mask
        // against the character's — which, against nothing, is the stage's
        // whole. Measured on the shipped creature: `npc_puppy_slug` and
        // `npc_carl_stargan` (who authors no body at all) came out identical,
        // `jump=true double_jump=true attack=true`. A slithering wall-crawler
        // double-jumped on the Smash stage.
        //
        // ⚠ **`attack: false` is not an oversight.** Its damage is CONTACT
        // damage, authored above — it hurts you by being touched, not by
        // swinging. Granting `attack` would give it a swipe nothing describes,
        // which is the "generic swipe" half of the same acceptance test.
        //
        // ⭐ and the seat is the POINT rather than a casualty: a body with one
        // verb must still seat, simulate and survive on a stage. That is the
        // compositional claim, and it is what `puppy_slug_forced_seat.rs`
        // measures.
        .with_abilities(ambition_platformer2d_core::AbilitySet {
            move_horizontal: true,
            ..ambition_platformer2d_core::AbilitySet::NONE
        })
        // The slug-only psychedelic pass, and the reason `dream_seed`
        // became a character fact.
        .with_dream_seed(0.271828)
        .with_autonomous_profile(BrainProfile {
            template: CharacterBrainTemplate::Wanderer,
            // Wildlife: it notices nobody and commits to nothing. The
            // Wanderer template ignores both, and authoring them as zero
            // says so rather than leaving a reader to guess.
            aggro_radius: 0.0,
            attack_range: 0.0,
            ..Default::default()
        });
    definition.vitals.max_health = Some(2);
    definition
}
