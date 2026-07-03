//! ECS backbone of the actor / world-object simulation.
//!
//! Despite the `features/` name, this is NOT a set of toggleable feature
//! slices — it is the enemy / NPC / boss ACTOR SIMULATION plus the authored
//! room objects they share a world with (pickups, chests, breakables,
//! switches, hazards, mounts). Every one is a Bevy entity spawned and ticked
//! here; this is the authoritative implementation.
//!
//! Module map (each sibling owns one slice of that sim):
//! - `spawn*` — spawn authored room objects, encounter mobs, mounts/riders;
//! - `actors` / `actor_clusters` / `bosses` — the per-frame actor tick over the
//!   unified ECS cluster components that hold actor state (NPCs + enemies share
//!   one cluster; bosses are their own);
//! - `damage*` / `aggression` / `interact` — hit routing, provocation, and
//!   player interactions;
//! - `encounter_rewards` / `reset` / `save_sync` — reward chests, room reset,
//!   and save-state mirroring;
//! - `view_index` / `anim_helpers` / `target_volumes` — per-frame read models
//!   and overlays consumed by presentation, engine, and combat code.
//!
//! Facade: many `ecs::<name>` paths re-export from the reusable
//! `combat` kit (`banner`, `breakables`, `chests`, `hazards`,
//! `hitbox`, `overlay`, `pickups`, `boss_clusters`, ...) so call sites stay
//! stable while the generic mechanics live DOWN in that kit (ADR 0019).

use super::*;
// `BodyCombat`/`BodyHealth` live on the reusable actor crate (D2). This module
// surfaces them to the `ecs/` submodules that name `super::BodyCombat` — the
// `super::*` glob no longer carries them since the `features` facade stopped
// re-exporting the shared body vocabulary.
use ambition_characters::actor::{BodyCombat, BodyHealth};
#[cfg(test)]
use ambition_sfx::SfxMessage;
use crate::platformer_runtime::lifecycle::RoomVisual;
use crate::world::physics::{DebrisBurstMessage, PhysicsDebrisCue};
use ambition_vfx::vfx::{ParticleKind, VfxMessage};
use bevy::prelude::{
    Commands, Component, Entity, MessageReader, MessageWriter, NextState, Query, Res, ResMut,
    Resource, With, Without,
};

use ambition_time::WorldTime;

pub mod actor_clusters;
mod actors;
mod aggression;
mod anim_helpers;
mod bosses;
mod brain_builders;
mod brain_effects;
mod damage;
mod damage_drops;
mod damage_predicates;
mod encounter_rewards;
#[cfg(test)]
mod fighter_harness;
mod interact;
mod mount;
pub mod perception;
pub use mount::{rider_hand_world_pos, rider_hand_world_pos_in_frame};
mod reset;
mod save_sync;
mod spawn;
mod spawn_actors;
mod spawn_mounts;
mod target_volumes;
mod view_index;

// Combat-kit aliases keep `ecs::<module>` paths stable for callers.
pub use crate::combat::boss_clusters;
pub use crate::combat::{
    banner, breakables, chests, falling_chest, hazards, held_items, hitbox, overlay, pickups,
    spawn_static, targeting, variation,
};

