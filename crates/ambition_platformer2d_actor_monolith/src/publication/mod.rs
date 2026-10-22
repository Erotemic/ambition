//! Room publication: prepare a hidden candidate room, check its construction
//! receipt, and publish it into the live world or refuse it, as one operation.
//!
//! The coordinator joins the domains it reads; none of them owns it.
//! Construction builds the typed plan (`crate::construction`), the session
//! owns admitted operations, restore facts and retirement
//! (`crate::session`), and the world owns geometry and live-room identity
//! (`crate::rooms`). See K4 in `docs/planning/engine/actor-monolith-decomposition.md`.

pub(crate) mod stage;
pub(crate) mod transaction;

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
