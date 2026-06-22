//! Authoritative ECS components for the UNIFIED actor cluster (every actor —
//! was-NPC, was-enemy, encounter mob — shares this one cluster) + the `ActorMut`
//! view the per-tick integration mutates in place.
//!
//! Actor state lives as ECS components. Per-tick systems borrow those components
//! through [`ActorMut`] instead of rebuilding a runtime blob. Hostility is the
//! `ActorDisposition` state, not a cluster *type*.
//!
//! Field → component map:
//! - pos/vel/size/facing      → [`BodyKinematics`]
//! - on_ground/normal/gravity/air_jumps → [`ActorSurfaceState`] (component)
//! - attack windup/active/cooldown/axis → [`ActorAttackState`] (component)
//! - alive/respawn/hit_flash/ai_mode/health → [`ActorStatus`]
//! - tuning/brain_spec/brain/spawn baseline/sprite override/id/name → [`ActorConfig`]
//! - patrol path             → [`ActorMotionPath`]

use bevy::ecs::query::QueryData;
use bevy::prelude::Component;

use super::super::components::ActorAttackState;
use super::super::enemies::{
    spec_for_brain, ActorSpawnState, ActorSurfaceState, EnemyArchetypeSpec,
};
use super::super::path_motion::PathMotion;
use super::super::MAX_ENEMY_AIR_JUMPS;
use crate::engine_core as ae;
use crate::engine_core::AabbExt;

pub use crate::platformer_runtime::body::BodyKinematics;

/// Liveness + per-tick status scalars: alive flag, respawn countdown,
/// hit-flash timer, last-evaluated AI mode, and current health.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ActorStatus {
    pub alive: bool,
    pub respawn_timer: f32,
    pub hit_flash: f32,
    pub ai_mode: crate::actor::ai::CharacterAiMode,
    pub health: crate::actor::Health,
}

/// Authored configuration + identity for an actor (any disposition). Archetype-
/// free by construction: the named roster enum is resolved at spawn and projected
/// into generic kit data (`tuning` + `brain_spec` + the `CombatCapabilities`
/// component), so neither the per-frame integration nor the runtime brain
/// rebuilds (provoke, dismount) call back into the content roster. `spawn` records
/// the authored baseline `reset_to_spawn` restores.
#[derive(Component, Clone, Debug)]
pub struct ActorConfig {
    pub id: String,
    pub name: String,
    /// Per-frame runtime tuning snapshot (kit vocabulary), projected
    /// from the archetype's authored spec at spawn.
    pub tuning: crate::combat::ActorTuning,
    /// Generic brain-construction inputs (kit vocabulary), projected
    /// from the archetype at spawn so the runtime brain rebuilds
    /// reconstruct a brain without naming the roster enum.
    pub brain_spec: crate::combat::EnemyBrainSpec,
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
/// assembled from a Bevy query via [`ActorClusterQueryData`].
pub struct ActorMut<'a> {
    pub kin: &'a mut BodyKinematics,
    pub status: &'a mut ActorStatus,
    pub surface: &'a mut ActorSurfaceState,
    pub attack: &'a mut ActorAttackState,
    pub config: &'a mut ActorConfig,
    pub motion: &'a mut ActorMotionPath,
    /// Spawn-resolved special-behavior flags (kit vocabulary). Read-only:
    /// the per-frame integration and the damage hook branch on these
    /// instead of calling back into the named archetype enum.
    pub caps: &'a crate::combat::CombatCapabilities,
}

#[derive(QueryData)]
#[query_data(mutable)]
pub struct ActorClusterQueryData {
    pub kin: &'static mut BodyKinematics,
    pub status: &'static mut ActorStatus,
    pub surface: &'static mut ActorSurfaceState,
    pub attack: &'static mut ActorAttackState,
    pub config: &'static mut ActorConfig,
    pub motion: &'static mut ActorMotionPath,
    pub caps: &'static crate::combat::CombatCapabilities,
}

