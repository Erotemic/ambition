//! **The Hall's AI Slop, as a placed enemy.** One spawn in the sandbox,
//! one archetype row, and the same creature already standing on a Hall
//! pedestal — which is the ontology this campaign is about: one
//! character, two contexts.
//!
//! ⚠ its catalog row's `default_brain` is `melee_brute_striker`, and that
//! is NOT what this authors. The catalog default is what a PEACEFUL Hall
//! NPC of this character does; the profile below is what the placed enemy
//! does, and they are allowed to differ because the first is a catalog
//! fact and the second is this character's own default policy.

use ambition_characters::actor::{CharacterLocomotion, ContactDamage};
use ambition_characters::brain::{BrainProfile, CharacterBrainTemplate, MoveStyleSpec};
use ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    {
        let mut definition = definition
            .with_locomotion(CharacterLocomotion {
                run_speed: 42.0,
                move_style: MoveStyleSpec::Walk,
                ..Default::default()
            })
            .with_contact_damage(ContactDamage {
                strength: 0.5,
                amount: 1,
            })
            .with_autonomous_profile(BrainProfile {
                // Walks forward, reverses at walls, notices nobody. Its only
                // offense is the body it walks into you with.
                template: CharacterBrainTemplate::Wanderer,
                aggro_radius: 0.0,
                attack_range: 0.0,
                ..Default::default()
            });
        definition.vitals.max_health = Some(1);
        definition
    }
}
