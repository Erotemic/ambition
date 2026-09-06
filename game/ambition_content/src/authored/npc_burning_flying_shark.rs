//! The burning flying shark — the first MOUNT to become a character.
//!
//! `mass: 6.0` is the other half — the pair rolls around a centre of gravity near the heavier
//! body — and it rides on `vitals`, which already carried mass.
//!
//! `is_aerial` and `default_size` do NOT come across: the catalog says
//! `body_kind: Floating`, and a named character sizes its body to its
//! authored sprite, which is the same silhouette the row was restating.

use ambition_characters::actor::CharacterDeathTraits;
use ambition_characters::actor::{CharacterLocomotion, ContactDamage};
use ambition_characters::brain::{
    BrainProfile, CharacterBrainTemplate, MeleeActionSpec, MoveStyleSpec,
};
use ambition_platformer2d::character::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(CharacterLocomotion {
            run_speed: 260.0,
            move_style: MoveStyleSpec::Float,
            // see the parrot: a flying MOUNT states its own flight
            // rather than inheriting it from a body-kind enum.
            baseline_free_flight: Some(true),
            ..Default::default()
        })
        .with_contact_damage(ContactDamage {
            strength: 1.10,
            amount: 2,
        })
        .with_death_traits(CharacterDeathTraits {
            // A riderless shark's fast charge, stopped dead by a wall,
            // detonates the shark.
            charge_crash_explodes: true,
            ..Default::default()
        })
        .with_mount(ambition_characters::actor::CharacterMount {
            class: Some("shark".to_string()),
            // It rides nothing, and it splashes nothing on death: a dead
            // shark drops its rider unharmed.
            ..Default::default()
        })
        .with_autonomous_profile(BrainProfile {
            // Dive at the target, crash, recover.
            template: CharacterBrainTemplate::ChargeCrash,
            aggro_radius: 1200.0,
            attack_range: 200.0,
            ..Default::default()
        })
        .with_action_set(ambition_characters::brain::ActionSet {
            melee: Some(MeleeActionSpec::Bite(
                ambition_characters::brain::BiteSpec {
                    windup_s: 0.18,
                    active_s: 0.10,
                    recover_s: 0.30,
                    damage: 2,
                    reach_px: 42.0,
                },
            )),
            move_style: MoveStyleSpec::Float,
            ..Default::default()
        });
    definition.vitals.max_health = Some(6);
    definition.vitals.mass = Some(6.0);
    definition
}