pub use actors::{
    actor_component_snapshot, enemy_component_snapshot, sync_actor_components_from_cluster,
};
// Test-only re-export: `conversion_tests` pins this resolver via the `features::ecs`
// path. Production callers use it through its own `actors::conversion` module, so
// the re-export is unused (a dead import warning) outside `cfg(test)`.
#[cfg(test)]
pub(crate) use actors::hostile_brain_id_for_actor;
pub use actors::{
    apply_actor_contact_damage, integrate_sim_bodies, sync_actor_poses_from_feature_aabbs,
    sync_actor_read_model, tick_actor_brains, tick_npc_idle_barks, ActorSteering,
};
pub use aggression::{
    apply_actor_stimuli, tick_pending_challenges, PendingChallenge, CHALLENGE_GRACE_S,
};
pub use anim_helpers::{
    advance_actor_anim_overlays, ecs_boss_anim_state, ecs_boss_anim_state_and_entity,
    ecs_boss_animation_frame_sample, ecs_breakable_state, ecs_chest_opened,
    rebuild_actor_anim_index, ActorAnimFrame, ActorAnimIndex, ActorSpriteData,
};
pub use banner::{apply_gameplay_banner_requests, tick_gameplay_banner};
pub use boss_clusters::{
    boss_is_cleared, BossClusterQueryData, BossClusterRef, BossClusterScratch, BossConfig, BossMut,
    BossRef, BossEncounter,
};
pub(crate) use bosses::boss_component_snapshot;
#[allow(
    unused_imports,
    reason = "marker re-exported for tests / external visualizers"
)]
pub use bosses::BossSpriteMetricsApplied;
pub use bosses::{
    boss_spawn_hurtboxes, derive_boss_sprite_metrics, integrate_boss_bodies,
    sync_boss_actor_components, sync_boss_encounter_phase,
    tick_boss_brains_system, trigger_boss_attack_moves, update_ecs_bosses,
};
pub use brain_effects::spawn_enemy_projectiles_from_brain_actions;
pub use breakables::update_ecs_breakables;
pub use chests::open_ecs_chests;
pub use damage::apply_feature_hit_events;
pub use damage_predicates::{
    ecs_hit_event_hits_actor, ecs_hit_event_hits_boss, ecs_hit_event_hits_breakable,
};
pub use encounter_rewards::{
    clear_encounter_reward_ecs, sync_boss_reward_chests_ecs, sync_encounter_reward_chests_ecs,
};
pub use falling_chest::update_ecs_falling_chests;
pub use hazards::update_ecs_hazards;
pub use held_items::HeldItem;
pub use hitbox::{
    apply_hitbox_damage, spawn_melee_hitbox, tick_and_despawn_hitboxes, Hitbox, HitboxAnchor,
    HitboxHits, HitboxLifetime,
};
pub use interact::interact_ecs_actors_and_switches;
pub use mount::{
    enforce_mount_rider_link, pirate_on_shark_rider_offset, sync_riders_to_mounts, Mass, MountSlot,
    Mountable, Mounted, MountedBrainCache, MountedSize, RidingOn,
};
pub use overlay::{rebuild_feature_ecs_world_overlay, FeatureEcsWorldOverlay};
pub use pickups::{collect_ecs_pickups, magnetize_pickups};
pub use reset::reset_ecs_room_features;
pub use save_sync::{
    sync_ecs_actors_with_save, sync_ecs_bosses_with_save, sync_ecs_switches_from_save,
};
pub(crate) use spawn::spawn_runtime_minion;
pub use spawn::{despawn_encounter_mobs, spawn_encounter_mob, spawn_room_feature_entities};
pub use spawn_actors::{
    apply_spawn_actor_requests, apply_summon_effects, BossOverrides, SpawnActorKind,
    SpawnActorRequest,
};
pub use target_volumes::{
    derive_pogo_target_volumes, refresh_actor_damageable_volumes, refresh_boss_damageable_volumes,
    refresh_breakable_damageable_volumes,
};
pub use targeting::{
    can_damage, damage_lands, dissolve_settled_grudges, select_actor_targets, FactionRelations,
    FriendlyFire,
};
pub use view_index::{
    rebuild_actor_render_index, rebuild_boss_render_index, rebuild_feature_view_index,
    ActorRenderIndex, ActorRenderView, BossRenderIndex, BossRenderView, FeatureViewIndex,
};

// `FeatureSimEntity` is a generic entity-marker queried by the reusable
// mechanics, so its definition lives DOWN in
// `ambition_platformer_primitives::markers` (ADR 0019). Re-exported here so all
// existing `crate::features::ecs::FeatureSimEntity` call sites compile
// unchanged.
pub use ambition_platformer_primitives::markers::FeatureSimEntity;

// `HazardFeature` moved to the combat kit with the hazard runtime.
pub use crate::combat::hazard_runtime::HazardFeature;

#[cfg(test)]
mod tests;
