//! CARL STARGAN.
//!
//! > *"There is no separate 'can fight' character property. A character can
//! > fight exactly to the extent that its body has abilities/capabilities that
//! > can produce combat effects."*
//!
//! So the long-open question *"does Carl Stargan fight?"* is
//! not answered here with a flag. It is answered by this file authoring a
//! moveset: he has a swipe, therefore he can swing it. A body with no damaging
//! ability simply has nothing useful to execute when asked to attack, and no
//! `can_fight` / `combatant` / peaceful-vs-fighter taxonomy is needed to say so.
//!
//! The two content facts he owns, both verbatim from that handoff:
//!
//! - *"Carl does not have the fly ability."* and not by omission — by an
//!   authored locomotion that says he walks. *"Do not infer flight from his art,
//!   body kind, NPC role, or any legacy archetype."*
//! - *"Carl can fight … his combat capability is intrinsic to the character/body,
//!   not something granted by being controlled or by entering a particular
//!   mode."*
//!
//! friendly is CONTEXT, and it stays out of this file. *"Do not make him
//! permanently peaceful/passive merely because he may normally appear as a
//! friendly NPC. His placement/disposition can be friendly while his body still
//! possesses combat capabilities and his autonomous policy can defend allies."*
//! His one shipped placement is a Hall `NpcSpawn` with
//! `brain_override: stand_still`, and that placement keeps its override — a
//! standing statue with a sword is a body that is not being asked to swing, not
//! a body that cannot.

use ambition_characters::actor::CharacterLocomotion;
use ambition_characters::brain::{
    BrainProfile, CharacterBrainTemplate, MeleeActionSpec, MoveStyleSpec, SwipeSpec,
};
use ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        // HE WALKS. Authored rather than defaulted, because the question
        // this file closes was whether he flies, and a default is not an answer
        // to a question somebody asked.
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
        // The policy that lets him answer a fight he did not start. It is
        // CONTROLLER policy, not body identity — his Hall placement overrides it
        // with `stand_still`, and overriding it is what a placement is for.
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
