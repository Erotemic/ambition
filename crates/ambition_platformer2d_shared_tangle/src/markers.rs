//! Generic entity-marker components shared by reusable mechanics.
//!
//! These are pure `Component` markers — query filters with no fields and no
//! sandbox-internal dependencies. They live in the runtime crate so portal,
//! gravity, and other extracted mechanics can query the player / simulated
//! feature entities without depending on the sandbox's `player` or `features`
//! modules. The host (`ambition_platformer2d_actor_monolith`) re-exports them from their original
//! paths so existing call sites compile unchanged.

use bevy::prelude::*;

/// Marker for a body in the player population. Use it when a query wants
/// every such body regardless of locality or which slot drives it.
///
/// it does not mean "one", and it does not mean "the protagonist". An
/// exploration session lowers a home avatar carrying `PlayerSlot(0)`,
/// [`PrimaryPlayer`] and `LocalPlayer` together, which made "the only
/// `PlayerEntity` is *the* player" look like an invariant for a long time. It is
/// not one: a match under `InitialBodyPolicy::NoInitialBody` has zero, and local
/// multiplayer has several. A query that wants the camera/HUD/dev-tool target
/// wants [`PrimaryPlayer`] or `ControlledSubject`, and must still be correct when
/// there is none.
///
/// generic simulation should not filter on this at all. A body decides,
/// moves, fights and rides because of its capabilities and control authority; the
/// six overlapping "player" names and what each is really for are laid out in
/// `docs/concepts/one-body-one-path.md`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerEntity;

/// The entity currently driven by the primary local control authority.
///
/// `None` is only expected during startup/load frames before the primary player
/// brain has been resolved. This lives with the content-free player markers so
/// presentation/host adapters can follow the controlled body without depending
/// on the sandbox actor-systems crate.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlledSubject(pub Option<Entity>);

/// Ordered bodies to frame when no local authority drives a subject.
///
/// The session/match publishes this list; the camera does not infer which bodies
/// belong in the frame. Empty means no cast has been declared.
#[derive(Resource, Default, Clone, Debug, PartialEq, Eq)]
pub struct FramedCast(pub Vec<Entity>);

/// Marker for the exploration home body selected for primary-player policy.
///
/// This is distinct from the currently controlled subject: possession can move
/// control to another body, and match compositions may have no `PrimaryPlayer`.
/// Use [`ControlledSubject`] for input/camera/ability authority. Home-body concerns
/// such as respawn/save policy may use this marker. It is also distinct from
/// [`crate::body::PrimaryBody`], which drives live room-gravity resolution.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PrimaryPlayer;

/// Query filter selecting the ONE primary player body — `With<PlayerEntity>` AND
/// `With<PrimaryPlayer>`. A pure composition of two markers that both live here,
/// so it belongs beside them: reusable mechanics + presentation can filter on the
/// camera/HUD/dev-tool target without depending on the sandbox's `player` module.
pub type PrimaryPlayerOnly = (With<PlayerEntity>, With<PrimaryPlayer>);
