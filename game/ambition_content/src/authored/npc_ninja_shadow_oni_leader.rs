//! THE SHADOW ONI LEADER'S ANSWERS. The fourth adopter removed from
//! the generic floor (P3.24), and the first table authored from a
//! character's BARKS rather than a design note — his row carries no
//! `gameplay_description`, and *"the shadow answers"* / *"one breath
//! left"* / *"the order obeyed instantly"* are one.
//!
//! MOVES ONLY.

use ambition_platformer2d::character::CharacterDefinition;

/// AC5: it authors its LOCOMOTION too, which is the one fact that stood
/// between this character and building its own body. It ships as
/// `melee_brute_striker` (chase 110), and that preset's speed is absolute, so
/// stating the body's run speed here changes nothing a player sees today — it
/// makes the body complete, which is what let the body-assist seam go.
///
/// a moveset without a body was the exact shape the assist seam existed for: a
/// character rich enough to state its swings and not yet able to state its walk.
///
/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(ambition_characters::actor::CharacterLocomotion {
            run_speed: 110.0,
            move_style: ambition_characters::brain::MoveStyleSpec::Walk,
            ..Default::default()
        });
        // ⭐⭐ ITS MOVES ARE CONTENT NOW, NOT CODE (fast-iteration I2, step 5).
        // The table this line compiled in is `assets/data/movesets/ninja_shadow_oni_leader.ron`,
        // declared in `pack.ron`, validated by the `moveset` schema and applied
        // in `crate::character_catalog::authored_intrinsics` — the one seam
        // every buildable character passes through.
        // ⛔ The Rust table still exists as the EXPORTER's source and the parity
        // oracle's subject. The host reads NEITHER, so editing it changes nothing
        // until it is re-exported.
    definition.vitals.max_health = Some(6);
    definition
}
