//! **THE PIRATE ADMIRAL'S CUTLASS.** The second adopter removed from
//! `smash_fighter_kit()` (P3.24), and the character was already telling us
//! what its moves are: its row says `default_action_set: "pirate_pistol"`,
//! the roster comment beside its id reads "pistol + cutlass", and its
//! sprite is authored at `collision_scale: 1.6` — the largest of the three
//! fighters with a table.
//!
//! ⛔ MOVES ONLY. The admiral's body still comes from its catalog row and
//! its archetype; authoring vitals or locomotion here would be a retune
//! wearing a migration's commit, and it is not what removes the adopter.
//! A table is the whole job.

use ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition;

/// **AC5: it authors its LOCOMOTION too**, which is the one fact that stood
/// between this character and building its own body. It ships as
/// `melee_brute_striker` (chase 110), and that preset's speed is absolute, so
/// stating the body's run speed here changes nothing a player sees today — it
/// makes the body complete, which is what lets `adopt_character_intrinsics` go.
///
/// ⚠ a moveset without a body was the exact shape the assist seam existed for: a
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
        })
        .with_moveset(crate::pirate_admiral_moveset::pirate_admiral_moveset());
    // Jon 2026-08-13: heavy/large pirate variants 6. An admiral is one.
    definition.vitals.max_health = Some(6);
    definition
}
