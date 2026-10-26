//! Gameplay-system assembly layer for platformer actors, world interaction, abilities,
//! items, sessions, and related Bevy plugins.
//!
//! Pure movement lives below this crate in `ambition_platformer2d_core`; content-free
//! Bevy primitives live in `ambition_platformer2d_shared_tangle`; named game content
//! lives above it. This crate is intended to converge on assembly/cross-domain wiring,
//! with coherent gameplay domains carved into their own crates rather than growing new
//! resident subsystems here. Player simulation remains component-authoritative; do not
//! introduce a replacement god-object runtime resource.

// External API surface — bins, tests, and Android/wasm entry points reach
// into these modules. Everything else stays `pub(crate)` so the compiler
// can tell us what's actually depended on from outside.
/// Local human avatar state and policy, separate from shared actor mechanics.
pub mod avatar;
pub mod brain_tick;
#[cfg(feature = "causal")]
pub mod causal;
#[cfg(test)]
mod character_roster;
/// The local control seam: device frame -> slot -> the body carrying that slot's
/// player brain. See `control/mod.rs`.
pub mod control;

// Themed module umbrellas. Each owns a coherent slice of the sandbox.
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
#[cfg(test)]
#[path = "tests/projectile/mod.rs"]
mod projectile_tests;
pub mod repertoire;

// Public re-exports double as the external API for bins, tests, and docs.
/// The live actor cluster view (`ActorMut`, `ActorClusterQueryData`) and the
/// combat timing that rides it. Kernel vocabulary: every per-tick actor system
/// reads it, so it lives here and not in the spawn capability that builds bodies.
pub mod features;

// Keep generic time vocabulary in `ambition_time`; export only the local bridge.


#[cfg(test)]
mod clock_discipline_tests;
#[cfg(test)]
mod safe_pos_tests;

// Domain-owned rollback declaration; the host supplies the backend registrar.
mod rollback_registration;
pub use rollback_registration::register_rollback_state;
