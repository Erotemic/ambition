//! The splitter: four hit points, slower and tankier, and it becomes two
//! on death.

use ambition_characters::actor::CharacterDeathTraits;
use ambition_characters::actor::{CharacterLocomotion, ContactDamage};
use ambition_characters::brain::{
    BrainProfile, CharacterBrainTemplate, MeleeActionSpec, MoveStyleSpec, SwipeSpec,
};
use ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_death_traits(CharacterDeathTraits {
            divides_on_death: true,
            ..Default::default()
        })
        .with_locomotion(CharacterLocomotion {
            run_speed: 130.0,
            move_style: MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_contact_damage(ContactDamage {
            strength: 0.70,
            amount: 1,
        })
        .with_autonomous_profile(BrainProfile {
            template: CharacterBrainTemplate::Smash,
            aggro_radius: 380.0,
            attack_range: 55.0,
            smash_hit_band: 34.0,
            ..Default::default()
        })
        .with_action_set(ambition_characters::brain::ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec {
                windup_s: 0.30,
                active_s: 0.10,
                recover_s: 0.34,
                damage: 1,
                reach_px: 30.0,
            })),
            move_style: MoveStyleSpec::Walk,
            ..Default::default()
        });
    definition.vitals.max_health = Some(4);
    definition
}
