//! **THE LAB RAIDER.** The intro raid corridor's other spawn, and the
//! SECOND creature to point at the shared `medium_striker` policy — which
//! is what makes that entry a role rather than the goblin's private
//! profile under a general name. The campaign named this one explicitly:
//! *"`npc_lab_raider` and `npc_salvage_guard` for the two intro
//! placements that are literally named that."*
//!
//! ⚠ its body facts are the goblin's, because the archetype it wore gave
//! both the same ones — 5 HP, 170 px/s, 0.70 contact. Carried across
//! unchanged; making a raider tougher than a goblin is a design decision
//! and it should be made where design decisions are visible, not
//! smuggled in by a migration.
//!
//! ⛔ no `action_set` here, exactly like the goblin: its kit comes from
//! its catalog row's `default_action_set: "striker_swipe"`. Authoring one
//! would be a SECOND declaration of the same fact, which is the muddle
//! this campaign removes rather than a completeness improvement.

use ambition_characters::actor::{CharacterLocomotion, ContactDamage};
use ambition_characters::brain::MoveStyleSpec;
use ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    {
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
            .with_autonomous_profile_named("medium_striker");
        definition.vitals.max_health = Some(5);
        definition
    }
}
