//! The versus match, prepared: a roster of participants, the stage rules,
//! and the immutable plan a kernel activates without a lookup.
//!
//! Three ways to stage a cast project into one character demand
//! ([`staging`]). [`prepared::prepare_match`] answers every fallible question
//! (who is seated, what they wear, who drives them, what they may do) before
//! a body exists. [`seating`] is the rollback-safe receipt of the live match.
//! The actor kernel (`character_runtime::match_activation`) spawns the
//! bodies, binds their control, and runs the opening.
//!
//! Split from the actor kernel's `character_runtime` (D33: character
//! preparation versus actor simulation).

pub mod prepared;

#[cfg(test)]
mod prepared_policy_tests;
pub mod seating;
pub mod settlement;
mod snapshot_impls;
pub mod staging;

pub use prepared::{
    effective_abilities, prepare_match, seat_placement, ControlAuthority, MatchPreparationProblems,
    MatchRules, OpeningPhase, PreparedMatch, PreparedSeat, OPENING_BEATS,
};
pub use seating::{
    match_participants, ActiveMatch, MatchInstance, MatchScoped, MatchSeat,
};
pub use staging::{
    ControllerBinding, DirectStartupSpec, MatchItemSpawns, MatchParticipant,
    MatchParticipantRoster, NormalizedEffort, RoomStagingPlan, RosterProblem, RosterSeating,
    StagesCharacters,
};

pub use settlement::{the_live_match_is_settled, StocksMatchSettled, SuddenDeathEntered};
