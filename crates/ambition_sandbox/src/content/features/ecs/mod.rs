//! ECS-native feature simulation.
//!
//! Authored and dynamic pickups, chests, breakables, switches, NPCs, enemies,
//! hazards, and bosses are spawned as Bevy entities and updated by the systems
//! in this module. This is the authoritative feature implementation.
//!
//! ## Submodule layout (post-2026-05-19 split)
//!
//! - [`spawn`] — `spawn_room_feature_entities` /
//!   `spawn_room_feature_entity` per-`RoomObjectKind` factory plus
//!   `spawn_encounter_mob` / `despawn_encounter_mobs`.
//! - [`actors`] — `ActorRuntime` enum + `update_ecs_actors` (slot
//!   board, holding-position fallback, attack publication).
//! - Per-family update systems: [`pickups`], [`chests`],
//!   [`breakables`], [`hazards`], [`bosses`].
//! - [`interact`] — buffered-interact resolver (peaceful NPCs,
//!   switches).
//! - [`damage`] — typed slash/projectile/pogo damage application,
//!   breakable shatter side effects, hit predicates.
//! - [`encounter_rewards`] — reward chest spawn/despawn for cleared
//!   mob + boss encounters.
//! - [`falling_chest`] — boss-reward chest gravity tick plus
//!   precomputed "settled at save load" pass.
//! - [`overlay`] — `FeatureEcsWorldOverlay` collision contribution
//!   rebuilt each frame for engine code.
//! - [`view_index`] — `FeatureViewIndex` per-frame read model that
//!   presentation systems consult by id.
//! - [`anim_helpers`] — per-id sprite/anim lookups for the
//!   presentation pipeline.
//! - [`save_sync`] — boss / actor / switch mirror systems run at
//!   room-load.
//! - [`reset`] — `reset_ecs_room_features` same-room sandbox-reset
//!   handler.
//! - [`banner`] — gameplay banner tick + deferred-request applier.

use super::*;
use crate::audio::SfxMessage;
use crate::presentation::fx::{ParticleKind, VfxMessage};
use crate::presentation::rendering::RoomVisual;
use crate::world::physics::{DebrisBurstMessage, PhysicsDebrisCue};
use bevy::prelude::{
    Commands, Component, Entity, MessageReader, MessageWriter, NextState, Query, Res, ResMut,
    Resource, With,
};

use crate::WorldTime;

mod actors;
mod anim_helpers;
mod banner;
mod bosses;
mod breakables;
mod chests;
mod damage;
mod encounter_rewards;
mod falling_chest;
mod hazards;
mod interact;
mod overlay;
mod pickups;
mod reset;
mod save_sync;
mod spawn;
mod targeting;
mod view_index;

pub(crate) use actors::{actor_component_snapshot, sync_actor_components_from_runtime};
pub use actors::{update_ecs_actors, ActorRuntime};
pub use anim_helpers::{
    ecs_boss_anim_state, ecs_boss_name, ecs_breakable_state, ecs_chest_opened,
    ecs_enemy_anim_state, ecs_enemy_name, ecs_enemy_sprite_override, ecs_npc_anim_state,
    ecs_npc_name,
};
pub use banner::{apply_gameplay_banner_requests, tick_gameplay_banner};
pub use bosses::update_ecs_bosses;
pub use breakables::update_ecs_breakables;
pub use chests::open_ecs_chests;
pub use damage::{
    apply_feature_damage_events, ecs_damage_event_hits_actor, ecs_damage_event_hits_boss,
    ecs_damage_event_hits_breakable,
};
pub use encounter_rewards::{
    clear_encounter_reward_ecs, sync_boss_reward_chests_ecs, sync_encounter_reward_chests_ecs,
};
pub use falling_chest::update_ecs_falling_chests;
pub use hazards::update_ecs_hazards;
pub use interact::interact_ecs_actors_and_switches;
pub use overlay::{rebuild_feature_ecs_world_overlay, FeatureEcsWorldOverlay};
pub use pickups::collect_ecs_pickups;
pub use reset::reset_ecs_room_features;
pub use save_sync::{
    sync_ecs_actors_with_save, sync_ecs_bosses_with_save, sync_ecs_switches_from_save,
};
pub use spawn::{despawn_encounter_mobs, spawn_encounter_mob, spawn_room_feature_entities};
pub use targeting::select_actor_targets;
pub use view_index::{rebuild_feature_view_index, FeatureViewIndex};

use damage::{begin_ecs_breakable_respawn, emit_breakable_destroyed};

/// Marker for simulation-side feature entities spawned from the active room.
/// They are deliberately separate from presentation `FeatureVisual` sprites;
/// visible builds keep using the existing visual entities and look up live ECS
/// state by `FeatureId`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FeatureSimEntity;

#[derive(Component, Clone, Debug)]
pub struct HazardFeature {
    pub hazard: HazardRuntime,
    pub spawn: ae::Vec2,
}

impl HazardFeature {
    pub fn new(hazard: HazardRuntime) -> Self {
        let spawn = hazard.pos;
        Self { hazard, spawn }
    }
}

#[derive(Component, Clone, Debug)]
pub struct BossFeature {
    pub boss: BossRuntime,
}

impl BossFeature {
    pub fn new(boss: BossRuntime) -> Self {
        Self { boss }
    }
}

#[cfg(test)]
mod tests;
