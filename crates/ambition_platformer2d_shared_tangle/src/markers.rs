//! Content-free entity markers shared by reusable mechanics and presentation.

use bevy::prelude::*;

/// Marks a body in the player population; zero or many may exist.
///
/// Generic simulation should use body capabilities/control authority instead.
/// Use [`PrimaryPlayer`] for home-avatar policy and [`ControlledSubject`] for the
/// body currently driven by primary local control.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerEntity;

/// Entity currently driven by the primary local control authority.
///
/// `None` is valid while no primary controlled subject is resolved.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlledSubject(pub Option<Entity>);

/// Ordered bodies to frame when no local authority drives a subject.
///
/// The session/match publishes this list; the camera does not infer which bodies
/// belong in the frame. Empty means no cast has been declared.
#[derive(Resource, Default, Clone, Debug, PartialEq, Eq)]
pub struct FramedCast(pub Vec<Entity>);

/// Marks the exploration home body used by primary-player save/respawn policy.
///
/// Control may move elsewhere; use [`ControlledSubject`] for current control
/// authority. This is also distinct from [`crate::body::PrimaryBody`], which
/// selects live room-gravity resolution.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PrimaryPlayer;

/// Query filter for a body carrying both [`PlayerEntity`] and [`PrimaryPlayer`].
pub type PrimaryPlayerOnly = (With<PlayerEntity>, With<PrimaryPlayer>);
