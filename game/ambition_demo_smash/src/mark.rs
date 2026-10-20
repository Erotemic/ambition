//! The delayed mark, end to end in this game.
//!
//! The mark is the module `ambition_demo_smash_modules::mark`: a strike that
//! authors `smash.mark_body` puts a clock on the body it struck
//! (`ambition.combat.struck`, a status and a record on that body), and when
//! the clock runs out a blast goes off where the body is then, credited to
//! the striker (`ambition.combat.blast`). The clock is drawn above the body
//! from its status (`ambition.body.status_clock`).
//!
//! What stays here are the tests of the mark this game authors.

/// The status a marked body holds, by which a reader asks whether a body is
/// marked.
pub use ambition_demo_smash_modules::mark::MARKED;

#[cfg(test)]
mod tests;
