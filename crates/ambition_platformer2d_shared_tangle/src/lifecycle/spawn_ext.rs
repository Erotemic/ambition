//! `SpawnScopedExt` — `Commands` extension giving each spawn site an explicit
//! lifecycle scope.
//!
//! ⛔ **a verb here exists only if a sweep enforces it.** This trait used to
//! offer four: `spawn_run_scoped` and `spawn_persistent` inserted markers that
//! nothing anywhere read, so a call site declaring "dies with the run" got an
//! entity that outlived every boundary in the engine. Both were deleted with
//! their markers (D125 census, 2026-08-15). The run lifetime is
//! [`SessionSpawnScope`](super::SessionSpawnScope) — captured at request time,
//! id-carrying, and swept by `despawn_retired_session_entities` — and "survives
//! everything" is a plain `commands.spawn`, because every sweep culls on the
//! PRESENCE of its own marker.

use bevy::prelude::*;

use super::{ModeScopedEntity, RoomScopedEntity};

/// Spawn helpers that make entity lifecycle policy part of the call site.
pub trait SpawnScopedExt {
    /// Spawn an entity whose lifetime is scoped to the active authored room:
    /// retired by `RoomConstructionPlan::retire_outgoing` when the room unloads,
    /// and by the sandbox reset.
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
