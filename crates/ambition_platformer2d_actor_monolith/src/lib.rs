//! The body, control, and action kernel.
//!
//! Domain owners supply construction, sessions, items, and world policy.
//! This crate executes accepted control on live bodies. Runtime composes
//! domain schedules and rollback declarations.

// External API surface — bins, tests, and Android/wasm entry points reach
// into these modules. Everything else stays `pub(crate)` so the compiler
// can tell us what's actually depended on from outside.
/// Body integration and capabilities for controlled bodies.
pub mod avatar;
pub mod brain_tick;
#[cfg(feature = "causal")]
pub mod causal;
#[cfg(test)]
mod character_roster;
/// The local control seam: device frame -> slot -> the body carrying that slot's
/// player brain. See `control/mod.rs`.
pub mod control;

pub mod abilities;
pub mod body_custody;
pub mod body_mode;
#[cfg(test)]
mod character_integration_tests;
pub mod character_projection;
/// Swings and shots arbitrated in ONE contest, before either damage road runs.
pub mod clash;
#[cfg(test)]
pub mod enemy_projectile;
/// The `ParticipantId` ↔ `PlayerSlot` correspondence, in one place, so the
/// eventual split of "a person" from "a seat" is localized.
pub mod participant_seat;
pub mod repertoire;

pub mod body;

#[cfg(test)]
mod clock_discipline_tests;
#[cfg(test)]
mod safe_pos_tests;

// Domain-owned rollback declaration; the host supplies the backend registrar.
mod rollback_registration;
pub use rollback_registration::register_rollback_state;
