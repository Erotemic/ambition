//! **THE SHADOW ONI LEADER'S ANSWERS.** The fourth adopter removed from
//! the generic floor (P3.24), and the first table authored from a
//! character's BARKS rather than a design note — his row carries no
//! `gameplay_description`, and *"the shadow answers"* / *"one breath
//! left"* / *"the order obeyed instantly"* are one.
//!
//! ⛔ MOVES ONLY. His body still comes from his catalog row; authoring
//! vitals here would be a retune wearing a migration's commit, and a
//! table is the whole job.

use ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    definition
        .with_moveset(crate::ninja_shadow_oni_leader_moveset::ninja_shadow_oni_leader_moveset())
}
