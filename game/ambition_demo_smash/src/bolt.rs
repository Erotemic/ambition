//! The steered bolt, end to end in this game.
//!
//! The bolt is the module `ambition_demo_smash_modules::bolt`: a press puts a
//! module entity on the stage (`ambition.world.spawn_module_entity`), and the
//! module flies it each tick by its caster's stick
//! (`ambition.world.spawner_body`, `ambition.world.move_module_entity`),
//! hits a rival it reaches (`ambition.combat.blast`) or carries its caster
//! home (`ambition.world.launch_toucher`), telling the two apart by
//! identity (`ambition.world.toucher_credits`).
//!
//! What stays here are the tests of the bolt this game authors.

/// The kind of the module entity a bolt in flight is, by which a reader
/// finds the bolts on the stage.
pub use ambition_demo_smash_modules::bolt::BOLT;

#[cfg(test)]
mod tests;
