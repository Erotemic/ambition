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

/// Base-tier music track requested while in flight (the
/// [`EncounterMusicRequest`](crate::EncounterMusicRequest) `base_track`
/// source). A focused fight's `priority_track` still outranks it; an
/// encounter with adaptive stems authors no track and drives the adaptive
/// director instead.
#[derive(Component, Clone, Debug)]
pub struct EncounterTrack(pub String);
