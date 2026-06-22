//! Authoritative ECS components for an NPC actor + the `NpcMut` view
//! the per-tick brain integration mutates in place.
//!
//! Dissolves the legacy `NpcRuntime` blob (held in `ActorRuntime::Npc`)
//! into real ECS state, mirroring the enemy cluster pattern. NPCs share
//! the actor-generic [`BodyKinematics`] / [`ActorSurfaceState`] /
//! [`ActorMotionPath`] components with enemies; the NPC-specific config
//! (identity, dialogue interactable, patrol/talk radii) and status
//! (ai_mode / hit_flash / hostility / strikes) live in [`NpcConfig`] /
//! [`NpcStatus`].

use bevy::ecs::query::QueryData;
use bevy::prelude::Component;

use super::super::enemies::ActorSurfaceState;
use super::super::path_motion::PathMotion;
use super::enemy_clusters::{ActorMotionPath, BodyKinematics};
use crate::engine_core as ae;
use crate::engine_core::AabbExt;

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
    /// This NPC is in a gravity-free FLIGHT state (a `Floating` catalog body —
    /// the parrot). Distinct from merely being airborne: a jump or knockback is
    /// NOT flight. Drives the `Fly` animation (vs `Idle`/`Walk`) when moving.
    pub aerial: bool,
}

/// Per-tick NPC status: last-evaluated AI mode, hit-flash timer,
/// hostility flag, and accumulated strike count.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct NpcStatus {
    pub ai_mode: crate::actor::ai::CharacterAiMode,
    pub hit_flash: f32,
    pub hostile: bool,
    pub strikes: i32,
}

/// Mutable borrow of every component the NPC tick touches.
pub struct NpcMut<'a> {
    pub kin: &'a mut BodyKinematics,
    pub surface: &'a mut ActorSurfaceState,
    pub motion: &'a mut ActorMotionPath,
    pub config: &'a mut NpcConfig,
    pub status: &'a mut NpcStatus,
}

#[derive(QueryData)]
#[query_data(mutable)]
pub struct NpcClusterQueryData {
    pub kin: &'static mut BodyKinematics,
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
    pub kin: BodyKinematics,
    pub surface: ActorSurfaceState,
    pub motion: ActorMotionPath,
    pub config: NpcConfig,
    pub status: NpcStatus,
    /// Explicit sprite render-quad size when the collision was derived from
    /// published sprite `body_metrics`. The spawn site lifts this onto the
    /// SHARED [`crate::features::ActorRenderSize`] component (so it survives a
    /// peaceful→hostile flip); `None` ⇒ legacy `collision_scale` render path.
    pub render_size: Option<ae::Vec2>,
}

impl NpcClusterScratch {
    /// Build the NPC clusters directly from spawn inputs — the
    /// cluster-native replacement for `NpcRuntime::new_with_paths`.
    pub fn new_with_paths(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        interactable: crate::interaction::Interactable,
        paths: &[(String, crate::actor::KinematicPath)],
    ) -> Self {
        let authored_pos = aabb.center();
        let (patrol_radius, motion) = match &interactable.kind {
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
                (patrol_radius.max(0.0), motion)
            }
            _ => (0.0, None),
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
        // A `Floating` catalog body_kind = a gravity-free flyer (the stochastic
        // parrot). Zero its gravity so the brain's full 2D `desired_vel` drives
        // flight (see `NpcRuntime::integrate_velocity_aerial`). Data-driven via
        // the NPC's catalog `character_id`.
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
        // Sprite metadata supersedes the LDtk spawn box: when the NPC's
        // character has published `body_metrics`, size the collision to the
        // visible body and remember the render-quad size so the sprite still
        // draws at its authored scale. Missing metadata → keep the LDtk box.
        let ldtk_collision = aabb.half_size() * 2.0;
        let body = character_id.and_then(|cid| {
            crate::character_sprites::sprite_body_collision_for_character_id(cid, ldtk_collision)
        });
        let (collision_size, render_size) = match body {
            Some(b) => (b.collision, Some(b.render_size)),
            None => (ldtk_collision, None),
        };
        Self {
            kin: BodyKinematics {
                pos,
                vel: ae::Vec2::ZERO,
                size: collision_size,
                facing: 1.0,
            },
            surface: ActorSurfaceState {
                on_ground: false,
                surface_normal: ae::Vec2::new(0.0, -1.0),
                gravity_scale,
                air_jumps_remaining: 0,
            },
            motion: ActorMotionPath(motion),
            config: NpcConfig {
                id: id.into(),
                name: name.into(),
                spawn: pos,
                interactable,
                patrol_radius,
                talk_radius: super::super::npcs::NPC_TALK_RADIUS,
                aerial: gravity_scale <= 0.001,
            },
            status: NpcStatus {
                ai_mode: crate::actor::ai::CharacterAiMode::Idle,
                hit_flash: 0.0,
                hostile: false,
                strikes: 0,
            },
            render_size,
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
        BodyKinematics,
        ActorSurfaceState,
        ActorMotionPath,
        NpcConfig,
        NpcStatus,
    ) {
        (
            self.kin,
            self.surface,
            self.motion,
            self.config,
            self.status,
        )
    }
}
