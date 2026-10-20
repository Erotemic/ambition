//! The tether reel, end to end in this game.
//!
//! The tether is the module `ambition_demo_smash_modules::tether`: a throw
//! bites the first ledge ahead within the move's reach
//! (`ambition.body.ledge_ahead`), from the air only
//! (`ambition.body.on_ground`), and a status on the fighter reels her to it
//! (`ambition.motion.velocity`) until the ledge authority would catch her.
//! The module catches no ledge: the movement kernel's ledge authority does,
//! with its own cooldown and eligibility.
//!
//! What stays here are the tests of the tether this game authors.

#[cfg(test)]
mod tests;
