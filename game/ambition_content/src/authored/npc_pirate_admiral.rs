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

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    {
        definition.with_moveset(crate::pirate_admiral_moveset::pirate_admiral_moveset())
    }
}
