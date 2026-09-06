//! Carl Stargan's character-owned locomotion, combat capability, and autonomous
//! policy.
//!
//! Carl walks and can fight; friendliness and `stand_still` are placement or
//! controller policy, not restrictions on the body itself.

use ambition_characters::actor::CharacterLocomotion;
use ambition_characters::brain::{
    BrainProfile, CharacterBrainTemplate, MeleeActionSpec, MoveStyleSpec, SwipeSpec,
};
use ambition_platformer2d::character::CharacterDefinition;

/// Authored through [`super::AUTHORED_CAST`], the canonical buildable-cast registry.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        // Walking is explicit; do not infer flight from art, role, or body kind.
        .with_locomotion(CharacterLocomotion {
            run_speed: 210.0,
            move_style: MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_action_set(ambition_characters::brain::ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec {
                windup_s: 0.30,
                active_s: 0.08,
                recover_s: 0.34,
                damage: 1,
                reach_px: 30.0,
            })),
            ranged: None,
            special: None,
            move_style: MoveStyleSpec::Walk,
        })
        // Autonomous policy is controller behavior; placements may override it.
        .with_moveset(crate::carl_stargan_moveset::carl_stargan_moveset())
        .with_autonomous_profile(BrainProfile {
            template: CharacterBrainTemplate::Smash,
            aggro_radius: 420.0,
            attack_range: 110.0,
            patrol_effort: 0.45,
            chase_effort: 1.0,
            ..Default::default()
        });
    definition.vitals.max_health = Some(4);
    definition
}
