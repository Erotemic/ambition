//! Runtime schedule vocabulary that is independent of Ambition content.
//!
//! `SandboxSet` remains the concrete app schedule for now. These labels document
//! the future crate-level concepts and give new runtime modules names that do
//! not depend on app assembly details.

use bevy::prelude::*;

/// Generic platformer runtime phases.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum PlatformerRuntimeSet {
    /// Build or refresh world-derived runtime inputs before actors tick.
    WorldPrep,
    /// Translate input/control intent into actor control frames.
    ControlInput,
    /// Integrate actors, held items, projectiles, and other gameplay bodies.
    ActorSimulation,
    /// Handle room unload/load, room-scoped cleanup, and authored room respawn.
    RoomLifecycle,
    /// Resolve damage, hitboxes, combat intents, and gameplay consequences.
    Combat,
    /// Publish simulation state to presentation-facing mirrors/caches.
    PresentationSync,
}
