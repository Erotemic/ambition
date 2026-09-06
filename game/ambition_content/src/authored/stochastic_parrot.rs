//! The aerial dive-bomber. Its `is_aerial` does NOT come across as a
//! character field: the catalog row already says `body_kind: Floating`,
//! and construction reads gravity-freedom from there — one authority for
//! "does this creature fly", which the archetype row was duplicating.
//!
//! `mass: 0.5` is not carried either. Mass weights a mount+rider centre
//! of gravity (ADR 0020) and a parrot is neither, so it was inert on the
//! row; the first mountable character to migrate is the one that needs a
//! home for it.

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
            run_speed: 240.0,
            move_style: MoveStyleSpec::Float,
            // This was inferred from `body_kind: Floating` in its catalog row — a
            // presentation/footprint fact that was doubling as locomotion authority. The fold is
            // deleted; a bird states its own flight.
            baseline_free_flight: Some(true),
            ..Default::default()
        })
        .with_contact_damage(ContactDamage {
            strength: 0.55,
            amount: 1,
        })
        .with_autonomous_profile(BrainProfile {
            // Stalks to an altitude above its target, dives, pecks on
            // contact, peels off to recover.
            template: CharacterBrainTemplate::Aerial,
            aggro_radius: 620.0,
            attack_range: 60.0,
            ..Default::default()
        })
        // AND ITS CATALOG ROW STILL NAMES `parrot_lively`, WHICH
        // DISAGREES WITH THIS. That
        // preset says `aggro_radius: 120.0` and `attack_range: 0.0`
        // against this profile's 620 and 60 — one bird, two authorities,
        // different answers. THIS one wins (`resolve_npc_brain` ranks a
        // definition's own profile above the row's `default_brain`, and
        // the enemy road builds character-first), so the preset is dead
        // weight stating wrong numbers rather than a live conflict.
        //
        // it cannot be deleted yet, and the blocker is a SCHEMA one:
        // `default_brain` is a required `String`, so the row has to name
        // SOME preset — and `parrot_lively` has exactly one namer, this
        // bird. A character whose definition states its policy should not
        // have to name a vocabulary it does not use; making that field
        // optional is what lets the preset go.
        .with_action_set(ambition_characters::brain::ActionSet {
            melee: Some(MeleeActionSpec::Bite(
                ambition_characters::brain::BiteSpec {
                    windup_s: 0.16,
                    active_s: 0.10,
                    recover_s: 0.28,
                    damage: 1,
                    reach_px: 48.0,
                },
            )),
            move_style: MoveStyleSpec::Float,
            ..Default::default()
        });
    definition.vitals.max_health = Some(3);
    definition
}
