//! Time dilation: a fighter on a slower clock for a while.
//!
//! The dilation itself is the module `ambition_demo_smash_modules::dilation`:
//! it asks `ambition.combat.dilate_time` to slow the body its move ran for,
//! so an edit to it compiles the module crate alone. The combat engine owns
//! the clock: it writes the body's `ProperTimeScale`, spends the seconds on
//! the world's clock and gives the body its own clock back
//! (`lower_time_dilations`).
//!
//! What stays here are the tests of how the dilation meets this game: the
//! clerk's Witch-Time slows the attacker through the counter's answer.

#[cfg(test)]
mod tests;
