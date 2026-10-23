//! Room publication: prepare a hidden candidate room, check its construction
//! receipt, and publish it into the live world or refuse it, as one operation.
//!
//! The coordinator joins the domains it reads; none of them owns it.
//! Construction builds the typed plan (`ambition_platformer2d_construction`), the session
//! owns admitted operations, restore facts and retirement
//! (`ambition_platformer2d_actor_monolith::session`), and the world owns geometry and live-room identity
//! (`ambition_platformer2d_actor_monolith::rooms`). See K4 in `docs/planning/engine/actor-monolith-decomposition.md`.

pub(crate) mod custody_handoffs;
mod first_room;
pub(crate) mod stage;
pub(crate) mod transaction;

pub use first_room::{publish_first_room, FirstRoom};
pub use stage::{
    LastRoomConstructionCommit, RoomConstructionError, RoomConstructionPlan, RoomConstructionPlanId,
};
pub use transaction::{
    commit_held_publication, finalize_room_publication, outstanding_publications,
    publication_is_held, publication_succeeded, refuse_held_publication, settle_publication,
    publications_holding_frozen_effects,
    retire_publication, ActiveContentBinding,
    LastConstructionVerification,
    CommitFactsSource, LiveRoomSuccession, PublicationHandle, PublicationRetention, PublicationVerdict,
    StagedArrival, StagedWorldViolation,
};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod construction_tests;
