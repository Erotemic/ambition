//! THE SALVAGE GUARD. The intro raid corridor's two `EnemySpawn`s,
//! which have been wearing `gradient_seeker` — an archetype whose whole
//! population is those two placements, both literally named "Salvage
//! Guard". A generic role with exactly one creature in it was never a
//! role; it was that creature's body filed under a different name.
//!
//! its policy is INLINE, and the goblin's is NAMED, and the
//! difference is the P2.16 rule rather than an inconsistency. A shared
//! `autonomous_profiles` entry earns its indirection when several
//! creatures point at it — `medium_striker` has a goblin band. This
//! policy has one adopter, so naming it would publish a shared thing
//! nobody shares and leave a second empty role behind exactly like the
//! one being deleted.
//!
//! `respawn: OnRoomReenter` is NOT here: it is the third authority
//! (placement policy), it is the engine default for a room-scoped enemy,
//! and the archetype stating it is the muddle this campaign removes.

use ambition_characters::actor::{CharacterLocomotion, ContactDamage};
use ambition_characters::brain::{
    BrainProfile, CharacterBrainTemplate, MeleeActionSpec, MoveStyleSpec, SwipeSpec,
};
use ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition;

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
