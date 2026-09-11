//! Pointed Polygon — sword archetype.
//!
//! This is intentionally an uncomplicated humanoid body. Its main authoring value
//! is that the sprite rig supplies safe reference poses that later humanoids can
//! copy before adding bespoke anatomy or personality.

use ambition_platformer2d::character::CharacterDefinition;

pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(ambition_characters::actor::CharacterLocomotion {
            run_speed: 220.0,
            move_style: ambition_characters::brain::MoveStyleSpec::Walk,
            ..Default::default()
        });
        // ⭐⭐ ITS MOVES ARE CONTENT NOW, NOT CODE (fast-iteration I2, step 5).
        // The table this line compiled in is `assets/data/movesets/pointed_polygon.ron`,
        // declared in `pack.ron`, validated by the `moveset` schema and applied
        // in `crate::character_catalog::authored_intrinsics` — the one seam
        // every buildable character passes through.
        // ⛔ The Rust table still exists as the EXPORTER's source and the parity
        // oracle's subject. The host reads NEITHER, so editing it changes nothing
        // until it is re-exported.
    definition.vitals.max_health = Some(5);
    definition
}
