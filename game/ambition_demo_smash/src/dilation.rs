//! Time dilation: a fighter on a slower clock for a while.
//!
//! The dilation itself is the module `ambition_demo_smash_modules::dilation`,
//! clock and all: it slows the body its move ran for
//! (`ambition.body.time_scale`), keeps the remainder in a record on that body,
//! spends it each tick of its status (`ambition.body.status_tick`) and gives
//! the body its own clock back. An edit to any of that compiles the module
//! crate alone.
//!
//! What stays here are the tests of how the dilation meets this game: the
//! clerk's Witch-Time slows the attacker through the counter's answer.

#[cfg(test)]
mod tests;
