//! The remote mine, end to end in this game.
//!
//! The mine is the module `ambition_demo_smash_modules::mine`: a press with no
//! mine out places the move's held item as a module entity
//! (`ambition.world.spawn_module_item`); a press with one out sets it off
//! where it is, once it has armed (`ambition.world.owned_entities`,
//! `ambition.combat.blast`).
//!
//! What stays here are the tests of the mine this game authors.

/// The kind of the module entity a placed mine is, by which a reader finds
/// the mines on the stage.
pub use ambition_demo_smash_modules::mine::MINE;

#[cfg(test)]
mod tests;
