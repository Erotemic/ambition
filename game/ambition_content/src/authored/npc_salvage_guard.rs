//! Salvage Guard definition for the intro raid corridor.
//!
//! Its single-adopter controller policy stays inline rather than becoming a
//! shared autonomous profile. Respawn behavior remains placement policy.

use ambition_characters::actor::{CharacterLocomotion, ContactDamage};
use ambition_characters::brain::{
    BrainProfile, CharacterBrainTemplate, MeleeActionSpec, MoveStyleSpec, SwipeSpec,
};
use ambition_platformer2d::character::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(CharacterLocomotion {
            run_speed: 225.0,
            move_style: MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_contact_damage(ContactDamage {
            strength: 0.80,
            amount: 1,
        })
        .with_action_set(ambition_characters::brain::ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec {
                windup_s: 0.28,
                active_s: 0.08,
                recover_s: 0.32,
                damage: 1,
                reach_px: 28.0,
            })),
            ranged: None,
            special: None,
            move_style: MoveStyleSpec::Walk,
        })
        .with_autonomous_profile(BrainProfile {
            template: CharacterBrainTemplate::Smash,
            // 900 px is LONG — it is a corridor, and the guard is
            // meant to notice you from the far end of it. Carried across
            // unchanged; a retune is a separate, visible decision.
            aggro_radius: 900.0,
            attack_range: 150.0,
            patrol_effort: 0.5778,
            chase_effort: 1.0,
            ..Default::default()
        });
    definition.vitals.max_health = Some(4);
    definition
}
