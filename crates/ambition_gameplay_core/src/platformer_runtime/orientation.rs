//! Actor orientation facade.
//!
//! The shared body-orientation component ([`ActorRoll`]) and its righting
//! systems ([`ensure_actor_roll`] / [`update_actor_roll`]) now live in the
//! content-free `ambition_platformer_primitives::orientation` module (Stage 16 /
//! S5). With gravity (`GravityCtx`) in-crate, scaled dt via `SimDt`, and the
//! unified `BodyKinematics`, the dual player/actor query arms collapsed to a
//! single `With<BodyKinematics>` query. This module re-exports the surface
//! unchanged so every `crate::platformer_runtime::orientation::{…}` path (portal
//! transit / plugin / presentation / rendering / view_index) keeps resolving.
pub use ambition_platformer_primitives::orientation::{
    ensure_actor_roll, update_actor_roll, ActorRoll,
};
