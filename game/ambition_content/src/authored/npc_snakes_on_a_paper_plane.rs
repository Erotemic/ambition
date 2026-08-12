//! **The drifting swarms.** Mary-O flies them over her levels; they are
//! Ambition's characters, and this is where they say what they are.
//!
//! ⚠ they author `flies` even though their catalog rows say
//! `body_kind: Floating` — the catalog is not always THERE. A standalone
//! demo that borrows a character has no row for it, and a body that
//! reads its gravity-freedom from a row it cannot see falls out of the
//! sky. Stating it on the character is what makes the fact travel.

use ambition_characters::actor::{CharacterLocomotion, ContactDamage};
use ambition_characters::brain::{BrainProfile, CharacterBrainTemplate, MoveStyleSpec};
use ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let paper = id == "npc_snakes_on_a_paper_plane";
    let mut definition = definition
        .with_locomotion(CharacterLocomotion {
            run_speed: if paper { 58.0 } else { 38.0 },
            move_style: MoveStyleSpec::Float,
            baseline_free_flight: Some(true),
            ..Default::default()
        })
        .with_contact_damage(ContactDamage {
            strength: 0.5,
            amount: 1,
        })
        .with_autonomous_profile(BrainProfile {
            // It baseline_free_flight, it notices nobody, and running into it is the
            // entire threat.
            template: CharacterBrainTemplate::Aerial,
            aggro_radius: 0.0,
            attack_range: 0.0,
            ..Default::default()
        });
    definition.vitals.max_health = Some(if paper { 1 } else { 2 });
    definition
}
