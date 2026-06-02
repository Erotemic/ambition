//! Authoritative ECS components for an enemy actor + the `EnemyMut`
//! view that the per-tick integration mutates in place.
//!
//! This dissolves the legacy `EnemyRuntime` blob (which lived inside the
//! `ActorRuntime::Enemy` enum and was shadowed by one-way mirror
//! components) into real ECS state, following the player cluster
//! pattern (`engine_core::player_clusters`): each concept is a
//! component, and the integration borrows them all through a single
//! view struct rather than reconstructing a runtime scratchpad.
//!
//! Field → component map (see `dev/reviews/enemyruntime-ecs-inventory.md`):
//! - pos/vel/size/facing      → [`EnemyKinematics`]
//! - on_ground/normal/gravity/air_jumps → [`ActorSurfaceState`] (component)
//! - attack windup/active/cooldown/axis → [`ActorAttackState`] (component)
//! - alive/respawn/hit_flash/ai_mode/health → [`EnemyStatus`]
//! - archetype/brain/spawn baseline/sprite override/id/name → [`EnemyConfig`]
//! - patrol path             → [`EnemyMotionPath`]

use bevy::ecs::query::QueryData;
use bevy::prelude::Component;

use super::super::components::ActorAttackState;
use super::super::enemies::{ActorSpawnState, ActorSurfaceState, EnemyArchetype};
use super::super::path_motion::PathMotion;
use crate::engine_core as ae;

/// Authoritative kinematic state (position / velocity / body size /
/// facing). Mirrors the player's `PlayerKinematics`.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct EnemyKinematics {
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    pub size: ae::Vec2,
    pub facing: f32,
}

/// Liveness + per-tick status scalars: alive flag, respawn countdown,
/// hit-flash timer, last-evaluated AI mode, and current health.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct EnemyStatus {
    pub alive: bool,
    pub respawn_timer: f32,
    pub hit_flash: f32,
    pub ai_mode: crate::character_ai::CharacterAiMode,
    pub health: crate::actor::Health,
}

/// Authored configuration + identity for an enemy actor. `archetype`
/// can mutate at runtime (PirateOnShark dismounts), so it is not const;
/// `spawn` records the authored baseline `reset_to_spawn` restores.
#[derive(Component, Clone, Debug)]
pub struct EnemyConfig {
    pub id: String,
    pub name: String,
    pub archetype: EnemyArchetype,
    pub brain: crate::actor::EnemyBrain,
    pub spawn: ActorSpawnState,
    /// LDtk display name of the original NPC when this enemy was spawned
    /// by migrating a hostile NPC (keeps its own sprite sheet). `None`
    /// uses the default enemy sprite.
    pub sprite_override_npc_name: Option<String>,
}

/// Optional patrol path the kinematic step advances each tick.
#[derive(Component, Clone, Debug, Default)]
pub struct EnemyMotionPath(pub Option<PathMotion>);

/// Mutable borrow of every component the enemy integration touches,
/// assembled from a Bevy query via [`EnemyClusterQueryData`]. Field
/// names mirror the old `EnemyRuntime` layout so the ported integration
/// reads naturally (`self.kin.pos`, `self.surface.on_ground`,
/// `self.attack.cooldown`, `self.status.alive`, `self.config.archetype`).
pub struct EnemyMut<'a> {
    pub kin: &'a mut EnemyKinematics,
    pub status: &'a mut EnemyStatus,
    pub surface: &'a mut ActorSurfaceState,
    pub attack: &'a mut ActorAttackState,
    pub config: &'a mut EnemyConfig,
    pub motion: &'a mut EnemyMotionPath,
}

#[derive(QueryData)]
#[query_data(mutable)]
pub struct EnemyClusterQueryData {
    pub kin: &'static mut EnemyKinematics,
    pub status: &'static mut EnemyStatus,
    pub surface: &'static mut ActorSurfaceState,
    pub attack: &'static mut ActorAttackState,
    pub config: &'static mut EnemyConfig,
    pub motion: &'static mut EnemyMotionPath,
}

impl<'w, 's> EnemyClusterQueryDataItem<'w, 's> {
    /// Borrow the components as an [`EnemyMut`] view for one tick.
    pub fn as_enemy_mut<'a>(&'a mut self) -> EnemyMut<'a>
    where
        'w: 'a,
        's: 'a,
    {
        EnemyMut {
            kin: &mut self.kin,
            status: &mut self.status,
            surface: &mut self.surface,
            attack: &mut self.attack,
            config: &mut self.config,
            motion: &mut self.motion,
        }
    }
}

/// Owned aggregate of the enemy clusters, for spawn construction and
/// non-ECS callers (e.g. the NPC→enemy hostility conversion). Mirrors
/// the player's `PlayerClusterScratch`.
#[derive(Clone, Debug)]
pub struct EnemyClusterScratch {
    pub kin: EnemyKinematics,
    pub status: EnemyStatus,
    pub surface: ActorSurfaceState,
    pub attack: ActorAttackState,
    pub config: EnemyConfig,
    pub motion: EnemyMotionPath,
}

impl EnemyClusterScratch {
    pub fn as_mut(&mut self) -> EnemyMut<'_> {
        EnemyMut {
            kin: &mut self.kin,
            status: &mut self.status,
            surface: &mut self.surface,
            attack: &mut self.attack,
            config: &mut self.config,
            motion: &mut self.motion,
        }
    }
}
