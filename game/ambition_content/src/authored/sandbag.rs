//! THE PRACTICE TARGET. A body that exists to be hit: no aggro, no
//! strike back, excluded from the save file, and skipped by the path
//! assignment — all of which is what `practice_target` says in one word.
//!
//! The flag is the gate, so the comment described an intention nobody had implemented, and a
//! migration that believed the prose would have given the dummy a hitbox it never had.
//!
//! its `respawn: InPlace(0.85)` moves to the placement, where a respawn policy belongs (ADR 0022) —
//! and `sandbag_infinite` does NOT migrate with it: `never_dies` is a character trait, so the
//! immortal dummy is a different creature and needs its own registered character.

use ambition_characters::actor::CharacterLocomotion;
use ambition_characters::brain::{BrainProfile, CharacterBrainTemplate, MoveStyleSpec};
use ambition_platformer2d::character::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .as_practice_target()
        .with_locomotion(CharacterLocomotion {
            // It never walks anywhere — StandStill drives it — but the
            // row authored a speed and a gait, so the character does too.
            run_speed: 155.0,
            move_style: MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_autonomous_profile(BrainProfile {
            template: CharacterBrainTemplate::StandStill,
            // Notices nobody and swings at nobody; the old row's
            // `attack_range: 150.0` sat beside `melee: None`.
            aggro_radius: 0.0,
            attack_range: 0.0,
            patrol_effort: 0.6774,
            chase_effort: 1.0,
            ..Default::default()
        });
    definition.vitals.max_health = Some(6);
    definition
}
