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
//! - pos/vel/size/facing      → [`BodyKinematics`]
//! - on_ground/normal/gravity/air_jumps → [`ActorSurfaceState`] (component)
//! - attack windup/active/cooldown/axis → [`ActorAttackState`] (component)
//! - alive/respawn/hit_flash/ai_mode/health → [`EnemyStatus`]
//! - archetype/brain/spawn baseline/sprite override/id/name → [`EnemyConfig`]
//! - patrol path             → [`ActorMotionPath`]

use bevy::ecs::query::QueryData;
use bevy::prelude::Component;

use super::super::components::ActorAttackState;
use super::super::enemies::{ActorSpawnState, ActorSurfaceState, EnemyArchetype};
use super::super::path_motion::PathMotion;
use super::super::MAX_ENEMY_AIR_JUMPS;
use crate::engine_core as ae;
use crate::engine_core::AabbExt;

pub use crate::platformer_runtime::body::BodyKinematics;

/// Liveness + per-tick status scalars: alive flag, respawn countdown,
/// hit-flash timer, last-evaluated AI mode, and current health.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct EnemyStatus {
    pub alive: bool,
    pub respawn_timer: f32,
    pub hit_flash: f32,
    pub ai_mode: crate::actor::ai::CharacterAiMode,
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
    /// Per-frame runtime tuning snapshot (kit vocabulary), projected
    /// from the archetype's authored spec at spawn.
    pub tuning: crate::mechanics::combat::EnemyTuning,
    pub brain: crate::actor::EnemyBrain,
    pub spawn: ActorSpawnState,
    /// LDtk display name of the original NPC when this enemy was spawned
    /// by migrating a hostile NPC (keeps its own sprite sheet). `None`
    /// uses the default enemy sprite.
    pub sprite_override_npc_name: Option<String>,
}

/// Optional patrol path the kinematic step advances each tick.
#[derive(Component, Clone, Debug, Default)]
pub struct ActorMotionPath(pub Option<PathMotion>);

/// Mutable borrow of every component the enemy integration touches,
/// assembled from a Bevy query via [`EnemyClusterQueryData`]. Field
/// names mirror the old `EnemyRuntime` layout so the ported integration
/// reads naturally (`self.kin.pos`, `self.surface.on_ground`,
/// `self.attack.cooldown`, `self.status.alive`, `self.config.archetype`).
pub struct EnemyMut<'a> {
    pub kin: &'a mut BodyKinematics,
    pub status: &'a mut EnemyStatus,
    pub surface: &'a mut ActorSurfaceState,
    pub attack: &'a mut ActorAttackState,
    pub config: &'a mut EnemyConfig,
    pub motion: &'a mut ActorMotionPath,
}

#[derive(QueryData)]
#[query_data(mutable)]
pub struct EnemyClusterQueryData {
    pub kin: &'static mut BodyKinematics,
    pub status: &'static mut EnemyStatus,
    pub surface: &'static mut ActorSurfaceState,
    pub attack: &'static mut ActorAttackState,
    pub config: &'static mut EnemyConfig,
    pub motion: &'static mut ActorMotionPath,
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
    pub kin: BodyKinematics,
    pub status: EnemyStatus,
    pub surface: ActorSurfaceState,
    pub attack: ActorAttackState,
    pub config: EnemyConfig,
    pub motion: ActorMotionPath,
}

impl EnemyClusterScratch {
    /// Build the enemy clusters directly from spawn inputs — the
    /// cluster-native replacement for `EnemyRuntime::new`. Every spawn
    /// site and the NPC→enemy flip construct one of these instead of a
    /// legacy blob.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        brain: crate::actor::EnemyBrain,
        paths: &[(String, crate::actor::KinematicPath)],
    ) -> Self {
        let archetype = EnemyArchetype::from_brain(&brain);
        let motion = match &brain {
            crate::actor::EnemyBrain::Patrol {
                path_id: Some(path_id),
            } if !archetype.is_sandbag() => paths
                .iter()
                .find(|(p_id, _)| p_id == path_id)
                .map(|(_, path)| PathMotion::new(path.clone())),
            _ => None,
        };
        let pos = motion
            .as_ref()
            .and_then(PathMotion::start_pos)
            .unwrap_or_else(|| aabb.center());
        let size = archetype
            .default_size()
            .unwrap_or_else(|| aabb.half_size() * 2.0);
        Self {
            kin: BodyKinematics {
                pos,
                vel: ae::Vec2::ZERO,
                size,
                facing: -1.0,
            },
            status: EnemyStatus {
                alive: true,
                respawn_timer: 0.0,
                hit_flash: 0.0,
                ai_mode: crate::actor::ai::CharacterAiMode::Idle,
                health: crate::actor::Health::new(archetype.max_health()),
            },
            surface: ActorSurfaceState {
                on_ground: false,
                surface_normal: ae::Vec2::new(0.0, -1.0),
                gravity_scale: if archetype.is_aerial() { 0.0 } else { 1.0 },
                air_jumps_remaining: MAX_ENEMY_AIR_JUMPS,
            },
            attack: ActorAttackState::default(),
            config: EnemyConfig {
                id: id.into(),
                name: name.into(),
                archetype,
                tuning: archetype.tuning(),
                brain,
                spawn: ActorSpawnState {
                    pos,
                    archetype,
                    size,
                },
                sprite_override_npc_name: None,
            },
            motion: ActorMotionPath(motion),
        }
    }
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

    /// The six authoritative components as a spawnable Bundle.
    pub fn into_components(
        self,
    ) -> (
        BodyKinematics,
        EnemyStatus,
        EnemyConfig,
        ActorMotionPath,
        ActorSurfaceState,
        ActorAttackState,
        crate::mechanics::combat::CombatCapabilities,
    ) {
        let caps = self.config.archetype.combat_capabilities();
        (
            self.kin,
            self.status,
            self.config,
            self.motion,
            self.surface,
            self.attack,
            caps,
        )
    }
}
