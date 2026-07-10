//! Lifetime-scope marker components (room / run / persistent) and the rendered
//! room-visual markers, all runtime-owned so sim systems can tag entities
//! without importing presentation.

use bevy::prelude::*;

/// Lifetime-scope marker: despawn when the current authored room is unloaded.
///
/// This marker is deliberately runtime-owned so simulation-only entities and
/// rendered room visuals can share the same lifecycle policy without depending
/// on presentation modules.
#[derive(Component, Default)]
pub struct RoomScopedEntity;

/// Marker for a RENDERED room-scoped entity — a visual the presentation layer
/// draws/syncs for the current room. Presentation systems query `With<RoomVisual>`
/// to filter to the active room's rendered entities; the required
/// [`RoomScopedEntity`] gives it the room-unload/reset teardown automatically.
///
/// Lives here (not in `presentation`) deliberately: the marker is content-free
/// vocabulary, so sim systems can tag the visual entities they spawn WITHOUT
/// importing a presentation module (the whole point of the runtime-owned
/// lifecycle markers above).
#[derive(Component, Default)]
#[require(RoomScopedEntity)]
pub struct RoomVisual;

/// Lifetime-scope marker: despawn when the current gameplay run/session ends,
/// but survive room transitions.
///
/// No cleanup pass consumes this marker yet; it establishes the vocabulary for
/// the refactor branch that splits sandbox reset, run reset, and room unload.
#[derive(Component, Default)]
pub struct RunScopedEntity;

/// Lifetime-scope marker: despawn when the named GAME MODE deactivates.
///
/// A mode is the demo-hosting seam (decomposition D-C): the active room's
/// `RoomMetadata::mode` names which ruleset owns the room, so a mode-scoped
/// entity SURVIVES room transitions inside its own mode and dies the moment the
/// active room's mode is something else. That is a distinct lifetime from
/// [`RoomScopedEntity`] (dies every room load) and [`RunScopedEntity`] (dies
/// with the session) — a hosted demo's mode-owner entity carries its rules'
/// resources across every room in its own zone.
///
/// The marker lives here with its lifetime-scope siblings; the sweep that
/// consumes it needs the active room's metadata and therefore lives a tier up
/// (`ambition_runtime::mode_scope`), exactly as the `RoomScopedEntity` sweep
/// lives above this crate.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct ModeScopedEntity(pub String);

/// Explicit marker for entities that intentionally survive room and run resets.
///
/// This is mostly documentation in ECS form. Use `spawn_persistent` when a raw
/// `commands.spawn` would make lifecycle intent unclear.
#[derive(Component, Default)]
pub struct PersistentEntity;

/// Marker on the player's rendered sprite entity. Content-free (a tag the renderer
/// queries + gameplay systems that manipulate the player visual reference); lives
/// here so neither side imports a presentation module to name it.
#[derive(Component, Default)]
pub struct PlayerVisual;

/// Marker for simulation-side feature entities spawned from the active room.
/// They are deliberately separate from presentation `FeatureVisual` sprites;
/// visible builds keep using the existing visual entities and look up live ECS
/// state by `FeatureId`. Lifecycle vocabulary: a room-scoped sim marker that
/// lives beside the other runtime-owned scope markers.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FeatureSimEntity;

/// The scene's root entity handles (player sprite + HUD surfaces). Opaque `Entity`
/// slots shared by setup, input, and presentation; runtime-owned so sim systems can
/// reference them without importing presentation.
#[derive(Resource)]
pub struct SceneEntities {
    pub player: Entity,
    pub hud: Entity,
    pub quest_panel: Entity,
}

/// Marker on the rendered loading-zone indicator entity (keyed by zone `id`).
/// World/room systems spawn + reconcile these; content-free so they need no
/// presentation import.
#[derive(Component, Clone, Debug)]
pub struct LoadingZoneVisual {
    pub id: String,
}
