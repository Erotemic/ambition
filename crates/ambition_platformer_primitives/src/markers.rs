//! Generic entity-marker components shared by reusable mechanics.
//!
//! These are pure `Component` markers — query filters with no fields and no
//! sandbox-internal dependencies. They live in the runtime crate so portal,
//! gravity, and other extracted mechanics can query the player / simulated
//! feature entities without depending on the sandbox's `player` or `features`
//! modules. The host (`ambition_gameplay_core`) re-exports them from their original
//! paths so existing call sites compile unchanged.

use bevy::prelude::*;

/// Marker for **a player entity** — there may eventually be more than
/// one. Use this when a query wants every player regardless of locality
/// or which slot they occupy.
///
/// The game currently spawns exactly one player, with `PlayerSlot(0)`,
/// [`PrimaryPlayer`], and `LocalPlayer` all attached. Systems that
/// want the camera/HUD/dev-tool target should filter on `PrimaryPlayer`
/// (or use the helpers in the sandbox's `player::queries`) rather than
/// assuming the only `PlayerEntity` is *the* player.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerEntity;

/// Marks the player that the camera, HUD, dev tools, and pause menu
/// follow by default. Exactly one entity in the world should carry
/// this component; today every spawned player is also primary.
///
/// Distinct from `LocalPlayer` because in a future split-screen
/// build the local players would each be `LocalPlayer` but only one
/// would be `PrimaryPlayer` (e.g. the host's view in a guest-joined
/// session).
///
/// Distinct from [`crate::body::PrimaryBody`] too: `PrimaryPlayer` names the
/// *player* the presentation layer follows, while `PrimaryBody` names the body
/// whose position drives the room's live gravity resolution. Today the spawn
/// bundle attaches both to the same entity, but they are kept separate so the
/// content-free gravity runtime never has to filter on the player-specific
/// marker.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PrimaryPlayer;

/// Marker for simulation-side feature entities spawned from the active room.
/// They are deliberately separate from presentation `FeatureVisual` sprites;
/// visible builds keep using the existing visual entities and look up live ECS
/// state by `FeatureId`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FeatureSimEntity;
