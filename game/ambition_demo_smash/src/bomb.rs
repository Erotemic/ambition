//! The bomb, end to end in this game.
//!
//! The bomb is the module `ambition_demo_smash_modules::bomb`: a press drops
//! the move's held item as a module entity (`ambition.world.spawn_module_item`),
//! so any fighter can pick it up and throw it, and the module burns its fuse
//! and sets it off on the fuse or on a hard impact on the stage
//! (`ambition.world.item_state`, `ambition.combat.blast`).
//!
//! What stays here are the tests of the bomb this game authors.

#[cfg(test)]
mod tests;
