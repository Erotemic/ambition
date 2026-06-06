use bevy::prelude::*;

/// Lifetime-scope marker: despawn when the current authored room is unloaded.
///
/// This marker is deliberately runtime-owned so simulation-only entities and
/// rendered room visuals can share the same lifecycle policy without depending
/// on presentation modules.
#[derive(Component, Default)]
pub struct RoomScopedEntity;

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
