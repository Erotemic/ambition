//! Pugnacious Polygon — brawler archetype.
//!
//! The unarmed companion to the sword reference rig. It intentionally shares the
//! same broad safe-pose vocabulary while using larger fists and close-range body
//! mechanics so future humanoid rigs have both armed and unarmed references.

use ambition_platformer2d::character::CharacterDefinition;

pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(ambition_characters::actor::CharacterLocomotion {
            run_speed: 230.0,
            move_style: ambition_characters::brain::MoveStyleSpec::Walk,
            ..Default::default()
        });
        // ⭐⭐ ITS MOVES ARE CONTENT NOW, NOT CODE (fast-iteration I2, step 5).
        // The table this line compiled in is `assets/data/movesets/pugnacious_polygon.ron`,
        // declared in `pack.ron`, validated by the `moveset` schema and applied
        // in `crate::character_catalog::authored_intrinsics` — the one seam
        // every buildable character passes through.
        // ⛔ The Rust table still exists as the EXPORTER's source and the parity
        // oracle's subject. The host reads NEITHER, so editing it changes nothing
        // until it is re-exported.
    definition.vitals.max_health = Some(6);
    definition
}
