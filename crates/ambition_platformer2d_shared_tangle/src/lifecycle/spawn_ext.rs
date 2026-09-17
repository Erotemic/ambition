//! `Commands` extensions for lifecycle scopes that have corresponding cleanup
//! sweeps. Entities with no scope marker use ordinary `commands.spawn`.

use bevy::prelude::*;

use super::{ModeScopedEntity, RoomScopedEntity};

/// Spawn helpers that make entity lifecycle policy part of the call site.
pub trait SpawnScopedExt {
    /// Spawn an entity whose lifetime is scoped to the active authored room:
    /// swept when the room unloads by `room_transition::commit`, whose roster is
    /// the `RoomResident` alias — `(With<RoomScopedEntity>, Without<InCustodyOf>)`,
    /// so an object in a hand is not the room's to retire — and by the sandbox
    /// reset. ⚠ This named a `RoomConstructionPlan::retire_outgoing` until <!-- cite-ok: quotes the dead name this correction is about -->
    /// 2026-09-17; no such method exists.
    fn spawn_room_scoped<B: Bundle>(&mut self, bundle: B) -> EntityCommands<'_>;

    /// Spawn an entity whose lifetime is scoped to the named game mode: it
    /// survives room transitions inside that mode and is despawned by
    /// `despawn_departed_mode_entities` when the active room's mode becomes
    /// anything else. This is how a hosted demo's rules plugin owns its
    /// resources without a global state.
    fn spawn_mode_scoped<B: Bundle>(&mut self, mode: &str, bundle: B) -> EntityCommands<'_>;
}

impl<'w, 's> SpawnScopedExt for Commands<'w, 's> {
    fn spawn_room_scoped<B: Bundle>(&mut self, bundle: B) -> EntityCommands<'_> {
        let mut entity = self.spawn(bundle);
        entity.insert(RoomScopedEntity);
        entity
    }

    fn spawn_mode_scoped<B: Bundle>(&mut self, mode: &str, bundle: B) -> EntityCommands<'_> {
        let mut entity = self.spawn(bundle);
        entity.insert(ModeScopedEntity(mode.to_string()));
        entity
    }
}
