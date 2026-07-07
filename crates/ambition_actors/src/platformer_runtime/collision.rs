//! Solid-world raycasting facade.
//!
//! The generic raycast surface ([`SolidWorldQuery`] / [`raycast_solids`] /
//! [`ray_aabb`]) AND the `impl SolidWorldQuery for ambition_engine_core::World` adapter
//! now live in the content-free `ambition_engine_core::cast`
//! module (Stage 16 / S1). Both the trait and `ambition_engine_core::World` are
//! foundation types, so the adapter is sandbox-free and the orphan rule keeps
//! the impl in-crate with the trait. This module re-exports the surface
//! unchanged so every `crate::platformer_runtime::collision::{…}` path keeps
//! resolving (blink / dive / grapple / item_pickup / portal placement).
pub use ambition_engine_core::cast::{ray_aabb, raycast_solids, SolidWorldQuery};
