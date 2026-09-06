//! The sandbag kamikaze mite: two hit points and a corpse that detonates.

use ambition_characters::actor::CharacterDeathTraits;
use ambition_characters::actor::{CharacterLocomotion, ContactDamage};
use ambition_characters::brain::{
    BrainProfile, CharacterBrainTemplate, MeleeActionSpec, MoveStyleSpec, SwipeSpec,
};
use ambition_platformer2d::character::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_death_traits(CharacterDeathTraits {
            explodes_on_death: true,
            ..Default::default()
        })
        .with_locomotion(CharacterLocomotion {
            run_speed: 245.0,
            move_style: MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_contact_damage(ContactDamage {
            strength: 0.60,
            amount: 1,
        })
        .with_autonomous_profile(BrainProfile {
            template: CharacterBrainTemplate::Smash,
            aggro_radius: 460.0,
            attack_range: 60.0,
            smash_hit_band: 30.0,
            ..Default::default()
        })
        .with_action_set(ambition_characters::brain::ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec {
                windup_s: 0.22,
                active_s: 0.08,
                recover_s: 0.30,
                damage: 1,
                reach_px: 26.0,
            })),
            move_style: MoveStyleSpec::Walk,
            ..Default::default()
        });
    definition.vitals.max_health = Some(2);
    definition
}
