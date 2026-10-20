//! The homing dash: the fighter is carried at whoever they were pointing at.
//!
//! The dash is the module `ambition_demo_smash_modules::homing`: its clock,
//! its cone and its steering, as a status on the fighter. It asks the engine
//! only who is a foe (`ambition.combat.foes`) and sets its fighter's velocity
//! (`ambition.motion.velocity`), so an edit to any of the move compiles the
//! module crate alone.
//!
//! What stays here are the tests of the dash this game authors: the cone is
//! the move, so every test pairs a hit with a miss.

#[cfg(test)]
mod tests;
