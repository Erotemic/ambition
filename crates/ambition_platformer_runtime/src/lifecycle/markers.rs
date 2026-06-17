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
