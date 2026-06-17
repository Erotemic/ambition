//! `SpawnScopedExt` — `Commands` extension giving each spawn site an explicit
//! lifecycle scope (room / run / persistent).

use bevy::prelude::*;

use super::{PersistentEntity, RoomScopedEntity, RunScopedEntity};

/// Spawn helpers that make entity lifecycle policy part of the call site.
pub trait SpawnScopedExt {
    /// Spawn an entity whose lifetime is scoped to the active authored room.
    fn spawn_room_scoped<B: Bundle>(&mut self, bundle: B) -> EntityCommands<'_>;

    /// Spawn an entity whose lifetime is scoped to the active gameplay run.
    fn spawn_run_scoped<B: Bundle>(&mut self, bundle: B) -> EntityCommands<'_>;

    /// Spawn an entity that intentionally survives room and run lifecycle
    /// cleanup. Prefer this over raw `spawn` when persistence is a design fact.
    fn spawn_persistent<B: Bundle>(&mut self, bundle: B) -> EntityCommands<'_>;
}

impl<'w, 's> SpawnScopedExt for Commands<'w, 's> {
    fn spawn_room_scoped<B: Bundle>(&mut self, bundle: B) -> EntityCommands<'_> {
        let mut entity = self.spawn(bundle);
        entity.insert(RoomScopedEntity);
        entity
    }

    fn spawn_run_scoped<B: Bundle>(&mut self, bundle: B) -> EntityCommands<'_> {
        let mut entity = self.spawn(bundle);
        entity.insert(RunScopedEntity);
        entity
    }

    fn spawn_persistent<B: Bundle>(&mut self, bundle: B) -> EntityCommands<'_> {
        let mut entity = self.spawn(bundle);
        entity.insert(PersistentEntity);
        entity
    }
}
