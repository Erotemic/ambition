//! The versus MATCH, prepared: a roster of participants, the rules of the
//! stage, and the immutable plan a kernel activates without a lookup.
//!
//! Three ways to stage a cast project into one character demand
//! ([`staging`]); [`prepared::prepare_match`] answers every fallible question
//! -- who is seated, what they wear, who drives them, what they may do --
//! before a body exists; [`seating`] is the rollback-safe receipt of the match
//! that is live. Spawning the bodies, binding their control and running the
//! opening are the actor kernel's (its `character_runtime::match_activation`).
//!
//! Carved from the actor kernel's `character_runtime` (D33, character
//! preparation versus actor simulation, 2026-09-03).

pub mod prepared;

#[cfg(test)]
mod prepared_policy_tests;
pub mod seating;
mod snapshot_impls;
pub mod staging;

pub use prepared::{
    effective_abilities, prepare_match, seat_placement, ControlAuthority, MatchPreparationProblems,
    MatchRules, OpeningPhase, PreparedMatch, PreparedSeat, OPENING_BEATS,
};
pub use seating::{match_participants, ActiveMatch, MatchInstance, MatchSeat};
pub use staging::{
    ControllerBinding, DirectStartupSpec, MatchItemSpawns, MatchParticipant,
    MatchParticipantRoster, NormalizedEffort, RoomStagingPlan, RosterProblem, RosterSeating,
    StagesCharacters,
};
