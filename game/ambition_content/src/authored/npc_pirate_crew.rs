//! THE SIX REMAINING PIRATES, whose bodies were the last thing keeping
//! `REGISTERED_WITHOUT_A_BODY` alive.
//!
//! They were registered as buildable and authored NOTHING — the shape that
//! list's own doc calls dangerous by default. What held them there was a real
//! question rather than an oversight: their VITALS were a content decision
//!, and authoring a
//! number to empty a list would have been inventing one.
//!
//! Pick reasonable explicit health values and AUTHOR THEM. Do not retain fallback health or
//! incomplete body definitions because we are waiting for balance decisions."* His initial numbers
//! — ordinary pirate 4, heavy/large pirate variant 6 — are the two used here.
//!
//! so these numbers are TUNING and the authoring is ARCHITECTURE. He drew
//! that line himself. Retuning a health value later is not reopening a decision;
//! putting the fallback back would be.
//!
//! one file for six, and that is the P2.16 rule rather than laziness. They
//! are one crew in three shapes — two strikers, two brutes, two rangers — and the
//! shapes are the three brain presets their catalog rows already named. Six files
//! differing by a literal would be the copy `AUTHORED_CAST` exists to refuse, and
//! the raider file next door already carries two creatures for the same reason.

use ambition_characters::actor::CharacterLocomotion;
use ambition_characters::brain::{
    BrainProfile, CharacterBrainTemplate, MeleeActionSpec, MoveStyleSpec, SwipeSpec,
};
use ambition_platformer2d::character::CharacterDefinition;

/// Each pirate's authored controller policy matches its shipped role. The
/// lookout and navigator use `skirmisher_ranger`; do not infer behavior from
/// neighboring pirate IDs.
#[derive(Clone, Copy)]
enum Crew {
    /// `melee_brute_striker`: aggro 220, reach 36, chase 110.
    Striker,
    Brute,
    /// `skirmisher_ranger`: aggro 320, standoff 140, strafe 85, refire 0.8.
    Ranger,
}

fn crew_of(id: &str) -> Crew {
    match id {
        "npc_pirate_heavy_broadside_bess" | "npc_pirate_heavy_salt_annet" => Crew::Brute,
        "npc_pirate_lookout" | "npc_pirate_navigator" => Crew::Ranger,
        // cutlass_viper, quartermaster
        _ => Crew::Striker,
    }
}

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes these characters buildable — there is no second list to remember.
pub(crate) fn author(id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let crew = crew_of(id);
    let heavy = matches!(crew, Crew::Brute);
    let move_style = if heavy {
        MoveStyleSpec::WalkHeavy
    } else {
        MoveStyleSpec::Walk
    };
    let mut definition = definition
        .with_locomotion(CharacterLocomotion {
            // The preset's own chase speed, so a migrated pirate closes at the
            // pace it always did.
            run_speed: match crew {
                Crew::Striker => 110.0,
                Crew::Brute => 75.0,
                Crew::Ranger => 85.0,
            },
            move_style,
            ..Default::default()
        })
        .with_action_set(ambition_characters::brain::ActionSet {
            melee: match crew {
                Crew::Ranger => None,
                _ => Some(MeleeActionSpec::Swipe(SwipeSpec {
                    windup_s: 0.26,
                    active_s: 0.09,
                    recover_s: 0.30,
                    damage: if heavy { 2 } else { 1 },
                    // The preset's `attack_range`, which is what a melee brute's
                    // reach WAS.
                    reach_px: if heavy { 44.0 } else { 36.0 },
                })),
            },
            ranged: match crew {
                // The standoff shooter the `Skirmisher` preset describes. Its
                // fire cooldown lives on the autonomous profile below, where the
                // preset kept it.
                Crew::Ranger => Some(ambition_characters::brain::RangedActionSpec::bolt(500.0, 1)),
                _ => None,
            },
            special: None,
            move_style,
        })
        .with_autonomous_profile(BrainProfile {
            template: match crew {
                Crew::Ranger => CharacterBrainTemplate::Skirmisher,
                _ => CharacterBrainTemplate::Smash,
            },
            aggro_radius: match crew {
                Crew::Striker => 220.0,
                Crew::Brute => 240.0,
                Crew::Ranger => 320.0,
            },
            attack_range: match crew {
                Crew::Striker => 36.0,
                Crew::Brute => 44.0,
                Crew::Ranger => 140.0,
            },
            patrol_effort: 0.5,
            chase_effort: 1.0,
            ..Default::default()
        });
    definition.vitals.max_health = Some(if heavy { 6 } else { 4 });
    definition
}
