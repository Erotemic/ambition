//! Session construction, admitted lifecycle operations, restoration, and retirement.

/// Returning a session to its checkpoint: the startup resume and the
/// death/retry reset. Carved out of `shrine` in A1b — a shrine owns an
/// interaction, coming back owns the session's room, subject and lifecycle slot.
pub mod checkpoint;
pub mod data;
/// The death interlude and the roster question that decides a level reset
/// (ADR 0033).
pub mod death;
/// The DURABLE save horizon: occurrence whereabouts, custody and runtime-minted
/// descriptions on disk, and the load that resumes from them.
pub mod durable_horizon;
/// A second seat joins the session (Q153 default).
pub mod join;
pub mod lifecycle_commit;
pub mod reset;
pub mod setup;
pub mod teardown;

pub use teardown::{reset_session_scoped_resources_on_retire, SessionTeardownPlugin};

pub mod rooms;
mod snapshot_impls;
mod rollback_registration;
pub use rollback_registration::register_rollback_state;
#[cfg(test)]
mod item_schedule_tests;

pub mod match_activation;
mod match_lifecycle;
pub use match_lifecycle::MatchLifecyclePlugin;

pub mod initial_body;
