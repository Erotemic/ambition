//! Authored STAGING policy (E12): the presentation effects an encounter wants
//! while it is in flight, as components the generic consumers derive from the
//! LIFECYCLE — never from what kind of encounter it is.
//!
//! A wave arena gets these installed from its `EncounterSpec` at populate; a
//! boss wrap or a signal-driven puzzle authors exactly the same components to
//! get exactly the same effects. The consumers (lock-wall contribution, camera
//! read-model, base music request) ask two questions only: "is the lifecycle
//! in flight?" and "what does the staging policy say?" — the E12 bar.

use bevy::prelude::Component;

use crate::spec::LockWallSpec;

/// Seal the authored wall while the lifecycle locks exits. Derived onto the
/// collision overlay's `gate_solids` every frame by the host contributor.
#[derive(Component, Clone, Debug)]
pub struct EncounterLockWall(pub LockWallSpec);

/// Camera zoom multiplier while in flight (`1.0` = no zoom). Published into
/// the [`EncounterView`](crate::EncounterView) read-model (max over all
/// in-flight encounters, order-independent).
#[derive(Component, Clone, Copy, Debug)]
pub struct EncounterCameraZoom(pub f32);

/// Base-tier track of this encounter occurrence while it is in flight.
/// The source keeps its identity when presentation selects an adaptive program.
#[derive(Component, Clone, Debug)]
pub struct EncounterTrack(pub String);
