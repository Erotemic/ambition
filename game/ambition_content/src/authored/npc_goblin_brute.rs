//! Author a distinct Goblin Brute character for `large_brute`. It should be a separate reusable
//! character identity rather than a generic archetype row or an alias for an existing named goblin
//! … Give it an explicit complete character definition/body rather than retaining `combatant` as an
//! unresolved casting fallback."*
//!
//! "not an alias for an existing named goblin" is the load-bearing clause.
//! The cheap close was `large_brute → goblin` — the goblin already exists, it
//! already fights, and three encounter waves would have stopped drawing magenta
//! boxes immediately. That would have made the lab's heavy the same creature as
//! its regulars at a different size, which is the archetype muddle wearing a
//! character's name.
//!
//! However, its sprite must have a SEPARATE Python sprite generator/target, even if that
//! generator initially shares helpers or visual vocabulary with the ordinary goblin.
//!
//! the numbers preserve the `large_brute` waves, per *"the brute's initial
//! gameplay/body values may preserve the current `large_brute` behavior closely;
//! exact balance is tunable later."* Its policy is the `melee_brute_brute`
//! preset the generator config already names — aggro 240, reach 44, chase 75 —
//! which is the same preset the two heavy pirates carry, and the same one the
//! encounter's other heavies use.

use ambition_characters::actor::{CharacterLocomotion, ContactDamage};
use ambition_characters::brain::{
    BrainProfile, CharacterBrainTemplate, MeleeActionSpec, MoveStyleSpec, SwipeSpec,
};
use ambition_platformer2d::character::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(CharacterLocomotion {
            // The `melee_brute_brute` chase speed — slower than the goblin band's
            // 170, which is what makes a brute read as one.
            run_speed: 75.0,
            move_style: MoveStyleSpec::WalkHeavy,
            ..Default::default()
        })
        // Heavier than the goblin's 0.70/1: walking into a brute should cost
        // more than walking into its regulars.
        .with_contact_damage(ContactDamage {
            strength: 0.85,
            amount: 1,
        })
        // its own swing, not `medium_striker`'s. The goblin band NAMES a
        // shared policy because several creatures point at it; the brute's
        // numbers are its own and are stated inline, which is the P2.16 rule —
        // an indirection earns itself when it has adopters.
        .with_action_set(ambition_characters::brain::ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec {
                // A hammer is slow and it hurts.
                windup_s: 0.40,
                active_s: 0.12,
                recover_s: 0.42,
                damage: 2,
                reach_px: 44.0,
            })),
            ranged: None,
            special: None,
            move_style: MoveStyleSpec::WalkHeavy,
        })
        .with_autonomous_profile(BrainProfile {
            template: CharacterBrainTemplate::Smash,
            aggro_radius: 240.0,
            attack_range: 44.0,
            patrol_effort: 0.5,
            chase_effort: 1.0,
            ..Default::default()
        });
    definition.vitals.max_health = Some(8);
    definition
}
