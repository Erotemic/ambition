//! Authoritative ECS components for an enemy actor + the `EnemyMut`
//! view that the per-tick integration mutates in place.
//!
//! Enemy state lives as ECS components. Per-tick systems borrow those
//! components through [`EnemyMut`] instead of rebuilding a runtime blob.
//!
//! Field → component map:
//! - pos/vel/size/facing      → [`BodyKinematics`]
//! - on_ground/normal/gravity/air_jumps → [`ActorSurfaceState`] (component)
//! - attack windup/active/cooldown/axis → [`ActorAttackState`] (component)
//! - alive/respawn/hit_flash/ai_mode/health → [`EnemyStatus`]
//! - tuning/brain_spec/brain/spawn baseline/sprite override/id/name → [`EnemyConfig`]
//! - patrol path             → [`ActorMotionPath`]

use bevy::ecs::query::QueryData;
use bevy::prelude::Component;

use super::super::components::ActorAttackState;
use super::super::enemies::{spec_for_brain, ActorSpawnState, ActorSurfaceState, EnemyArchetypeSpec};
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

/// Authored configuration + identity for an enemy actor. Archetype-free
/// by construction: the named roster enum is resolved at spawn and
/// projected into generic kit data (`tuning` + `brain_spec` + the
/// `CombatCapabilities` component), so neither the per-frame integration
/// nor the runtime brain rebuilds (provoke, dismount) call back into the
/// content roster. `spawn` records the authored baseline
/// `reset_to_spawn` restores.
#[derive(Component, Clone, Debug)]
pub struct EnemyConfig {
    pub id: String,
    pub name: String,
    /// Per-frame runtime tuning snapshot (kit vocabulary), projected
    /// from the archetype's authored spec at spawn.
    pub tuning: crate::mechanics::combat::EnemyTuning,
    /// Generic brain-construction inputs (kit vocabulary), projected
    /// from the archetype at spawn so the runtime brain rebuilds
    /// reconstruct a brain without naming the roster enum.
    pub brain_spec: crate::mechanics::combat::EnemyBrainSpec,
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
/// assembled from a Bevy query via [`EnemyClusterQueryData`].
pub struct EnemyMut<'a> {
    pub kin: &'a mut BodyKinematics,
    pub status: &'a mut EnemyStatus,
    pub surface: &'a mut ActorSurfaceState,
    pub attack: &'a mut ActorAttackState,
    pub config: &'a mut EnemyConfig,
    pub motion: &'a mut ActorMotionPath,
    /// Spawn-resolved special-behavior flags (kit vocabulary). Read-only:
    /// the per-frame integration and the damage hook branch on these
    /// instead of calling back into the named archetype enum.
    pub caps: &'a crate::mechanics::combat::CombatCapabilities,
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
    pub caps: &'static crate::mechanics::combat::CombatCapabilities,
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
            caps: self.caps,
        }
    }
}

/// Owned seed used to construct the enemy ECS component cluster before spawn.
/// Runtime systems should query [`EnemyClusterQueryData`] instead.
#[derive(Clone, Debug)]
pub struct EnemyClusterSeed {
    pub kin: BodyKinematics,
    pub status: EnemyStatus,
    pub surface: ActorSurfaceState,
    pub attack: ActorAttackState,
    pub config: EnemyConfig,
    pub motion: ActorMotionPath,
    /// Spawn-resolved special-behavior flags (kit vocabulary), spawned
    /// alongside the clusters by [`Self::into_components`].
    pub caps: crate::mechanics::combat::CombatCapabilities,
    /// The authored roster spec (resolved by string key from the spawn
    /// brain). Spawn-time ONLY: brain / combat-kit / held-item construction
    /// reads it here before the entity exists; it is deliberately NOT
    /// carried onto any spawned component, so the persisted [`EnemyConfig`]
    /// stays roster-free. The named `EnemyArchetype` enum never reaches the
    /// spawn path — only this data does.
    pub spec: EnemyArchetypeSpec,
}

impl EnemyClusterSeed {
    /// Build enemy component seed state from authored spawn inputs.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        brain: crate::actor::EnemyBrain,
        paths: &[(String, crate::actor::KinematicPath)],
    ) -> Self {
        let spec = spec_for_brain(&brain);
        let motion = match &brain {
            crate::actor::EnemyBrain::Patrol {
                path_id: Some(path_id),
            } if !spec.is_sandbag => paths
                .iter()
                .find(|(p_id, _)| p_id == path_id)
                .map(|(_, path)| PathMotion::new(path.clone())),
            _ => None,
        };
        let pos = motion
            .as_ref()
            .and_then(PathMotion::start_pos)
            .unwrap_or_else(|| aabb.center());
        let size = spec
            .default_size
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
                health: crate::actor::Health::new(spec.max_health),
            },
            surface: ActorSurfaceState {
                on_ground: false,
                surface_normal: ae::Vec2::new(0.0, -1.0),
                gravity_scale: if spec.is_aerial { 0.0 } else { 1.0 },
                air_jumps_remaining: MAX_ENEMY_AIR_JUMPS,
            },
            attack: ActorAttackState::default(),
            config: EnemyConfig {
                id: id.into(),
                name: name.into(),
                tuning: spec.tuning(),
                brain_spec: spec.brain_spec(),
                brain,
                spawn: ActorSpawnState { pos, size },
                sprite_override_npc_name: None,
            },
            motion: ActorMotionPath(motion),
            caps: spec.combat_capabilities(),
            spec,
        }
    }
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub fn update_for_test(
        &mut self,
        world: &ae::World,
        target_pos: ae::Vec2,
        tuning: crate::mechanics::combat::FeatureCombatTuning,
        nearest_neighbor: Option<ae::Vec2>,
        dt: f32,
        is_mounted: bool,
        frame: crate::actor::control::ActorControlFrame,
        gravity_sign: f32,
    ) -> crate::actor::control::ActorControlFrame {
        EnemyMut {
            kin: &mut self.kin,
            status: &mut self.status,
            surface: &mut self.surface,
            attack: &mut self.attack,
            config: &mut self.config,
            motion: &mut self.motion,
            caps: &self.caps,
        }
        .update(
            world,
            target_pos,
            tuning,
            nearest_neighbor,
            dt,
            is_mounted,
            frame,
            gravity_sign,
        )
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
        (
            self.kin,
            self.status,
            self.config,
            self.motion,
            self.surface,
            self.attack,
            self.caps,
        )
    }
}
