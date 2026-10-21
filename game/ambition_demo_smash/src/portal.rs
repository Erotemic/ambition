//! The portal recovery, end to end in this game.
//!
//! The recovery is the module `ambition_demo_smash_modules::portal`: a press
//! opens a linked pair, the entrance at the fighter and the exit above, aimed
//! by the move's latched stick (`ambition.fighter.aimed_stick`), and a status
//! on her holds it open (`ambition.portal.held_pair`) for the move's
//! lifetime. No portal behaviour lives here or there: `ambition_portal2d`
//! owns apertures, linking and transit.
//!
//! What stays here are the tests of the recovery this game authors.

#[cfg(test)]
mod tests;
