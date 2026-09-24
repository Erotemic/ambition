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
//! A grounded hybrid: `is_aerial: Some(false)` with `can_fly: true`. It
//! prefers to fight on the ground and flies only to cross a long gap. Reading
//! `can_fly` as "aerial" would perch it permanently.

use ambition_characters::actor::{CharacterLocomotion, ContactDamage};
use ambition_characters::brain::{
    BrainProfile, CharacterBrainTemplate, MeleeActionSpec, MoveStyleSpec, SwipeSpec,
};
use ambition_platformer2d::character::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which also
/// makes this character buildable; there is no second list.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        // The duel arena's fighters carry a `grudge_against`, so they are provoked,
        // not spawned hostile, and a provoked creature rebuilds its mind from this
        // reference.
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
            // The four body-enforced capabilities the row authored. A possessing
            // player inherits exactly these, which is why they are body facts, not
            // brain facts.
            blink: true,
            fly: true,
            fly_toggle: true,
            shield: true,
            dash: true,
            // Nothing platform-fighter-specific. This kit belongs to the creature
            // wherever it stands: no double jump, fast fall, dodge or ledge grab.
            // Giving the PCA a ledge grab would be a separate decision about the
            // creature.
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
        // Jon: *"PCA needs to shoot a glider."* This is that request; the
        // attribution keeps a polish pass from swapping the ranged action without
        // knowing the maintainer asked for it.
        //
        // It is the ranged action, not the side-B, on purpose. The automaton's
        // `glider_launch` special displaces it instead of spawning a second glider
        // (two spawners would be two authorities on one pattern), so the ranged
        // button shoots the glider. If the request meant the special, raise it
        // here.
        //
        // The glider, a cellular-automaton spaceship, is the zoning tool. The
        // projectile is a functional `Rock`; the authored visual id below selects
        // the Conway glider through the content-owned projectile catalog, not the
        // owner's id string.
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
            // Not the pulse. The moveset's verb map already binds
            // `special → cellular_pulse`; putting it here too would take the slot the
            // shield uses, and the PCA's reactive block would silently stop.
            special: None,
            move_style: MoveStyleSpec::Walk,
        });
    // Its moves are content, not code (fast-iteration I2, step 5): the table is
    // `assets/data/movesets/cellular_automaton.ron`, declared in `pack.ron`,
    // validated by the `moveset` schema and applied in
    // `crate::character_catalog::authored_intrinsics`, the one seam every
    // buildable character passes through. The Rust table is only the exporter's
    // source and the parity oracle's subject; the host reads neither, so editing
    // it changes nothing until it is re-exported.
    definition.vitals.max_health = Some(60);
    definition
}
