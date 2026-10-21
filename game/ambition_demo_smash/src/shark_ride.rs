//! The pirate's up-special, end to end in this game: summon a burning flying
//! shark and ride it.
//!
//! The ride is the module `ambition_demo_smash_modules::shark`: it summons
//! the move's character (`ambition.world.summon_ride`), and a status on the
//! rider takes her off when the genre says so (`ambition.mount.dismount`),
//! while a status on the shark sends it away once nobody rides it
//! (`ambition.body.hold_course`, `ambition.world.dismiss`). The mount system
//! owns the rest (ADR 0020): the saddle, the lease, steering the shark by
//! its rider's stick while she keeps her attacks and specials, and boarding.
//! The "no recast from the saddle" rule is the move's gate
//! (`MoveGates::forbidden_while_held`).
//!
//! What stays here is the shark's id, which a save test reads; the tests of
//! the ride this game authors are the app's (`smash_ride`).

/// The summoned shark's id, and its save key.
pub use ambition_demo_smash_modules::shark::SHARK_ID;
