//! Lifecycle markers shared by simulation and presentation.
//!
//! Scope sweeps are presence-driven; entities that survive every scope carry no scope marker.

use bevy::prelude::*;

/// Despawn when the current authored room unloads.
#[derive(Component, Default)]
pub struct RoomScopedEntity;

/// Suspends room residency while this entity is in another entity's custody.
///
/// The entity keeps [`RoomScopedEntity`], so reset/scope queries still see it;
/// [`RoomResident`] excludes it only from room-transition sweeps. Ending custody
/// resumes residency in whichever room is then active. The custodian is generic
/// body vocabulary, not an item-specific relationship.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InCustodyOf(pub Entity);

/// Room-scoped entities currently resident in the room.
///
/// Room-transition sweeps use this filter; full world/session resets intentionally do not.
pub type RoomResident = (With<RoomScopedEntity>, Without<InCustodyOf>);

/// Ordering boundary after all body-side [`InCustodyOf`] derivation for this tick.
///
/// Item residency reads body custody, so consumers that inspect a custodian's
/// [`RoomResident`] status run after this set. Ordering against this semantic
/// boundary remains valid if the owning derive moves between systems or crates.
/// In compositions without a body-custody derive, `.after(BodyCustodySettled)` is a no-op.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct BodyCustodySettled;

/// Rendered entity scoped to the current room.
///
/// Requiring [`RoomScopedEntity`] gives presentation entities the same teardown as simulation.
#[derive(Component, Default, Clone)]
#[require(RoomScopedEntity)]
pub struct RoomVisual;

/// Despawn when the named game mode deactivates.
///
/// Mode-scoped entities survive room transitions within that mode, unlike
/// [`RoomScopedEntity`], but do not survive mode changes for the session.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct ModeScopedEntity(pub String);

/// Marker on a rendered player sprite entity.
#[derive(Component, Clone, Default)]
pub struct PlayerVisual;

/// PUBLISH THIS BODY'S POSE READ MODEL — which row it draws, which clip, which
/// frame — whether or not anything is rendering.
///
/// ⛔⛔ THE POSE READ MODEL WAS GATED ON [`PlayerVisual`], AND A MATCH FIGHTER
/// NEVER RECEIVES ONE. `PlayerVisual` is granted in exactly one production place
/// — the exploration player's avatar — so `BodyPoseView` was simply not built
/// for a seated `MatchSeat` body. A headless diagnostic could therefore say
/// where a fighter was and what move it was playing, but not which POSE, CLIP
/// and FRAME the game intends to draw, and the moveset inspector reconstructed
/// that in JavaScript from sprite sheets.
///
/// ⭐ IT IS A SEPARATE MARKER AND NOT A WIDER `PlayerVisual` GRANT. That marker
/// means "this is the player's own drawn avatar" and other presentation keys on
/// it; handing it to every seat would turn those on too. This one says only the
/// thing the read model needs, and says it for any body somebody wants a pose
/// answer about.
///
/// ⛔ IT DOES NOT IMPLY A RENDERER. `BodyPoseView` is a pure function of sim
/// state rebuilt every tick, declared rollback-DERIVED, so a `NoWindow`
/// composition publishes it exactly as a windowed one does — which is the whole
/// point: the engine's animation decision becomes readable without a rasterizer.
#[derive(Component, Clone, Default)]
pub struct PosedBody;

/// Simulation-side feature entity spawned from the active room.
///
/// Presentation visuals remain separate and join live state by `FeatureId`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FeatureSimEntity;

/// Rendered loading-zone indicator keyed by zone `id`.
#[derive(Component, Clone, Debug)]
pub struct LoadingZoneVisual {
    pub id: String,
}
