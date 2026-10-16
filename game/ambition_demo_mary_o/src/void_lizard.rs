//! The Void Lizard: the boss of the castle.
//!
//! A lizard made of the void: a body with no light in it and a white edge. It
//! is an enemy body like the others, not a boss of the engine's boss encounter: it goes along its bridge
//! and turns at the post at each end (its row is the walker the AI Slop is).
//! Its throws and its hop are its brain, an extension module
//! (`ambition_demo_mary_o_modules::void_lizard`), so an edit to them does not
//! compile this crate.
//!
//! No rule makes its top safe: a stomp on it is a hit. Sparks kill it (it has
//! the health of five), a shell or a quasar kills it, and it falls with its
//! bridge when the goal of the room is taken
//! (`bricks::drop_planks_when_the_goal_is_taken`).

use ambition_platformer2d::entity_catalog::placements::CharacterBrain;

/// The brain key a level placement carries beside `character_id`: the key
/// its module binds.
pub use ambition_demo_mary_o_modules::void_lizard::BRAIN_KEY as VOID_LIZARD_BRAIN_KEY;
/// It acts only when its target is this near, side to side (px). The boss
/// gauge shows for the same distance.
pub use ambition_demo_mary_o_modules::void_lizard::SIGHT;
/// The catalog id, and the sheet target.
pub const VOID_LIZARD_CHARACTER_ID: &str = "void_lizard";
/// The `ProjectileVisualId` its orb authors (`provider.rs` registers it).
pub const VOID_ORB_VISUAL: &str = "mary_o_void_orb";

/// Is this body the Void Lizard?
pub fn is_void_lizard_brain(brain: &CharacterBrain) -> bool {
    matches!(brain, CharacterBrain::Custom(key) if key == VOID_LIZARD_BRAIN_KEY)
}
