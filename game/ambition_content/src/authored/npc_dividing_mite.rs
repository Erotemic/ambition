//! The splitter: four hit points, slower and tankier, and it becomes two
//! Puppy Slugs on death.
//!
//! it names its own offspring now (AC5.4, closing engine half). A reusable platformer has
//! no business knowing what an Ambition mite becomes: any other game linking it inherited the
//! creature name, and changing the answer meant editing the engine.
//!
//!  the parent states it, the engine reads it, and the split path names no
//! creature at all.

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
            divides_into: Some("npc_puppy_slug".to_string()),
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