impl<'w, 's> ActorClusterQueryDataItem<'w, 's> {
    /// Borrow the components as an [`ActorMut`] view for one tick.
    pub fn as_actor_mut<'a>(&'a mut self) -> ActorMut<'a>
    where
        'w: 'a,
        's: 'a,
    {
        ActorMut {
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
/// Runtime systems should query [`ActorClusterQueryData`] instead.
#[derive(Clone, Debug)]
pub struct ActorClusterSeed {
    pub kin: BodyKinematics,
    pub status: ActorStatus,
    pub surface: ActorSurfaceState,
    pub attack: ActorAttackState,
    pub config: ActorConfig,
    pub motion: ActorMotionPath,
    /// Spawn-resolved special-behavior flags (kit vocabulary), spawned
    /// alongside the clusters by [`Self::into_components`].
    pub caps: crate::combat::CombatCapabilities,
    /// The authored roster spec (resolved by string key from the spawn
    /// brain). Spawn-time ONLY: brain / combat-kit / held-item construction
    /// reads it here before the entity exists; it is deliberately NOT
    /// carried onto any spawned component, so the persisted [`ActorConfig`]
    /// stays roster-free. The named `EnemyArchetype` enum never reaches the
    /// spawn path — only this data does. `pub(crate)`: the seed type itself is
    /// publicly re-exported (content builds peaceful seeds) but this archetype
    /// field is internal-only.
    pub(crate) spec: EnemyArchetypeSpec,
}

impl ActorClusterSeed {
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
        let size = spec.default_size.unwrap_or_else(|| aabb.half_size() * 2.0);
        Self {
            kin: BodyKinematics {
                pos,
                vel: ae::Vec2::ZERO,
                size,
                facing: -1.0,
            },
            status: ActorStatus {
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
            config: ActorConfig {
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
    /// Build a PEACEFUL actor seed from catalog/NPC spawn inputs — the unified
    /// replacement for `NpcClusterScratch::new_with_paths`. A peaceful actor is
    /// the same cluster as a hostile enemy, just with peaceful tuning
    /// (`attacks_player = false`, zero aggro, `max_run_speed = NPC_PATROL_SPEED`,
    /// `health = 1`) and a `Passive`/`Patrol` AI brain; its movement is driven by
    /// the catalog `Brain` component attached at spawn, not by this `config.brain`
    /// (which only feeds the integrator's patrol-stall intent). The seed's `spec`
    /// field is filled with an inert default (peaceful actors never spawn through
    /// the archetype path), so callers — including the content crate — need no
    /// `EnemyArchetypeSpec`. Returns the seed plus the optional sprite render size
    /// (lifted onto the shared `ActorRenderSize` at spawn so it survives a flip).
    pub fn new_peaceful_npc(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        interactable: &crate::interaction::Interactable,
        paths: &[(String, crate::actor::KinematicPath)],
    ) -> (Self, Option<ae::Vec2>) {
        let authored_pos = aabb.center();
        let (patrol_radius, patrol_path_id, motion) = match &interactable.kind {
            crate::interaction::InteractionKind::Npc {
                patrol_radius,
                patrol_path_id,
                ..
            } => {
                let motion = patrol_path_id.as_deref().and_then(|path_id| {
                    paths
                        .iter()
                        .find(|(p_id, _)| p_id == path_id)
                        .map(|(_, path)| PathMotion::new(path.clone()))
                });
                (patrol_radius.max(0.0), patrol_path_id.clone(), motion)
            }
            _ => (0.0, None, None),
        };
        let pos = motion
            .as_ref()
            .and_then(PathMotion::start_pos)
            .unwrap_or(authored_pos);
        let character_id = match &interactable.kind {
            crate::interaction::InteractionKind::Npc {
                character_id: Some(cid),
                ..
            } => Some(cid.as_str()),
            _ => None,
        };
        // A `Floating` catalog body = a gravity-free flyer (the stochastic
        // parrot): zero gravity so the brain's full 2D velocity drives flight
        // through the shared aerial integrator.
        let gravity_scale = match character_id {
            Some(cid)
                if matches!(
                    crate::character_roster::body_kind_for_character_id(cid),
                    Some(crate::actor::character_catalog::CharacterBodyKind::Floating)
                ) =>
            {
                0.0
            }
            _ => 1.0,
        };
        let is_aerial = gravity_scale <= 0.001;
        // Sprite metadata supersedes the LDtk spawn box (see the old
        // `NpcClusterScratch`): size the collision to the visible body and
        // remember the render-quad size so the sprite still draws at scale.
        let ldtk_collision = aabb.half_size() * 2.0;
        let body = character_id.and_then(|cid| {
            crate::character_sprites::sprite_body_collision_for_character_id(cid, ldtk_collision)
        });
        let (collision_size, render_size) = match body {
            Some(b) => (b.collision, Some(b.render_size)),
            None => (ldtk_collision, None),
        };
        let has_patrol = patrol_radius > 0.0 || motion.is_some();
        let tuning = crate::combat::ActorTuning {
            max_health: 1,
            patrol_speed: crate::brain::NPC_PATROL_SPEED,
            chase_speed: crate::brain::NPC_PATROL_SPEED,
            max_run_speed: crate::brain::NPC_PATROL_SPEED,
            is_aerial,
            ..Default::default()
        };
        let config_brain = if has_patrol {
            crate::actor::EnemyBrain::Patrol {
                path_id: patrol_path_id,
            }
        } else {
            crate::actor::EnemyBrain::Passive
        };
        let seed = Self {
            kin: BodyKinematics {
                pos,
                vel: ae::Vec2::ZERO,
                size: collision_size,
                facing: 1.0,
            },
            status: ActorStatus {
                alive: true,
                respawn_timer: 0.0,
                hit_flash: 0.0,
                ai_mode: crate::actor::ai::CharacterAiMode::Idle,
                health: crate::actor::Health::new(1),
            },
            surface: ActorSurfaceState {
                on_ground: false,
                surface_normal: ae::Vec2::new(0.0, -1.0),
                gravity_scale,
                air_jumps_remaining: 0,
            },
            attack: ActorAttackState::default(),
            config: ActorConfig {
                id: id.into(),
                name: name.into(),
                tuning,
                brain_spec: crate::combat::EnemyBrainSpec::default(),
                brain: config_brain,
                spawn: ActorSpawnState {
                    pos,
                    size: collision_size,
                },
                sprite_override_npc_name: None,
            },
            motion: ActorMotionPath(motion),
            caps: crate::combat::CombatCapabilities::default(),
            // Inert: peaceful actors never spawn through the archetype path that
            // reads `spec`. `Passive` resolves to the roster's fallback row.
            spec: spec_for_brain(&crate::actor::EnemyBrain::Passive),
        };
        (seed, render_size)
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub fn update_for_test(
        &mut self,
        world: &ae::World,
        target_pos: ae::Vec2,
        tuning: crate::combat::FeatureCombatTuning,
        nearest_neighbor: Option<ae::Vec2>,
        dt: f32,
        is_mounted: bool,
        frame: crate::actor::control::ActorControlFrame,
        gravity_dir: ae::Vec2,
    ) -> crate::actor::control::ActorControlFrame {
        ActorMut {
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
            gravity_dir,
        )
    }

    /// The six authoritative components as a spawnable Bundle.
    pub fn into_components(
        self,
    ) -> (
        BodyKinematics,
        ActorStatus,
        ActorConfig,
        ActorMotionPath,
        ActorSurfaceState,
        ActorAttackState,
        crate::combat::CombatCapabilities,
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
