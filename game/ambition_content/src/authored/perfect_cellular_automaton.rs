//! ```text
//! body        health, run speed, gait, contact damage, the swipe
//! controller  the Smash policy: aggro 460, commit at 60, hit band 30
//! placement   respawn, which the LDtk spawn already carries
//! ```
//!
//! A creature that states its own facts needs no matcher.
//!
//! ```text
//! body        60 HP, 168 run speed, the swipe, the glider, the pulse,
//!             and the four capabilities (blink / fly / shield / dash)
//! controller  the Smash policy: notice at 540, commit at 150, duelist
//! placement   respawn, which the placement carries
//! ```
//!
//! GROUNDED HYBRID, and the row said so in two fields that read as a
//! contradiction: `is_aerial: Some(false)` beside `can_fly: true`. It
//! prefers to fight on the ground and takes to the air only to cover a
//! long gap. Reading `can_fly` as "aerial" would perch it permanently.

use ambition_characters::actor::{CharacterLocomotion, ContactDamage};
use ambition_characters::brain::{
    BrainProfile, CharacterBrainTemplate, MeleeActionSpec, MoveStyleSpec, SwipeSpec,
};
use ambition_platformer2d::character::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        // The duel arena's fighters carry a `grudge_against`, so they are PROVOKED rather than
        // spawned hostile — and a provoked creature rebuilds its mind from this reference.
        .with_provoked_profile_named("cellular_duelist")
        .with_locomotion(CharacterLocomotion {
            run_speed: 168.0,
            move_style: MoveStyleSpec::Walk,
            baseline_free_flight: Some(false),
            ..Default::default()
        })
        .with_contact_damage(ContactDamage {
            strength: 0.75,
            amount: 1,
        })
        .with_abilities(ambition_platformer2d_core::AbilitySet {
            attack: true,
            // The four body-enforced capabilities the row authored. A
            // possessing player inherits exactly these, which is the
            // property that made them body facts rather than brain ones.
            blink: true,
            fly: true,
            fly_toggle: true,
            shield: true,
            dash: true,
            // AND NOTHING ABOUT A PLATFORM FIGHTER. This kit is a duel
            // arena's and describes the CREATURE: what it may do wherever it
            // stands. It has no double jump, no fast fall, no dodge and no ledge
            // grab, and on the smash grid it had none of them either — because
            // `fighter_abilities` was a lone MASK and a mask can only remove.
            // The one fighter on that roster whose sheet has ten ledge rows
            // drawn for it was the one who could not use them.
            //
            // An earlier pass authored `ledge_grab: true` here and it is removed with the reason
            // that justified it — if the PCA should grab ledges in its OWN room as well, that is a
            // statement about the creature and a different decision.
            ..ambition_platformer2d_core::AbilitySet::basic()
        })
        .with_autonomous_profile(BrainProfile {
            template: CharacterBrainTemplate::Smash,
            aggro_radius: 540.0,
            attack_range: 150.0,
            patrol_effort: 0.5714,
            chase_effort: 1.0,
            smash_sprint_to_close: true,
            // Footsies and spacing rather than close-and-camp.
            smash_duelist: true,
            ..Default::default()
        })
        // ⭐⭐ JON'S DESIGN, 2026-09-05: *"PCA needs to shoot a glider."* ⇒ THIS
        // IS THAT, and the attribution lives here because the request was
        // SATISFIED and UNRECORDED — a polish pass could have swapped this
        // ranged action for something else without ever learning that the
        // maintainer asked for it by name.
        //
        // ⚠ AND IT IS THE RANGED ACTION, NOT THE SIDE-B, DELIBERATELY. The
        // automaton's `glider_launch` special DISPLACES it rather than spawning a
        // second one — *"a second spawner here would be two authorities on one
        // pattern"* — so the button that shoots a glider is the ranged one. If
        // the intent was the SPECIAL, this comment is where somebody should
        // notice the difference and say so.
        //
        // the glider — a cellular-automaton spaceship as the
        // zoning tool. The projectile is a functional `Rock`; the Conway
        // glider is chosen by the authored visual id below, which the
        // render layer resolves through the content-owned projectile
        // catalog rather than from the owner's id string.
        .with_ranged_vfx("glider")
        .with_action_set(ambition_characters::brain::ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec {
                windup_s: 0.24,
                active_s: 0.08,
                recover_s: 0.30,
                damage: 1,
                reach_px: 30.0,
            })),
            ranged: Some(ambition_characters::brain::RangedActionSpec::new(
                ambition_characters::brain::action_set::RangedStyle::Rock,
                300.0,
                1,
            )),
            // NOT the pulse. The MOVESET's verb map already binds
            // `special → cellular_pulse`; putting it in this slot too
            // takes the slot the SHIELD uses, and the PCA's reactive
            // block silently stops happening. The archetype row kept
            // them apart by construction — `signature_move` was a
            // different field from `can_shield` — and authoring both on
            // one character is where they can collide.
            special: None,
            move_style: MoveStyleSpec::Walk,
        })
        .with_moveset(crate::cellular_automaton_moveset::cellular_pulse_moveset());
    definition.vitals.max_health = Some(60);
    definition
}
