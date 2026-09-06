//! THE SHARK RIDERS. Two creatures, one policy, different numbers —
//! the case the archetype file answered with two nearly-identical rows
//! (`pirate_shark_rider`, `pirate_heavy_shark_rider`) whose only real
//! differences are health, weight, reach and which gun-sword they hold.
//!
//! `body_contact_damage: false` on both rows, so neither authors
//! `contact_damage`. The rows carried a `contact_strength` and a
//! `damage_amount` beside a flag that turned them off — numbers that
//! described nothing. A character says what is true: touching a raider
//! does not hurt; its gun-sword does.
//!
//! `default_size` does not come across either: both are sized by their
//! authored placements (44x78 and 72x110 in `sandbox.ldtk`), which is the
//! same silhouette the rows were restating.

use ambition_characters::actor::CharacterLocomotion;
use ambition_characters::brain::{BrainProfile, CharacterBrainTemplate, MoveStyleSpec};
use ambition_platformer2d::character::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let heavy = id == "npc_pirate_heavy_iron_mary";
    let mut definition = definition
        .with_locomotion(CharacterLocomotion {
            run_speed: if heavy { 215.0 } else { 230.0 },
            move_style: if heavy {
                MoveStyleSpec::WalkHeavy
            } else {
                MoveStyleSpec::Walk
            },
            ..Default::default()
        })
        // A cove raider can board a "shark"-class mount. It is not itself
        // rideable, which is the other half of the same sentence.
        .with_mount(ambition_characters::actor::CharacterMount {
            pilotable_classes: vec!["shark".to_string()],
            ..Default::default()
        })
        .with_held_item(if heavy {
            "gun_sword_heavy"
        } else {
            "gun_sword"
        })
        .with_autonomous_profile(BrainProfile {
            // Orbit-and-fire standoff: notice from across the cove,
            // commit from just inside it.
            template: CharacterBrainTemplate::Skirmisher,
            aggro_radius: 1200.0,
            attack_range: 1100.0,
            patrol_effort: if heavy { 0.5116 } else { 0.4783 },
            chase_effort: 1.0,
            ..Default::default()
        })
        .with_action_set(ambition_characters::brain::ActionSet {
            // The bolt the gun-sword fires — the SAME verb
            // `held_item_by_id` grants, authored here because a
            // character states what it DOES and the item states what it
            // HOLDS.
            ranged: Some(ambition_characters::brain::RangedActionSpec::bolt(
                500.0,
                if heavy { 3 } else { 2 },
            )),
            move_style: if heavy {
                MoveStyleSpec::WalkHeavy
            } else {
                MoveStyleSpec::Walk
            },
            ..Default::default()
        });
    definition.vitals.max_health = Some(if heavy { 6 } else { 4 });
    definition
}
