//! Authoritative ECS components for an NPC actor + the `NpcMut` view
//! the per-tick brain integration mutates in place.
//!
//! Dissolves the legacy `NpcRuntime` blob (held in `ActorRuntime::Npc`)
//! into real ECS state, mirroring the enemy cluster pattern. NPCs share
//! the actor-generic [`ActorKinematics`] / [`ActorSurfaceState`] /
//! [`ActorMotionPath`] components with enemies; the NPC-specific config
//! (identity, dialogue interactable, patrol/talk radii) and status
//! (ai_mode / hit_flash / hostility / strikes) live in [`NpcConfig`] /
//! [`NpcStatus`].

use bevy::ecs::query::QueryData;
use bevy::prelude::Component;

use super::super::enemies::ActorSurfaceState;
use super::enemy_clusters::{ActorKinematics, ActorMotionPath};
use crate::engine_core as ae;

/// Authored configuration + identity for an NPC actor.
#[derive(Component, Clone, Debug)]
pub struct NpcConfig {
    pub id: String,
    pub name: String,
    /// Authored spawn position; patrol bounds derive from `spawn.x ±
    /// patrol_radius` and `reset` restores `pos` to it.
    pub spawn: ae::Vec2,
    pub interactable: crate::interaction::Interactable,
    pub patrol_radius: f32,
    pub talk_radius: f32,
}

/// Per-tick NPC status: last-evaluated AI mode, hit-flash timer,
/// hostility flag, and accumulated strike count.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct NpcStatus {
    pub ai_mode: crate::character_ai::CharacterAiMode,
    pub hit_flash: f32,
    pub hostile: bool,
    pub strikes: i32,
}

/// Mutable borrow of every component the NPC tick touches.
pub struct NpcMut<'a> {
    pub kin: &'a mut ActorKinematics,
    pub surface: &'a mut ActorSurfaceState,
    pub motion: &'a mut ActorMotionPath,
    pub config: &'a mut NpcConfig,
    pub status: &'a mut NpcStatus,
}

#[derive(QueryData)]
#[query_data(mutable)]
pub struct NpcClusterQueryData {
    pub kin: &'static mut ActorKinematics,
    pub surface: &'static mut ActorSurfaceState,
    pub motion: &'static mut ActorMotionPath,
    pub config: &'static mut NpcConfig,
    pub status: &'static mut NpcStatus,
}

impl<'w, 's> NpcClusterQueryDataItem<'w, 's> {
    pub fn as_npc_mut<'a>(&'a mut self) -> NpcMut<'a>
    where
        'w: 'a,
        's: 'a,
    {
        NpcMut {
            kin: &mut self.kin,
            surface: &mut self.surface,
            motion: &mut self.motion,
            config: &mut self.config,
            status: &mut self.status,
        }
    }
}

/// Owned aggregate for spawn construction / non-ECS callers.
#[derive(Clone, Debug)]
pub struct NpcClusterScratch {
    pub kin: ActorKinematics,
    pub surface: ActorSurfaceState,
    pub motion: ActorMotionPath,
    pub config: NpcConfig,
    pub status: NpcStatus,
}

impl NpcClusterScratch {
    /// Build the clusters from a legacy `NpcRuntime` (spawn transition aid).
    pub fn from_runtime(n: &super::super::npcs::NpcRuntime) -> Self {
        Self {
            kin: ActorKinematics {
                pos: n.pos,
                vel: n.vel,
                size: n.size,
                facing: n.facing,
            },
            surface: ActorSurfaceState {
                on_ground: n.on_ground,
                surface_normal: ae::Vec2::new(0.0, -1.0),
                gravity_scale: 1.0,
                air_jumps_remaining: 0,
            },
            motion: ActorMotionPath(n.motion.clone()),
            config: NpcConfig {
                id: n.id.clone(),
                name: n.name.clone(),
                spawn: n.spawn,
                interactable: n.interactable.clone(),
                patrol_radius: n.patrol_radius,
                talk_radius: n.talk_radius,
            },
            status: NpcStatus {
                ai_mode: n.ai_mode,
                hit_flash: n.hit_flash,
                hostile: n.hostile,
                strikes: n.strikes,
            },
        }
    }

    pub fn as_mut(&mut self) -> NpcMut<'_> {
        NpcMut {
            kin: &mut self.kin,
            surface: &mut self.surface,
            motion: &mut self.motion,
            config: &mut self.config,
            status: &mut self.status,
        }
    }

    pub fn into_components(
        self,
    ) -> (
        ActorKinematics,
        ActorSurfaceState,
        ActorMotionPath,
        NpcConfig,
        NpcStatus,
    ) {
        (self.kin, self.surface, self.motion, self.config, self.status)
    }
}

/// Spawnable NPC-cluster bundle built from a legacy `NpcRuntime`.
pub fn npc_cluster_bundle(
    n: &super::super::npcs::NpcRuntime,
) -> (
    ActorKinematics,
    ActorSurfaceState,
    ActorMotionPath,
    NpcConfig,
    NpcStatus,
) {
    NpcClusterScratch::from_runtime(n).into_components()
}
